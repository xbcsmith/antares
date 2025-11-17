# Integrating Phase 9 Modules into CLI Tools

## Overview

This guide documents how to integrate Phase 9 SDK modules (tool configuration, error formatting, and caching) into Antares CLI tools. These modules provide shared configuration, enhanced error messages, and performance optimization.

**Target Audience**: Tool developers and contributors extending the Antares SDK

**Phase 9 Modules**:
- `sdk::tool_config` - Shared configuration management
- `sdk::error_formatter` - Enhanced error formatting
- `sdk::cache` - Performance optimization through caching

---

## Quick Start

### Basic Integration Pattern

```rust
use antares::sdk::tool_config::ToolConfig;
use antares::sdk::error_formatter::{ErrorFormatter, ErrorContext, ProgressReporter};
use antares::sdk::cache::{ContentCache, CacheConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load shared configuration
    let config = ToolConfig::load_or_default()?;
    
    // 2. Create error formatter using config settings
    let formatter = ErrorFormatter::new(config.display.color);
    
    // 3. Create cache for performance
    let mut cache = ContentCache::new(CacheConfig::default());
    
    // 4. Use in your tool...
    
    Ok(())
}
```

---

## Module 1: Tool Configuration

### Purpose

Provides consistent configuration across all SDK tools.

### Integration Steps

#### 1. Add Configuration Loading

```rust
use antares::sdk::tool_config::ToolConfig;
use clap::Parser;

#[derive(Parser)]
struct Args {
    // Your existing args...
    
    /// Override config file path
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Load configuration
    let config = if let Some(config_path) = args.config {
        ToolConfig::load_from_file(&config_path)?
    } else {
        ToolConfig::load_or_default()?
    };
    
    // Use configuration
    let verbose = config.display.verbose;
    let data_dir = config.get_data_dir();
    
    Ok(())
}
```

#### 2. Respect Configuration Settings

**Editor Preferences:**
```rust
// Auto-save behavior
if config.editor.auto_save {
    save_on_exit(&data)?;
}

// Validation on save
if config.editor.validate_on_save {
    let errors = validate(&data)?;
    if !errors.is_empty() {
        eprintln!("Validation errors found!");
        for error in errors {
            eprintln!("  - {}", error);
        }
    }
}

// Confirm destructive operations
if config.editor.confirm_destructive {
    println!("Are you sure you want to delete? (y/N)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        return Ok(());
    }
}
```

**Display Preferences:**
```rust
// Verbose output
if config.display.verbose {
    println!("Loading data from {}", path.display());
}

// Page size for lists
let page_size = config.display.page_size;
for chunk in items.chunks(page_size) {
    display_page(chunk);
    if should_continue()? {
        continue;
    } else {
        break;
    }
}

// Show hints
if config.display.show_hints {
    println!("Hint: Use Ctrl+C to cancel");
}
```

**Validation Settings:**
```rust
// Strict mode (warnings become errors)
let validation_errors = validator.validate_all()?;
for error in validation_errors {
    if config.validation.strict_mode && error.is_warning() {
        // Treat warning as error in strict mode
        eprintln!("ERROR: {}", error);
        has_errors = true;
    } else if error.is_error() {
        eprintln!("ERROR: {}", error);
        has_errors = true;
    } else {
        eprintln!("WARNING: {}", error);
    }
}

// Balance checking
if config.validation.check_balance {
    check_game_balance(&content)?;
}

// Max errors displayed
let errors_to_show = validation_errors
    .iter()
    .take(config.validation.max_errors_displayed);
```

#### 3. Update Recent Files

```rust
// After successfully opening a file
let mut config = ToolConfig::load_or_default()?;
config.add_recent_file(opened_file_path.clone());
config.save()?;

// Display recent files
println!("Recent files:");
for (i, path) in config.paths.recent_files.iter().enumerate() {
    println!("  {}. {}", i + 1, path.display());
}
```

#### 4. Allow Configuration Override

```rust
// Command-line overrides
if args.verbose {
    config.display.verbose = true;
}
if args.no_color {
    config.display.color = false;
}
if let Some(data_dir) = args.data_dir {
    config.paths.data_dir = Some(data_dir);
}
```

---

## Module 2: Error Formatter

### Purpose

Provides enhanced error messages with context and actionable suggestions.

### Integration Steps

#### 1. Replace Basic Error Output

**Before:**
```rust
match validate_campaign(path) {
    Ok(_) => println!("Valid"),
    Err(e) => eprintln!("Error: {}", e),
}
```

**After:**
```rust
use antares::sdk::error_formatter::{ErrorFormatter, ErrorContext};

let formatter = ErrorFormatter::new(config.display.color);

match validate_campaign(path) {
    Ok(_) => println!("✓ Valid"),
    Err(validation_errors) => {
        for error in validation_errors {
            let context = ErrorContext {
                file_path: Some(path.clone()),
                line_number: None,
                available_ids: vec![],
            };
            
            let formatted = formatter.format_validation_error(&error, Some(&context));
            eprintln!("{}", formatted);
        }
    }
}
```

#### 2. Add Context Information

```rust
// When you know the file and line
let context = ErrorContext {
    file_path: Some(PathBuf::from("data/items.ron")),
    line_number: Some(42),
    available_ids: vec![],
};

// When you have available IDs for suggestions
let available_item_ids: Vec<u32> = item_db.all_items()
    .iter()
    .map(|item| u32::from(item.id))
    .collect();

let context = ErrorContext {
    file_path: Some(PathBuf::from("data/monsters.ron")),
    line_number: Some(25),
    available_ids: available_item_ids,
};

let formatted = formatter.format_validation_error(&error, Some(&context));
```

#### 3. Format Multiple Errors as Report

```rust
let validation_errors = validator.validate_all()?;

if !validation_errors.is_empty() {
    let report = formatter.format_error_report(&validation_errors);
    eprintln!("{}", report);
    
    std::process::exit(1);
}
```

#### 4. Add Progress Reporting

```rust
use antares::sdk::error_formatter::ProgressReporter;

let mut progress = ProgressReporter::new(5, config.display.color);

progress.step("Loading classes...");
let classes = load_classes(&data_dir)?;

progress.step("Loading items...");
let items = load_items(&data_dir)?;

progress.step("Loading monsters...");
let monsters = load_monsters(&data_dir)?;

progress.step("Loading spells...");
let spells = load_spells(&data_dir)?;

progress.step("Validating...");
let errors = validate_all(&classes, &items, &monsters, &spells)?;

if errors.is_empty() {
    progress.success("Validation complete!");
} else {
    progress.error(&format!("Found {} errors", errors.len()));
}
```

---

## Module 3: Content Cache

### Purpose

Improves performance by caching frequently loaded content.

### Integration Steps

#### 1. Initialize Cache

```rust
use antares::sdk::cache::{ContentCache, CacheConfig};

// Use default configuration
let mut cache = ContentCache::new(CacheConfig::default());

// Or customize
let cache_config = CacheConfig {
    enable_file_cache: true,
    enable_memory_cache: true,
    max_memory_entries: 200,  // Increase for large campaigns
    ttl: Duration::from_secs(7200),  // 2 hours
    cache_dir: PathBuf::from(".my_tool_cache"),
};
let mut cache = ContentCache::new(cache_config);
```

#### 2. Cache Content Loading

```rust
// Load items with caching
let items = cache.load_items("data/items.ron")?;

// First load: reads from disk (slower)
// Subsequent loads: returns from cache (faster)
let items_again = cache.load_items("data/items.ron")?;
```

#### 3. Clear Cache When Needed

```rust
// After user edits a file
if file_was_modified {
    cache.clear();
    println!("Cache cleared");
}

// Or selectively clear
// (Note: Current API clears all; selective clearing would be an enhancement)
```

#### 4. Display Cache Statistics

```rust
let stats = cache.stats();

if config.display.verbose {
    println!("Cache statistics:");
    println!("  Entries: {}", stats.memory_entries);
    println!("  Total accesses: {}", stats.total_accesses);
    
    // Calculate hit rate if you track total requests
    let total_requests = /* your tracking */;
    println!("  Hit rate: {:.1}%", stats.hit_rate(total_requests) * 100.0);
}
```

#### 5. Use Validation Cache

```rust
use antares::sdk::cache::ValidationCache;

let mut val_cache = ValidationCache::new(CacheConfig::default());

// Check if campaign was already validated
if let Some(cached_errors) = val_cache.get(&campaign_path.to_string_lossy()) {
    if config.display.verbose {
        println!("Using cached validation results");
    }
    return Ok(cached_errors.to_vec());
}

// Otherwise, validate and cache results
let errors = perform_validation(&campaign)?;
val_cache.put(campaign_path.to_string_lossy().to_string(), errors.clone());

errors
```

---

## Complete Integration Example

### Example: campaign_validator with Full Phase 9 Integration

```rust
use antares::sdk::tool_config::ToolConfig;
use antares::sdk::error_formatter::{ErrorFormatter, ErrorContext, ProgressReporter};
use antares::sdk::cache::{ValidationCache, CacheConfig};
use antares::sdk::validation::Validator;
use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "campaign_validator")]
#[command(about = "Validate Antares campaigns with Phase 9 enhancements")]
struct Args {
    /// Campaign directory to validate
    campaign: PathBuf,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Disable colors
    #[arg(long)]
    no_color: bool,

    /// Skip cache
    #[arg(long)]
    no_cache: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // 1. Load configuration
    let mut config = ToolConfig::load_or_default()?;
    
    // Apply command-line overrides
    if args.verbose {
        config.display.verbose = true;
    }
    if args.no_color {
        config.display.color = false;
    }

    // 2. Create formatter
    let formatter = ErrorFormatter::new(config.display.color);

    // 3. Create cache
    let mut val_cache = ValidationCache::new(CacheConfig::default());

    // 4. Check cache first (unless --no-cache)
    let campaign_key = args.campaign.to_string_lossy().to_string();
    if !args.no_cache {
        if let Some(cached_errors) = val_cache.get(&campaign_key) {
            if config.display.verbose {
                println!("Using cached validation results");
            }
            
            if cached_errors.is_empty() {
                println!("✓ Campaign is valid (cached)");
                return Ok(());
            } else {
                println!("✗ Campaign has errors (cached):");
                for error in cached_errors {
                    println!("  - {}", error);
                }
                process::exit(1);
            }
        }
    }

    // 5. Load and validate with progress reporting
    let mut progress = ProgressReporter::new(3, config.display.color);

    progress.step("Loading campaign...");
    let campaign = Campaign::load(&args.campaign)?;

    progress.step("Loading content database...");
    let db = campaign.load_content()?;

    if config.display.verbose {
        let stats = db.stats();
        println!("  Loaded {} classes, {} items, {} maps", 
                 stats.class_count, stats.item_count, stats.map_count);
    }

    progress.step("Validating...");
    let validator = Validator::new(&db);
    let validation_errors = validator.validate_all()?;

    // 6. Format and display errors
    if validation_errors.is_empty() {
        progress.success("Validation complete - no errors!");
        
        // Cache the result
        val_cache.put(campaign_key, Vec::new());
        
        // Update recent files
        config.add_recent_file(args.campaign.clone());
        config.save()?;
        
        Ok(())
    } else {
        progress.error(&format!("Found {} errors", validation_errors.len()));
        
        // Filter by severity if not in strict mode
        let errors_to_show = if config.validation.strict_mode {
            validation_errors
        } else {
            validation_errors.into_iter()
                .filter(|e| e.is_error())
                .collect()
        };

        // Format errors with context
        for error in &errors_to_show {
            let context = ErrorContext {
                file_path: Some(args.campaign.clone()),
                line_number: None,
                available_ids: vec![],  // Could be populated from db
            };
            
            let formatted = formatter.format_validation_error(error, Some(&context));
            eprintln!("{}", formatted);
        }

        // Cache the errors
        let error_messages: Vec<String> = errors_to_show.iter()
            .map(|e| e.to_string())
            .collect();
        val_cache.put(campaign_key, error_messages);

        process::exit(1);
    }
}
```

---

## Integration Checklist

Use this checklist when integrating Phase 9 modules into a tool:

### Configuration Integration
- [ ] Load `ToolConfig` at startup
- [ ] Respect `display.color` setting
- [ ] Respect `display.verbose` setting
- [ ] Respect `display.page_size` for paginated output
- [ ] Respect `display.show_hints` for hint messages
- [ ] Respect `editor.auto_save` if applicable
- [ ] Respect `editor.validate_on_save` if applicable
- [ ] Respect `editor.confirm_destructive` for dangerous operations
- [ ] Respect `validation.strict_mode` for error/warning handling
- [ ] Respect `validation.check_balance` if applicable
- [ ] Update recent files when opening files
- [ ] Allow command-line overrides of config

### Error Formatting Integration
- [ ] Create `ErrorFormatter` using config
- [ ] Replace basic error output with formatted errors
- [ ] Add `ErrorContext` when available (file, line, IDs)
- [ ] Use `format_error_report()` for multiple errors
- [ ] Add `ProgressReporter` for long operations
- [ ] Use visual indicators (✓, ✗, ⚠, ℹ)

### Caching Integration
- [ ] Create cache at startup
- [ ] Cache frequently loaded content
- [ ] Check cache before expensive operations
- [ ] Clear cache when content changes
- [ ] Display cache stats in verbose mode
- [ ] Allow `--no-cache` flag to skip cache

### Testing
- [ ] Test with default config
- [ ] Test with custom config
- [ ] Test with all display options
- [ ] Test with strict mode
- [ ] Test cache hit/miss scenarios
- [ ] Test error formatting with/without context
- [ ] Test progress reporting

---

## Migration Strategy

### For Existing Tools

**Incremental Approach** (Recommended):

1. **Phase 1: Configuration** (Low Risk)
   - Add configuration loading
   - Respect display settings only
   - Test thoroughly

2. **Phase 2: Error Formatting** (Medium Risk)
   - Replace error output with formatter
   - Add basic context
   - Test error scenarios

3. **Phase 3: Caching** (Low Risk)
   - Add cache initialization
   - Cache expensive operations
   - Test cache invalidation

4. **Phase 4: Full Integration** (Low Risk)
   - Add progress reporting
   - Implement all config settings
   - Add recent files
   - Final testing

### For New Tools

Start with full Phase 9 integration from the beginning using the complete example above.

---

## Common Patterns

### Pattern 1: Conditional Verbose Output

```rust
fn log_verbose(config: &ToolConfig, message: &str) {
    if config.display.verbose {
        println!("{}", message);
    }
}

// Usage
log_verbose(&config, "Loading data...");
```

### Pattern 2: Colorized Output

```rust
fn print_success(config: &ToolConfig, message: &str) {
    if config.display.color {
        println!("\x1b[32m✓\x1b[0m {}", message);
    } else {
        println!("✓ {}", message);
    }
}

fn print_error(config: &ToolConfig, message: &str) {
    if config.display.color {
        eprintln!("\x1b[31m✗\x1b[0m {}", message);
    } else {
        eprintln!("✗ {}", message);
    }
}
```

### Pattern 3: Cached Content Loading

```rust
fn load_with_cache<T>(
    cache: &mut ContentCache,
    path: &Path,
    loader: impl FnOnce(&Path) -> Result<T, Box<dyn std::error::Error>>,
    config: &ToolConfig,
) -> Result<T, Box<dyn std::error::Error>>
where
    T: Clone + Send + 'static,
{
    // Try cache first
    // (Note: Current ContentCache API is limited; this shows the pattern)
    
    if config.display.verbose {
        println!("Loading {}...", path.display());
    }
    
    let data = loader(path)?;
    
    if config.display.verbose {
        println!("Loaded successfully");
    }
    
    Ok(data)
}
```

---

## Best Practices

### 1. Always Use Configuration

**DO:**
```rust
let config = ToolConfig::load_or_default()?;
let verbose = config.display.verbose;
```

**DON'T:**
```rust
let verbose = args.verbose;  // Ignores shared config
```

### 2. Provide Helpful Error Context

**DO:**
```rust
let context = ErrorContext {
    file_path: Some(path.clone()),
    line_number: Some(line),
    available_ids: get_valid_ids(),
};
formatter.format_validation_error(&error, Some(&context))
```

**DON'T:**
```rust
formatter.format_validation_error(&error, None)  // Missing helpful context
```

### 3. Cache Expensive Operations

**DO:**
```rust
let items = cache.load_items(path)?;  // Cached
```

**DON'T:**
```rust
let items = load_items_from_disk(path)?;  // Always slow
```

### 4. Show Progress for Long Operations

**DO:**
```rust
let mut progress = ProgressReporter::new(5, config.display.color);
progress.step("Step 1...");
// ... do work
progress.step("Step 2...");
// ... do work
progress.success("Done!");
```

**DON'T:**
```rust
// Long operation with no feedback
perform_long_operation()?;
```

---

## Troubleshooting

### Configuration Not Loading

**Problem:** Tool uses default settings instead of user config.

**Solution:**
```rust
// Debug configuration loading
match ToolConfig::load() {
    Ok(config) => {
        println!("Loaded config from: {:?}", ToolConfig::default_path()?);
    }
    Err(e) => {
        eprintln!("Config not found, using defaults: {}", e);
        ToolConfig::default()
    }
}
```

### Cache Not Working

**Problem:** Cache doesn't seem to improve performance.

**Solution:**
```rust
// Verify cache is enabled
let config = CacheConfig::default();
assert!(config.enable_memory_cache);
assert!(config.enable_file_cache);

// Check cache stats
let stats = cache.stats();
println!("Cache entries: {}", stats.memory_entries);
println!("Total accesses: {}", stats.total_accesses);
```

### Colors Not Showing

**Problem:** ANSI codes displayed as text.

**Solution:**
```rust
// Check if terminal supports colors
if !atty::is(atty::Stream::Stdout) {
    config.display.color = false;
}

// Or respect NO_COLOR environment variable
if std::env::var("NO_COLOR").is_ok() {
    config.display.color = false;
}
```

---

## Future Enhancements

### Planned Improvements

1. **Selective Cache Invalidation**: Clear specific cache entries
2. **Cache Preloading**: Preload common content on startup
3. **Error Recovery Suggestions**: AI-assisted error fixing
4. **Interactive Error Fixing**: Prompt user to fix errors immediately
5. **Configuration Profiles**: Switch between dev/prod configs
6. **Telemetry Integration**: Track tool usage and performance

### Contributing

To contribute integration improvements:

1. Follow the patterns in this guide
2. Add tests for new integration features
3. Update this document with new patterns
4. Submit PR with integration examples

---

## Summary

Phase 9 modules provide:
- **Consistency**: Shared configuration across all tools
- **Clarity**: Enhanced error messages with suggestions
- **Performance**: Intelligent caching for faster operations

**Integration is optional but recommended** for all SDK tools to provide a cohesive user experience.

**Next Steps:**
1. Review this guide
2. Choose a tool to integrate
3. Follow the incremental migration strategy
4. Test thoroughly
5. Update other tools with the same pattern

For questions or support, see:
- `docs/reference/sdk_api.md` - API reference
- `docs/explanation/phase9_release_notes.md` - Release notes
- `docs/how-to/sdk_migration_guide.md` - Migration guide
