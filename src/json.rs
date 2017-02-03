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
use serde_json as json;
use serde_json::value::{Value, ToJson};
use std::io;
use std::io::prelude::*;

/// Writes a complete JSON document representing the `Item`s to the `Write`
///
/// The `Write` is flushed after the JSON document is written.
pub fn write_items<W: Write>(w: W, items: &[Item]) -> io::Result<()> {
    let mut w = w;
    let mut root = json::Map::new();
    // We know for a fact that our implementation of ToJson cannot return an error.
    root.insert("items".to_string(), Value::Array(items.into_iter()
                                                       .map(|x| x.to_json().unwrap())
                                                       .collect()));
    try!(write!(&mut w, "{}", Value::Object(root)));
    w.flush()
}

impl<'a> ToJson for Item<'a> {
    fn to_json(&self) -> json::error::Result<Value> {
        let mut d = json::Map::new();
        d.insert("title".to_string(), json!(self.title));
        if let Some(ref subtitle) = self.subtitle {
            d.insert("subtitle".to_string(), json!(subtitle));
        }
        if let Some(ref icon) = self.icon {
            d.insert("icon".to_string(), icon.to_json()?);
        }
        if let Some(ref uid) = self.uid {
            d.insert("uid".to_string(), json!(uid));
        }
        if let Some(ref arg) = self.arg {
            d.insert("arg".to_string(), json!(arg));
        }
        match self.type_ {
            ItemType::Default => {}
            ItemType::File => {
                d.insert("type".to_string(), json!("file"));
            }
            ItemType::FileSkipCheck => {
                d.insert("type".to_string(), json!("file:skipcheck"));
            }
        }
        if !self.valid {
            d.insert("valid".to_string(), Value::Bool(false));
        }
        if let Some(ref autocomplete) = self.autocomplete {
            d.insert("autocomplete".to_string(), json!(autocomplete));
        }
        if self.text_copy.is_some() || self.text_large_type.is_some() {
            let mut text = json::Map::new();
            if let Some(ref text_copy) = self.text_copy {
                text.insert("copy".to_string(), json!(text_copy));
            }
            if let Some(ref text_large_type) = self.text_large_type {
                text.insert("largetype".to_string(), json!(text_large_type));
            }
            d.insert("text".to_string(), Value::Object(text));
        }
        if let Some(ref url) = self.quicklook_url {
            d.insert("quicklookurl".to_string(), json!(url));
        }
        if !self.modifiers.is_empty() {
            let mut mods = json::Map::new();
            for (modifier, data) in self.modifiers.iter() {
                let key = match *modifier {
                    Modifier::Command => "cmd",
                    Modifier::Option => "alt",
                    Modifier::Control => "ctrl",
                    Modifier::Shift => "shift",
                    Modifier::Fn => "fn"
                }.to_string();
                let mut mod_ = json::Map::new();
                if let Some(ref subtitle) = data.subtitle {
                    mod_.insert("subtitle".to_string(), json!(subtitle));
                }
                if let Some(ref arg) = data.arg {
                    mod_.insert("arg".to_string(), json!(arg));
                }
                if let Some(valid) = data.valid {
                    mod_.insert("valid".to_string(), json!(valid));
                }
                mods.insert(key, Value::Object(mod_));
            }
            d.insert("mods".to_string(), Value::Object(mods));
        }
        Ok(Value::Object(d))
    }
}

impl<'a> ToJson for Icon<'a> {
    fn to_json(&self) -> json::error::Result<Value> {
        Ok(match *self {
            Icon::Path(ref s) => json!({"path": s}),
            Icon::File(ref s) => json!({"type": "fileicon", "path": s}),
            Icon::FileType(ref s) => json!({"type": "filetype", "path": s})
        })
    }
}

#[test]
fn test_to_json() {
    let item1 = Item::new("Item 1");
    assert_eq!(item1.to_json().unwrap(), json!({"title": "Item 1"}));
    let item2 = ::ItemBuilder::new("Item 2")
                               .subtitle("Subtitle")
                               .into_item();
    assert_eq!(item2.to_json().unwrap(),
              json!({
                  "title": "Item 2",
                  "subtitle": "Subtitle"
              }));
    let item3 = ::ItemBuilder::new("Item 3")
                               .arg("Argument")
                               .subtitle("Subtitle")
                               .icon_filetype("public.folder")
                               .into_item();
    assert_eq!(item3.to_json().unwrap(),
               json!({
                   "title": "Item 3",
                   "subtitle": "Subtitle",
                   "arg": "Argument",
                   "icon": { "type": "filetype", "path": "public.folder" }
               }));
}
