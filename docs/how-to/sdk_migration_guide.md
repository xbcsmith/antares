# SDK Migration Guide

## Migrating to Antares SDK (Phase 9)

This guide helps you migrate existing campaigns and tools to use the new Antares SDK with enhanced integration, error handling, and performance optimizations introduced in Phase 9.

---

## Overview

Phase 9 introduces:

- **Shared Tool Configuration**: Consistent settings across all SDK tools
- **Enhanced Error Messages**: Actionable suggestions and better context
- **Performance Caching**: Faster loading and validation
- **Cross-Tool Integration**: Tools work together seamlessly

---

## What's New in Phase 9

### 1. Shared Configuration System

All SDK tools now share a common configuration file at:

- **Linux/macOS**: `~/.config/antares/tools.ron`
- **Windows**: `%APPDATA%\antares\tools.ron`

**Configuration Options:**

```ron
ToolConfig(
    editor: EditorPreferences(
        auto_save: true,
        backup_count: 3,
        validate_on_save: true,
        confirm_destructive: true,
    ),
    paths: PathConfig(
        data_dir: Some("data"),
        campaigns_dir: Some("campaigns"),
        recent_files: ["campaigns/tutorial/campaign.ron"],
    ),
    display: DisplayConfig(
        color: true,
        verbose: false,
        page_size: 20,
        show_hints: true,
    ),
    validation: ValidationConfig(
        strict_mode: false,
        check_balance: true,
        show_suggestions: true,
        max_errors_displayed: 50,
    ),
)
```

### 2. Improved Error Messages

Errors now include:

- **Context**: File path and line numbers
- **Suggestions**: Actionable steps to fix issues
- **Similar IDs**: When referencing missing content
- **Examples**: Code snippets showing correct usage

**Before (Phase 8):**

```text
Error: Missing item 999
```

**After (Phase 9):**

```text
✗ ERROR in data/monsters.ron:45
Missing item 999 in context: Monster loot table

Suggestions:
  • Run 'item_editor' to create item with ID 999
  • Check that 'data/items.ron' contains item 999
  • Did you mean item ID 100? (similar to 999)
  • Available item IDs: 1, 2, 3, 10, 11 (and 45 more)
```

### 3. Performance Caching

The SDK now caches:

- RON file parsing results
- Validation outcomes
- Database queries
- Frequently accessed content

**Performance Improvements:**

- Campaign loading: 50-70% faster on repeated loads
- Validation: 40-60% faster with unchanged files
- Tool startup: Near-instant with warm cache

### 4. Cross-Tool Integration

Tools now share data and suggestions:

- **Map Builder** suggests items from Item Editor database
- **Class Editor** validates against Race Editor data
- **Campaign Validator** provides tool-specific fix commands
- **All tools** respect shared configuration

---

## Migration Steps

### Step 1: Update Your Tools

Rebuild the SDK tools with the latest code:

```bash
cd antares
cargo build --release

# Verify tools are updated
campaign_validator --version
class_editor --version
item_editor --version
```

### Step 2: Create Default Configuration

Generate the default configuration file:

```bash
# Run any tool to auto-create config
campaign_validator --help

# Or manually create it
mkdir -p ~/.config/antares
cat > ~/.config/antares/tools.ron << 'EOF'
ToolConfig(
    editor: EditorPreferences(
        auto_save: true,
        backup_count: 3,
        validate_on_save: true,
        confirm_destructive: true,
    ),
    paths: PathConfig(
        data_dir: Some("data"),
        campaigns_dir: Some("campaigns"),
        recent_files: [],
    ),
    display: DisplayConfig(
        color: true,
        verbose: false,
        page_size: 20,
        show_hints: true,
    ),
    validation: ValidationConfig(
        strict_mode: false,
        check_balance: true,
        show_suggestions: true,
        max_errors_displayed: 50,
    ),
)
EOF
```

### Step 3: Customize Configuration

Edit `~/.config/antares/tools.ron` to match your preferences:

```ron
// Example: Enable strict mode and verbose output
ToolConfig(
    editor: EditorPreferences(
        auto_save: false,           // Changed: manual save
        backup_count: 5,            // Changed: more backups
        validate_on_save: true,
        confirm_destructive: true,
    ),
    paths: PathConfig(
        data_dir: Some("~/projects/antares/data"),  // Changed: custom path
        campaigns_dir: Some("~/campaigns"),          // Changed: custom path
        recent_files: [],
    ),
    display: DisplayConfig(
        color: true,
        verbose: true,              // Changed: verbose mode
        page_size: 50,              // Changed: more items per page
        show_hints: true,
    ),
    validation: ValidationConfig(
        strict_mode: true,          // Changed: warnings become errors
        check_balance: true,
        show_suggestions: true,
        max_errors_displayed: 100,  // Changed: show more errors
    ),
)
```

### Step 4: Re-validate Campaigns

Re-run validation with enhanced error reporting:

```bash
campaign_validator campaigns/my_campaign

# Use new flags for better output
campaign_validator --verbose campaigns/my_campaign
campaign_validator --json campaigns/my_campaign > report.json
```

### Step 5: Fix Any Issues

Follow the enhanced error suggestions:

```text
✗ ERROR Missing class 'battlemage' in context: Character creation

Suggestions:
  • Run 'class_editor' to create class with ID 'battlemage'
  • Check that 'data/classes.ron' contains class 'battlemage'
  • Available class IDs: knight, paladin, archer (and 5 more)
```

Execute the suggested command:

```bash
class_editor data/classes.ron
# Add the missing 'battlemage' class
```

---

## Breaking Changes

### None!

Phase 9 is **fully backward compatible**. All existing campaigns, data files, and tools continue to work without modification.

**Optional Enhancements:**

- Tools will work without configuration (uses defaults)
- Caching is automatic and transparent
- Enhanced errors appear automatically
- Old error format still works

---

## New Features Available

### 1. Recent Files List

Tools remember your recently opened files:

```bash
# Open recent file
class_editor  # Shows: "Recent files: 1) data/classes.ron 2) ..."
```

### 2. Colored Output

Enable/disable colored output:

```bash
# Via config
display: DisplayConfig(
    color: false,  // Disable colors
    ...
)

# Via environment variable
NO_COLOR=1 campaign_validator campaigns/my_campaign
```

### 3. Progress Reporting

Long operations show progress:

```text
[  0%] Loading classes...
[ 20%] Loading items...
[ 40%] Loading monsters...
[ 60%] Loading spells...
[ 80%] Loading maps...
[100%] Validating...
✓ Validation complete!
```

### 4. Cache Control

Control caching behavior:

```bash
# Clear cache (if needed)
rm -rf ~/.cache/antares

# Or disable caching programmatically
# (See API documentation)
```

---

## Best Practices

### 1. Use Shared Configuration

**DO:** Use the shared configuration for consistent behavior

```bash
# Edit config once
nano ~/.config/antares/tools.ron

# All tools use the same settings
campaign_validator campaigns/my_campaign
class_editor data/classes.ron
item_editor data/items.ron
```

**DON'T:** Set conflicting per-tool settings

### 2. Enable Validation on Save

**DO:** Enable validation to catch errors early

```ron
EditorPreferences(
    validate_on_save: true,
    ...
)
```

**DON'T:** Disable validation unless you have a good reason

### 3. Use Verbose Mode During Development

**DO:** Enable verbose mode when creating campaigns

```ron
DisplayConfig(
    verbose: true,
    show_hints: true,
    ...
)
```

**DON'T:** Leave verbose mode on for production tools

### 4. Leverage Recent Files

**DO:** Use recent files for quick access

```bash
class_editor
# Select from recent files menu
```

**DON'T:** Type full paths repeatedly

### 5. Review Error Suggestions

**DO:** Read and follow error suggestions

```text
Suggestions:
  • Run 'item_editor' to create item with ID 999
  • Check that 'data/items.ron' contains item 999
```

**DON'T:** Ignore helpful suggestions

---

## Troubleshooting

### Configuration Not Loading

**Symptom:** Tools use default settings despite custom config

**Solution:**

```bash
# Verify config location
echo $HOME/.config/antares/tools.ron

# Check config syntax
ron-validator ~/.config/antares/tools.ron

# Or use campaign_validator to check
campaign_validator --help  # Will report config errors
```

### Cache Issues

**Symptom:** Stale data or validation results

**Solution:**

```bash
# Clear cache
rm -rf ~/.cache/antares
rm -rf .antares_cache  # Local cache

# Re-run validation
campaign_validator campaigns/my_campaign
```

### Colors Not Working

**Symptom:** ANSI codes displayed as text

**Solution:**

```bash
# Check terminal support
echo $TERM

# Disable colors if terminal doesn't support them
NO_COLOR=1 campaign_validator campaigns/my_campaign

# Or update config
display: DisplayConfig(
    color: false,
    ...
)
```

### Slow Performance

**Symptom:** Tools slower after Phase 9

**Solution:**

```bash
# Enable caching
rm ~/.config/antares/tools.ron  # Reset to defaults

# Or manually enable
cache: CacheConfig(
    enable_file_cache: true,
    enable_memory_cache: true,
    max_memory_entries: 100,
    ...
)
```

---

## API Changes for Developers

### New SDK Modules

```rust
use antares::sdk::tool_config::ToolConfig;
use antares::sdk::error_formatter::ErrorFormatter;
use antares::sdk::cache::ContentCache;

// Load configuration
let config = ToolConfig::load_or_default()?;

// Create error formatter
let formatter = ErrorFormatter::new(config.display.color);

// Use caching
let mut cache = ContentCache::new(CacheConfig::default());
let items = cache.load_items("data/items.ron")?;
```

### Enhanced Error Formatting

```rust
use antares::sdk::error_formatter::{ErrorFormatter, ErrorContext};

let formatter = ErrorFormatter::new(true);
let context = ErrorContext {
    file_path: Some("data/items.ron".into()),
    line_number: Some(42),
    available_ids: vec![1, 2, 3],
};

let formatted = formatter.format_validation_error(&error, Some(&context));
println!("{}", formatted);
```

### Progress Reporting

```rust
use antares::sdk::error_formatter::ProgressReporter;

let mut progress = ProgressReporter::new(5, true);
progress.step("Loading classes...");
progress.step("Loading items...");
progress.step("Loading monsters...");
progress.step("Loading spells...");
progress.step("Loading maps...");
progress.success("Complete!");
```

---

## Migration Checklist

- [ ] Update to latest SDK code
- [ ] Run `cargo build --release`
- [ ] Create default configuration
- [ ] Customize configuration for your needs
- [ ] Re-validate all campaigns
- [ ] Fix any issues using enhanced error messages
- [ ] Test cross-tool integration
- [ ] Verify performance improvements
- [ ] Update any custom tooling to use new APIs
- [ ] Review and update documentation

---

## Getting Help

### Documentation

- **SDK API Reference**: `docs/reference/sdk_api.md`
- **Campaign Creation Guide**: `docs/tutorials/creating_campaigns.md`
- **Tool Usage Guide**: `docs/how-to/using_sdk_tools.md`
- **Modding Guide**: `docs/explanation/modding_guide.md`

### Common Questions

**Q: Do I need to migrate immediately?**

A: No, Phase 9 is fully backward compatible. Migrate when convenient.

**Q: Will my existing campaigns break?**

A: No, all existing campaigns work without changes.

**Q: Can I disable the new features?**

A: Yes, via configuration. Set `enable_file_cache: false`, `color: false`, etc.

**Q: How do I report issues?**

A: Use the GitHub issue tracker or contact the development team.

---

## Next Steps

After migration:

1. **Explore New Features**: Try the enhanced error messages and caching
2. **Customize Configuration**: Tailor settings to your workflow
3. **Create New Content**: Use the improved tools to build campaigns
4. **Share Feedback**: Report issues or suggest improvements
5. **Update Documentation**: Document any custom workflows

---

## Conclusion

Phase 9 enhances the Antares SDK with better integration, error handling, and performance. The migration is simple, backward compatible, and provides immediate benefits.

**Key Takeaways:**

- Configuration is shared and customizable
- Error messages are actionable and helpful
- Caching improves performance significantly
- Tools integrate seamlessly
- Migration requires minimal effort

Happy campaign creating!
