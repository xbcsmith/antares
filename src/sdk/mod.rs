// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! SDK Foundation Module - Content management and validation for game campaigns
//!
//! This module provides unified content database access, cross-reference validation,
//! RON serialization helpers, and content templates for campaign creation tools.
//!
//! # Architecture Reference
//!
//! See `docs/explanation/sdk_implementation_plan.md` Phase 3 for implementation details.
//!
//! # Module Organization
//!
//! - `database`: Unified content database for all game content types
//! - `validation`: Cross-reference validation and balance checking
//! - `serialization`: RON format helpers and utilities
//! - `templates`: Pre-configured content templates for quick creation
//!
//! # Examples
//!
//! ```no_run
//! use antares::sdk::database::ContentDatabase;
//! use antares::sdk::validation::Validator;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load a campaign's content
//! let db = ContentDatabase::load_campaign("campaigns/my_campaign")?;
//!
//! // Validate all content and cross-references
//! let validator = Validator::new(&db);
//! let errors = validator.validate_all()?;
//!
//! if errors.is_empty() {
//!     println!("Campaign is valid!");
//! } else {
//!     for error in errors {
//!         eprintln!("Validation error: {}", error);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod database;
pub mod serialization;
pub mod templates;
pub mod validation;

// Re-export commonly used types
pub use database::{ContentDatabase, ContentStats, DatabaseError};
pub use serialization::{format_ron, merge_ron_data, validate_ron_syntax, SerializationError};
pub use templates::{basic_armor, basic_weapon, dungeon_map, town_map};
pub use validation::{Severity, ValidationError, Validator};
