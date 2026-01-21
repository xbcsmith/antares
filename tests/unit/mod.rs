// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Unit test aggregator for `tests/unit/`
//!
//! This module collects small unit tests implemented as submodules so that
//! Cargo compiles them together as a single integration test target named
//! `unit`. Add additional unit tests as separate files in this directory
//! and expose them here with a `mod` declaration (e.g., `mod my_test;`).

mod menu_state_test;
