//! Helpers for writing Alfred script filter JSON output (Alfred 3)
//!
//! # Examples
//!
//! ### Writing items
//!
//! ```
//! # extern crate alfred;
//! # use std::io::{self, Write};
//! #
//! # fn write_items() -> io::Result<()> {
//! alfred::json::write_items(io::stdout(), &[
//!     alfred::Item::new("Item 1"),
//!     alfred::ItemBuilder::new("Item 2")
//!                         .subtitle("Subtitle")
//!                         .into_item(),
//!     alfred::ItemBuilder::new("Item 3")
//!                         .arg("Argument")
//!                         .subtitle("Subtitle")
//!                         .icon_filetype("public.folder")
//!                         .into_item()
//! ])
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
//!
//! ### Writing items with variables
//!
//! ```
//! # extern crate alfred;
//! # use std::io::{self, Write};
//! #
//! # fn write_items() -> io::Result<()> {
//! alfred::json::Builder::with_items(&[
//!     alfred::Item::new("Item 1"),
//!     alfred::ItemBuilder::new("Item 2")
//!                         .subtitle("Subtitle")
//!                         .into_item(),
//!     alfred::ItemBuilder::new("Item 3")
//!                         .arg("Argument")
//!                         .subtitle("Subtitle")
//!                         .icon_filetype("public.folder")
//!                         .into_item()
//! ]).variable("fruit", "banana")
//!   .variable("vegetable", "carrot")
//!   .write(io::stdout())
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

use ::{Item, ItemType, Modifier, Icon};
use serde_json as json;
use serde_json::value::Value;
use std::collections::HashMap;
use std::io;
use std::io::prelude::*;

/// Writes a complete JSON document representing the `Item`s to the `Write`.
///
/// The `Write` is flushed after the JSON document is written.
pub fn write_items<W: Write>(w: W, items: &[Item]) -> io::Result<()> {
    Builder::with_items(items).write(w)
}

/// A helper type for writing out items with top-level variables.
///
/// Note: If you don't need top-level variables the `write_items()` function is easier to use.
pub struct Builder<'a> {
    /// The items that will be written out.
    pub items: &'a [Item<'a>],
    /// The variables that will be written out.
    pub variables: HashMap<&'a str, &'a str>
}

impl<'a> Builder<'a> {
    /// Returns a new `Builder` with no items.
    pub fn new() -> Builder<'a> {
        Builder { items: &[], variables: HashMap::new() }
    }

    /// Returns a new `Builder` with the given items.
    pub fn with_items(items: &'a [Item]) -> Builder<'a> {
        Builder { items, variables: HashMap::new() }
    }

    /// Writes a complete JSON document representing the items and variables to the `Write`.
    ///
    /// The `Write` is flushed after the JSON document is written.
    pub fn write<W: Write>(self, mut w: W) -> io::Result<()> {
        write!(&mut w, "{}", self.into_json())?;
        w.flush()
    }

    fn into_json(self) -> Value {
        let mut root = json::Map::new();
        // We know for a fact that our implementation of ToJson cannot return an error.
        root.insert("items".to_string(), Value::Array(self.items.into_iter()
                                                                .map(|x| x.into_json())
                                                                .collect()));
        let mut iter = self.variables.into_iter();
        if let Some(first) = iter.next() {
            let mut vars = json::Map::new();
            vars.insert(first.0.into(), Value::String(first.1.into()));
            for elt in iter {
                vars.insert(elt.0.into(), Value::String(elt.1.into()));
            }
            root.insert("variables".to_owned(), Value::Object(vars));
        }
        Value::Object(root)
    }

    /// Replaces the builder's items with `items`.
    pub fn items(mut self, items: &'a [Item]) -> Builder<'a> {
        self.set_items(items);
        self
    }

    /// Replaces the builder's variables with `variables`.
    pub fn variables(mut self, variables: HashMap<&'a str, &'a str>) -> Builder<'a> {
        self.set_variables(variables);
        self
    }

    /// Inserts a new variable into the builder's variables.
    pub fn variable(mut self, key: &'a str, value: &'a str) -> Builder<'a> {
        self.set_variable(key, value);
        self
    }

    /// Replaces the builder's items with `items`.
    pub fn set_items(&mut self, items: &'a [Item]) {
        self.items = items
    }

    /// Replaces the builder's variables with `variables`.
    pub fn set_variables(&mut self, variables: HashMap<&'a str, &'a str>) {
        self.variables = variables
    }

    /// Inserts a new variable into the builder's variables.
    pub fn set_variable(&mut self, key: &'a str, value: &'a str) {
        self.variables.insert(key, value);
    }
}

impl<'a> Item<'a> {
    fn into_json(&self) -> Value {
        let mut d = json::Map::new();
        d.insert("title".to_string(), json!(self.title));
        if let Some(ref subtitle) = self.subtitle {
            d.insert("subtitle".to_string(), json!(subtitle));
        }
        if let Some(ref icon) = self.icon {
            d.insert("icon".to_string(), icon.into_json());
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
            for (modifier, data) in &self.modifiers {
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
                if let Some(ref icon) = data.icon {
                    mod_.insert("icon".to_string(), icon.into_json());
                }
                mods.insert(key, Value::Object(mod_));
            }
            d.insert("mods".to_string(), Value::Object(mods));
        }
        Value::Object(d)
    }
}

impl<'a> Icon<'a> {
    fn into_json(&self) -> Value {
        match *self {
            Icon::Path(ref s) => json!({"path": s}),
            Icon::File(ref s) => json!({"type": "fileicon", "path": s}),
            Icon::FileType(ref s) => json!({"type": "filetype", "path": s})
        }
    }
}

#[test]
fn test_into_json() {
    let item = Item::new("Item 1");
    assert_eq!(item.into_json(), json!({"title": "Item 1"}));
    let item = ::ItemBuilder::new("Item 2")
                              .subtitle("Subtitle")
                              .into_item();
    assert_eq!(item.into_json(),
              json!({
                  "title": "Item 2",
                  "subtitle": "Subtitle"
              }));
    let item = ::ItemBuilder::new("Item 3")
                              .arg("Argument")
                              .subtitle("Subtitle")
                              .icon_filetype("public.folder")
                              .into_item();
    assert_eq!(item.into_json(),
               json!({
                   "title": "Item 3",
                   "subtitle": "Subtitle",
                   "arg": "Argument",
                   "icon": { "type": "filetype", "path": "public.folder" }
               }));
    let item = ::ItemBuilder::new("Item 4")
                              .arg("Argument")
                              .subtitle("Subtitle")
                              .arg_mod(Modifier::Option, "Alt Argument")
                              .valid_mod(Modifier::Option, false)
                              .icon_file_mod(Modifier::Option, "opt.png")
                              .arg_mod(Modifier::Control, "Ctrl Argument")
                              .subtitle_mod(Modifier::Control, "Ctrl Subtitle")
                              .icon_path_mod(Modifier::Control, "ctrl.png")
                              .arg_mod(Modifier::Shift, "Shift Argument")
                              .into_item();
    assert_eq!(item.into_json(),
               json!({
                   "title": "Item 4",
                   "subtitle": "Subtitle",
                   "arg": "Argument",
                   "mods": {
                       "alt": {
                            "arg": "Alt Argument",
                            "valid": false,
                            "icon": { "type": "fileicon", "path": "opt.png" }
                       },
                       "ctrl": {
                           "arg": "Ctrl Argument",
                           "subtitle": "Ctrl Subtitle",
                           "icon": { "path": "ctrl.png" }
                       },
                       "shift": {
                           "arg": "Shift Argument"
                       }
                   }
               }));
}

#[test]
fn test_builder() {
    let json = Builder::with_items(&[
        Item::new("Item 1"),
        ::ItemBuilder::new("Item 2")
                      .subtitle("Subtitle")
                      .into_item(),
        ::ItemBuilder::new("Item 3")
                      .arg("Argument")
                      .subtitle("Subtitle")
                      .icon_filetype("public.folder")
                      .into_item()
    ]).variable("fruit", "banana")
      .variable("vegetable", "carrot")
      .into_json();
    assert_eq!(json,
               json!({
                   "items": [
                       {
                           "title": "Item 1"
                       },
                       {
                           "title": "Item 2",
                           "subtitle": "Subtitle"
                       },
                       {
                           "title": "Item 3",
                           "arg": "Argument",
                           "subtitle": "Subtitle",
                           "icon": { "type": "filetype", "path": "public.folder" }
                       }
                   ],
                   "variables": {
                       "fruit": "banana",
                       "vegetable": "carrot"
                   }
               }));
}
