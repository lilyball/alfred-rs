//! Helpers for writing Alfred script filter XML output (Alfred 2)
//!
//! Unless you specifically need Alfred 2 compatibility, you should use the `alfred::json` module
//! instead.
//!
//! # Example
//!
//! ```
//! # extern crate alfred;
//! #
//! # use std::io::{self, Write};
//! #
//! # fn write_items() -> io::Result<()> {
//! let mut xmlw = try!(alfred::XMLWriter::new(io::stdout()));
//!
//! let item1 = alfred::Item::new("Item 1");
//! let item2 = alfred::ItemBuilder::new("Item 2")
//!                                 .subtitle("Subtitle")
//!                                 .into_item();
//! let item3 = alfred::ItemBuilder::new("Item 3")
//!                                 .arg("Argument")
//!                                 .subtitle("Subtitle")
//!                                 .icon_filetype("public.folder")
//!                                 .into_item();
//!
//! try!(xmlw.write_item(&item1));
//! try!(xmlw.write_item(&item2));
//! try!(xmlw.write_item(&item3));
//!
//! let mut stdout = try!(xmlw.close());
//! stdout.flush()
//! # }
//! #
//! # fn main() {
//! #     match write_items() {
//! #         Ok(()) => {},
//! #         Err(err) => {
//! #             let _ = writeln!(&mut io::stderr(), "Error writing items: {}", err);
//! #         }
//! #     }
//! # }
//! ```

use std::borrow::Cow;
use std::error;
use std::fmt;
use std::io;
use std::io::prelude::*;
use std::mem;
use std::sync;

use ::{Item, ItemType, Modifier, Icon};

/// Helper struct used to manage the XML serialization of `Item`s.
///
/// When the `XMLWriter` is first created, the XML header is immediately
/// written. When the `XMLWriter` is dropped, the XML footer is written
/// and the `Write` is flushed.
///
/// Any errors produced by writing the footer are silently ignored. The
/// `close()` method can be used to return any such error.
pub struct XMLWriter<W: Write> {
    // Option so close() can remove it
    // Otherwise this must always be Some()
    w: Option<W>,
    last_err: Option<SavedError>
}

// FIXME: If io::Error gains Clone again, go back to just cloning it
enum SavedError {
    Os(i32),
    Custom(SharedError)
}

#[derive(Clone)]
struct SharedError {
    error: sync::Arc<io::Error>
}

impl From<io::Error> for SavedError {
    fn from(err: io::Error) -> SavedError {
        if let Some(code) = err.raw_os_error() {
            SavedError::Os(code)
        } else {
            SavedError::Custom(SharedError { error: sync::Arc::new(err) })
        }
    }
}

impl SavedError {
    fn make_io_error(&self) -> io::Error {
        match *self {
            SavedError::Os(code) => io::Error::from_raw_os_error(code),
            SavedError::Custom(ref err) => {
                let shared_err: SharedError = err.clone();
                io::Error::new(err.error.kind(), shared_err)
            }
        }
    }
}

impl error::Error for SharedError {
    fn description(&self) -> &str {
        self.error.description()
    }

    fn cause(&self) -> Option<&error::Error> {
        Some(&*self.error)
    }
}

impl fmt::Debug for SharedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <io::Error as fmt::Debug>::fmt(&self.error, f)
    }
}

impl fmt::Display for SharedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <io::Error as fmt::Display>::fmt(&self.error, f)
    }
}

impl<W: Write> XMLWriter<W> {
    /// Returns a new `XMLWriter` that writes to the given `Write`.
    ///
    /// The XML header is written immediately.
    pub fn new(mut w: W) -> io::Result<XMLWriter<W>> {
        match w.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<items>\n") {
            Ok(()) => {
                Ok(XMLWriter {
                    w: Some(w),
                    last_err: None
                })
            }
            Err(err) => Err(err)
        }
    }

    /// Writes an `Item` to the underlying `Write`.
    ///
    /// If a previous write produced an error, any subsequent write will do
    /// nothing and return the same error. This is because the previous write
    /// may have partially completed, and attempting to write any more data
    /// will be unlikely to work properly.
    pub fn write_item(&mut self, item: &Item) -> io::Result<()> {
        if let Some(ref err) = self.last_err {
            return Err(err.make_io_error());
        }
        let result = item.write_xml(self.w.as_mut().unwrap(), 1);
        match result {
            Err(err) => {
                let err: SavedError = err.into();
                let io_err = err.make_io_error();
                self.last_err = Some(err);
                Err(io_err)
            }
            x@Ok(_) => x
        }
    }

    /// Consumes the `XMLWriter` and writes the XML footer.
    ///
    /// This method can be used to get any error resulting from writing the
    /// footer. If this method is not used, the footer will be written when the
    /// `XMLWriter` is dropped and any error will be silently ignored.
    ///
    /// As with `write_item()`, if a previous invocation of `write_item()`
    /// returned an error, `close()` will return the same error without
    /// attempting to write the XML footer.
    ///
    /// When this method is used, the XML footer is written, but the `Write`
    /// is not flushed. When the `XMLWriter` is dropped without calling
    /// `close()`, the `Write` is flushed after the footer is written.
    pub fn close(mut self) -> io::Result<W> {
        let last_err = self.last_err.take();
        let mut w = self.w.take().unwrap();
        mem::forget(self);
        if let Some(err) = last_err {
            return Err(err.make_io_error());
        }
        try!(write_footer(&mut w));
        Ok(w)
    }
}

fn write_footer<W: Write>(w: &mut W) -> io::Result<()> {
    w.write_all(b"</items>\n")
}

impl<W: Write> Drop for XMLWriter<W> {
    fn drop(&mut self) {
        if self.last_err.is_some() {
            return;
        }
        let mut w = self.w.take().unwrap();
        if write_footer(&mut w).is_ok() {
            let _ = w.flush();
        }
    }
}

/// Writes a complete XML document representing the `Item`s to the `Write`.
///
/// The `Write` is flushed after the XML document is written.
pub fn write_items<W: Write>(w: W, items: &[Item]) -> io::Result<()> {
    let mut xmlw = try!(XMLWriter::new(w));
    for item in items.iter() {
        try!(xmlw.write_item(item));
    }
    let mut w = try!(xmlw.close());
    w.flush()
}

impl<'a> Item<'a> {
    /// Writes the XML fragment representing the `Item` to the `Write`.
    ///
    /// `XMLWriter` should be used instead if at all possible, in order to
    /// write the XML header/footer and maintain proper error discipline.
    pub fn write_xml(&self, w: &mut Write, indent: u32) -> io::Result<()> {
        fn write_indent(w: &mut Write, indent: u32) -> io::Result<()> {
            for _ in 0..indent {
                try!(w.write_all(b"    "));
            }
            Ok(())
        }

        let mut w = io::BufWriter::with_capacity(512, w);

        try!(write_indent(&mut w, indent));
        try!(w.write_all(b"<item"));
        if let Some(ref uid) = self.uid {
            try!(write!(&mut w, r#" uid="{}""#, encode_entities(uid)));
        }
        if let Some(ref arg) = self.arg {
            try!(write!(&mut w, r#" arg="{}""#, encode_entities(arg)));
        }
        match self.type_ {
            ItemType::Default => {}
            ItemType::File => {
                try!(w.write_all(br#" type="file""#));
            }
            ItemType::FileSkipCheck => {
                try!(w.write_all(br#" type="file:skipcheck""#));
            }
        }
        if !self.valid {
            try!(w.write_all(br#" valid="no""#));
        }
        if let Some(ref auto) = self.autocomplete {
            try!(write!(&mut w, r#" autocomplete="{}""#, encode_entities(auto)));
        }
        try!(w.write_all(b">\n"));

        try!(write_indent(&mut w, indent+1));
        try!(write!(&mut w, "<title>{}</title>\n", encode_entities(&self.title)));

        if let Some(ref subtitle) = self.subtitle {
            try!(write_indent(&mut w, indent+1));
            try!(write!(&mut w, "<subtitle>{}</subtitle>\n", encode_entities(subtitle)));
        }

        if let Some(ref icon) = self.icon {
            try!(write_indent(&mut w, indent+1));
            match *icon {
                Icon::Path(ref s) => {
                    try!(write!(&mut w, "<icon>{}</icon>\n", encode_entities(s)));
                }
                Icon::File(ref s) => {
                    try!(write!(&mut w, "<icon type=\"fileicon\">{}</icon>\n",
                                    encode_entities(s)));
                }
                Icon::FileType(ref s) => {
                    try!(write!(&mut w, "<icon type=\"filetype\">{}</icon>\n",
                                    encode_entities(s)));
                }
            }
        }

        for (modifier, data) in &self.modifiers {
            try!(write_indent(&mut w, indent+1));
            try!(write!(&mut w, r#"<mod key="{}""#, match *modifier {
                Modifier::Command => "cmd",
                Modifier::Option => "alt",
                Modifier::Control => "ctrl",
                Modifier::Shift => "shift",
                Modifier::Fn => "fn"
            }));
            try!(w.write_all(b"<mod"));
            if let Some(ref subtitle) = data.subtitle {
                try!(write!(&mut w, r#" subtitle="{}""#, encode_entities(subtitle)));
            }
            if let Some(ref arg) = data.arg {
                try!(write!(&mut w, r#" arg="{}""#, encode_entities(arg)));
            }
            if let Some(valid) = data.valid {
                try!(write!(&mut w, r#" valid="{}""#, if valid { "yes" } else { "no" }));
            }
            try!(w.write_all(b"/>\n"));
        }

        if let Some(ref text) = self.text_copy {
            try!(write_indent(&mut w, indent+1));
            try!(write!(&mut w, "<text type=\"copy\">{}</text>\n", encode_entities(text)));
        }
        if let Some(ref text) = self.text_large_type {
            try!(write_indent(&mut w, indent+1));
            try!(write!(&mut w, "<text type=\"largetype\">{}</text>\n", encode_entities(text)));
        }

        if let Some(ref url) = self.quicklook_url {
            try!(write_indent(&mut w, indent+1));
            try!(write!(&mut w, "<quicklookurl>{}</quicklookurl>\n", encode_entities(url)));
        }

        try!(write_indent(&mut w, indent));
        try!(w.write_all(b"</item>\n"));

        w.flush()
    }
}

fn encode_entities(s: &str) -> Cow<str> {
    fn encode_entity(c: char) -> Option<&'static str> {
        Some(match c {
            '<' => "&lt;",
            '>' => "&gt;",
            '"' => "&quot;",
            '&' => "&amp;",
            '\0'...'\x08' |
            '\x0B'...'\x0C' |
            '\x0E'...'\x1F' |
            '\u{FFFE}' | '\u{FFFF}' => {
                // these are all invalid characters in XML
                "\u{FFFD}"
            }
            _ => return None
        })
    }

    if s.chars().any(|c| encode_entity(c).is_some()) {
        let mut res = String::with_capacity(s.len());
        for c in s.chars() {
            match encode_entity(c) {
                Some(ent) => res.push_str(ent),
                None => res.push(c)
            }
        }
        Cow::Owned(res)
    } else {
        Cow::Borrowed(s)
    }
}
