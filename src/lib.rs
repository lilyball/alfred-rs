// Copyright (c) 2015 Kevin Ballard.
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Helpers for writing Alfred script filter output
//!
//! # Examples
//!
//! ### JSON output (Alfred 3)
//!
//! ```
//! # extern crate alfred;
//! #
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
//! ### JSON output with variables (Alfred 3)
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
//!
//! ### XML output (Alfred 2)
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

#![warn(missing_docs)]

#![doc(html_root_url = "https://docs.rs/alfred/3.0.3")]

#[macro_use]
extern crate serde_json;

pub mod json;
pub mod xml;
pub mod env;

use std::borrow::Cow;
use std::collections::HashMap;

pub use self::xml::XMLWriter;

/// Representation of a script filter item.
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Item<'a> {
    /// Title for the item.
    pub title: Cow<'a, str>,
    /// Subtitle for the item.
    ///
    /// The subtitle may be overridden by modifiers.
    pub subtitle: Option<Cow<'a, str>>,
    /// Icon for the item
    pub icon: Option<Icon<'a>>,

    /// Identifier for the results.
    ///
    /// If given, must be unique among items, and is used for prioritizing
    /// feedback results based on usage. If blank, Alfred presents results in
    /// the order given and does not learn from them.
    pub uid: Option<Cow<'a, str>>,
    /// The value that is passed to the next portion of the workflow when this
    /// item is selected.
    ///
    /// The arg may be overridden by modifiers.
    pub arg: Option<Cow<'a, str>>,
    /// What type of result this is.
    pub type_: ItemType,

    /// Whether or not the result is valid.
    ///
    /// When `false`, actioning the result will populate the search field with
    /// the `autocomplete` value instead.
    ///
    /// The validity may be overridden by modifiers.
    pub valid: bool,
    /// Autocomplete data for the item.
    ///
    /// This value is populated into the search field if the tab key is
    /// pressed. If `valid = false`, this value is populated if the item is
    /// actioned.
    pub autocomplete: Option<Cow<'a, str>>,
    /// What text the user gets when copying the result.
    ///
    /// This value is copied if the user presses ⌘C.
    pub text_copy: Option<Cow<'a, str>>,
    /// What text the user gets when displaying large type.
    ///
    /// This value is displayed if the user presses ⌘L.
    pub text_large_type: Option<Cow<'a, str>>,
    /// A URL to use for Quick Look.
    pub quicklook_url: Option<Cow<'a, str>>,

    /// Optional overrides of subtitle, arg, and valid by modifiers.
    pub modifiers: HashMap<Modifier, ModifierData<'a>>,

    /// Disallow struct literals for `Item`.
    _priv: ()
}

impl<'a> Item<'a> {
    /// Returns a new `Item` with the given title.
    pub fn new<S: Into<Cow<'a, str>>>(title: S) -> Item<'a> {
        Item {
            title: title.into(),
            subtitle: None,
            icon: None,
            uid: None,
            arg: None,
            type_: ItemType::Default,
            valid: true,
            autocomplete: None,
            text_copy: None,
            text_large_type: None,
            quicklook_url: None,
            modifiers: HashMap::new(),
            _priv: ()
        }
    }
}

/// Helper for building `Item` values.
#[derive(Clone,Debug)]
pub struct ItemBuilder<'a> {
    item: Item<'a>
}

impl<'a> ItemBuilder<'a> {
    /// Returns a new `ItemBuilder` with the given title.
    pub fn new<S: Into<Cow<'a, str>>>(title: S) -> ItemBuilder<'a> {
        ItemBuilder {
            item: Item::new(title)
        }
    }

    /// Returns the built `Item`.
    pub fn into_item(self) -> Item<'a> {
        self.item
    }
}

impl<'a> ItemBuilder<'a> {
    /// Sets the `title` to the given value.
    pub fn title<S: Into<Cow<'a, str>>>(mut self, title: S) -> ItemBuilder<'a> {
        self.set_title(title);
        self
    }

    /// Sets the default `subtitle` to the given value.
    ///
    /// This sets the default subtitle, used when no modifier is pressed,
    /// or when no subtitle is provided for the pressed modifier.
    pub fn subtitle<S: Into<Cow<'a, str>>>(mut self, subtitle: S) -> ItemBuilder<'a> {
        self.set_subtitle(subtitle);
        self
    }

    /// Sets the `subtitle` to the given value with the given modifier.
    ///
    /// This sets the subtitle to use when the given modifier is pressed.
    pub fn subtitle_mod<S: Into<Cow<'a, str>>>(mut self, modifier: Modifier, subtitle: S)
                                              -> ItemBuilder<'a> {
        self.set_subtitle_mod(modifier, subtitle);
        self
    }

    /// Sets the `icon` to an image file on disk.
    ///
    /// The path is interpreted relative to the workflow directory.
    pub fn icon_path<S: Into<Cow<'a, str>>>(mut self, path: S) -> ItemBuilder<'a> {
        self.set_icon_path(path);
        self
    }

    /// Sets the `icon` to the icon for a given file on disk.
    ///
    /// The path is interpreted relative to the workflow directory.
    pub fn icon_file<S: Into<Cow<'a, str>>>(mut self, path: S) -> ItemBuilder<'a> {
        self.set_icon_file(path);
        self
    }

    /// Sets the `icon` to the icon for a given file type.
    ///
    /// The type is a UTI, such as "public.jpeg".
    pub fn icon_filetype<S: Into<Cow<'a, str>>>(mut self, filetype: S) -> ItemBuilder<'a> {
        self.set_icon_filetype(filetype);
        self
    }

    /// Sets the `icon` to an image file on disk for the given modifier.
    ///
    /// The path is interpreted relative to the workflow directory.
    ///
    /// This property is only used with JSON output. The legacy XML output does not include
    /// per-modifier icons.
    ///
    /// This property is only used with Alfred 3.4.1 or later.
    pub fn icon_path_mod<S: Into<Cow<'a, str>>>(mut self, modifier: Modifier, path: S)
                                               -> ItemBuilder<'a> {
        self.set_icon_path_mod(modifier, path);
        self
    }

    /// Sets the `icon` to the icon for a given file on disk for the given modifier.
    ///
    /// The path is interpreted relative to the workflow directory.
    ///
    /// This property is only used with JSON output. The legacy XML output does not include
    /// per-modifier icons.
    ///
    /// This property is only used with Alfred 3.4.1 or later.
    pub fn icon_file_mod<S: Into<Cow<'a, str>>>(mut self, modifier: Modifier, path: S)
                                               -> ItemBuilder<'a> {
        self.set_icon_file_mod(modifier, path);
        self
    }

    /// Sets the `icon` to the icon for a given file type for the given modifier.
    ///
    /// The type is a UTI, such as "public.jpeg".
    ///
    /// This property is only used with JSON output. The legacy XML output does not include
    /// per-modifier icons.
    ///
    /// This property is only used with Alfred 3.4.1 or later.
    pub fn icon_filetype_mod<S: Into<Cow<'a, str>>>(mut self, modifier: Modifier, filetype: S)
                                                   -> ItemBuilder<'a> {
        self.set_icon_filetype_mod(modifier, filetype);
        self
    }

    /// Sets the `uid` to the given value.
    pub fn uid<S: Into<Cow<'a, str>>>(mut self, uid: S) -> ItemBuilder<'a> {
        self.set_uid(uid);
        self
    }

    /// Sets the `arg` to the given value.
    pub fn arg<S: Into<Cow<'a, str>>>(mut self, arg: S) -> ItemBuilder<'a> {
        self.set_arg(arg);
        self
    }

    /// Sets the `arg` to the given value with the given modifier.
    ///
    /// This sets the arg to use when the given modifier is pressed.
    pub fn arg_mod<S: Into<Cow<'a, str>>>(mut self, modifier: Modifier, arg: S)
                                         -> ItemBuilder<'a> {
        self.set_arg_mod(modifier, arg);
        self
    }

    /// Sets the `type` to the given value.
    pub fn type_(mut self, type_: ItemType) -> ItemBuilder<'a> {
        self.set_type(type_);
        self
    }

    /// Sets `valid` to the given value.
    pub fn valid(mut self, valid: bool) -> ItemBuilder<'a> {
        self.set_valid(valid);
        self
    }

    /// Sets `valid` to the given value with the given modifier.
    ///
    /// This sets the validity to use when the given modifier is pressed.
    pub fn valid_mod(mut self, modifier: Modifier, valid: bool) -> ItemBuilder<'a> {
        self.set_valid_mod(modifier, valid);
        self
    }

    /// Sets the subtitle, arg, validity, and icon to use with the given modifier.
    pub fn modifier<S: Into<Cow<'a, str>>, S2: Into<Cow<'a, str>>>(mut self,
                                                                   modifier: Modifier,
                                                                   subtitle: Option<S>,
                                                                   arg: Option<S2>,
                                                                   valid: bool,
                                                                   icon: Option<Icon<'a>>)
                                                                  -> ItemBuilder<'a> {
        self.set_modifier(modifier, subtitle, arg, valid, icon);
        self
    }

    /// Sets `autocomplete` to the given value.
    pub fn autocomplete<S: Into<Cow<'a, str>>>(mut self, autocomplete: S) -> ItemBuilder<'a> {
        self.set_autocomplete(autocomplete);
        self
    }

    /// Sets `text_copy` to the given value.
    pub fn text_copy<S: Into<Cow<'a, str>>>(mut self, text: S) -> ItemBuilder<'a> {
        self.set_text_copy(text);
        self
    }

    /// Sets `text_large_type` to the given value.
    pub fn text_large_type<S: Into<Cow<'a, str>>>(mut self, text: S) -> ItemBuilder<'a> {
        self.set_text_large_type(text);
        self
    }

    /// Sets `quicklook_url` to the given value.
    pub fn quicklook_url<S: Into<Cow<'a, str>>>(mut self, url: S) -> ItemBuilder<'a> {
        self.set_quicklook_url(url);
        self
    }
}

impl<'a> ItemBuilder<'a> {
    /// Sets the `title` to the given value.
    pub fn set_title<S: Into<Cow<'a, str>>>(&mut self, title: S) {
        self.item.title = title.into();
    }

    /// Sets the default `subtitle` to the given value.
    pub fn set_subtitle<S: Into<Cow<'a, str>>>(&mut self, subtitle: S) {
        self.item.subtitle = Some(subtitle.into());
    }

    /// Unsets the default `subtitle`.
    pub fn unset_subtitle(&mut self) {
        self.item.subtitle = None;
    }

    /// Sets the `subtitle` to the given value for the given modifier.
    pub fn set_subtitle_mod<S: Into<Cow<'a, str>>>(&mut self, modifier: Modifier, subtitle: S) {
        self.data_for_modifier(modifier).subtitle = Some(subtitle.into());
    }

    /// Unsets the `subtitle` for the given modifier.
    ///
    /// This unsets the subtitle that's used when the given modifier is pressed.
    pub fn unset_subtitle_mod(&mut self, modifier: Modifier) {
        use std::collections::hash_map::Entry;
        if let Entry::Occupied(mut entry) = self.item.modifiers.entry(modifier) {
            entry.get_mut().subtitle = None;
            if entry.get().is_empty() {
                entry.remove();
            }
        }
    }

    /// Clears the `subtitle` for all modifiers.
    ///
    /// This unsets both the default subtitle and the per-modifier subtitles.
    pub fn clear_subtitle(&mut self) {
        self.item.subtitle = None;
        for &modifier in ALL_MODIFIERS {
            self.unset_subtitle_mod(modifier);
        }
    }

    /// Sets the `icon` to an image file on disk.
    ///
    /// The path is interpreted relative to the workflow directory.
    pub fn set_icon_path<S: Into<Cow<'a, str>>>(&mut self, path: S) {
        self.item.icon = Some(Icon::Path(path.into()));
    }

    /// Sets the `icon` to the icon for a given file on disk.
    ///
    /// The path is interpreted relative to the workflow directory.
    pub fn set_icon_file<S: Into<Cow<'a, str>>>(&mut self, path: S) {
        self.item.icon = Some(Icon::File(path.into()));
    }

    /// Sets the `icon` to the icon for a given file type.
    ///
    /// The type is a UTI, such as "public.jpeg".
    pub fn set_icon_filetype<S: Into<Cow<'a, str>>>(&mut self, filetype: S) {
        self.item.icon = Some(Icon::FileType(filetype.into()));
    }

    /// Unsets the `icon`.
    pub fn unset_icon(&mut self) {
        self.item.icon = None;
    }

    /// Sets `icon` to an image file on disk for the given modifier.
    ///
    /// The path is interpreted relative to the workflow directory.
    ///
    /// This property is only used with JSON output. The legacy XML output does not include
    /// per-modifier icons.
    ///
    /// This property is only used with Alfred 3.4.1 or later.
    pub fn set_icon_path_mod<S: Into<Cow<'a, str>>>(&mut self, modifier: Modifier, path: S) {
        self.data_for_modifier(modifier).icon = Some(Icon::Path(path.into()));
    }

    /// Sets `icon` to the icon for a given file on disk for the given modifier.
    ///
    /// The path is interpreted relative to the workflow directory.
    ///
    /// This property is only used with JSON output. The legacy XML output does not include
    /// per-modifier icons.
    ///
    /// This property is only used with Alfred 3.4.1 or later.
    pub fn set_icon_file_mod<S: Into<Cow<'a, str>>>(&mut self, modifier: Modifier, path: S) {
        self.data_for_modifier(modifier).icon = Some(Icon::File(path.into()));
    }

    /// Sets `icon` to the icon for a given file type for the given modifier.
    ///
    /// The type is a UTI, such as "public.jpeg".
    ///
    /// This property is only used with JSON output. The legacy XML output does not include
    /// per-modifier icons.
    ///
    /// This property is only used with Alfred 3.4.1 or later.
    pub fn set_icon_filetype_mod<S: Into<Cow<'a, str>>>(&mut self, modifier: Modifier,
                                                        filetype: S) {
        self.data_for_modifier(modifier).icon = Some(Icon::FileType(filetype.into()));
    }

    /// Unsets `icon` for the given modifier.
    ///
    /// This unsets the result icon that's used when the given modifier is pressed.
    pub fn unset_icon_mod(&mut self, modifier: Modifier) {
        use std::collections::hash_map::Entry;
        if let Entry::Occupied(mut entry) = self.item.modifiers.entry(modifier) {
            entry.get_mut().icon = None;
            if entry.get().is_empty() {
                entry.remove();
            }
        }
    }

    /// Clears the `icon` for all modifiers.
    ///
    /// This unsets both the default icon and the per-modifier icons.
    pub fn clear_icon(&mut self) {
        self.item.icon = None;
        for &modifier in ALL_MODIFIERS {
            self.unset_icon_mod(modifier);
        }
    }

    /// Sets the `uid` to the given value.
    pub fn set_uid<S: Into<Cow<'a, str>>>(&mut self, uid: S) {
        self.item.uid = Some(uid.into());
    }

    /// Unsets the `uid`.
    pub fn unset_uid(&mut self) {
        self.item.uid = None;
    }

    /// Sets the `arg` to the given value.
    pub fn set_arg<S: Into<Cow<'a, str>>>(&mut self, arg: S) {
        self.item.arg = Some(arg.into());
    }

    /// Unsets the `arg`.
    pub fn unset_arg(&mut self) {
        self.item.arg = None;
    }

    /// Sets the `arg` to the given value for the given modifier.
    pub fn set_arg_mod<S: Into<Cow<'a, str>>>(&mut self, modifier: Modifier, arg: S) {
        self.data_for_modifier(modifier).arg = Some(arg.into());
    }

    /// Unsets the `arg` for the given modifier.
    ///
    /// This unsets the arg that's used when the given modifier is pressed.
    pub fn unset_arg_mod(&mut self, modifier: Modifier) {
        use std::collections::hash_map::Entry;
        if let Entry::Occupied(mut entry) = self.item.modifiers.entry(modifier) {
            entry.get_mut().arg = None;
            if entry.get().is_empty() {
                entry.remove();
            }
        }
    }

    /// Clears the `arg` for all modifiers.
    ///
    /// This unsets both the default arg and the per-modifier args.
    pub fn clear_arg(&mut self) {
        self.item.arg = None;
        for &modifier in ALL_MODIFIERS {
            self.unset_arg_mod(modifier);
        }
    }

    /// Sets the `type` to the given value.
    pub fn set_type(&mut self, type_: ItemType) {
        self.item.type_ = type_;
    }

    // `type` doesn't need unsetting, it uses a default of DefaultItemType instead

    /// Sets `valid` to the given value.
    pub fn set_valid(&mut self, valid: bool) {
        self.item.valid = valid;
    }

    /// Sets `valid` to the given value for the given modifier.
    pub fn set_valid_mod(&mut self, modifier: Modifier, valid: bool) {
        self.data_for_modifier(modifier).valid = Some(valid);
    }

    /// Unsets `valid` for the given modifier.
    ///
    /// This unsets the validity that's used when the given modifier is pressed.
    pub fn unset_valid_mod(&mut self, modifier: Modifier) {
        use std::collections::hash_map::Entry;
        if let Entry::Occupied(mut entry) = self.item.modifiers.entry(modifier) {
            entry.get_mut().valid = None;
            if entry.get().is_empty() {
                entry.remove();
            }
        }
    }

    /// Unsets `valid` for all modifiers.
    ///
    /// This resets `valid` back to the default and clears all per-modifier validity.
    pub fn clear_valid(&mut self) {
        self.item.valid = true;
        for &modifier in ALL_MODIFIERS {
            self.unset_valid_mod(modifier);
        }
    }

    /// Sets `autocomplete` to the given value.
    pub fn set_autocomplete<S: Into<Cow<'a, str>>>(&mut self, autocomplete: S) {
        self.item.autocomplete = Some(autocomplete.into());
    }

    /// Unsets `autocomplete`.
    pub fn unset_autocomplete(&mut self) {
        self.item.autocomplete = None;
    }

    /// Sets subtitle, arg, validity, and icon for the given modifier.
    pub fn set_modifier<S: Into<Cow<'a, str>>, S2: Into<Cow<'a, str>>>(&mut self,
                                                                       modifier: Modifier,
                                                                       subtitle: Option<S>,
                                                                       arg: Option<S2>,
                                                                       valid: bool,
                                                                       icon: Option<Icon<'a>>) {
        let data = ModifierData {
            subtitle: subtitle.map(Into::into),
            arg: arg.map(Into::into),
            valid: Some(valid),
            icon: icon,
            _priv: ()
        };
        self.item.modifiers.insert(modifier, data);
    }

    /// Unsets subtitle, arg, and validity for the given modifier.
    pub fn unset_modifier(&mut self, modifier: Modifier) {
        self.item.modifiers.remove(&modifier);
    }

    /// Sets `text_copy` to the given value.
    pub fn set_text_copy<S: Into<Cow<'a, str>>>(&mut self, text: S) {
        self.item.text_copy = Some(text.into());
    }

    /// Unsets `text_copy`.
    pub fn unset_text_copy(&mut self) {
        self.item.text_copy = None;
    }

    /// Sets `text_large_type` to the given value.
    pub fn set_text_large_type<S: Into<Cow<'a, str>>>(&mut self, text: S) {
        self.item.text_large_type = Some(text.into());
    }

    /// Unsets `text_large_type`.
    pub fn unset_text_large_type(&mut self) {
        self.item.text_large_type = None;
    }

    /// Sets `quicklook_url` to the given value.
    pub fn set_quicklook_url<S: Into<Cow<'a, str>>>(&mut self, url: S) {
        self.item.quicklook_url = Some(url.into());
    }

    /// Unsets `quicklook_url`.
    pub fn unset_quicklook_url(&mut self) {
        self.item.quicklook_url = None;
    }

    fn data_for_modifier(&mut self, modifier: Modifier) -> &mut ModifierData<'a> {
        self.item.modifiers.entry(modifier).or_insert_with(Default::default)
    }
}

/// Keyboard modifiers.
// As far as I can tell, Alfred doesn't support modifier combinations.
#[derive(Copy,Clone,Debug,PartialEq,Eq,Hash)]
pub enum Modifier {
    /// Command key
    Command,
    /// Option/Alt key
    Option,
    /// Control key
    Control,
    /// Shift key
    Shift,
    /// Fn key
    Fn
}

const ALL_MODIFIERS: &'static [Modifier] = &[Modifier::Command, Modifier::Option,
                                             Modifier::Control, Modifier::Shift, Modifier::Fn];

/// Optional overrides of subtitle, arg, and valid for modifiers.
#[derive(Clone,Debug,PartialEq,Eq,Default)]
pub struct ModifierData<'a> {
    /// The subtitle to use for the current modifier.
    pub subtitle: Option<Cow<'a, str>>,
    /// The arg to use for the current modifier.
    pub arg: Option<Cow<'a, str>>,
    /// The validity to use for the current modifier.
    pub valid: Option<bool>,
    /// The result icon to use for the current modifier.
    ///
    /// This icon is only supported when using JSON output. The legacy XML output format does not
    /// support per-modifier icons.
    ///
    /// This icon is only used with Alfred 3.4.1 or later.
    pub icon: Option<Icon<'a>>,

    /// Disallow struct literals for `ModifierData`.
    _priv: ()
}

impl<'a> ModifierData<'a> {
    /// Returns a new `ModifierData` where all fields are `None`.
    pub fn new() -> ModifierData<'a> {
        Default::default()
    }

    fn is_empty(&self) -> bool {
        self.subtitle.is_none() && self.arg.is_none() && self.valid.is_none()
    }
}

/// Item icons
#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub enum Icon<'a> {
    /// Path to an image file on disk relative to the workflow directory.
    Path(Cow<'a, str>),
    /// Path to a file whose icon will be used.
    File(Cow<'a, str>),
    /// UTI for a file type to use (e.g. public.folder).
    FileType(Cow<'a, str>)
}

/// Item types
#[derive(Copy,Clone,Debug,PartialEq,Eq,Hash)]
pub enum ItemType {
    /// Default type for an item.
    Default,
    /// Type representing a file.
    ///
    /// Alredy checks that the file exists on disk, and hides the result if it
    /// does not.
    File,
    /// Type representing a file, with filesystem checks skipped.
    ///
    /// Similar to `File` but skips the check to ensure the file exists.
    FileSkipCheck
}
