// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared UI helper utilities for the campaign builder.
//!
//! This module contains reusable UI components, helper functions, and constants
//! used by the editor UI components (items, spells, monsters, etc.). These
//! helpers centralize layout constants and logic so multiple editors behave
//! consistently when windows are resized.
//!
//! # Sub-modules
//!
//! - [`layout`]      – Layout types, toolbar, list components, validation warnings
//! - [`file_io`]     – CSV helpers, import/export dialog, file load/save/reload
//! - [`attribute`]   – `AttributePair` and `AttributePair16` input widgets
//! - [`autocomplete`]– Autocomplete widgets, selectors, candidate extraction, cache

pub mod attribute;
pub mod autocomplete;
pub mod file_io;
pub mod layout;

pub use attribute::*;
pub use autocomplete::*;
pub use file_io::*;
pub use layout::*;

// Re-export SearchableSelectorConfig and SearchableSelectorContext at the crate
// helpers level so editors can import them alongside the other ui_helper types.
pub use layout::SearchableSelectorConfig;
pub use layout::SearchableSelectorContext;

// Re-export the parameter-bundle structs introduced to keep function argument
// counts within the Clippy `too_many_arguments` limit.
pub use autocomplete::AutocompleteListSelectorConfig;
pub use autocomplete::AutocompleteSelectorConfig;
pub use autocomplete::DispatchActionState;

#[cfg(test)]
mod tests;
