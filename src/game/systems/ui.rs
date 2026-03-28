// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameLog>();
    }
}

/// Category for a game log entry.
///
/// Categories are used both for visual styling and for log filtering in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogCategory {
    Combat,
    Dialogue,
    Item,
    Exploration,
    System,
}

impl LogCategory {
    /// Returns all log categories in display order.
    pub fn all() -> [Self; 5] {
        [
            Self::Combat,
            Self::Dialogue,
            Self::Item,
            Self::Exploration,
            Self::System,
        ]
    }

    /// Returns the default display color for this category.
    pub fn default_color(self) -> Color {
        match self {
            Self::Combat => Color::srgb(0.86, 0.45, 0.45),
            Self::Dialogue => Color::srgb(0.85, 0.80, 0.50),
            Self::Item => Color::srgb(0.40, 0.78, 0.40),
            Self::Exploration => Color::srgb(0.55, 0.75, 0.95),
            Self::System => Color::srgb(0.70, 0.70, 0.70),
        }
    }
}

/// A typed game log entry.
///
/// Entries carry enough metadata for category-aware rendering, filtering, and
/// stable ordering.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub category: LogCategory,
    pub text: String,
    pub color: Color,
    pub sequence: u64,
}

impl std::fmt::Display for LogEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

impl LogEntry {
    /// Compatibility helper that forwards to the underlying entry text.
    ///
    /// This preserves existing tests and transitional code that still treat
    /// log entries like string values during the Phase 1 migration.
    pub fn contains(&self, pattern: &str) -> bool {
        self.text.contains(pattern)
    }

    /// Compatibility helper that forwards to the underlying entry text.
    ///
    /// This preserves existing tests and transitional code that still check
    /// for string prefixes directly on log entries.
    pub fn starts_with(&self, pattern: &str) -> bool {
        self.text.starts_with(pattern)
    }

    /// Compatibility helper that forwards to the underlying entry text.
    ///
    /// This preserves existing tests and transitional code that still compare
    /// entries to exact string values during the migration.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl LogEntry {
    /// Creates a new log entry using the default color for `category`.
    pub fn new(category: LogCategory, text: impl Into<String>, sequence: u64) -> Self {
        Self {
            category,
            text: text.into(),
            color: category.default_color(),
            sequence,
        }
    }
}

#[derive(Resource, Debug)]
pub struct GameLog {
    pub entries: Vec<LogEntry>,
    pub messages: Vec<String>,
    pub filter: HashSet<LogCategory>,
    pub sequence_counter: u64,
}

impl Default for GameLog {
    fn default() -> Self {
        Self::new()
    }
}

impl GameLog {
    pub const MAX_LOG_ENTRIES: usize = 200;

    /// Create a new empty game log with all categories enabled.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            messages: Vec::new(),
            filter: LogCategory::all().into_iter().collect(),
            sequence_counter: 0,
        }
    }

    /// Add a typed entry to the game log.
    pub fn add_entry(&mut self, text: String, category: LogCategory) {
        let entry = LogEntry::new(category, text.clone(), self.sequence_counter);
        self.sequence_counter = self.sequence_counter.saturating_add(1);
        self.entries.push(entry);
        self.messages.push(text);

        if self.entries.len() > Self::MAX_LOG_ENTRIES {
            let overflow = self.entries.len() - Self::MAX_LOG_ENTRIES;
            self.entries.drain(0..overflow);
            self.messages.drain(0..overflow);
        }
    }

    /// Add a system entry to the game log using the legacy compatibility API.
    ///
    /// This preserves existing call sites that still use the pre-Phase-1
    /// string-only interface while routing them through the typed log-entry
    /// storage.
    pub fn add(&mut self, text: String) {
        self.add_system(text);
    }

    /// Add a combat entry to the game log.
    pub fn add_combat(&mut self, text: String) {
        self.add_entry(text, LogCategory::Combat);
    }

    /// Add a dialogue entry to the game log.
    pub fn add_dialogue(&mut self, text: String) {
        self.add_entry(text, LogCategory::Dialogue);
    }

    /// Add an item entry to the game log.
    pub fn add_item(&mut self, text: String) {
        self.add_entry(text, LogCategory::Item);
    }

    /// Add an exploration entry to the game log.
    pub fn add_exploration(&mut self, text: String) {
        self.add_entry(text, LogCategory::Exploration);
    }

    /// Add a system entry to the game log.
    pub fn add_system(&mut self, text: String) {
        self.add_entry(text, LogCategory::System);
    }

    /// Get all log entries.
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Get only the entries enabled by the current filter.
    pub fn filtered_entries(&self) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|entry| self.filter.contains(&entry.category))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color_components(color: Color) -> [f32; 4] {
        color.to_srgba().to_f32_array()
    }

    #[test]
    fn test_log_entry_category_defaults() {
        let mut log = GameLog::new();

        log.add_combat("combat".to_string());
        log.add_dialogue("dialogue".to_string());
        log.add_item("item".to_string());
        log.add_exploration("exploration".to_string());
        log.add_system("system".to_string());

        let entries = log.entries();
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].category, LogCategory::Combat);
        assert_eq!(
            color_components(entries[0].color),
            color_components(LogCategory::Combat.default_color())
        );
        assert_eq!(entries[1].category, LogCategory::Dialogue);
        assert_eq!(
            color_components(entries[1].color),
            color_components(LogCategory::Dialogue.default_color())
        );
        assert_eq!(entries[2].category, LogCategory::Item);
        assert_eq!(
            color_components(entries[2].color),
            color_components(LogCategory::Item.default_color())
        );
        assert_eq!(entries[3].category, LogCategory::Exploration);
        assert_eq!(
            color_components(entries[3].color),
            color_components(LogCategory::Exploration.default_color())
        );
        assert_eq!(entries[4].category, LogCategory::System);
        assert_eq!(
            color_components(entries[4].color),
            color_components(LogCategory::System.default_color())
        );
    }

    #[test]
    fn test_log_filter_excludes_category() {
        let mut log = GameLog::new();
        log.filter.remove(&LogCategory::Combat);

        log.add_combat("attack".to_string());
        log.add_dialogue("hello".to_string());

        let filtered = log.filtered_entries();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].category, LogCategory::Dialogue);
        assert_eq!(filtered[0].text, "hello");
    }

    #[test]
    fn test_log_max_entries_ring() {
        let mut log = GameLog::new();

        for idx in 0..(GameLog::MAX_LOG_ENTRIES + 10) {
            log.add_system(format!("entry {}", idx));
        }

        assert_eq!(log.entries.len(), GameLog::MAX_LOG_ENTRIES);
        assert_eq!(
            log.entries.first().map(|entry| entry.text.as_str()),
            Some("entry 10")
        );
        assert_eq!(
            log.entries.last().map(|entry| entry.text.as_str()),
            Some("entry 209")
        );
    }

    #[test]
    fn test_log_sequence_monotonic() {
        let mut log = GameLog::new();

        log.add_system("first".to_string());
        log.add_system("second".to_string());
        log.add_system("third".to_string());

        assert!(log.entries[2].sequence > log.entries[1].sequence);
        assert!(log.entries[1].sequence > log.entries[0].sequence);
    }
}
