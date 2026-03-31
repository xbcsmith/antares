# Game Feature Completion Plan

## Overview

This plan addresses incomplete game systems, placeholder stubs, and missing
features in the Antares game engine. The analysis identified **24 placeholder
stubs** from the codebase audit plus **4 user-reported issues** covering game
log placement, recruited character mesh persistence, time advancement
granularity, and lock UI input handling. The work is organized into five
phases, ordered by player-visible impact (highest first) and implementation
dependency (foundations first).

We do not care about backwards compatibility.

## Current State Analysis

### Existing Infrastructure

- **Time system**: `GameTime` struct in `src/domain/types.rs` with
  minute-level resolution, `advance_minutes()`, `advance_hours()`, and
  `advance_days()` methods. A single authoritative `GameState::advance_time()`
  in `src/application/mod.rs` L1746 handles all time advancement including
  active spell ticking, stat boost expiry, and merchant restocking.
- **Time constants**: `TIME_COST_STEP_MINUTES = 5`,
  `TIME_COST_COMBAT_ROUND_MINUTES = 5`, `TIME_COST_MAP_TRANSITION_MINUTES = 30`
  defined as compile-time constants in `src/domain/resources.rs` L76–84.
- **Game log**: Full category-based filter system with 5 categories
  (`Combat`, `Dialogue`, `Item`, `Exploration`, `System`) in
  `src/game/systems/ui.rs`. Currently positioned bottom-left at 300×200px.
  Toggle via `L` key. No full-screen log view exists.
- **Lock UI**: Runs as an egui overlay during `GameMode::Exploration` with no
  dedicated `GameMode` variant. Input falls through to exploration systems.
  `LockUiPlugin` registers systems with no ordering constraints relative to
  `InputPlugin`.
- **Recruitment**: Dialogue-driven recruitment via `execute_recruit_to_party`
  in `src/game/systems/dialogue.rs` L699–827 correctly despawns meshes. The
  `RecruitmentDialogPlugin` path in `recruitment_dialog.rs` is dead code that
  lacks despawn logic. `RecruitToInn` dialogue action removes the map event
  but does not emit `DespawnRecruitableVisual`.
- **Audio**: `AudioPlugin` in `src/game/systems/audio.rs` is logging-only.
  `handle_audio_messages` calls `info!()`/`debug!()` but plays no sound.
- **Events**: `MapEvent::Trap` and `MapEvent::Treasure` in
  `src/game/systems/events.rs` log messages but never apply damage or add
  items to inventory.

### Identified Issues

1. **Game log is in the wrong position** — bottom-left obscures the game
   world line of sight. Should be upper-left.
2. **No full-screen game log view** — only a 300×200px panel with 12 visible
   lines. No way to review full history.
3. **Time advancement is too coarse** — `GameTime` has minute-level resolution
   only. User wants 30-second movement steps and 10-second combat turns, both
   configurable.
4. **Combat time charges per round, not per turn** — `tick_combat_time` tracks
   `last_timed_round` but user wants per-turn charging.
5. **Lock UI input falls through** — arrow keys move the party while the lock
   prompt is open; ESC both closes the prompt and opens the game menu.
6. **Recruited character mesh may persist** — `RecruitToInn` action and the
   dead-code `RecruitmentDialogPlugin` path lack `DespawnRecruitableVisual`
   emission.
7. **24 placeholder stubs** — traps, treasure, recruitment actions, audio,
   mesh streaming, LOD, inn identity, and multiple validation stubs are no-ops.

## Implementation Phases

### Phase 1: Input and UI Fixes (Highest Player-Visible Impact)

These are bugs the player encounters every session. Fix them first.

#### 1.1 Fix Lock UI Input Consumption

**Root cause**: The lock prompt runs during `GameMode::Exploration` with no
input coordination. Both `handle_global_input_toggles` and
`handle_exploration_input_movement` execute normally.

**Files**:

- `src/game/systems/input.rs` — `handle_global_input_toggles` (L144),
  `handle_exploration_input_movement` (L219)
- `src/game/systems/input/global_toggles.rs` — `handle_global_mode_toggles`
  (L66)
- `src/game/systems/input/mode_guards.rs` — `movement_blocked_for_mode` (L45)
- `src/game/systems/lock_ui.rs` — `lock_prompt_ui_system` (L152),
  `LockUiPlugin::build` (L42), `LockNavState` (L112)
- `src/game/resources/mod.rs` — `LockInteractionPending` (L155)

**Changes**:

1. Add `Res<LockInteractionPending>` as a system parameter to
   `handle_global_input_toggles` and `handle_exploration_input_movement`.
   Early-return from both when `lock_pending.lock_id.is_some()`. This blocks
   ESC menu toggle and arrow-key movement while the lock prompt is visible.
2. Add arrow key navigation to `lock_prompt_ui_system` — Up/Down to cycle
   through party members (currently only digit keys 1–6 work). This makes
   the lock prompt navigable without a keyboard number row.
3. Add system ordering: `.before(handle_global_input_toggles)` on
   `lock_prompt_ui_system` registration in `LockUiPlugin::build` to ensure
   lock UI processes input first.

#### 1.2 Relocate Game Log to Upper-Left Corner

**Files**:

- `src/game/systems/ui.rs` — `setup_game_log_panel` (L321–476)

**Changes**:

1. Change positioning from `bottom` + `left` to `top` + `left`:
   - Remove `bottom: Val::Px(hud_height + hud_gap + 8.0)` (L345)
   - Add `top: Val::Px(8.0)` to place the panel in the upper-left corner
   - Keep `left: Val::Px(8.0)`
2. Verify the panel does not overlap with any existing top-left UI elements
   (combat log bubble is top-right, so no conflict).

#### 1.3 Implement Full-Screen Game Log View

**Files**:

- `src/game/systems/ui.rs` — new systems and components
- `src/application/mod.rs` — `GameMode` enum (L48)
- `src/game/systems/input/mode_guards.rs` — `movement_blocked_for_mode`
- `src/sdk/game_config.rs` — `GameLogConfig` (L247)

**Changes**:

1. Add `GameMode::GameLog` variant to the `GameMode` enum in
   `src/application/mod.rs`.
2. Add `GameMode::GameLog` to `movement_blocked_for_mode` in `mode_guards.rs`
   so all exploration input is blocked while viewing the full log.
3. Create `spawn_fullscreen_game_log` system that renders a full-screen
   overlay with:
   - Scrollable list of all `GameLog` entries (not limited to 12 lines)
   - Category filter buttons (reuse existing `LogCategory` filter toggle
     logic from `handle_log_filter_buttons`)
   - Search/text filter input field (optional, can defer)
   - Close button and ESC to return to previous mode
4. Add a toggle mechanism: clicking the "Game Log" header text in the small
   log panel (or pressing a configurable key) switches to
   `GameMode::GameLog`. Add `fullscreen_toggle_key` to `GameLogConfig`
   (default: `KeyCode::KeyG` or reuse `L` as a second press).
5. Handle `GameMode::GameLog` in `handle_global_mode_toggles` — ESC returns
   to the previous `GameMode`.
6. Add `sync_fullscreen_game_log_ui` system that renders all filtered entries
   with scroll support.
7. Add `GameMode::GameLog` to the combat log visibility sync so the small
   panel hides when the full-screen view is active.

#### 1.4 Fix Recruited Character Mesh Persistence

**Files**:

- `src/game/systems/dialogue.rs` — `execute_action` `RecruitToInn` branch
  (L1089–1103)
- `src/game/systems/recruitment_dialog.rs` — `process_recruitment_responses`
  (L163–191)
- `src/game/systems/dialogue.rs` — `handle_recruitment_actions` stub
  (L1649–1705)

**Changes**:

1. In `execute_action`'s `RecruitToInn` branch (dialogue.rs ~L1089–1103),
   after `remove_event()` succeeds, emit `DespawnRecruitableVisual` — matching
   the pattern already used in `execute_recruit_to_party`. This requires
   adding the `MessageWriter<DespawnRecruitableVisual>` parameter.
2. Fix `process_recruitment_responses` in `recruitment_dialog.rs` (currently
   dead code but should be correct for future use):
   - Add `MessageWriter<DespawnRecruitableVisual>` to system params
   - After `recruit_from_map()` succeeds, scan the current map for the
     matching `RecruitableCharacter` event, call `remove_event()`, and emit
     `DespawnRecruitableVisual`
   - The `RecruitmentResponseMessage::Accept` variant needs to carry the event
     `Position` (or the system must look it up by `character_id`)
3. Remove or complete the dead-code stub `handle_recruitment_actions`
   (dialogue.rs L1649–1705) — it runs every frame during dialogue, reads
   actions, and logs `info!` but does nothing. Since `execute_action` already
   handles recruitment, this stub should be removed.

#### 1.5 Testing Requirements

- Test that ESC from the lock prompt does NOT open the game menu.
- Test that arrow keys during lock prompt do NOT move the party.
- Test that arrow keys cycle through party members in the lock prompt.
- Test game log panel renders in upper-left quadrant of the screen.
- Test full-screen log view opens and closes via toggle key and ESC.
- Test full-screen log view respects category filters.
- Test `RecruitToInn` action despawns the recruitable visual entity.
- Test `process_recruitment_responses` Accept path removes event and despawns.

#### 1.6 Deliverables

- [x] Lock UI blocks exploration movement and ESC menu toggle
- [x] Lock UI supports arrow key navigation for character selection
- [x] Game log relocated to upper-left corner
- [x] Full-screen game log view implemented with scroll and category filters
- [x] Full-screen log toggle from small panel header and configurable key
- [x] `RecruitToInn` dialogue action emits `DespawnRecruitableVisual`
- [x] Dead-code `handle_recruitment_actions` stub removed
- [x] `process_recruitment_responses` fixed for future use

#### 1.7 Success Criteria

- Lock prompt consumes all keyboard input exclusively.
- Game log panel appears in upper-left corner, not bottom-left.
- Full-screen log is scrollable, filterable, and dismissable via ESC.
- Recruited NPCs disappear immediately on all recruitment paths.
- All quality gates pass.

---

### Phase 2: Time Advancement System (Core Game Mechanic)

Time advancement affects resource consumption, spell durations, merchant
restocking, and day/night cycles. Getting the granularity right is foundational
for all other time-dependent systems.

#### 2.1 Add Sub-Minute Resolution to `GameTime`

**Files**:

- `src/domain/types.rs` — `GameTime` struct (L511–524)

**Changes**:

1. Add `second: u8` field to `GameTime` with `#[serde(default)]` for save
   compatibility:
   ```
   /// Current second (0-59)
   #[serde(default)]
   pub second: u8,
   ```
2. Add `advance_seconds(seconds: u32)` method that rolls over into minutes,
   hours, days, months, years. This becomes the new primitive — all other
   advance methods delegate to it:
   - `advance_minutes(m)` → `advance_seconds(m * 60)`
   - `advance_hours(h)` → `advance_seconds(h * 3600)`
   - `advance_days(d)` → `advance_seconds(d * 86400)`
3. Update `Display` / `format` implementations to include seconds where
   appropriate.
4. Update all tests in `types.rs` for the new field and method.

#### 2.2 Add `TimeConfig` to Game Configuration

**Files**:

- `src/sdk/game_config.rs` — `GameConfig` struct
- `src/domain/resources.rs` — time cost constants (L76–84)

**Changes**:

1. Add `TimeConfig` sub-struct to `GameConfig`:
   ```
   TimeConfig {
       movement_step_seconds: u32,       // default: 30
       combat_turn_seconds: u32,         // default: 10
       map_transition_seconds: u32,      // default: 1800 (30 minutes)
       portal_transition_seconds: u32,   // default: 0 (instant)
   }
   ```
2. Keep the existing constants in `resources.rs` as fallback defaults but make
   them overridable by `TimeConfig` values loaded from the campaign config.
3. Add `TimeConfig` deserialization to campaign `config.ron`.
4. Update `data/test_campaign/config.ron` with `TimeConfig` defaults.

#### 2.3 Update `GameState::advance_time` for Seconds

**Files**:

- `src/application/mod.rs` — `advance_time` (L1746–1766)

**Changes**:

1. Change signature from `advance_time(minutes: u32, ...)` to
   `advance_time_seconds(seconds: u32, ...)`.
2. Update the internal tick loop — active spells and timed stat boosts
   currently tick per-minute. Decide on granularity:
   - **Option A**: Keep ticking per-minute, only advance the clock in seconds.
     Accumulate seconds and tick effects when a full minute boundary is
     crossed. This minimizes churn.
   - **Option B**: Tick per-second for sub-minute effects. More precise but
     60× more loop iterations for the same real-time period.
   - **Recommended**: Option A. Spell durations and stat boosts are measured in
     minutes; sub-minute ticking adds complexity with no gameplay benefit.
3. Add a convenience `advance_time_minutes(minutes: u32, ...)` wrapper that
   calls `advance_time_seconds(minutes * 60, ...)` for callers that still
   think in minutes (rest, for example).

#### 2.4 Wire Time Advancement to Movement

**Files**:

- `src/application/mod.rs` — `move_party_and_handle_events` (L1308)

**Changes**:

1. Replace `self.advance_time(TIME_COST_STEP_MINUTES, None)` with
   `self.advance_time_seconds(config.time.movement_step_seconds, None)` where
   `config` is the loaded `TimeConfig`. If `TimeConfig` is not available, fall
   back to `30` seconds (the new default).
2. The `TimeConfig` should be accessible from `GameState` or passed as a
   parameter to `move_party_and_handle_events`.

#### 2.5 Wire Time Advancement to Combat (Per-Turn)

**Files**:

- `src/game/systems/combat.rs` — `tick_combat_time` (L4432–4447),
  `CombatResource` (L398)

**Changes**:

1. Add `last_timed_turn: usize` field to `CombatResource` (alongside existing
   `last_timed_round`).
2. Change `tick_combat_time` to track individual turn advancement instead of
   (or in addition to) round advancement:
   - On each frame, compare `combat.state.current_turn_index` to
     `last_timed_turn`.
   - For each new turn, call `advance_time_seconds(config.time.combat_turn_seconds, None)`.
3. Remove or keep `last_timed_round` depending on whether round-level tracking
   is still needed for other purposes.

#### 2.6 Wire Time Advancement to Portals (Instant)

**Files**:

- `src/game/systems/map.rs` — map transition handling (~L502)

**Changes**:

1. Distinguish between portal transitions and regular map transitions.
2. Portal transitions use `config.time.portal_transition_seconds` (default 0 —
   instant).
3. Regular map transitions continue using
   `config.time.map_transition_seconds` (default 1800 seconds = 30 minutes).
4. If the `MapTransition` event or portal data doesn't currently carry a
   `is_portal` flag, add one to the transition type or derive it from the
   event type (e.g., `MapEvent::Portal` vs `MapEvent::Exit`).

#### 2.7 Update HUD Clock Display

**Files**:

- `src/game/systems/hud.rs` — `update_clock` system

**Changes**:

1. If the clock display currently shows only `HH:MM`, update it to show
   `HH:MM:SS` or at least ensure it updates smoothly with sub-minute
   time changes (every 30 seconds for movement, every 10 seconds for combat).
2. Ensure the `TimeOfDay` label (Dawn/Morning/etc.) updates correctly with
   sub-minute advancement.

#### 2.8 Testing Requirements

- Test `GameTime::advance_seconds()` with rollover across seconds → minutes →
  hours → days → months → years.
- Test `advance_seconds(30)` from `12:00:00` yields `12:00:30`.
- Test `advance_seconds(90)` from `12:00:30` yields `12:02:00`.
- Test movement advances time by exactly `movement_step_seconds` per tile.
- Test combat advances time by exactly `combat_turn_seconds` per turn.
- Test portal transitions advance zero seconds by default.
- Test `TimeConfig` deserialization from RON.
- Test backward-compatible deserialization of `GameTime` without `second` field
  (defaults to 0).

#### 2.9 Deliverables

- [ ] `GameTime.second` field added with `advance_seconds()` method
- [ ] All existing advance methods delegate to `advance_seconds()`
- [ ] `TimeConfig` struct added to `GameConfig`
- [ ] `advance_time_seconds()` replaces `advance_time()` as primary method
- [ ] Movement wired to configurable seconds (default 30)
- [ ] Combat wired to per-turn configurable seconds (default 10)
- [ ] Portal transitions are instant (0 seconds)
- [ ] HUD clock updated for sub-minute display
- [ ] `data/test_campaign/config.ron` updated with `TimeConfig`

#### 2.10 Success Criteria

- Moving one tile advances the clock by exactly 30 seconds (default).
- Each combat turn advances the clock by exactly 10 seconds (default).
- Portal transitions do not advance the clock.
- Map transitions advance the clock by 30 minutes (default).
- Resting continues to work correctly (hours × 3600 seconds).
- All time-dependent systems (spells, stat boosts, restocking) continue to
  tick correctly at minute boundaries.
- All quality gates pass.

---

### Phase 3: Core Game Mechanics (Traps, Treasure, Recruitment)

These are fundamental RPG mechanics that are currently non-functional.

#### 3.1 Implement Trap Damage Application

**Files**:

- `src/game/systems/events.rs` — `MapEvent::Trap` handler (L349–359)
- `src/domain/resources.rs` or `src/domain/combat/` — damage application

**Changes**:

1. After logging the trap message, apply `damage` to party members:
   - Distribute damage across party (all members? random member? front-row
     only? — consult architecture.md for trap damage rules).
   - Respect resistances and conditions (e.g., dead members take no damage).
   - Use `Character::take_damage()` or equivalent.
2. Apply `effect` if present (the `effect` field is currently `_`-ignored).
   Effects could be conditions like poison, paralysis, etc.
3. Check for party wipe after trap damage — transition to `GameMode::GameOver`
   if all members are dead.
4. Log damage dealt per character to the game log with `LogCategory::Combat`.

#### 3.2 Implement Treasure Loot Distribution

**Files**:

- `src/game/systems/events.rs` — `MapEvent::Treasure` handler (L360–370)

**Changes**:

1. After logging the treasure message, distribute `loot` items to party:
   - Iterate through loot items.
   - For each item, find the first party member with inventory space and call
     `inventory.add_item()`.
   - If no member has space, log a warning ("Inventory full — item lost" or
     queue for ground drop).
   - Handle the `Result` from `add_item()` — do not use `let _ =`.
2. Add gold/gems if the treasure includes currency rewards.
3. Log each item received to the game log with `LogCategory::Item`.
4. Mark the treasure event as consumed so it doesn't respawn (call
   `current_map.remove_event(position)` or set a flag).

#### 3.3 Implement Dialogue Recruitment Actions

**Files**:

- `src/game/systems/dialogue.rs` — `handle_recruitment_actions` stub
  (L1649–1705)

Note: The `execute_recruit_to_party` function (L699–827) already works for the
dialogue-tree path. The stub at L1649–1705 is for the `DialogueAction` enum
variants `RecruitToParty` and `RecruitToInn` processed in `execute_action`.

**Changes**:

1. In `execute_action`'s `DialogueAction::RecruitToParty` branch
   (dialogue.rs ~L1677):
   - Call `game_state.recruit_from_map(character_id, content.db())`.
   - Handle `RecruitResult::AddedToParty` — log success.
   - Handle `RecruitResult::SentToInn` — log that the character was sent
     to the inn.
   - Remove the map event and emit `DespawnRecruitableVisual` (same pattern
     as `execute_recruit_to_party`).
2. In `execute_action`'s `DialogueAction::RecruitToInn` branch
   (dialogue.rs ~L1691):
   - Call the equivalent inn-assignment logic.
   - Remove the map event and emit `DespawnRecruitableVisual`.
3. Remove the dead-code `handle_recruitment_actions` stub (L1649–1705) since
   `execute_action` now handles both variants.

#### 3.4 Wire NPC Dialogue with `npc_id` Context

**Files**:

- `src/application/mod.rs` — `EventResult::NpcDialogue` handler (L1374–1378)
- `src/application/dialogue.rs` — `DialogueState` struct

**Changes**:

1. Stop discarding `npc_id` with `let _ = npc_id`.
2. Pass `npc_id` into `DialogueState::new()` (or a new constructor that
   accepts NPC context).
3. Store `npc_id` on `DialogueState` so dialogue systems can reference which
   NPC the party is speaking to (for NPC-specific responses, stock lookups,
   etc.).

#### 3.5 Implement Quest Reward `UnlockQuest`

**Files**:

- `src/application/quests.rs` — `QuestReward::UnlockQuest` handler (L309–312)

**Changes**:

1. Replace the no-op with actual quest availability logic:
   - Mark the target quest as available/unlocked in the quest log.
   - If a quest availability system doesn't exist yet, add an
     `available_quests: HashSet<QuestId>` field to `QuestLog` and check it
     in `start_quest`.
2. Log the unlock to the game log with `LogCategory::System`.

#### 3.6 Testing Requirements

- Test trap damage reduces party member HP by the correct amount.
- Test trap with effect applies the condition to party members.
- Test trap that kills all members triggers game over state.
- Test treasure items are distributed to party members with space.
- Test treasure with full inventories logs appropriate warnings.
- Test treasure event is consumed (not repeatable).
- Test `RecruitToParty` dialogue action adds character to party and despawns
  mesh.
- Test `RecruitToInn` dialogue action assigns character to inn and despawns
  mesh.
- Test NPC dialogue state carries `npc_id`.
- Test `UnlockQuest` makes the target quest available.

#### 3.7 Deliverables

- [ ] Trap damage applied to party members
- [ ] Trap effects (conditions) applied
- [ ] Party wipe check after trap damage
- [ ] Treasure loot distributed to party inventories
- [ ] Treasure events consumed after collection
- [ ] `RecruitToParty` and `RecruitToInn` dialogue actions fully implemented
- [ ] `npc_id` passed through to `DialogueState`
- [ ] `UnlockQuest` reward functional

#### 3.8 Success Criteria

- Stepping on a trap tile reduces HP and applies effects.
- Stepping on a treasure tile adds items to inventory.
- Recruiting via any dialogue path despawns the NPC mesh immediately.
- NPC dialogues know which NPC is speaking.
- Completing a quest with `UnlockQuest` reward makes the target quest
  available.
- All quality gates pass.

---

### Phase 4: System Stubs and Validation (Infrastructure Quality)

These items improve the robustness of the SDK, campaign validation, and
internal systems.

#### 4.1 Fix Starting Map String-to-ID Conversion

**Files**:

- `src/sdk/campaign_loader.rs` — `TryFrom<CampaignMetadata>` (L503–513)

**Changes**:

1. Replace the hardcoded `"starter_town" → 1` hack with a proper map name
   resolution system.
2. Either:
   - Store a `HashMap<String, MapId>` in the campaign metadata that maps
     human-readable names to IDs, OR
   - Require `starting_map` to always be a numeric ID in `campaign.ron`, OR
   - Scan loaded maps to find the one matching the name string.
3. Return a proper error if the starting map cannot be resolved.

#### 4.2 Implement Semantic Save Version Checking

**Files**:

- `src/application/save_game.rs` — `validate_version` (L180–190)

**Changes**:

1. Replace exact string match with semantic version comparison.
2. Use the `semver` crate (or manual parsing) to check compatibility:
   - Same major version: compatible (load with migration if needed).
   - Different major version: incompatible (return error).
   - Minor/patch differences: compatible with optional warnings.
3. Add version migration hooks for known schema changes.

#### 4.3 Implement `validate_references` in SDK Validation

**Files**:

- `src/sdk/validation.rs` — `validate_references` (L476–493)

**Changes**:

1. **Item references**: Check that item `disablement` flags reference valid
   class IDs from the class database.
2. **Spell references**: Check that spells reference valid classes (for class
   restrictions) and valid items (for material components).
3. **Monster references**: Check that monster loot tables reference valid item
   IDs from the item database.
4. **Map references**: Check that map events reference valid monster IDs, item
   IDs, NPC IDs, and event IDs.

#### 4.4 Implement `validate_connectivity` in SDK Validation

**Files**:

- `src/sdk/validation.rs` — `validate_connectivity` (L974–984)

**Changes**:

1. Build a graph of maps connected by transitions/portals.
2. Perform BFS/DFS from the starting map.
3. Report any maps not reachable from the starting map as warnings.
4. Report maps with no exits as warnings.

#### 4.5 Load Monster/Item IDs Dynamically in `validate_map`

**Files**:

- `src/bin/validate_map.rs` — hardcoded ID arrays (L18–27)

**Changes**:

1. Replace `VALID_MONSTER_IDS` and `VALID_ITEM_IDS` constants with dynamic
   loading from `data/monsters.ron` and `data/items.ron`.
2. Use the existing `MonsterDatabase::load_from_file` and
   `ItemDatabase::load_from_file` methods.
3. Fall back to the hardcoded arrays if the data files don't exist (with a
   warning).

#### 4.6 Implement `current_inn_id()`

**Files**:

- `src/application/mod.rs` — `current_inn_id` (L1125–1133)

**Changes**:

1. Determine the current inn based on the party's location:
   - Check if the current map tile has an inn event or the current map is an
     inn map.
   - Return the `InnkeeperId` from the map event or map metadata.
2. This enables inn-specific roster management.

#### 4.7 Testing Requirements

- Test starting map resolution with string names and numeric IDs.
- Test save version compatibility across major/minor/patch differences.
- Test `validate_references` catches broken item→class, monster→loot, and
  map→content references.
- Test `validate_connectivity` detects unreachable maps.
- Test `validate_map` loads IDs dynamically from RON files.
- Test `current_inn_id()` returns correct ID when at an inn.

#### 4.8 Deliverables

- [ ] Starting map resolution uses proper name→ID mapping
- [ ] Save version checking uses semantic versioning
- [ ] `validate_references` checks items, spells, monsters, and maps
- [ ] `validate_connectivity` performs graph traversal
- [ ] `validate_map` loads monster/item IDs from data files
- [ ] `current_inn_id()` returns actual inn ID based on location

#### 4.9 Success Criteria

- Campaigns with broken cross-references are caught by validation.
- Disconnected maps are reported as warnings.
- Save files from compatible versions load successfully.
- All quality gates pass.

---

### Phase 5: Audio, Mesh Streaming, and LOD (Polish)

These are larger subsystems that enhance the player experience but don't block
core gameplay.

#### 5.1 Implement Audio Playback

**Files**:

- `src/game/systems/audio.rs` — `handle_audio_messages` (L286–337),
  `AudioPlugin::build`

**Changes**:

1. Replace logging-only `handle_audio_messages` with actual Bevy Audio
   integration.
2. Load audio assets on demand (SFX) or at startup (music).
3. Respect `AudioSettings` (master volume, SFX volume, music volume, mute).
4. Support `PlaySfx { sfx_id }` and `PlayMusic { track_id, looped }` messages
   that are already defined and emitted by combat and other systems.
5. Add audio asset paths to campaign data or a config file.

#### 5.2 Implement Mesh Streaming Load/Unload

**Files**:

- `src/game/systems/performance.rs` — mesh streaming stubs (L144–150)

**Changes**:

1. Replace no-op `TODO: Load mesh data` with actual asset loading using
   Bevy's `AssetServer`.
2. Replace no-op `TODO: Unload mesh data` with entity despawning or mesh
   handle dropping.
3. Tie to the existing `MeshStreaming` component's `load_distance` and
   `unload_distance` fields.

#### 5.3 Implement LOD Mesh Simplification

**Files**:

- `src/game/systems/procedural_meshes.rs` — `create_simplified_mesh`
  (L2848–2861)

**Changes**:

1. Replace `mesh.clone()` with actual vertex reduction based on
   `reduction_ratio`.
2. Implement basic mesh decimation (vertex clustering or edge collapse).
3. This can start simple — even reducing polygon count by removing every Nth
   vertex is better than returning the original.

#### 5.4 Handle Unknown Combat Conditions

**Files**:

- `src/domain/combat/engine.rs` — unknown condition handling (L1080–1082)

**Changes**:

1. Replace silent no-op with a `tracing::warn!` log and defensive handling.
2. Consider making the match exhaustive (no wildcard arm) so new condition
   variants cause compile errors instead of silent no-ops.

#### 5.5 Provide Feedback for Failed Spell Casts

**Files**:

- `src/game/systems/combat.rs` — failed spell cast handling (L3048–3052)

**Changes**:

1. Replace silent no-op with combat log feedback explaining why the cast
   failed (insufficient SP, wrong class, silenced condition, etc.).
2. Show a `CombatFeedbackEffect::Fizzle` or similar visual indicator.

#### 5.6 Testing Requirements

- Test audio messages trigger Bevy Audio playback (or mock).
- Test mesh streaming loads/unloads based on distance thresholds.
- Test `create_simplified_mesh` reduces vertex count proportional to ratio.
- Test unknown combat conditions produce warnings, not silence.
- Test failed spell casts produce feedback messages.

#### 5.7 Deliverables

- [ ] Audio system plays SFX and music via Bevy Audio
- [ ] Mesh streaming loads/unloads based on distance
- [ ] LOD mesh simplification produces reduced geometry
- [ ] Unknown combat conditions logged with warning
- [ ] Failed spell casts produce player-visible feedback

#### 5.8 Success Criteria

- Audio plays during combat, exploration, and menu interactions.
- Distant meshes are unloaded; nearby meshes are loaded dynamically.
- LOD meshes have measurably fewer vertices than originals.
- No game mechanic silently fails without player feedback.
- All quality gates pass.

---

## Appendix A: Deferred Items (Low Priority)

These items are acknowledged but not prioritized in this plan. They can be
addressed opportunistically or in a future plan.

| #   | File                                    | Description                                                                          |
| --- | --------------------------------------- | ------------------------------------------------------------------------------------ |
| 1   | `game/components/creature.rs` L748      | `CreatureAnimationState` is empty struct — placeholder for animation keyframes       |
| 2   | `game/systems/dialogue_visuals.rs` L115 | Choice UI container spawned empty — populated by separate choice UI                  |
| 3   | `application/quests.rs` L228            | Some quest objective types (TalkToNpc, DeliverItem, EscortNpc, CustomFlag) unhandled |
| 4   | `sdk/campaign_loader.rs` L418           | Campaign ID override has no warning when file ID ≠ directory name                    |
| 5   | `sdk/quest_editor.rs` L437              | Cross-quest existence not validated for `UnlockQuest` targets                        |
| 6   | `sdk/game_config.rs` L748               | `allow_partial_rest` config exists but partial rest UI not implemented               |
| 7   | `game/systems/combat.rs` L3550          | Ranged attack miss uses placeholder `CombatFeedbackEffect::Miss`                     |

## Appendix B: Files Changed Per Phase

| Phase                 | Estimated Files Touched   | Complexity  |
| --------------------- | ------------------------- | ----------- |
| 1: Input & UI Fixes   | ~12 files                 | Medium      |
| 2: Time Advancement   | ~8 files + config updates | Medium-High |
| 3: Core Mechanics     | ~6 files                  | Medium      |
| 4: Validation & Stubs | ~6 files                  | Medium      |
| 5: Audio, Mesh, LOD   | ~5 files                  | High        |
| **Total**             | ~35 unique files          |             |
