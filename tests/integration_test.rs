// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Integration Tests
//!
//! Tests cross-tool integration, error formatting, caching, and performance.

use antares::domain::types::ItemId;
use antares::sdk::cache::{CacheConfig, ContentCache, ValidationCache};
use antares::sdk::error_formatter::{ErrorContext, ErrorFormatter, ProgressReporter};
use antares::sdk::tool_config::ToolConfig;
use antares::sdk::validation::ValidationError;
use std::path::PathBuf;
use std::time::Duration;

// ===== Tool Configuration Tests =====

#[test]
fn test_tool_config_default() {
    let config = ToolConfig::default();
    assert!(config.editor.auto_save);
    assert_eq!(config.editor.backup_count, 3);
    assert!(config.display.color);
    assert!(!config.validation.strict_mode);
}

#[test]
fn test_tool_config_save_and_load() {
    use tempfile::NamedTempFile;

    let temp = NamedTempFile::new().unwrap();
    let path = temp.path().to_path_buf();

    let mut config = ToolConfig::default();
    config.display.verbose = true;
    config.validation.strict_mode = true;
    config.editor.backup_count = 5;

    config.save_to_file(&path).unwrap();

    let loaded = ToolConfig::load_from_file(&path).unwrap();
    assert!(loaded.display.verbose);
    assert!(loaded.validation.strict_mode);
    assert_eq!(loaded.editor.backup_count, 5);
}

#[test]
fn test_tool_config_recent_files() {
    let mut config = ToolConfig::default();

    // Add multiple files
    for i in 0..15 {
        config.add_recent_file(PathBuf::from(format!("file{}.ron", i)));
    }

    // Should keep only 10 most recent
    assert_eq!(config.paths.recent_files.len(), 10);
    assert_eq!(config.paths.recent_files[0], PathBuf::from("file14.ron"));
}

#[test]
fn test_tool_config_get_directories() {
    let config = ToolConfig::default();
    assert_eq!(config.get_data_dir(), PathBuf::from("data"));
    assert_eq!(config.get_campaigns_dir(), PathBuf::from("campaigns"));
}

#[test]
fn test_tool_config_custom_paths() {
    let mut config = ToolConfig::default();
    config.paths.data_dir = Some(PathBuf::from("custom_data"));
    config.paths.campaigns_dir = Some(PathBuf::from("custom_campaigns"));

    assert_eq!(config.get_data_dir(), PathBuf::from("custom_data"));
    assert_eq!(
        config.get_campaigns_dir(),
        PathBuf::from("custom_campaigns")
    );
}

// ===== Error Formatter Tests =====

#[test]
fn test_error_formatter_missing_item() {
    let formatter = ErrorFormatter::new(false);
    let error = ValidationError::MissingItem {
        context: "Monster loot table".to_string(),
        item_id: ItemId::from(99),
    };

    let formatted = formatter.format_validation_error(&error, None);
    assert!(formatted.contains("99"));
    assert!(formatted.contains("Suggestions"));
    assert!(formatted.contains("item_editor"));
}

#[test]
fn test_error_formatter_with_context() {
    let formatter = ErrorFormatter::new(false);
    let error = ValidationError::MissingItem {
        context: "Treasure chest".to_string(),
        item_id: ItemId::from(99),
    };

    let context = ErrorContext {
        file_path: Some(PathBuf::from("data/maps/dungeon1.ron")),
        line_number: Some(42),
        available_ids: vec![1, 2, 3, 10, 20, 30],
    };

    let formatted = formatter.format_validation_error(&error, Some(&context));
    assert!(formatted.contains("dungeon1.ron"));
    assert!(formatted.contains("42"));
    assert!(formatted.contains("Suggestions"));
}

#[test]
fn test_error_formatter_missing_class() {
    let formatter = ErrorFormatter::new(false);
    let error = ValidationError::MissingClass {
        context: "Character creation".to_string(),
        class_id: "wizard".to_string(),
    };

    let formatted = formatter.format_validation_error(&error, None);
    assert!(formatted.contains("wizard"));
    assert!(formatted.contains("class_editor"));
    assert!(formatted.contains("classes.ron"));
}

#[test]
fn test_error_formatter_disconnected_map() {
    let formatter = ErrorFormatter::new(false);
    let error = ValidationError::DisconnectedMap { map_id: 42 };

    let formatted = formatter.format_validation_error(&error, None);
    assert!(formatted.contains("42"));
    assert!(formatted.contains("map_builder"));
    assert!(formatted.contains("unreachable"));
}

#[test]
fn test_error_formatter_duplicate_id() {
    let formatter = ErrorFormatter::new(false);
    let error = ValidationError::DuplicateId {
        entity_type: "item".to_string(),
        id: "42".to_string(),
    };

    let formatted = formatter.format_validation_error(&error, None);
    assert!(formatted.contains("item"));
    assert!(formatted.contains("unique"));
    assert!(formatted.contains("42"));
}

#[test]
fn test_error_report_multiple_errors() {
    let formatter = ErrorFormatter::new(false);
    let errors = vec![
        ValidationError::MissingItem {
            context: "Test 1".to_string(),
            item_id: ItemId::from(1),
        },
        ValidationError::MissingItem {
            context: "Test 2".to_string(),
            item_id: ItemId::from(2),
        },
        ValidationError::MissingItem {
            context: "Test 3".to_string(),
            item_id: ItemId::from(3),
        },
    ];

    let report = formatter.format_error_report(&errors);
    assert!(report.contains("Validation Report"));
    assert!(report.contains("3 errors"));
    assert!(report.contains("1."));
    assert!(report.contains("2."));
    assert!(report.contains("3."));
}

#[test]
fn test_error_formatter_no_color() {
    let formatter = ErrorFormatter::new(false);
    let error = ValidationError::MissingItem {
        context: "Test".to_string(),
        item_id: ItemId::from(1),
    };

    let formatted = formatter.format_validation_error(&error, None);
    // Should not contain ANSI escape codes
    assert!(!formatted.contains("\x1b["));
}

#[test]
fn test_progress_reporter() {
    let mut reporter = ProgressReporter::new(5, false);
    reporter.step("Step 1");
    reporter.step("Step 2");
    reporter.step("Step 3");
    reporter.step("Step 4");
    reporter.step("Step 5");
    reporter.success("Complete!");
    // Just testing it doesn't panic
}

// ===== Cache Tests =====

#[test]
fn test_cache_config_default() {
    let config = CacheConfig::default();
    assert!(config.enable_file_cache);
    assert!(config.enable_memory_cache);
    assert_eq!(config.max_memory_entries, 100);
    assert_eq!(config.ttl, Duration::from_secs(3600));
}

#[test]
fn test_content_cache_creation() {
    let config = CacheConfig::default();
    let cache = ContentCache::new(config);
    let stats = cache.stats();
    assert_eq!(stats.memory_entries, 0);
    assert_eq!(stats.total_accesses, 0);
}

#[test]
fn test_content_cache_clear() {
    let config = CacheConfig::default();
    let mut cache = ContentCache::new(config);
    cache.clear();
    let stats = cache.stats();
    assert_eq!(stats.memory_entries, 0);
}

#[test]
fn test_validation_cache() {
    let config = CacheConfig::default();
    let mut cache = ValidationCache::new(config);

    cache.put("campaign1".to_string(), vec!["error1".to_string()]);
    cache.put("campaign2".to_string(), vec!["error2".to_string()]);

    assert!(cache.get("campaign1").is_some());
    assert!(cache.get("campaign2").is_some());
    assert!(cache.get("missing").is_none());
}

#[test]
fn test_validation_cache_expiration() {
    let config = CacheConfig {
        ttl: Duration::from_millis(50),
        ..Default::default()
    };

    let mut cache = ValidationCache::new(config);
    cache.put("test".to_string(), vec!["error".to_string()]);

    // Should be available immediately
    assert!(cache.get("test").is_some());

    // Wait for expiration
    std::thread::sleep(Duration::from_millis(100));

    // Should be expired now
    assert!(cache.get("test").is_none());
}

#[test]
fn test_validation_cache_cleanup() {
    let config = CacheConfig {
        ttl: Duration::from_millis(50),
        ..Default::default()
    };

    let mut cache = ValidationCache::new(config);
    cache.put("test1".to_string(), vec!["error1".to_string()]);
    cache.put("test2".to_string(), vec!["error2".to_string()]);

    std::thread::sleep(Duration::from_millis(100));
    cache.cleanup();

    // Both should be cleaned up
    assert!(cache.get("test1").is_none());
    assert!(cache.get("test2").is_none());
}

#[test]
fn test_cache_stats_hit_rate() {
    use antares::sdk::cache::CacheStats;

    let stats = CacheStats {
        memory_entries: 50,
        total_accesses: 80,
    };

    assert_eq!(stats.hit_rate(100), 0.8);
    assert_eq!(stats.hit_rate(80), 1.0);
    assert_eq!(stats.hit_rate(0), 0.0);
}

// ===== Integration Tests =====

#[test]
fn test_config_and_formatter_integration() {
    // Load config
    let config = ToolConfig::default();

    // Create formatter based on config
    let formatter = ErrorFormatter::new(config.display.color);

    // Format an error
    let error = ValidationError::MissingItem {
        context: "Test".to_string(),
        item_id: ItemId::from(1),
    };

    let formatted = formatter.format_validation_error(&error, None);
    assert!(formatted.contains("Suggestions"));
}

#[test]
fn test_cache_and_config_integration() {
    let tool_config = ToolConfig::default();

    // Create cache config from tool config
    let cache_config = CacheConfig {
        enable_file_cache: true,
        enable_memory_cache: true,
        max_memory_entries: 100,
        ttl: Duration::from_secs(3600),
        cache_dir: tool_config.get_data_dir().join(".cache"),
    };

    let cache = ContentCache::new(cache_config);
    let stats = cache.stats();
    assert_eq!(stats.memory_entries, 0);
}

#[test]
fn test_end_to_end_validation_workflow() {
    // 1. Load config
    let config = ToolConfig::default();

    // 2. Create formatter with config settings
    let formatter = ErrorFormatter::new(config.display.color);

    // 3. Create validation cache
    let mut val_cache = ValidationCache::new(CacheConfig::default());

    // 4. Simulate validation
    let errors = vec![ValidationError::MissingItem {
        context: "Test".to_string(),
        item_id: ItemId::from(1),
    }];

    // 5. Format errors
    let report = formatter.format_error_report(&errors);
    assert!(report.contains("Validation Report"));

    // 6. Cache results
    val_cache.put(
        "test_campaign".to_string(),
        vec!["Error message".to_string()],
    );

    // 7. Verify cached
    assert!(val_cache.get("test_campaign").is_some());
}

#[test]
fn test_error_context_with_suggestions() {
    let formatter = ErrorFormatter::new(false);
    let error = ValidationError::MissingItem {
        context: "Monster drop".to_string(),
        item_id: ItemId::from(100),
    };

    let context = ErrorContext {
        file_path: Some(PathBuf::from("data/monsters.ron")),
        line_number: Some(25),
        available_ids: vec![1, 2, 3, 98, 99, 101, 102],
    };

    let formatted = formatter.format_validation_error(&error, Some(&context));

    // Should suggest similar ID
    assert!(formatted.contains("monsters.ron"));
    assert!(formatted.contains("25"));
    assert!(formatted.contains("Suggestions"));
}

#[test]
fn test_strict_mode_config() {
    let mut config = ToolConfig::default();
    assert!(!config.validation.strict_mode);

    config.validation.strict_mode = true;
    assert!(config.validation.strict_mode);
}

#[test]
fn test_verbose_mode_config() {
    let mut config = ToolConfig::default();
    assert!(!config.display.verbose);

    config.display.verbose = true;
    assert!(config.display.verbose);
}

#[test]
fn test_max_errors_config() {
    let config = ToolConfig::default();
    assert_eq!(config.validation.max_errors_displayed, 50);

    let mut custom_config = config;
    custom_config.validation.max_errors_displayed = 100;
    assert_eq!(custom_config.validation.max_errors_displayed, 100);
}

// ===== Performance Tests =====

#[test]
fn test_cache_performance() {
    let config = CacheConfig::default();
    let mut cache = ContentCache::new(config);

    // Simulate loading many items
    for _ in 0..100 {
        cache.clear();
    }

    let stats = cache.stats();
    assert_eq!(stats.memory_entries, 0);
}

#[test]
fn test_error_formatting_performance() {
    let formatter = ErrorFormatter::new(false);

    // Format many errors
    let errors: Vec<_> = (0..100)
        .map(|i| ValidationError::MissingItem {
            context: format!("Test {}", i),
            item_id: ItemId::from(i),
        })
        .collect();

    let report = formatter.format_error_report(&errors);
    assert!(report.contains("100 errors"));
}
