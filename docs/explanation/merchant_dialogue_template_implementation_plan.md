# Merchant Dialogue Template Implementation Plan

## Overview

This plan standardizes how merchant NPC dialogue is authored and maintained across the game runtime and the Campaign Builder SDK.

The target behavior is:

- Merchant NPCs must explicitly open trading through a dialogue tree that contains a `DialogueAction::OpenMerchant { npc_id }` path.
- The SDK manages a built-in standard merchant dialogue template.
- When `NpcDefinition::is_merchant == true` and the NPC has no dialogue, the SDK creates a new merchant dialogue tree and assigns it to `NpcDefinition::dialogue_id`.
- When `NpcDefinition::is_merchant == true` and the assigned dialogue exists but does not contain an `OpenMerchant` action, the SDK augments the dialogue by inserting a standard merchant branch at the root.
- When `NpcDefinition::is_merchant == false`, the SDK removes only SDK-managed merchant dialogue content and leaves the rest of the dialogue tree intact.

This approach preserves custom authored dialogue while enforcing a clear, explicit merchant-opening contract that can be validated automatically.

## Current State Analysis

### Existing Infrastructure

The following systems and data structures already exist and should be reused:

- `NpcDefinition` in `src/domain/world/npc.rs`
  - `is_merchant: bool`
  - `dialogue_id: Option<DialogueId>`
- `DialogueTree`, `DialogueNode`, `DialogueChoice`, and `DialogueAction` in `src/domain/dialogue.rs`
- `DialogueAction::OpenMerchant { npc_id }` runtime handling in `src/game/systems/dialogue.rs`
- Merchant inventory runtime transition paths:
  - dialogue action path via `OpenMerchant`
  - keyboard shortcut path via `I` during merchant dialogue in `src/game/systems/input/global_toggles.rs`
- Campaign Builder NPC editing UI in `sdk/campaign_builder/src/npc_editor.rs`
  - merchant flag editing
  - dialogue assignment editing
- Campaign Builder dialogue editing infrastructure in `sdk/campaign_builder/src/dialogue_editor.rs`
- Existing merchant design history in `docs/explanation/finished/buy_and_sell_plan.md`

### Identified Issues

The current system has the following gaps:

- There is no SDK-enforced rule that merchant NPCs must have explicit merchant-opening dialogue.
- `is_merchant` and `dialogue_id` are currently independent, which allows invalid merchant content states.
- There is no standard built-in merchant dialogue template in the SDK.
- There is no non-destructive augmentation path for an existing dialogue tree that lacks `OpenMerchant`.
- There is no reversible tracking mechanism to distinguish SDK-generated merchant dialogue content from author-created dialogue content.
- There is no validation rule that can report:
  - merchant NPC missing dialogue
  - merchant NPC dialogue missing `OpenMerchant`
  - non-merchant NPC dialogue still containing SDK-managed merchant content
- There is no defined lifecycle for toggling `is_merchant` on and off in the Campaign Builder UI.

## Implementation Phases

### Phase 1: Merchant Dialogue Policy and Metadata Foundation

#### 1.1 Foundation Work

Define a precise SDK-owned merchant dialogue contract and the metadata needed to support non-destructive insertion and removal.

Files to update:

- `src/domain/dialogue.rs`
- `sdk/campaign_builder/src/dialogue_editor.rs`
- `sdk/campaign_builder/src/npc_editor.rs`
- `docs/explanation/merchant_dialogue_template_implementation_plan.md`

Required design decisions to encode in the codebase:

- A merchant-capable dialogue is valid only if it contains an explicit `DialogueAction::OpenMerchant { npc_id }`.
- SDK-managed merchant dialogue content must be distinguishable from author-created content.
- SDK-managed merchant content must be removable without deleting unrelated dialogue content.

Recommended implementation detail:

- Add dialogue-level and/or node-level SDK metadata sufficient to mark:
  - an SDK-generated merchant template tree
  - an SDK-inserted merchant branch inside an existing tree
- Prefer structured metadata over text-label heuristics.
- If metadata must be minimal, use a deterministic marker scheme that is machine-checkable and unique.

#### 1.2 Add Foundation Functionality

Define standard merchant template construction rules.

The built-in standard merchant template must include:

- a root greeting node
- a player-visible choice such as “Show me your wares.”
- a target node that contains `DialogueAction::OpenMerchant { npc_id }`
- a goodbye / exit path

The template generation logic must accept:

- `npc_id`
- `npc_name`
- `DialogueId`
- node ID allocation inputs when augmenting an existing tree

The SDK-managed inserted content must be deterministic so that repeated runs are idempotent.

#### 1.3 Integrate Foundation Work

Add reusable planning-level APIs or helpers for later phases to call.

Planned helper responsibilities:

- detect whether a dialogue tree contains an explicit `OpenMerchant` action for a given NPC
- detect whether a dialogue tree contains SDK-managed merchant content
- create a full merchant template tree for an NPC with no dialogue
- augment an existing dialogue tree at the root with a merchant branch
- remove only SDK-managed merchant content when merchant mode is disabled

The augmentation strategy should be:

- root-node insertion only
- append a standard merchant choice to the root node when not already present
- route to a generated merchant action node
- preserve all existing root text, conditions, actions, and other choices

#### 1.4 Testing Requirements

Add unit tests near the merchant-template helper implementation covering:

- empty dialogue tree → generated merchant template contains `OpenMerchant`
- existing dialogue with no merchant path → augmentation inserts one merchant branch
- existing dialogue already containing `OpenMerchant` → augmentation is a no-op
- removing SDK-managed merchant content leaves unrelated custom nodes and choices intact
- repeated augmentation is idempotent
- repeated removal is idempotent

Tests must use in-memory dialogue data only.

#### 1.5 Deliverables

- [ ] Merchant dialogue validity contract defined and documented
- [ ] SDK-managed merchant metadata strategy defined
- [ ] Standard merchant template shape defined
- [ ] Helper surface for detect/create/augment/remove merchant dialogue behavior designed
- [ ] Foundation tests specified and implemented

#### 1.6 Success Criteria

Phase 1 is complete when:

- the project has a machine-checkable definition of valid merchant dialogue
- merchant template generation and merchant branch insertion are fully specified
- SDK-managed merchant content can be distinguished from user-authored dialogue
- non-destructive removal rules are explicit and testable

### Phase 2: SDK Merchant Template Generation and Dialogue Augmentation

#### 2.1 Feature Work

Implement the built-in merchant dialogue template generator inside the SDK.

Files likely to change:

- `sdk/campaign_builder/src/npc_editor.rs`
- `sdk/campaign_builder/src/dialogue_editor.rs`
- new SDK helper module under `sdk/campaign_builder/src/` if needed
- `src/domain/dialogue.rs` only if Phase 1 metadata support requires it

Required behavior:

- If `NpcDefinition::is_merchant == true` and `dialogue_id == None`:
  - allocate a new `DialogueId`
  - create a new merchant template tree
  - add it to the loaded dialogue collection
  - assign its ID to the NPC
- If `NpcDefinition::is_merchant == true` and `dialogue_id != None`:
  - load the assigned dialogue tree
  - detect whether explicit `OpenMerchant` already exists
  - if not present, augment the root with the standard merchant branch

#### 2.2 Integrate Feature

Hook merchant-template enforcement into the NPC editing lifecycle.

Integration points should include:

- toggling `is_merchant` from false to true in `sdk/campaign_builder/src/npc_editor.rs`
- saving an NPC edit buffer back into `NpcDefinition`
- optional explicit “repair merchant dialogue” action in the NPC editor

Recommended behavior:

- auto-apply when `is_merchant` is checked
- auto-apply again on save if the merchant dialogue has drifted out of compliance
- never duplicate merchant branch content if the NPC is edited repeatedly

#### 2.3 Configuration Updates

Expose the merchant-dialogue state clearly in the Campaign Builder UI.

Recommended UI additions:

- merchant dialogue status badge:
  - “No dialogue assigned”
  - “Merchant dialogue valid”
  - “Merchant dialogue missing OpenMerchant”
  - “SDK-managed merchant branch present”
- one-click maintenance actions:
  - “Create merchant dialogue”
  - “Repair merchant dialogue”
  - “Open assigned dialogue”

The UI must remain non-destructive to unrelated dialogue content.

#### 2.4 Testing Requirements

Add Campaign Builder-focused tests covering:

- checking `is_merchant` on an NPC with no dialogue creates and assigns a new merchant dialogue
- checking `is_merchant` on an NPC with existing non-merchant dialogue augments the assigned tree
- checking `is_merchant` on an NPC whose dialogue already contains merchant content does not duplicate nodes or choices
- saving after merchant auto-generation persists both:
  - NPC `dialogue_id`
  - modified or new `DialogueTree`

Tests must use in-memory fixture data or `data/`-rooted stable fixtures only.

#### 2.5 Deliverables

- [ ] Built-in standard merchant dialogue template implemented
- [ ] Merchant auto-generation for NPCs with no dialogue implemented
- [ ] Merchant dialogue augmentation for existing dialogues implemented
- [ ] NPC editor wiring for merchant enablement implemented
- [ ] Campaign Builder UI exposes merchant dialogue status and repair actions
- [ ] SDK tests cover generation, augmentation, and idempotence

#### 2.6 Success Criteria

Phase 2 is complete when:

- every newly marked merchant NPC can be brought into a valid state automatically
- merchant dialogue generation is idempotent
- existing custom dialogue is preserved
- the SDK can create or repair merchant dialogue without manual dialogue authoring

### Phase 3: Merchant Disablement and Non-Destructive Removal

#### 3.1 Feature Work

Implement the reverse lifecycle for merchant removal.

Required behavior when `NpcDefinition::is_merchant` changes from true to false:

- remove only SDK-managed merchant content from the assigned dialogue tree
- keep all unrelated dialogue nodes, choices, and actions intact
- do not delete the dialogue tree asset outright
- do not clear `dialogue_id` automatically unless the tree is proven to be fully SDK-generated merchant-only content and the product decision explicitly allows clearing it

Per the agreed policy, the dialogue tree itself is left intact.

#### 3.2 Integrate Feature

Hook merchant-disable cleanup into the same NPC editing/save lifecycle used for merchant enablement.

Recommended integration points:

- unchecking `is_merchant` in `sdk/campaign_builder/src/npc_editor.rs`
- saving the edited NPC
- optional explicit action such as “Remove merchant branch”

Removal must target only SDK-managed merchant content identified via Phase 1 metadata.

#### 3.3 Configuration Updates

Expose removal consequences clearly in the Campaign Builder UI.

Recommended UX behavior:

- when merchant is unchecked, show a message indicating:
  - merchant branch/action will be removed
  - non-merchant dialogue content will remain intact
- after removal, update status badges immediately

#### 3.4 Testing requirements

Add tests covering:

- merchant disable removes SDK-managed merchant branch from an augmented custom dialogue
- merchant disable leaves non-merchant dialogue choices and nodes untouched
- merchant disable on a generated merchant-only dialogue leaves a valid, non-crashing dialogue asset state
- repeated disable operations are idempotent
- unchecking merchant on an NPC with no merchant branch is a no-op

Tests must use only in-memory data or stable `data/` fixtures.

#### 3.5 Deliverables

- [ ] Merchant disable removes SDK-managed merchant content only
- [ ] Merchant disable preserves unrelated dialogue content
- [ ] Merchant disable flow integrated into NPC editor save/toggle lifecycle
- [ ] Removal behavior is idempotent and test-covered

#### 3.6 Success Criteria

Phase 3 is complete when:

- turning off `is_merchant` never destroys authored non-merchant dialogue
- all SDK-managed merchant content can be removed reliably
- the post-removal dialogue asset remains valid and editable

### Phase 4: Validation, Repair, and Data Integrity Enforcement

#### 4.1 Feature Work

Add validation rules that detect invalid merchant dialogue states anywhere in loaded campaign content.

Validation rules must report:

- merchant NPC with no `dialogue_id`
- merchant NPC whose assigned dialogue tree is missing explicit `OpenMerchant`
- merchant NPC whose assigned `OpenMerchant` targets the wrong `npc_id`
- non-merchant NPC whose assigned dialogue still contains SDK-managed merchant content
- NPC `dialogue_id` referencing a missing dialogue tree

Likely files:

- `sdk/campaign_builder/src/advanced_validation.rs`
- `sdk/campaign_builder/src/npc_editor.rs`
- any existing SDK validation modules that already report cross-asset consistency issues

#### 4.2 Integrate Feature

Provide fix-it actions from validation results.

Recommended repair actions:

- generate new merchant dialogue
- augment assigned dialogue with merchant branch
- remove SDK-managed merchant branch
- rebind `OpenMerchant` to the correct `npc_id`

Validation should be usable both:

- interactively in Campaign Builder
- and in any non-UI validation pipeline already present

#### 4.3 Configuration Updates

Add status visibility to the NPC editor and dialogue editor.

Recommended signals:

- NPC list badges for merchant dialogue validity
- dialogue editor indication that a dialogue contains SDK-managed merchant content
- validation panel entries with direct jump-to-edit actions

#### 4.4 Testing requirements

Add validation tests covering all invalid and repairable merchant cases.

Required cases:

- merchant with missing dialogue
- merchant with wrong dialogue
- merchant with wrong `OpenMerchant.npc_id`
- non-merchant with leftover SDK merchant branch
- repair action successfully resolves each case

#### 4.5 Deliverables

- [ ] Merchant dialogue validation rules implemented
- [ ] Merchant repair actions implemented
- [ ] Editor status surfaces merchant validity clearly
- [ ] Validation tests cover missing, invalid, and stale merchant dialogue states

#### 4.6 Success Criteria

Phase 4 is complete when:

- invalid merchant dialogue states are always detectable
- repair actions can bring invalid merchant NPCs back into compliance automatically
- the Campaign Builder makes merchant dialogue correctness obvious to authors

### Phase 5: Runtime Contract Alignment and Documentation

#### 5.1 Feature Work

Align runtime expectations with the SDK-enforced authoring standard.

Runtime behavior should continue to support:

- `DialogueAction::OpenMerchant { npc_id }`
- `I` as a convenience shortcut during dialogue with a merchant NPC

But the documented content contract must clearly state:

- valid merchant content requires explicit `OpenMerchant`
- `I` is a runtime shortcut, not the authoring standard

Files to update:

- `docs/explanation/implementations.md`
- `docs/explanation/finished/buy_and_sell_plan.md` or follow-up explanation docs as appropriate
- SDK/editor help text where merchant behavior is described
- any relevant doc comments in NPC and dialogue editor modules

#### 5.2 Integrate Feature

Document the standard merchant template and lifecycle rules for content authors.

Required documentation topics:

- what happens when `is_merchant` is enabled
- what happens when `is_merchant` is disabled
- how standard merchant dialogue is generated
- how custom dialogue is augmented
- how merchant validation and repair work

#### 5.3 Configuration Updates

If the Campaign Builder has inline help or tooltips for merchant NPCs, update them to state:

- merchant NPCs require explicit `OpenMerchant`
- the SDK will create or repair merchant dialogue automatically
- custom dialogue is preserved where possible

#### 5.4 Testing requirements

Add final regression coverage ensuring:

- runtime still opens merchant inventory from `OpenMerchant`
- runtime still supports `I` shortcut during merchant dialogue
- generated merchant dialogues are valid at runtime
- repaired merchant dialogues remain valid after save/load round-trips

Use stable `data/` fixtures only where file-backed coverage is needed.

#### 5.5 Deliverables

- [ ] Runtime and SDK contract documented consistently
- [ ] Merchant template lifecycle documented for campaign authors
- [ ] Merchant tooltip/help text updated
- [ ] Final regression coverage verifies runtime compatibility

#### 5.6 Success Criteria

Phase 5 is complete when:

- the SDK and runtime agree on one explicit merchant dialogue contract
- campaign authors have a predictable merchant authoring workflow
- merchant NPCs can be added at any time in campaign creation without leaving broken dialogue states
- all merchant dialogue lifecycle transitions are documented, validated, and repairable
