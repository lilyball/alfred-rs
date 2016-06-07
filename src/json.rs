//! Helpers for writing Alfred script filter JSON output (Alfred 3)
//!
//! # Example
//!
//! ### JSON output (Alfred 3)
//!
//! ```
//! extern crate alfred;
//!
//! use std::io::{self, Write};
//!
//! fn write_items() -> io::Result<()> {
//!     alfred::json::write_items(io::stdout(), &[
//!         alfred::Item::new("Item 1"),
//!         alfred::ItemBuilder::new("Item 2")
//!                             .subtitle("Subtitle")
//!                             .into_item(),
//!         alfred::ItemBuilder::new("Item 3")
//!                             .arg("Argument")
//!                             .subtitle("Subtitle")
//!                             .icon_filetype("public.folder")
//!                             .into_item()
//!     ])
//! }
//!
//! fn main() {
//!     match write_items() {
//!         Ok(()) => {},
//!         Err(err) => {
//!             let _ = writeln!(&mut io::stderr(), "Error writing items: {}", err);
//!         }
//!     }
//! }
//! ```

use ::{Item, ItemType, Modifier, Icon};
use rustc_serialize::json::{Json, ToJson};
use std::collections::BTreeMap;
use std::io;
use std::io::prelude::*;

/// Writes a complete JSON document representing the `Item`s to the `Write`
///
/// The `Write` is flushed after the JSON document is written.
pub fn write_items<W: Write>(w: W, items: &[Item]) -> io::Result<()> {
    let mut w = w;
    let mut root = BTreeMap::new();
    root.insert("items".to_string(), Json::Array(items.into_iter().map(ToJson::to_json).collect()));
    try!(write!(&mut w, "{}", Json::Object(root)));
    w.flush()
}

impl<'a> ToJson for Item<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("title".to_string(), self.title.to_json());
        if let Some(ref subtitle) = self.subtitle {
            d.insert("subtitle".to_string(), subtitle.to_json());
        }
        if let Some(ref icon) = self.icon {
            d.insert("icon".to_string(), icon.to_json());
        }
        if let Some(ref uid) = self.uid {
            d.insert("uid".to_string(), uid.to_json());
        }
        if let Some(ref arg) = self.arg {
            d.insert("arg".to_string(), arg.to_json());
        }
        match self.type_ {
            ItemType::Default => {}
            ItemType::File => {
                d.insert("type".to_string(), "file".to_json());
            }
            ItemType::FileSkipCheck => {
                d.insert("type".to_string(), "file:skipcheck".to_json());
            }
        }
        if !self.valid {
            d.insert("valid".to_string(), false.to_json());
        }
        if let Some(ref autocomplete) = self.autocomplete {
            d.insert("autocomplete".to_string(), autocomplete.to_json());
        }
        if self.text_copy.is_some() || self.text_large_type.is_some() {
            let mut text = BTreeMap::new();
            if let Some(ref text_copy) = self.text_copy {
                text.insert("copy".to_string(), text_copy.to_json());
            }
            if let Some(ref text_large_type) = self.text_large_type {
                text.insert("largetype".to_string(), text_large_type.to_json());
            }
            d.insert("text".to_string(), text.to_json());
        }
        if let Some(ref url) = self.quicklook_url {
            d.insert("quicklookurl".to_string(), url.to_json());
        }
        if !self.modifiers.is_empty() {
            let mut mods = BTreeMap::new();
            for (modifier, data) in self.modifiers.iter() {
                let key = match *modifier {
                    Modifier::Command => "cmd",
                    Modifier::Option => "alt",
                    Modifier::Control => "ctrl",
                    Modifier::Shift => "shift",
                    Modifier::Fn => "fn"
                }.to_string();
                let mut mod_ = BTreeMap::new();
                if let Some(ref subtitle) = data.subtitle {
                    mod_.insert("subtitle".to_string(), subtitle.to_json());
                }
                if let Some(ref arg) = data.arg {
                    mod_.insert("arg".to_string(), arg.to_json());
                }
                if let Some(valid) = data.valid {
                    mod_.insert("valid".to_string(), valid.to_json());
                }
                mods.insert(key, mod_.to_json());
            }
            d.insert("mods".to_string(), mods.to_json());
        }
        Json::Object(d)
    }
}

impl<'a> ToJson for Icon<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        match *self {
            Icon::Path(ref s) => {
                d.insert("path".to_string(), s.to_json());
            }
            Icon::File(ref s) => {
                d.insert("type".to_string(), "fileicon".to_json());
                d.insert("path".to_string(), s.to_json());
            }
            Icon::FileType(ref s) => {
                d.insert("type".to_string(), "filetype".to_json());
                d.insert("path".to_string(), s.to_json());
            }
        }
        d.to_json()
    }
}
