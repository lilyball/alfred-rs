# alfred-rs

[![Build Status](https://travis-ci.org/kballard/alfred-rs.svg?branch=master)](https://travis-ci.org/kballard/alfred-rs)
[![crates.io/crates/alfred](http://meritbadge.herokuapp.com/alfred)](https://crates.io/crates/alfred)

Rust library to help with creating [Alfred 2][alfred] [Workflows][].

[alfred]: http://www.alfredapp.com
[Workflows]: http://support.alfredapp.com/workflows

[API Documentation](http://www.rust-ci.org/kballard/alfred-rs/doc/alfred/)

## Installation

Add the following to your `Cargo.toml` file:

```toml
[dependencies]

alfred = "1.0.0"
```

## Version History

#### 1.0.0

Rust 1.0 is out!

#### 0.3.1

Remove `#[unsafe_destructor]`, which no longer exists in the latest nightlies.

#### 0.3.0

Switch from `IntoCow<'a, str>` to `Into<Cow<'a, str>>`.
This is technically a breaking change, but it is unlikely to affect anyone.

#### 0.2.2

Compatibility with the latest Rust nightly.

#### 0.2.1

Compatibility with the latest Rust nightly.

#### 0.2

Switch from `std::old_io` to `std::io`.

#### 0.1.1

Compatibility with the Rust nightly for 2015-02-21.

#### 0.1

Compatibility with the Rust 1.0 Alpha release.
