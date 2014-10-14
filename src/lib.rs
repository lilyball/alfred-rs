//! Helpers for writing Alfred XML output
//!
//! # Example
//!
//! ```
//! extern crate alfred;
//!
//! use std::io;
//!
//! fn write_items() -> io::IoResult<()> {
//!     let mut xmlw = try!(alfred::XMLWriter::new(io::stdout()));
//!
//!     let item1 = alfred::Item::new("Item 1");
//!     let item2 = alfred::Item {
//!         subtitle: Some("Subtitle".into_maybe_owned()),
//!         ..alfred::Item::new("Item 2")
//!     };
//!     let item3 = alfred::Item {
//!         arg: Some("Argument".into_maybe_owned()),
//!         subtitle: Some("Subtitle".into_maybe_owned()),
//!         icon: Some(alfred::FileType("public.folder".into_maybe_owned())),
//!         ..alfred::Item::new("Item 3")
//!     };
//!
//!     try!(xmlw.write_item(&item1));
//!     try!(xmlw.write_item(&item2));
//!     try!(xmlw.write_item(&item3));
//!
//!     xmlw.close()
//! }
//!
//! fn main() {
//!     match write_items() {
//!         Ok(()) => {},
//!         Err(err) => {
//!             let _ = writeln!(io::stderr(), "Error writing items: {}", err);
//!         }
//!     }
//! }
//! ```

#![feature(if_let)]
#![warn(missing_doc)]

use std::io;
use std::io::BufferedWriter;
use std::str;

/// Representation of an `<item>`
#[deriving(PartialEq,Eq,Clone)]
pub struct Item<'a> {
    /// Identifier for the results. If given, must be unique among items, and is used for
    /// prioritizing feedback results based on usage. If blank, Alfred uses a UUID and does
    /// not learn from the results.
    pub uid: Option<str::MaybeOwned<'a>>,
    /// The value that is passed to the next portion of the workflow when this item
    /// is selected.
    pub arg: Option<str::MaybeOwned<'a>>,
    /// What type of result this is, if any.
    pub type_: Option<ItemType>,
    /// Whether or not the result item is 'valid'. If `false`, `autocomplete` may be used.
    pub valid: bool,
    /// Autocomplete data for valid=false items. When this item is selected, the autocomplete
    /// value is inserted into the Alfred window.
    pub autocomplete: Option<str::MaybeOwned<'a>>,

    /// Title for the item
    pub title: str::MaybeOwned<'a>,
    /// Subtitle for the item
    pub subtitle: Option<str::MaybeOwned<'a>>,
    /// Icon for the item
    pub icon: Option<Icon<'a>>
}

impl<'a> Item<'a> {
    /// Returns a new Item with the given title
    pub fn new<S: str::IntoMaybeOwned<'a>>(title: S) -> Item<'a> {
        Item {
            uid: None,
            arg: None,
            type_: None,
            valid: true,
            autocomplete: None,
            title: title.into_maybe_owned(),
            subtitle: None,
            icon: None
        }
    }
}

/// Item icons
#[deriving(PartialEq,Eq,Clone)]
pub enum Icon<'a> {
    /// Path to an image file on disk relative to the workflow directory
    PathIcon(str::MaybeOwned<'a>),
    /// Path to a file whose icon will be used
    FileIcon(str::MaybeOwned<'a>),
    /// UTI for a file type to use (e.g. public.folder)
    FileType(str::MaybeOwned<'a>)
}

/// Item types
#[deriving(PartialEq,Eq,Clone)]
pub enum ItemType {
    /// Type representing a file
    FileItemType
}

/// Helper struct used to manage the XML serialization of `Item`s
///
/// When the `XMLWriter` is first created, the XML header is immediately
/// written. When the `XMLWriter` is dropped, the XML footer is written.
///
/// Any errors produced by writing the footer are silently ignored. The
/// `close()` method can be used to return any such error.
pub struct XMLWriter<'a, W: Writer + 'a> {
    w: W,
    last_err: Option<io::IoError>
}

impl<'a, W: Writer + 'a> XMLWriter<'a, W> {
    /// Returns a new `XMLWriter` that writes to the given `Writer`
    ///
    /// The XML header is written immediately.
    pub fn new(mut w: W) -> io::IoResult<XMLWriter<'a, W>> {
        match w.write_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<items>\n") {
            Ok(()) => {
                Ok(XMLWriter {
                    w: w,
                    last_err: None
                })
            }
            Err(err) => Err(err)
        }
    }

    /// Writes an `Item` to the underlying `Writer`
    ///
    /// If a previous write produced an error, any subsequent write will do
    /// nothing and return the same error. This is because the previous write
    /// may have partially completed, and attempting to write any more data
    /// will be unlikely to work properly.
    pub fn write_item(&mut self, item: &Item) -> io::IoResult<()> {
        if let Some(ref err) = self.last_err {
            return Err(err.clone());
        }
        let result = item.write_xml(&mut self.w, 1);
        if let Err(ref err) = result {
            self.last_err = Some(err.clone());
        }
        result
    }

    /// Consumes the `XMLWriter` and writes the XML footer
    ///
    /// This method can be used to get any error resulting from writing the
    /// footer. If this method is not used, the footer will be written when the
    /// `XMLWriter` is dropped and any error will be silently ignored.
    ///
    /// As with `write_item()`, if a previous invocation of `write_item()`
    /// returned an error, `close()` will return the same error without
    /// attempting to write the XML footer.
    pub fn close(mut self) -> io::IoResult<()> {
        if let Some(err) = self.last_err {
            return Err(err);
        }
        self.w.write_str("</items>\n")
    }
}

/// Writes a complete XML document representing the `Item`s to the `Writer`
pub fn write_items<W: Writer>(w: W, items: &[Item]) -> io::IoResult<()> {
    let mut xmlw = try!(XMLWriter::new(w));
    for item in items.iter() {
        try!(xmlw.write_item(item));
    }
    xmlw.close()
}

impl<'a> Item<'a> {
    /// Writes the XML fragment representing the `Item` to the `Writer`
    ///
    /// `XMLWriter` should be used instead if at all possible, in order to
    /// write the XML header/footer and maintain proper error discipline.
    pub fn write_xml(&self, w: &mut io::Writer, indent: uint) -> io::IoResult<()> {
        fn write_indent(w: &mut io::Writer, indent: uint) -> io::IoResult<()> {
            for _ in range(0, indent) {
                try!(w.write_str("    "));
            }
            Ok(())
        }

        let mut w = BufferedWriter::with_capacity(512, w);

        try!(write_indent(&mut w, indent));
        try!(w.write_str("<item"));
        match self.uid {
            None => (),
            Some(ref uid) => {
                try!(write!(&mut w, r#" uid="{}""#, encode_entities(uid.as_slice())));
            }
        }
        match self.arg {
            None => (),
            Some(ref arg) => {
                try!(write!(&mut w, r#" arg="{}""#, encode_entities(arg.as_slice())));
            }
        }
        match self.type_ {
            None => (),
            Some(FileItemType) => {
                try!(w.write_str(r#" type="file""#));
            }
        }
        try!(write!(&mut w, r#" valid="{}""#, if self.valid { "yes" } else { "no" }));
        match self.autocomplete {
            None => (),
            Some(ref auto) => {
                try!(write!(&mut w, r#" autocomplete="{}""#, encode_entities(auto.as_slice())));
            }
        }
        try!(w.write_str(">\n"));

        try!(write_indent(&mut w, indent+1));
        try!(write!(&mut w, "<title>{}</title>\n", encode_entities(self.title.as_slice())));

        match self.subtitle {
            None => (),
            Some(ref s) => {
                try!(write_indent(&mut w, indent+1));
                try!(write!(&mut w, "<subtitle>{}</subtitle>\n", encode_entities(s.as_slice())));
            }
        }

        match self.icon {
            None => (),
            Some(ref icon) => {
                try!(write_indent(&mut w, indent+1));
                match *icon {
                    PathIcon(ref s) => {
                        try!(write!(&mut w, "<icon>{}</icon>\n", encode_entities(s.as_slice())));
                    }
                    FileIcon(ref s) => {
                        try!(write!(&mut w, "<icon type=\"fileicon\">{}</icon>\n",
                                      encode_entities(s.as_slice())));
                    }
                    FileType(ref s) => {
                        try!(write!(&mut w, "<icon type=\"filetype\">{}</icon>\n",
                                      encode_entities(s.as_slice())));
                    }
                }
            }
        }

        try!(write_indent(&mut w, indent));
        try!(w.write_str("</item>\n"));

        try!(w.flush());
        Ok(())
    }
}

fn encode_entities<'a>(s: &'a str) -> str::MaybeOwned<'a> {
    fn encode_entity(c: char) -> Option<&'static str> {
        Some(match c {
            '<' => "&lt;",
            '>' => "&gt;",
            '"' => "&quot;",
            '&' => "&amp;",
            '\0'...'\x08' |
            '\x0B'...'\x0C' |
            '\x0E'...'\x1F' |
            '\uFFFE' | '\uFFFF' => {
                // these are all invalid characters in XML
                "\uFFFD"
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
        str::Owned(res)
    } else {
        str::Slice(s)
    }
}
