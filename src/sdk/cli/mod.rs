// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! CLI subcommand implementations for the `antares-sdk` unified binary.
//!
//! Each submodule exposes a `pub fn run(args)` entry point that is wired to a
//! `clap` subcommand variant in `src/bin/antares_sdk.rs`.
//!
//! # Module Layout
//!
//! | Module               | Subcommand                          | Migrated From              |
//! |----------------------|-------------------------------------|----------------------------|
//! | [`names`]            | `antares-sdk names`                 | `src/bin/name_gen.rs`      |
//! | [`campaign_validator`] | `antares-sdk campaign validate`   | `src/bin/campaign_validator.rs` |
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::cli::names::{run, NamesArgs, ThemeArg};
//!
//! let args = NamesArgs {
//!     number: 5,
//!     theme: ThemeArg::Fantasy,
//!     lore: false,
//!     quiet: true,
//! };
//! run(args).expect("name generation should succeed");
//! ```

pub mod campaign_validator;
pub mod names;
