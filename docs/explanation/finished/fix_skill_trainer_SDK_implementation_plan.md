# Fix Skill Trainer SDK — Dialogue ID Visibility, Detail Panel, and Template Bug

## Overview

Three independent problems in the NPC trainer workflow are addressed together
because they all touch the same three files and share a common test dataset.

| Problem | Root cause | User impact |
|---------|-----------|-------------|
| Template bug | SDK repair path does not update `OpenTraining`/`OpenSkillTraining` actions whose `npc_id` field contains a stale or wrong identifier | Any trainer NPC whose auto-generated dialogue was created with the wrong NPC ID silently fails at runtime |
| No dialogue ID control near trainer checkboxes | The only dialogue picker is in the top-level "Dialogue & Quests" section; trainer sections show only a coloured status label | Authors must scroll back up, guess which ID is correct, and cannot tell at a glance what dialogue is assigned |
| Silent preview panel | `show_npc_preview` in `portrait_picker.rs` never shows trainer or skill trainer information | The list-view right panel gives no indication of trainer state, dialogue binding, or trainable skills |

The fix is split into four phases that can be implemented independently, each
leaving the codebase in a compiling and tested state.

---

## Current State Analysis

### Existing Infrastructure

| Symbol | Location | Notes |
|--------|----------|-------|
| `DialogueTree::standard_trainer_template` | `src/domain/dialogue.rs` L534 | Creates a correct `OpenTraining { npc_id }` node when a new template is generated |
| `DialogueTree::standard_skill_trainer_template` | `src/domain/dialogue.rs` L784 | Creates a correct `OpenSkillTraining { npc_id }` node |
| `DialogueTree::ensure_standard_trainer_branch` | `src/domain/dialogue.rs` L594 | Augments an existing tree — always uses the passed `npc_id` |
| `DialogueTree::ensure_standard_skill_trainer_branch` | `src/domain/dialogue.rs` L849 | Augments an existing tree with correct `npc_id` |
| `DialogueTree::contains_open_training_for_npc` | `src/domain/dialogue.rs` L477 | Returns `true` only for the exact `npc_id` — will miss wrong-ID actions |
| `DialogueTree::contains_open_skill_training_for_npc` | `src/domain/dialogue.rs` L725 | Same |
| `ensure_trainer_dialogue_for_npc` | `sdk/…/dialogue_editor.rs` L1412 | No repair path for wrong NPC IDs in SDK-managed nodes |
| `ensure_skill_trainer_dialogue_for_npc` | `sdk/…/dialogue_editor.rs` L1533 | No repair path for wrong NPC IDs |
| `show_edit_view` trainer section | `sdk/…/npc_editor/mod.rs` L1570 | Status label only; no local dialogue picker |
| `show_edit_view` skill trainer section | `sdk/…/npc_editor/mod.rs` L1693 | Status label only; no local dialogue picker |
| `show_npc_preview` | `sdk/…/npc_editor/portrait_picker.rs` L483 | Shows merchant badge/section but nothing for trainer or skill trainer |
| `DialogueSdkManagedContent::TrainerOpenNode` | `src/domain/dialogue.rs` | Marks SDK-owned trainer nodes; used for targeted repair |
| `DialogueSdkManagedContent::SkillTrainerOpenNode` | `src/domain/dialogue.rs` | Marks SDK-owned skill trainer nodes |

### Identified Gaps

1. **No `repair_sdk_trainer_npc_id` method on `DialogueTree`** — there is no way
   to update an existing SDK-managed trainer node that holds a stale or wrong
   `OpenTraining`/`OpenSkillTraining` NPC ID without blowing away and recreating
   the entire branch.
2. **`ensure_*_dialogue_for_npc` has no repair path** — when the dialogue already
   contains an SDK-managed action node with the wrong NPC ID,
   `contains_open_*_for_npc` returns `false`, the code falls through to
   `ensure_standard_*_branch`, and a duplicate branch is silently appended.
3. **Trainer sections show no dialogue picker** — the status label tells the
   author whether the state is good or bad, but offers no way to point the
   trainer at a specific existing dialogue without using the unrelated top-level
   "Dialogue ID" selector far up the form.
4. **Preview panel is silent on trainer state** — `show_npc_preview` renders
   `🏪 Merchant`, `🛏️ Innkeeper`, and `✝ Priest` badges but nothing for
   `is_trainer` or `is_skill_trainer`.

---

## Design: Repair Path Flow

```
ensure_*_dialogue_for_npc(npc)
  │
  ├─ dialogue.contains_open_*_for_npc(&npc.id)
  │    └─ true  → AlreadyValid (no change)
  │
  ├─ dialogue.repair_sdk_*_npc_id(&npc.id)          ← NEW in Phase 1
  │    └─ true  → AugmentedExisting (wrong ID fixed in-place, no new node)
  │
  └─ dialogue.ensure_standard_*_branch(&npc.id, &npc.name)
       └─ true  → AugmentedExisting (new branch appended)
       └─ false → Error (couldn't modify root)
```

---

## Implementation Phases

### Phase 1: Domain — `DialogueTree` Repair Methods

All changes in `src/domain/dialogue.rs`.

#### 1.1 Add `repair_sdk_trainer_npc_id`

Add immediately after `ensure_standard_trainer_branch`:

```rust
/// Finds all SDK-managed trainer nodes (those marked with
/// `DialogueSdkManagedContent::TrainerOpenNode`) and updates any
/// `DialogueAction::OpenTraining` action whose `npc_id` differs from
/// `correct_npc_id`.
///
/// Returns `true` when at least one action was updated.
///
/// This is the repair path for trainer dialogues that were generated with a
/// stale or wrong NPC ID.  It is called by
/// `DialogueEditorState::ensure_trainer_dialogue_for_npc` before the more
/// invasive branch-insertion path.
///
/// # Examples
///
/// ```
/// use antares::domain::dialogue::{DialogueAction, DialogueTree};
///
/// let mut tree = DialogueTree::standard_trainer_template(10, "old_npc", "Old NPC");
/// assert!(!tree.repair_sdk_trainer_npc_id("old_npc")); // already correct — no change
/// assert!(tree.repair_sdk_trainer_npc_id("new_npc"));  // wrong ID — fixed
/// assert!(tree.contains_open_training_for_npc("new_npc"));
/// ```
pub fn repair_sdk_trainer_npc_id(&mut self, correct_npc_id: &str) -> bool {
    ...
}
```

**Implementation logic**:
- Iterate over `self.nodes.values_mut()`
- For each node whose `sdk_metadata.managed_content` contains
  `DialogueSdkManagedContent::TrainerOpenNode`:
  - For each action in `node.actions`:
    - If it is `DialogueAction::OpenTraining { npc_id }` and `npc_id != correct_npc_id`:
      - Set `*npc_id = correct_npc_id.to_string()`
      - Set `changed = true`
- Also scan `node.choices[*].actions` with the same pattern
- Return `changed`

#### 1.2 Add `repair_sdk_skill_trainer_npc_id`

Add immediately after `ensure_standard_skill_trainer_branch`, mirroring 1.1:

- Targets nodes with `DialogueSdkManagedContent::SkillTrainerOpenNode`
- Updates `DialogueAction::OpenSkillTraining { npc_id }` actions
- Same pattern as `repair_sdk_trainer_npc_id`

Doc comment pattern and `#[must_use]` approach mirrors 1.1 above; swap
`OpenTraining` → `OpenSkillTraining` and `TrainerOpenNode` → `SkillTrainerOpenNode`.

#### 1.3 Testing Requirements

Tests live in the existing `mod tests` block in `src/domain/dialogue.rs`.

- `test_repair_sdk_trainer_npc_id_updates_wrong_id`:
  Create `DialogueTree::standard_trainer_template(10, "wrong_npc", "Wrong")`.
  Call `repair_sdk_trainer_npc_id("correct_npc")`.
  Assert return is `true`.
  Assert `contains_open_training_for_npc("correct_npc")`.
  Assert `!contains_open_training_for_npc("wrong_npc")`.

- `test_repair_sdk_trainer_npc_id_is_noop_when_already_correct`:
  Create template with `"correct_npc"`.
  Call `repair_sdk_trainer_npc_id("correct_npc")`.
  Assert return is `false`.

- `test_repair_sdk_skill_trainer_npc_id_updates_wrong_id`:
  Mirror of the trainer test using `standard_skill_trainer_template` and
  `repair_sdk_skill_trainer_npc_id`.

- `test_repair_sdk_skill_trainer_npc_id_is_noop_when_already_correct`:
  Mirror of the no-op test.

#### 1.4 Deliverables

- [ ] `DialogueTree::repair_sdk_trainer_npc_id` in `src/domain/dialogue.rs`
- [ ] `DialogueTree::repair_sdk_skill_trainer_npc_id` in `src/domain/dialogue.rs`
- [ ] All Phase 1 tests pass; `cargo check`, `cargo clippy -- -D warnings` clean

#### 1.5 Success Criteria

A dialogue built with `standard_trainer_template(id, "old_npc", "Old NPC")` can
be repaired to point at `"new_npc"` by calling `repair_sdk_trainer_npc_id("new_npc")`
in a single pass with no branch duplication.  The same holds for skill trainer.

---

### Phase 2: SDK Dialogue Editor — Wire the Repair Path

All changes in `sdk/campaign_builder/src/dialogue_editor.rs`.

#### 2.1 Update `ensure_trainer_dialogue_for_npc`

In the existing-dialogue branch (after `contains_open_training_for_npc` returns
`false`, before `ensure_standard_trainer_branch`), insert:

```rust
// Repair path: a SDK-managed trainer node exists but targets the wrong NPC ID.
// Update it in-place instead of appending a duplicate branch.
if dialogue.repair_sdk_trainer_npc_id(&npc.id) {
    self.has_unsaved_changes = true;
    return Ok(MerchantDialogueUpdate::AugmentedExisting { dialogue_id });
}
```

No other changes to the function body are needed.

#### 2.2 Update `ensure_skill_trainer_dialogue_for_npc`

Same pattern as 2.1, using `dialogue.repair_sdk_skill_trainer_npc_id(&npc.id)`.
Insert immediately after `contains_open_skill_training_for_npc` returns `false`
and before `ensure_standard_skill_trainer_branch`.

#### 2.3 Testing Requirements

Tests live in the existing `mod tests` block in
`sdk/campaign_builder/src/dialogue_editor.rs`.

- `test_ensure_trainer_dialogue_uses_npc_id`:
  Create `DialogueEditorState::new()`.
  Load a template tree created with `standard_trainer_template(10, "npc_1", "NPC 1")`.
  Create an NPC with `id: "trainer_npc"`, `is_trainer: true`, `dialogue_id: Some(10)`.
  Call `ensure_trainer_dialogue_for_npc(&mut npc)`.
  Assert result is `Ok(AugmentedExisting { dialogue_id: 10 })`.
  Assert the loaded dialogue now `contains_open_training_for_npc("trainer_npc")`.
  Assert the loaded dialogue does NOT `contains_open_training_for_npc("npc_1")`.

- `test_ensure_trainer_dialogue_already_correct_returns_already_valid`:
  Same setup, but template created with the correct NPC ID.
  Assert result is `Ok(AlreadyValid)`.
  Assert no duplicate nodes added.

- `test_ensure_skill_trainer_dialogue_uses_npc_id`:
  Mirror of `test_ensure_trainer_dialogue_uses_npc_id` using skill trainer
  variants (`standard_skill_trainer_template`, `ensure_skill_trainer_dialogue_for_npc`,
  `contains_open_skill_training_for_npc`).

- `test_ensure_skill_trainer_dialogue_already_correct_returns_already_valid`:
  Mirror of the already-valid trainer test.

#### 2.4 Deliverables

- [ ] `ensure_trainer_dialogue_for_npc` wires `repair_sdk_trainer_npc_id` (Phase 2.1)
- [ ] `ensure_skill_trainer_dialogue_for_npc` wires `repair_sdk_skill_trainer_npc_id` (Phase 2.2)
- [ ] All Phase 2 tests pass; quality gates clean

#### 2.5 Success Criteria

A trainer NPC whose dialogue was previously generated with the wrong NPC ID has
the action corrected in a single call to `ensure_trainer_dialogue_for_npc` /
`ensure_skill_trainer_dialogue_for_npc` without creating duplicate nodes.

---

### Phase 3: Edit Panel — Dialogue ID Picker in Trainer Sections

All changes in `sdk/campaign_builder/src/npc_editor/mod.rs`.

#### 3.1 Trainer Dialogue ComboBox

In `show_edit_view`, inside the `if self.edit_buffer.is_trainer { … }` block
(currently at L1570), **after** the coloured status label and **before** the
fee fields, insert:

```rust
// Dialogue picker — lets the author point this trainer at any existing dialogue.
ui.horizontal(|ui| {
    ui.label("Dialogue:");
    let trainer_selected_text = if self.edit_buffer.dialogue_id.is_empty() {
        "(none)".to_string()
    } else {
        self.available_dialogues
            .iter()
            .find(|d| d.id.to_string() == self.edit_buffer.dialogue_id)
            .map(|d| format!("{}: {}", d.id, d.name))
            .unwrap_or_else(|| self.edit_buffer.dialogue_id.clone())
    };

    egui::ComboBox::from_id_salt("npc_trainer_dialogue_picker")
        .selected_text(trainer_selected_text)
        .show_ui(ui, |ui| {
            ui.push_id("npc_trainer_dialogue_none", |ui| {
                if ui
                    .selectable_label(self.edit_buffer.dialogue_id.is_empty(), "(none)")
                    .clicked()
                {
                    self.edit_buffer.dialogue_id.clear();
                    needs_save = true;
                }
            });
            for dialogue in &self.available_dialogues {
                ui.push_id(dialogue.id, |ui| {
                    let label = format!("{}: {}", dialogue.id, dialogue.name);
                    if ui
                        .selectable_label(
                            self.edit_buffer.dialogue_id == dialogue.id.to_string(),
                            &label,
                        )
                        .clicked()
                    {
                        self.edit_buffer.dialogue_id = dialogue.id.to_string();
                        needs_save = true;
                    }
                });
            }
        });
});

ui.small("Dialogue must contain an OpenTraining action for this NPC.");
```

Key rules (SDK AGENTS.md):
- `ComboBox::from_id_salt` — satisfies Rule 3
- `push_id` on every iteration body — satisfies Rule 1
- `needs_save = true` on selection — keeps state consistent

#### 3.2 Skill Trainer Dialogue ComboBox

In `show_edit_view`, inside the `if self.edit_buffer.is_skill_trainer { … }` block
(currently at L1693), **after** the coloured status label and **before** the
trainable skills selector, insert an identical ComboBox using:

- `id_salt: "npc_skill_trainer_dialogue_picker"`
- Hint: `"Dialogue must contain an OpenSkillTraining action for this NPC."`

Otherwise structurally identical to 3.1.

#### 3.3 Expected Layout After Change

**Trainer section:**

```
[x] 🎓 Is Trainer
    Trainer dialogue valid                             ← coloured status label
    Dialogue: [1001: Lost Ranger Trainer Dialogue ▾]  ← ComboBox (NEW)
    Dialogue must contain an OpenTraining action for this NPC.  ← hint (NEW)
    Training Fee Base (gold per level): ___  empty = campaign default
    Training Fee Multiplier: ___  empty = campaign default
    [Create trainer dialogue] [Repair trainer dialogue] [Remove trainer branch]
    SDK workflow hint...
```

**Skill trainer section:**

```
[x] 🧠 Is Skill Trainer
    Skill trainer dialogue valid                              ← coloured status
    Dialogue: [1002: Lost Ranger Skill Trainer Dialogue ▾]   ← ComboBox (NEW)
    Dialogue must contain an OpenSkillTraining action for this NPC.  ← hint (NEW)
    Trainable Skills: stealth, athletics, ...
    [Override skill fee base] [Override skill fee multiplier] [Override max rank]
    [Create skill trainer dialogue] [Repair skill trainer dialogue] [Remove skill trainer branch]
    SDK workflow hint...
```

#### 3.4 Testing Requirements

Tests live in the existing `mod tests` block in `npc_editor/mod.rs`.  All new
tests must satisfy `sdk/AGENTS.md` Rules 2, 3 (every new `ScrollArea` carries
`id_salt`, every new `ComboBox` uses `from_id_salt`).

- `test_edit_panel_trainer_shows_dialogue_combobox`:
  Construct an `NpcEditorState` in `Edit` mode with `is_trainer: true`.
  Populate `available_dialogues` with two dummy `DialogueTree` values.
  Assert that after rendering via `eframe::egui::__run_test_ui`, the `dialogue_id`
  field can be set by selection from the picker (simulate state mutation directly
  since egui headless tests cannot click, but assert the ComboBox logic path is
  reachable and the `dialogue_id` string round-trips correctly).

  Concrete assertion approach (non-UI unit test):
  - Set `edit_buffer.is_trainer = true` and `available_dialogues = [...]`.
  - Manually invoke the same selection logic that the ComboBox closure runs.
  - Assert `edit_buffer.dialogue_id == "42"` after selecting dialogue `42`.

- `test_edit_panel_skill_trainer_shows_dialogue_combobox`:
  Mirror of the above using `is_skill_trainer = true` and the skill trainer
  ComboBox `id_salt`.

#### 3.5 Deliverables

- [ ] Trainer dialogue ComboBox added to `is_trainer` section with hint label
- [ ] Skill trainer dialogue ComboBox added to `is_skill_trainer` section with hint label
- [ ] Both ComboBoxes use `from_id_salt`; loop items use `push_id` (SDK Rules 1, 3)
- [ ] `needs_save = true` set on selection
- [ ] All Phase 3 tests pass; quality gates clean

#### 3.6 Success Criteria

An author can assign an existing hand-authored dialogue to a trainer or skill
trainer directly from the trainer section of the edit panel without touching the
unrelated top-level "Dialogue ID" selector.  The status label updates immediately
after selection.

---

### Phase 4: Preview Panel — Trainer Badges and Detail Sections

All changes in `sdk/campaign_builder/src/npc_editor/portrait_picker.rs`.

#### 4.1 Add Trainer Badges to the Role Badge Row

In `show_npc_preview`, in the `ui.vertical` closure that currently renders
`🏪 Merchant`, `🛏️ Innkeeper`, and `✝ Priest` labels (L525–L542), add:

```rust
if npc.is_trainer {
    ui.label(egui::RichText::new("🎓 Trainer").color(egui::Color32::from_rgb(220, 180, 80)));
}
if npc.is_skill_trainer {
    ui.label(egui::RichText::new("🧠 Skill Trainer").color(egui::Color32::from_rgb(180, 220, 80)));
}
```

Update the fallback `🧑 NPC` condition (L540–L542) to also gate on
`is_trainer` and `is_skill_trainer`:

```rust
if !npc.is_merchant && !npc.is_innkeeper && !npc.is_priest
    && !npc.is_trainer && !npc.is_skill_trainer
{
    ui.label(egui::RichText::new("🧑 NPC").color(egui::Color32::GRAY));
}
```

#### 4.2 Add Trainer Identity Grid Row

In `egui::Grid::new("npc_preview_identity_grid")`, after the existing
`"Merchant Dialogue:"` row, add a `"Trainer Dialogue:"` row that is shown only
when `npc.is_trainer || npc.is_skill_trainer`:

```rust
if npc.is_trainer || npc.is_skill_trainer {
    ui.label("Trainer Dialogue:");
    ui.label(
        npc.dialogue_id
            .map(|id| id.to_string())
            .as_deref()
            .unwrap_or("no dialogue assigned"),
    );
    ui.end_row();
}
```

#### 4.3 Add Trainer Detail Section

After the existing Merchant section block (L618–L641) and before the Services
section (L643), insert:

```rust
if npc.is_trainer {
    ui.add_space(10.0);
    ui.heading("Trainer");
    ui.separator();

    egui::Grid::new("npc_preview_trainer_grid")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            ui.label("Dialogue:");
            match npc.dialogue_id {
                Some(id) => ui.label(id.to_string()),
                None => ui.label(
                    egui::RichText::new("no dialogue assigned")
                        .color(egui::Color32::RED),
                ),
            };
            ui.end_row();

            ui.label("Fee Base:");
            ui.label(
                npc.training_fee_base
                    .map(|v| format!("{} gold/level", v))
                    .as_deref()
                    .unwrap_or("(campaign default)")
                    .to_string(),
            );
            ui.end_row();

            ui.label("Fee Multiplier:");
            ui.label(
                npc.training_fee_multiplier
                    .map(|v| format!("× {:.2}", v))
                    .as_deref()
                    .unwrap_or("(campaign default)")
                    .to_string(),
            );
            ui.end_row();
        });
}
```

#### 4.4 Add Skill Trainer Detail Section

Immediately after the Trainer section block (4.3), insert:

```rust
if npc.is_skill_trainer {
    ui.add_space(10.0);
    ui.heading("Skill Trainer");
    ui.separator();

    egui::Grid::new("npc_preview_skill_trainer_grid")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            ui.label("Dialogue:");
            match npc.dialogue_id {
                Some(id) => ui.label(id.to_string()),
                None => ui.label(
                    egui::RichText::new("no dialogue assigned")
                        .color(egui::Color32::RED),
                ),
            };
            ui.end_row();

            ui.label("Trainable Skills:");
            if npc.trainable_skill_ids.is_empty() {
                ui.label("(none)");
            } else {
                ui.label(npc.trainable_skill_ids.join(", "));
            }
            ui.end_row();

            if let Some(max_rank) = npc.skill_training_max_rank {
                ui.label("Max Rank:");
                ui.label(max_rank.to_string());
                ui.end_row();
            }

            ui.label("Fee Base:");
            ui.label(
                npc.skill_training_fee_base
                    .map(|v| format!("{} gold/rank", v))
                    .as_deref()
                    .unwrap_or("(campaign default)")
                    .to_string(),
            );
            ui.end_row();

            ui.label("Fee Multiplier:");
            ui.label(
                npc.skill_training_fee_multiplier
                    .map(|v| format!("× {:.2}", v))
                    .as_deref()
                    .unwrap_or("(campaign default)")
                    .to_string(),
            );
            ui.end_row();
        });
}
```

#### 4.5 Testing Requirements

Tests live in the existing `mod tests` block in `npc_editor/portrait_picker.rs`
(or in `npc_editor/mod.rs` where preview tests currently live).

- `test_show_npc_preview_shows_trainer_badge`:
  Build an `NpcDefinition` with `is_trainer: true`, all other role flags `false`.
  Assert `🎓 Trainer` badge renders (test via state inspection, not pixel-level
  egui render: verify that the code path for `is_trainer` is exercised by
  checking the flag gates directly in a unit test against the NPC struct).

  **Concrete approach**: call `show_npc_preview` inside an `eframe::egui::__run_test_ui`
  context with `is_trainer: true` and assert no panic.  Use a separate logic
  test to assert that `is_trainer = true` causes the `🧑 NPC` fallback NOT to
  appear (i.e., the gate `!npc.is_trainer` keeps the fallback off).

- `test_show_npc_preview_shows_skill_trainer_badge`:
  Mirror of the above with `is_skill_trainer: true`.

- `test_show_npc_preview_trainer_with_no_dialogue_id_shows_red_label`:
  Build an NPC with `is_trainer: true`, `dialogue_id: None`.
  Call preview; assert no panic (structural test — verifies the `None` arm of
  the dialogue ID display does not unwrap or panic).

- `test_show_npc_preview_skill_trainer_skills_list`:
  Build an NPC with `is_skill_trainer: true`,
  `trainable_skill_ids: vec!["stealth".to_string(), "athletics".to_string()]`.
  Call preview; assert no panic.

#### 4.6 Deliverables

- [ ] `🎓 Trainer` and `🧠 Skill Trainer` badges in role badge row
- [ ] Fallback `🧑 NPC` label gated on `!is_trainer && !is_skill_trainer`
- [ ] Optional `"Trainer Dialogue:"` row in identity grid
- [ ] Trainer detail section (dialogue ID, fee base, fee multiplier)
- [ ] Skill Trainer detail section (dialogue ID, skills list, max rank, fees)
- [ ] `egui::Grid` IDs are unique: `"npc_preview_trainer_grid"`, `"npc_preview_skill_trainer_grid"`
- [ ] All Phase 4 tests pass; quality gates clean

#### 4.7 Success Criteria

Selecting any NPC in the list view shows trainer or skill trainer badges in the
right panel immediately.  Trainers with no assigned dialogue show a red
`"no dialogue assigned"` label — not a silent empty cell.

---

## Affected Files Summary

| File | Phase | Change |
|------|-------|--------|
| `src/domain/dialogue.rs` | 1 | Add `repair_sdk_trainer_npc_id`, `repair_sdk_skill_trainer_npc_id` + tests |
| `sdk/campaign_builder/src/dialogue_editor.rs` | 2 | Wire repair methods into `ensure_trainer_dialogue_for_npc` and `ensure_skill_trainer_dialogue_for_npc` + tests |
| `sdk/campaign_builder/src/npc_editor/mod.rs` | 3 | Add dialogue ComboBox to trainer and skill trainer sections + tests |
| `sdk/campaign_builder/src/npc_editor/portrait_picker.rs` | 4 | Add trainer/skill trainer badges, identity grid row, and detail sections + tests |
| `docs/explanation/implementations.md` | all | Summary entry after all phases complete |

No RON data files change.  No campaign content changes.

---

## Decisions Log

| Decision | Rationale |
|----------|-----------|
| Repair methods live in `src/domain/dialogue.rs`, not in the SDK | The repair logic operates on `DialogueTree` internals (node metadata, action mutation). Keeping it in the domain layer lets both the SDK and any future tools use it without a crate dependency inversion. |
| Repair path inserted between `contains_open_*_for_npc` and `ensure_standard_*_branch` | The three-tier check (already valid → repair → augment) avoids duplicate nodes: if a node exists with the wrong ID it is fixed, not cloned. |
| `AugmentedExisting` returned for both the repair case and the branch-insertion case | Both operations modify an existing dialogue in a non-destructive way; the same variant communicates "something changed" without adding a new enum variant. |
| Trainer ComboBox uses a local `id_salt` per section (`"npc_trainer_dialogue_picker"`, `"npc_skill_trainer_dialogue_picker"`) | The global "Dialogue ID" picker in the "Dialogue & Quests" section already has `id_salt "npc_edit_dialogue_select"`. The two new pickers must have distinct salts to avoid egui ID collisions (SDK AGENTS.md Rule 3). |
| `push_id(dialogue.id, …)` used for every iteration in the new ComboBox loops | SDK AGENTS.md Rule 1 — every loop body must scope its widgets under a unique ID. |
| `🎓` and `🧠` emojis chosen for trainer badges | Consistent with the existing `🏪`, `🛏️`, `✝` emojis already in `show_npc_preview`. |
| No separate bottom hint bar in preview panel | All trainer information is rendered in collapsible grid sections, consistent with the Merchant section pattern already in `show_npc_preview`. |
