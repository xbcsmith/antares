// SPDX-FileCopyrightText: 2026 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Shared helpers for RON-based database loading.
//!
//! Many domain and SDK database types follow the same pattern:
//!
//! 1. Read a RON file into a `String`.
//! 2. Deserialize a `Vec<Entity>` from the string.
//! 3. Insert each entity into a `HashMap<K, Entity>`, rejecting duplicates.
//!
//! [`load_ron_entries`] encapsulates step 2–3 so that every database can
//! implement `load_from_string` as a thin wrapper.
//!
//! # Examples
//!
//! ```
//! use std::collections::HashMap;
//! use antares::domain::database_common::load_ron_entries;
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize)]
//! struct Widget { id: u32, name: String }
//!
//! #[derive(Debug)]
//! enum WidgetError {
//!     Parse(ron::error::SpannedError),
//!     Duplicate(u32),
//! }
//!
//! let ron = r#"[(id: 1, name: "Sprocket"), (id: 2, name: "Cog")]"#;
//!
//! let map: HashMap<u32, Widget> = load_ron_entries(
//!     ron,
//!     |w| w.id,
//!     WidgetError::Duplicate,
//!     WidgetError::Parse,
//! )
//! .unwrap();
//!
//! assert_eq!(map.len(), 2);
//! ```

use std::collections::HashMap;
use std::hash::Hash;

/// Deserializes a RON string containing a `Vec<T>`, then inserts each element
/// into a `HashMap<K, T>` keyed by the value returned from `id_of`.
///
/// Duplicate keys are rejected by calling `dup_err` to produce the
/// caller-specific error variant. RON parse failures are mapped through
/// `parse_err`.
///
/// # Type Parameters
///
/// * `T` — The entity type stored in the database (must be `Deserialize`).
/// * `K` — The key / ID type (must be `Eq + Hash + Clone`).
/// * `E` — The caller's error type.
///
/// # Arguments
///
/// * `ron_data`  — RON-formatted string containing a list of entities.
/// * `id_of`     — Closure that extracts the key from an entity reference.
/// * `dup_err`   — Closure (or variant constructor) that builds a duplicate-ID
///   error from a key.
/// * `parse_err` — Closure (or variant constructor) that wraps a
///   [`ron::error::SpannedError`] into the caller's error type.
///
/// # Returns
///
/// `Ok(HashMap<K, T>)` on success, or `Err(E)` on parse failure or duplicate
/// key.
///
/// # Examples
///
/// Using a domain error type whose `ParseError` variant wraps the RON error
/// directly (via `#[from]`):
///
/// ```
/// use antares::domain::database_common::load_ron_entries;
/// use serde::Deserialize;
/// use thiserror::Error;
///
/// #[derive(Debug, Deserialize)]
/// struct Item { id: u32 }
///
/// #[derive(Debug, Error)]
/// enum DbError {
///     #[error("parse: {0}")]
///     Parse(#[from] ron::error::SpannedError),
///     #[error("duplicate: {0}")]
///     Dup(u32),
/// }
///
/// let items = load_ron_entries("[(id: 1), (id: 2)]", |i| i.id, DbError::Dup, DbError::Parse).unwrap();
/// assert_eq!(items.len(), 2);
/// ```
///
/// Using a domain error type whose `ParseError` variant wraps a `String`:
///
/// ```
/// use antares::domain::database_common::load_ron_entries;
/// use serde::Deserialize;
/// use thiserror::Error;
///
/// #[derive(Debug, Error)]
/// enum MyErr {
///     #[error("parse: {0}")]
///     Parse(String),
///     #[error("dup: {0}")]
///     Dup(String),
/// }
///
/// #[derive(Debug, Deserialize)]
/// struct Entry { id: String }
///
/// let entries = load_ron_entries(
///     r#"[(id: "a"), (id: "b")]"#,
///     |e| e.id.clone(),
///     MyErr::Dup,
///     |e| MyErr::Parse(format!("RON parse error: {}", e)),
/// )
/// .unwrap();
/// assert_eq!(entries.len(), 2);
/// ```
pub fn load_ron_entries<T, K, E>(
    ron_data: &str,
    id_of: impl Fn(&T) -> K,
    dup_err: impl Fn(K) -> E,
    parse_err: impl FnOnce(ron::error::SpannedError) -> E,
) -> Result<HashMap<K, T>, E>
where
    T: for<'de> serde::Deserialize<'de>,
    K: Eq + Hash + Clone,
{
    let entities: Vec<T> = ron::from_str(ron_data).map_err(parse_err)?;
    let mut map = HashMap::with_capacity(entities.len());

    for entity in entities {
        let id = id_of(&entity);
        if map.contains_key(&id) {
            return Err(dup_err(id));
        }
        map.insert(id, entity);
    }

    Ok(map)
}

/// Convenience wrapper: reads a file to a string and delegates to
/// [`load_ron_entries`].
///
/// The `read_err` closure maps an [`std::io::Error`] into the caller's error
/// type. If the caller's error type already implements
/// `From<std::io::Error>`, pass `Into::into` (or the variant constructor
/// directly) as `read_err`.
///
/// # Arguments
///
/// * `path`      — Filesystem path to the RON file.
/// * `id_of`     — Extracts the key from an entity.
/// * `dup_err`   — Builds a duplicate-key error.
/// * `read_err`  — Wraps an I/O error.
/// * `parse_err` — Wraps a RON parse error.
///
/// # Examples
///
/// ```no_run
/// use antares::domain::database_common::load_ron_file;
/// use serde::Deserialize;
/// use thiserror::Error;
///
/// #[derive(Debug, Deserialize)]
/// struct Monster { id: u32 }
///
/// #[derive(Debug, Error)]
/// enum MErr {
///     #[error("{0}")]
///     Io(#[from] std::io::Error),
///     #[error("{0}")]
///     Parse(#[from] ron::error::SpannedError),
///     #[error("dup {0}")]
///     Dup(u32),
/// }
///
/// let monsters = load_ron_file(
///     "data/monsters.ron",
///     |m| m.id,
///     MErr::Dup,
///     Into::into,  // io::Error → MErr via #[from]
///     Into::into,  // SpannedError → MErr via #[from]
/// )
/// .unwrap();
/// ```
pub fn load_ron_file<T, K, E, P>(
    path: P,
    id_of: impl Fn(&T) -> K,
    dup_err: impl Fn(K) -> E,
    read_err: impl FnOnce(std::io::Error) -> E,
    parse_err: impl FnOnce(ron::error::SpannedError) -> E,
) -> Result<HashMap<K, T>, E>
where
    T: for<'de> serde::Deserialize<'de>,
    K: Eq + Hash + Clone,
    P: AsRef<std::path::Path>,
{
    let contents = std::fs::read_to_string(path).map_err(read_err)?;
    load_ron_entries(&contents, id_of, dup_err, parse_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct TestEntity {
        id: u32,
        #[allow(dead_code)]
        name: String,
    }

    #[derive(Debug, PartialEq)]
    enum TestError {
        Parse(String),
        Duplicate(u32),
    }

    #[test]
    fn test_load_ron_entries_success() {
        let ron = r#"[(id: 1, name: "Alpha"), (id: 2, name: "Beta")]"#;
        let map: HashMap<u32, TestEntity> = load_ron_entries(
            ron,
            |e: &TestEntity| e.id,
            TestError::Duplicate,
            |e| TestError::Parse(e.to_string()),
        )
        .unwrap();

        assert_eq!(map.len(), 2);
        assert!(map.contains_key(&1));
        assert!(map.contains_key(&2));
    }

    #[test]
    fn test_load_ron_entries_duplicate_id() {
        let ron = r#"[(id: 1, name: "A"), (id: 1, name: "B")]"#;
        let result: Result<HashMap<u32, TestEntity>, TestError> = load_ron_entries(
            ron,
            |e: &TestEntity| e.id,
            TestError::Duplicate,
            |e| TestError::Parse(e.to_string()),
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), TestError::Duplicate(1));
    }

    #[test]
    fn test_load_ron_entries_invalid_ron() {
        let result: Result<HashMap<u32, TestEntity>, TestError> = load_ron_entries(
            "not valid ron }}}",
            |e: &TestEntity| e.id,
            TestError::Duplicate,
            |e| TestError::Parse(e.to_string()),
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            TestError::Parse(_) => {} // expected
            other => panic!("Expected Parse error, got {:?}", other),
        }
    }

    #[test]
    fn test_load_ron_entries_empty_list() {
        let map: HashMap<u32, TestEntity> = load_ron_entries(
            "[]",
            |e: &TestEntity| e.id,
            TestError::Duplicate,
            |e| TestError::Parse(e.to_string()),
        )
        .unwrap();

        assert!(map.is_empty());
    }

    #[test]
    fn test_load_ron_entries_with_string_ids() {
        #[derive(Debug, Deserialize)]
        struct Named {
            id: String,
        }

        #[derive(Debug, PartialEq)]
        enum NameError {
            Parse(String),
            Dup(String),
        }

        let ron = r#"[(id: "foo"), (id: "bar")]"#;
        let map: HashMap<String, Named> = load_ron_entries(
            ron,
            |n: &Named| n.id.clone(),
            NameError::Dup,
            |e| NameError::Parse(e.to_string()),
        )
        .unwrap();

        assert_eq!(map.len(), 2);
        assert!(map.contains_key("foo"));
        assert!(map.contains_key("bar"));
    }

    #[test]
    fn test_load_ron_entries_string_id_duplicate() {
        #[derive(Debug, Deserialize)]
        struct Named {
            id: String,
        }

        #[derive(Debug, PartialEq)]
        enum NameError {
            Parse(String),
            Dup(String),
        }

        let ron = r#"[(id: "x"), (id: "x")]"#;
        let result: Result<HashMap<String, Named>, NameError> = load_ron_entries(
            ron,
            |n: &Named| n.id.clone(),
            NameError::Dup,
            |e| NameError::Parse(e.to_string()),
        );

        assert_eq!(result.unwrap_err(), NameError::Dup("x".to_string()));
    }

    #[test]
    fn test_load_ron_file_missing_file() {
        let result: Result<HashMap<u32, TestEntity>, TestError> = load_ron_file(
            "this/file/does/not/exist.ron",
            |e: &TestEntity| e.id,
            TestError::Duplicate,
            |_| TestError::Parse("io error".to_string()),
            |e| TestError::Parse(e.to_string()),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_load_ron_entries_preserves_insertion_data() {
        let ron = r#"[(id: 42, name: "TheAnswer")]"#;
        let map: HashMap<u32, TestEntity> = load_ron_entries(
            ron,
            |e: &TestEntity| e.id,
            TestError::Duplicate,
            |e| TestError::Parse(e.to_string()),
        )
        .unwrap();

        let entity = map.get(&42).unwrap();
        assert_eq!(entity.name, "TheAnswer");
    }
}
