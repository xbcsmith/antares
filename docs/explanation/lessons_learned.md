# Lessons Learned - Antares Development

**Last Updated:** 2025-01-25
**Compiled From:** All implementation phases documented in `implementations.md`

---

## Table of Contents

1. [Architecture & Design Patterns](#architecture--design-patterns)
2. [Data Structure Design](#data-structure-design)
3. [Testing Strategies](#testing-strategies)
4. [UI/UX Development](#uiux-development)
5. [Data Migration & Backward Compatibility](#data-migration--backward-compatibility)
6. [Error Handling](#error-handling)
7. [Code Quality & Validation](#code-quality--validation)
8. [Common Pitfalls & Solutions](#common-pitfalls--solutions)
9. [Performance Optimization](#performance-optimization)
10. [Documentation Best Practices](#documentation-best-practices)

---

## Architecture & Design Patterns

### 1. Separation of Concerns

**Lesson:** Keep domain logic, UI, and infrastructure strictly separated.

**Evidence from implementations:**

- Party management domain logic (`PartyManager`) is pure and testable
- UI systems (`InnUiPlugin`, `RecruitmentDialogPlugin`) handle only presentation
- Application layer (`GameState`) orchestrates domain and infrastructure

**Best Practice:**

```rust
// ✅ GOOD: Pure domain logic
pub struct PartyManager;
impl PartyManager {
    pub fn recruit_to_party(
        party: &mut Party,
        roster: &mut Roster,
        roster_index: usize
    ) -> Result<(), PartyManagementError> {
        // Pure logic, no side effects
    }
}

// ✅ GOOD: UI delegates to domain
fn inn_action_system(
    mut game_state: ResMut<GameState>,
    mut messages: MessageReader<InnRecruitCharacter>,
) {
    for msg in messages.read() {
        // UI just calls domain logic
        game_state.recruit_character(msg.roster_index);
    }
}
```

**Anti-Pattern:**

```rust
// ❌ BAD: UI contains business logic
fn inn_ui_system() {
    if button_clicked {
        // Don't put domain logic here!
        if party.len() >= 6 {
            return; // Business rule in UI layer
        }
        party.push(character);
    }
}
```

### 2. Message-Based Architecture (Bevy)

**Lesson:** Use message passing for decoupling UI from game state.

**Pattern from Inn UI and Recruitment Dialog:**

```rust
// 1. Define messages
#[derive(Message)]
pub struct RecruitmentResponseMessage {
    pub accept: bool,
    pub character_id: String,
}

// 2. UI emits messages
fn ui_system(mut writer: MessageWriter<RecruitmentResponseMessage>) {
    if accept_button.clicked() {
        writer.write(RecruitmentResponseMessage {
            accept: true,
            character_id: "npc_id".to_string(),
        });
    }
}

// 3. Game logic consumes messages
fn action_system(
    mut game_state: ResMut<GameState>,
    mut reader: MessageReader<RecruitmentResponseMessage>,
) {
    for msg in reader.read() {
        // Process business logic
    }
}
```

**Benefits:**

- UI and logic can be tested independently
- Easy to add new message types without changing existing code
- Clear data flow direction

### 3. Builder Pattern for Complex Structures

**Lesson:** Use builder methods for structures with optional fields.

**Evidence from TileVisualMetadata:**

```rust
pub struct TileVisualMetadata {
    pub height: Option<f32>,
    pub width_x: Option<f32>,
    pub color_tint: Option<[f32; 3]>,
    // ... more optional fields
}

impl TileVisualMetadata {
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn with_color_tint(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color_tint = Some([r, g, b]);
        self
    }
}

// Usage:
let visual = TileVisualMetadata::default()
    .with_height(2.0)
    .with_color_tint(1.0, 0.5, 0.5);
```

### 4. Effective Value Pattern

**Lesson:** Provide "effective value" methods that handle Option unwrapping with defaults.

**Pattern:**

```rust
impl TileVisualMetadata {
    pub const DEFAULT_HEIGHT: f32 = 1.0;

    pub fn effective_height(&self) -> f32 {
        self.height.unwrap_or(Self::DEFAULT_HEIGHT)
    }
}

// Usage: Always get a valid value
let height = tile.visual.effective_height(); // Never panics
```

**Benefits:**

- Callers don't need to handle Option
- Centralized default value logic
- Easier to read and maintain

---

## Data Structure Design

### 1. Enum Over Booleans for State

**Lesson:** Use enums instead of multiple boolean flags or Option<T> for location tracking.

**Evidence from CharacterLocation migration:**

```rust
// ❌ BAD: Old approach
pub struct Character {
    pub at_inn: Option<InnkeeperId>, // None = in party, Some = at inn (innkeeper ID)
}

// ✅ GOOD: New approach
pub enum CharacterLocation {
    InParty,
    AtInn(InnkeeperId),
    OnMap(MapId),
}
```

**Benefits:**

- Type-safe state representation
- Impossible to represent invalid states
- Clear intent in code
- Easy to extend (added OnMap later)

### 2. Parallel Vectors for Associated Data

**Lesson:** When you need 1:1 associated data, use parallel vectors with invariant enforcement.

**Evidence from Roster:**

```rust
pub struct Roster {
    pub characters: Vec<Character>,
    pub character_locations: Vec<CharacterLocation>,
}

impl Roster {
    pub fn add_character(
        &mut self,
        character: Character,
        location: CharacterLocation
    ) -> Result<(), CharacterError> {
        // Maintain invariant: same length
        self.characters.push(character);
        self.character_locations.push(location);
        Ok(())
    }
}
```

**Invariant to maintain:**

- `characters.len() == character_locations.len()` ALWAYS

**Test for invariants:**

```rust
#[test]
fn test_roster_invariants() {
    let roster = create_test_roster();
    assert_eq!(
        roster.characters.len(),
        roster.character_locations.len()
    );
}
```

### 3. HashMap Iteration is Non-Deterministic

**CRITICAL LESSON:** Never assume HashMap iteration order in tests or logic.

**Evidence from test fixes:**

```rust
// ❌ BAD: Assumes insertion order
for i in 0..2 {
    map.insert(format!("char_{}", i), character);
}
// Later assumes "char_0" is first - WRONG!

// ✅ GOOD: Query actual state
let first_char = roster.characters.first().unwrap();
let expected_name = first_char.name.clone();
// Test based on actual values
```

**Collections with non-deterministic iteration:**

- `HashMap<K, V>` - hash-based, randomized order
- `HashSet<T>` - hash-based, randomized order

**Use instead:**

- `BTreeMap` / `BTreeSet` - sorted, deterministic
- `Vec` - insertion order preserved
- Or query by key/ID instead of assuming position

### 4. Type Aliases for Domain Concepts

**Lesson:** Use type aliases to make domain concepts explicit.

**Pattern:**

```rust
pub type ItemId = u32;
pub type SpellId = u32;
pub type MonsterId = u32;
pub type MapId = u16;
pub type TownId = u32;
pub type CharacterId = String;

// ✅ GOOD: Clear intent
pub fn equip_item(character_id: CharacterId, item_id: ItemId) -> Result<()>;

// ❌ BAD: Unclear
pub fn equip_item(character: u32, item: u32) -> Result<()>;
```

**Benefits:**

- Self-documenting code
- Easy to change underlying type later
- Prevents mixing up parameters

---

## Testing Strategies

### 1. Test Behavior, Not Implementation

**Lesson:** Tests should verify what the system does, not how it does it.

**Evidence from save/load tests:**

```rust
// ✅ GOOD: Tests observable behavior
#[test]
fn test_save_party_locations() {
    let mut state = GameState::new();
    add_characters_to_party(&mut state, 3);

    manager.save("test", &state).unwrap();
    let loaded = manager.load("test").unwrap();

    // Verify behavior: party state preserved
    assert_eq!(loaded.party.members.len(), 3);
    assert_eq!(loaded.roster.character_locations[0], CharacterLocation::InParty);
}

// ❌ BAD: Tests implementation details
#[test]
fn test_save_uses_ron_format() {
    manager.save("test", &state).unwrap();
    let file_content = std::fs::read_to_string("test.ron").unwrap();
    assert!(file_content.contains("(version:")); // Fragile!
}
```

### 2. Three-Category Test Structure

**Lesson:** Organize tests into Arrange-Act-Assert pattern.

**Pattern:**

```rust
#[test]
fn test_recruit_character_to_party() {
    // ARRANGE: Set up initial state
    let mut state = GameState::new();
    let character = create_test_character("Hero");
    state.roster.add_character(character, CharacterLocation::AtInn(1)).unwrap();

    // ACT: Perform the operation
    let result = state.recruit_character(0);

    // ASSERT: Verify outcomes
    assert!(result.is_ok());
    assert_eq!(state.party.size(), 1);
    assert_eq!(state.roster.character_locations[0], CharacterLocation::InParty);
}
```

### 3. Test Edge Cases and Boundaries

**Lesson:** Always test limits, not just happy paths.

**Evidence from party management tests:**

```rust
#[test]
fn test_party_max_members() {
    let mut party = Party::new();

    // Fill to max (6)
    for i in 0..Party::MAX_MEMBERS {
        party.add_member(create_character(i)).unwrap();
    }

    // Try adding one more - should fail
    let result = party.add_member(create_character(99));
    assert!(matches!(result, Err(CharacterError::PartyFull(_))));
}

#[test]
fn test_dismiss_last_member_fails() {
    let mut state = create_state_with_one_character();

    let result = state.dismiss_character(0, 1);
    assert!(matches!(result, Err(PartyManagementError::PartyEmpty)));
}
```

### 4. Unit vs Integration Tests

**Lesson:** Use both unit tests (fast, focused) and integration tests (realistic, end-to-end).

**Unit Test Pattern:**

```rust
// Fast, isolated, tests one function
#[test]
fn test_party_manager_recruit() {
    let mut party = Party::new();
    let mut roster = Roster::new();
    roster.add_character(char, CharacterLocation::AtInn(1)).unwrap();

    let result = PartyManager::recruit_to_party(&mut party, &mut roster, 0);

    assert!(result.is_ok());
}
```

**Integration Test Pattern:**

```rust
// Slower, realistic, tests full workflow
#[test]
fn test_full_save_load_cycle_with_recruitment() {
    let temp_dir = TempDir::new().unwrap();
    let manager = SaveGameManager::new(temp_dir.path()).unwrap();
    let mut state = GameState::new();

    // Full workflow: recruit → save → load
    add_characters(&mut state);
    state.recruit_character(0).unwrap();
    manager.save("test", &state).unwrap();

    let loaded = manager.load("test").unwrap();
    assert_eq!(loaded.party.size(), 1);
    assert!(loaded.encountered_characters.contains("npc_id"));
}
```

### 5. Test Helpers for Common Setup

**Lesson:** Create helper functions to reduce boilerplate and improve readability.

**Pattern:**

```rust
// Helper function
fn create_test_character(name: &str) -> Character {
    Character::new(
        name.to_string(),
        "human".to_string(),
        "knight".to_string(),
        Sex::Male,
        Alignment::Good,
    )
}

// Usage in tests
#[test]
fn test_something() {
    let char1 = create_test_character("Hero1");
    let char2 = create_test_character("Hero2");
    // Test code is now more readable
}
```

### 6. Simulating Old Data Formats

**Lesson:** Test migration by simulating old save formats.

**Evidence from save migration test:**

```rust
#[test]
fn test_save_migration_from_old_format() {
    // Save normally
    manager.save("test", &state).unwrap();

    // Manually remove new field to simulate old format
    let save_path = manager.save_path("test");
    let mut ron_content = std::fs::read_to_string(&save_path).unwrap();
    ron_content = remove_field(&ron_content, "encountered_characters");
    std::fs::write(&save_path, &ron_content).unwrap();

    // Load should succeed with defaults
    let loaded = manager.load("test").unwrap();
    assert_eq!(loaded.encountered_characters.len(), 0); // Default
}
```

---

## UI/UX Development

### 1. State Management in UI

**Lesson:** Keep UI state separate from game state.

**Evidence from RecruitmentDialog:**

```rust
#[derive(Resource, Default)]
pub struct RecruitmentDialogState {
    pub active: bool,
    pub character_id: String,
    pub character_name: String,
    // UI-only state
}

// Game state remains clean
pub struct GameState {
    pub roster: Roster,
    pub party: Party,
    // No UI state here
}
```

### 2. Visual Feedback for User Actions

**Lesson:** Always provide immediate visual feedback.

**Pattern from Map Editor:**

```rust
// Selection highlighting
if is_selected {
    ui.visuals_mut().override_text_color = Some(Color32::YELLOW);
}

// Hover feedback
if ui.response().hovered() {
    ui.painter().rect_stroke(/* highlight border */);
}

// Disabled state
ui.add_enabled(can_recruit, egui::Button::new("Recruit"));
```

### 3. Autocomplete for User Input

**Lesson:** Provide autocomplete for IDs and references to reduce errors.

**Evidence from Portrait Selector:**

```rust
pub fn autocomplete_portrait_selector(
    ui: &mut egui::Ui,
    current_value: &mut String,
    candidates: &[String],
    label: &str,
) {
    ui.text_edit_singleline(current_value);

    // Filter candidates based on input
    let matches: Vec<_> = candidates
        .iter()
        .filter(|c| c.contains(current_value.as_str()))
        .collect();

    // Show dropdown with matches
    if !matches.is_empty() {
        egui::ComboBox::from_label(label)
            .selected_text(current_value.as_str())
            .show_ui(ui, |ui| {
                for candidate in matches {
                    ui.selectable_value(current_value, candidate.clone(), candidate);
                }
            });
    }
}
```

### 4. Preview Before Commit

**Lesson:** Show previews of user choices before applying changes.

**Evidence from Portrait Grid Picker:**

```rust
// Show portrait preview
if let Some(texture) = self.load_portrait_texture(ctx, portrait_id) {
    ui.image(texture, [64.0, 64.0]);
}

// Only commit on explicit action
if ui.button("Select").clicked() {
    self.selected_character.portrait_id = portrait_id.clone();
    self.show_portrait_picker = false; // Close popup
}
```

### 5. Tooltips for All Interactive Elements

**Lesson:** Add tooltips to explain functionality, especially for IDs.

**Pattern:**

```rust
ui.text_edit_singleline(&mut self.portrait_id)
    .on_hover_text("Portrait asset ID (e.g., '10', 'hero_1'). \
                    Use the picker button to browse available portraits.");

if ui.button("🎨").on_hover_text("Open portrait picker").clicked() {
    self.show_portrait_picker = true;
}
```

---

## Data Migration & Backward Compatibility

### 1. Use Serde Defaults for New Fields

**Lesson:** Add `#[serde(default)]` to new fields for backward compatibility.

**Evidence from encountered_characters:**

```rust
pub struct GameState {
    pub world: World,
    pub roster: Roster,

    // NEW field - old saves won't have this
    #[serde(default)]
    pub encountered_characters: HashSet<String>,
}
```

**Benefits:**

- Old saves deserialize successfully
- New field gets default value automatically
- No explicit migration code needed

### 2. Migration Scripts for Data Files

**Lesson:** Create migration scripts for bulk data updates.

**Evidence from event trigger removal:**

```rust
// migrations/remove_event_triggers.py
def migrate_map(map_path):
    with open(map_path) as f:
        data = ron.load(f)

    # Remove deprecated fields
    for tile in data['tiles']:
        if 'on_step' in tile:
            del tile['on_step']

    # Write back
    with open(map_path, 'w') as f:
        ron.dump(data, f)
```

### 3. Version Tracking in Save Files

**Lesson:** Always include version numbers in serialized data.

**Pattern:**

```rust
#[derive(Serialize, Deserialize)]
pub struct SaveGame {
    pub version: String, // e.g., "0.1.0"
    pub timestamp: DateTime<Utc>,
    pub game_state: GameState,
}

impl SaveGame {
    pub fn validate_version(&self) -> Result<(), SaveGameError> {
        let current = env!("CARGO_PKG_VERSION");
        if self.version != current {
            return Err(SaveGameError::VersionMismatch {
                expected: current.to_string(),
                found: self.version.clone(),
            });
        }
        Ok(())
    }
}
```

### 4. Explicit Overrides in Data Files

**Lesson:** Allow data files to override calculated values for flexibility.

**Evidence from CharacterDefinition:**

```rust
pub struct CharacterDefinition {
    pub base_stats: BaseStats,

    // Optional overrides
    pub hp_base: Option<u16>, // If Some, use this instead of calculation
    pub sp_base: Option<u16>,
}

// In instantiation:
let hp = def.hp_base.unwrap_or_else(|| {
    calculate_hp_from_class_and_endurance(&def, class_def)
});
```

**Use case:** Game designers can balance characters without changing class definitions.

---

## Error Handling

### 1. Use thiserror for Custom Error Types

**Lesson:** Define domain-specific error types with thiserror.

**Pattern:**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PartyManagementError {
    #[error("Party is full (max {0} members)")]
    PartyFull(usize),

    #[error("Cannot dismiss last party member")]
    PartyEmpty,

    #[error("Character at roster index {0} not found")]
    InvalidRosterIndex(usize),

    #[error("Character error: {0}")]
    CharacterError(#[from] CharacterError),
}
```

**Benefits:**

- Descriptive error messages
- Automatic Display implementation
- Easy error propagation with `?`

### 2. Result Types Everywhere

**Lesson:** Use `Result<T, E>` for all operations that can fail.

**Pattern:**

```rust
// ✅ GOOD: Returns Result
pub fn recruit_character(&mut self, roster_index: usize)
    -> Result<(), PartyManagementError>
{
    if self.party.is_full() {
        return Err(PartyManagementError::PartyFull(Party::MAX_MEMBERS));
    }
    // ... operation
    Ok(())
}

// ❌ BAD: Panics on error
pub fn recruit_character(&mut self, roster_index: usize) {
    assert!(!self.party.is_full(), "Party is full!");
    // ...
}
```

### 3. Validate Early, Fail Fast

**Lesson:** Validate inputs at function entry.

**Pattern:**

```rust
pub fn swap_party_member(
    party: &mut Party,
    roster: &mut Roster,
    party_index: usize,
    roster_index: usize,
) -> Result<(), PartyManagementError> {
    // Validate ALL inputs first
    if party_index >= party.members.len() {
        return Err(PartyManagementError::InvalidPartyIndex(party_index));
    }
    if roster_index >= roster.characters.len() {
        return Err(PartyManagementError::InvalidRosterIndex(roster_index));
    }

    // Now perform operation knowing inputs are valid
    // ...
}
```

### 4. Specific Error Variants

**Lesson:** Create specific error variants rather than generic messages.

**Pattern:**

```rust
// ✅ GOOD: Specific variants
pub enum RecruitmentError {
    AlreadyEncountered(String),
    CharacterNotFound(String),
    PartyFull,
}

// ❌ BAD: Generic variant
pub enum RecruitmentError {
    Error(String), // What went wrong? Who knows!
}
```

---

## Code Quality & Validation

### 1. The Four Quality Gates

**Lesson:** Every commit must pass all four checks.

**Mandatory checks:**

```bash
# 1. Formatting (auto-fixes)
cargo fmt --all

# 2. Compilation (fast check)
cargo check --all-targets --all-features

# 3. Linting (catches common mistakes)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Testing (verifies behavior)
cargo nextest run --all-features
```

**Integration into workflow:**

- Run before every commit
- Add as pre-commit hooks
- Required in CI/CD pipeline

### 2. Clippy Warnings as Errors

**Lesson:** Treat warnings as errors to maintain code quality.

**Configuration:**

```bash
# In CI/CD and locally:
cargo clippy --all-targets --all-features -- -D warnings
```

**Common clippy catches:**

- Unused variables
- Unnecessary clones
- Complex boolean expressions
- Non-idiomatic patterns

### 3. Documentation Comments for Public API

**Lesson:** Every public item needs doc comments with examples.

**Pattern:**

````rust
/// Recruits a character from the roster to the active party
///
/// Moves a character from inn/map storage to the active adventuring party.
/// The character must not already be in the party, and the party must have
/// space available.
///
/// # Arguments
///
/// * `roster_index` - Index of the character in the roster to recruit
///
/// # Returns
///
/// Returns `Ok(())` if recruitment succeeds
///
/// # Errors
///
/// - `PartyManagementError::PartyFull` if party is at max size (6)
/// - `PartyManagementError::AlreadyInParty` if character already in party
///
/// # Examples
///
/// ```
/// use antares::application::GameState;
///
/// let mut state = GameState::new();
/// state.recruit_character(0)?;
/// ```
pub fn recruit_character(&mut self, roster_index: usize)
    -> Result<(), PartyManagementError>
{
    // ...
}
````

### 4. Validation at Multiple Layers

**Lesson:** Validate data at SDK, domain, and application layers.

**Layers:**

```rust
// Layer 1: SDK validation (campaign authoring time)
impl MapBlueprint {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check NPC references exist
        // Check event data is valid
    }
}

// Layer 2: Domain validation (data load time)
impl NpcDatabase {
    pub fn validate(&self) -> Result<(), NpcError> {
        // Check no duplicate IDs
        // Check required fields present
    }
}

// Layer 3: Runtime validation (execution time)
pub fn trigger_event(event_id: u32) -> Result<(), EventError> {
    // Check event exists
    // Check prerequisites met
}
```

---

## Common Pitfalls & Solutions

### 1. Hardcoded Constants

**Pitfall:** Magic numbers scattered throughout code.

**Solution:** Extract constants.

```rust
// ❌ BAD
if inventory.items.len() >= 20 {
    return Err("Inventory full");
}

// ✅ GOOD
impl Inventory {
    pub const MAX_ITEMS: usize = 20;
}

if inventory.items.len() >= Inventory::MAX_ITEMS {
    return Err(InventoryError::Full(Inventory::MAX_ITEMS));
}
```

### 2. Modifying Core Structs Without Architecture Review

**Pitfall:** Adding fields to core data structures impulsively.

**Solution:** Check architecture.md first, discuss changes.

```rust
// ❌ BAD: Adding field without review
pub struct Character {
    pub name: String,
    pub level: u8,
    pub my_new_field: bool, // STOP! Is this in architecture?
}

// ✅ GOOD: Follow architecture.md definitions
// If field not in architecture, propose in discussion first
```

### 3. Assuming Collection Order

**Pitfall:** Tests/code assumes HashMap iteration order.

**Solution:** Query by key or use ordered collections.

```rust
// ❌ BAD
let first_char = roster.characters[0]; // Assumes order

// ✅ GOOD
let first_char = roster.characters
    .iter()
    .find(|c| c.id == "specific_id")
    .expect("Character not found");
```

### 4. Not Testing Backward Compatibility

**Pitfall:** Adding new fields breaks old save files.

**Solution:** Add `#[serde(default)]` and write migration tests.

```rust
// ✅ GOOD
pub struct GameState {
    #[serde(default)]
    pub new_field: HashSet<String>,
}

#[test]
fn test_load_old_save_without_new_field() {
    let old_save = simulate_old_format();
    let loaded = load_save(old_save);
    assert_eq!(loaded.new_field.len(), 0); // Default value
}
```

### 5. UI State in Domain Objects

**Pitfall:** Mixing UI state with game state.

**Solution:** Keep UI state in separate resources/components.

```rust
// ❌ BAD
pub struct Character {
    pub name: String,
    pub is_ui_selected: bool, // UI state doesn't belong here!
}

// ✅ GOOD
pub struct Character {
    pub name: String,
    // Only domain data
}

#[derive(Resource)]
pub struct CharacterEditorState {
    pub selected_character_id: Option<String>, // UI state here
}
```

---

## Performance Optimization

### 1. Mesh Caching for Repeated Geometry

**Lesson:** Cache expensive operations keyed by input parameters.

**Evidence from TileVisualMetadata rendering:**

```rust
type MeshDimensions = (f32, f32, f32); // (height, width_x, width_z)
type MeshCache = HashMap<MeshDimensions, Handle<Mesh>>;

fn get_or_create_mesh(
    meshes: &mut Assets<Mesh>,
    cache: &mut MeshCache,
    dimensions: MeshDimensions,
) -> Handle<Mesh> {
    cache.entry(dimensions).or_insert_with(|| {
        meshes.add(create_box_mesh(dimensions.0, dimensions.1, dimensions.2))
    }).clone()
}
```

### 2. Lazy Loading for Heavy Resources

**Lesson:** Load textures/assets on-demand, not all upfront.

**Evidence from Portrait Grid Picker:**

```rust
// Load on first access
pub fn load_portrait_texture(
    &mut self,
    ctx: &egui::Context,
    portrait_id: &str,
) -> Option<egui::TextureHandle> {
    // Check cache first
    if let Some(handle) = self.portrait_cache.get(portrait_id) {
        return Some(handle.clone());
    }

    // Load and cache
    if let Ok(texture) = load_image_from_path(portrait_path) {
        let handle = ctx.load_texture(portrait_id, texture, Default::default());
        self.portrait_cache.insert(portrait_id.to_string(), handle.clone());
        Some(handle)
    } else {
        None
    }
}
```

### 3. Avoid Unnecessary Clones

**Lesson:** Use references where possible, clone only when needed.

```rust
// ✅ GOOD: Borrow where possible
pub fn get_character(&self, index: usize) -> Option<&Character> {
    self.roster.characters.get(index)
}

// ❌ BAD: Unnecessary clone
pub fn get_character(&self, index: usize) -> Option<Character> {
    self.roster.characters.get(index).cloned() // Wasteful!
}
```

---

## Documentation Best Practices

### 1. Implementation Summaries

**Lesson:** Document WHAT was implemented, WHY decisions were made, and HOW to use it.

**Template:**

```markdown
## Phase X: Feature Name - COMPLETED

### Summary

One-paragraph overview of what was implemented.

### Changes Made

Detailed list of files and functions modified.

### Technical Decisions

Explain WHY you chose this approach over alternatives.

### Testing

List test coverage and results.

### Known Limitations

Be honest about what doesn't work yet.

### Next Steps

Point to follow-up work needed.
```

### 2. Inline Documentation

**Lesson:** Explain the "why", not the "what".

```rust
// ❌ BAD: Restates the code
// Increment i
i += 1;

// ✅ GOOD: Explains the reason
// Skip the first character (index 0) because it's the BOM marker
i += 1;

// ✅ GOOD: Explains non-obvious behavior
// CharacterDatabase uses HashMap internally, so iteration order
// is non-deterministic. We must find characters by ID, not index.
for def in db.characters.premade_characters() {
    // ...
}
```

### 3. Examples in Documentation

**Lesson:** Every public function should have a usage example.

**Pattern:**

````rust
/// # Examples
///
/// ```
/// use antares::application::GameState;
/// use antares::domain::character::CharacterLocation;
///
/// let mut state = GameState::new();
/// state.roster.add_character(character, CharacterLocation::InParty)?;
/// assert_eq!(state.party.size(), 1);
/// ```
````

---

## Summary of Key Takeaways

### Architecture

1. ✅ Separate domain, UI, and infrastructure layers
2. ✅ Use message-based architecture for decoupling
3. ✅ Keep game state pure and serializable

### Data Structures

4. ✅ Use enums over booleans for state representation
5. ✅ HashMap iteration is non-deterministic - query by key
6. ✅ Maintain parallel vector invariants rigorously
7. ✅ Type aliases make domain concepts explicit

### Testing

8. ✅ Test behavior, not implementation
9. ✅ Always test edge cases and boundaries
10. ✅ Use both unit and integration tests
11. ✅ Create test helpers to reduce boilerplate

### Error Handling

12. ✅ Use `Result<T, E>` everywhere
13. ✅ Define specific error types with thiserror
14. ✅ Validate early, fail fast

### Quality

15. ✅ Four quality gates: fmt, check, clippy, test
16. ✅ Treat warnings as errors
17. ✅ Document all public APIs with examples

### Backward Compatibility

18. ✅ Use `#[serde(default)]` for new fields
19. ✅ Version all serialized data
20. ✅ Test migration from old formats

### UI/UX

21. ✅ Keep UI state separate from game state
22. ✅ Provide visual feedback for all actions
23. ✅ Use autocomplete for ID entry
24. ✅ Show previews before committing changes

### Performance

25. ✅ Cache expensive operations
26. ✅ Lazy load heavy resources
27. ✅ Avoid unnecessary clones

---

## Conclusion

These lessons learned represent months of development across multiple feature areas. The patterns that emerged - separation of concerns, robust error handling, comprehensive testing, and careful data structure design - are applicable to any Rust game development project.

**Most Important Lesson:** Follow the architecture document, run the quality gates, write tests for everything, and document your decisions. When in doubt, look at existing implementations for patterns to follow.

---

**Document maintained by:** Development Team
**Based on:** All phases documented in `implementations.md`
**Next update:** As new phases are completed
