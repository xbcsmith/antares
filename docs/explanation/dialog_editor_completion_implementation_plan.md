# Dialog Editor Completion Implementation Plan

## Overview

The Campaign Builder's Dialog Editor currently has incomplete functionality that prevents users from effectively creating and editing dialogue trees. This plan addresses three critical missing features: functional node creation, node editing capabilities, and proper UI integration for managing dialog trees.

## Current State Analysis

### Existing Infrastructure

The dialogue editor has a solid foundation:

- **Domain Model**: [`dialogue.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/src/domain/dialogue.rs) defines `DialogueTree`, `DialogueNode`, and `DialogueChoice` with complete CRUD operations
- **Editor State**: [`dialogue_editor.rs`](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/dialogue_editor.rs) contains comprehensive state management with buffers for all entity types
- **Backend Methods**: Core methods exist including `add_node()`, `edit_node()`, `save_node()`, `delete_node()`, `add_choice()`, `edit_choice()`, `save_choice()`, and `delete_choice()`
- **UI Components**: Shared UI helpers (`EditorToolbar`, `ActionButtons`, `TwoColumnLayout`) are available and used by other editors

### Identified Issues

1. **Add Node Button Does Nothing**
   - The "➕ Add Node" button at [line 1603](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/dialogue_editor.rs#L1603) calls `add_node()` but requires `selected_dialogue` to be set
   - When in `DialogueEditorMode::Editing`, `selected_dialogue` is set, but the UI doesn't clearly indicate this workflow
   - The node ID field must be manually entered, which is error-prone

2. **Nodes Are Not Editable**
   - Nodes are displayed in a read-only scroll area at [lines 1626-1682](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/dialogue_editor.rs#L1626-L1682)
   - No "Edit" or "Delete" buttons are shown for individual nodes
   - The `edit_node()` and `save_node()` methods exist but have no UI integration
   - Users cannot modify node text, speaker overrides, or terminal status after creation

3. **No Node Editing Panel**
   - Unlike the choice editor panel (lines 1693-1742), there's no dedicated panel for editing selected nodes
   - The node buffer fields are only used for adding new nodes, not editing existing ones
   - No visual feedback when a node is selected for editing

## Implementation Phases

### Phase 1: Add Node Editing UI

#### 1.1 Add Edit/Delete Buttons to Node Display

Modify the node display loop in `show_dialogue_nodes_editor()` to include action buttons for each node:

- Add "✏️ Edit" button next to each node (except root node for edit)
- Add "🗑️ Delete" button next to each node (except root node)
- Use the `ActionButtons` pattern from NPC editor for consistency
- Store edit/delete actions in local variables to process outside the scroll area

#### 1.2 Create Node Editor Panel

Add a new method `show_node_editor_panel()` similar to `show_choice_editor_panel()`:

- Display when `selected_node` is set and user clicked "Edit"
- Show text field for node text (multiline with reasonable height)
- Show text field for speaker override (optional)
- Show checkbox for "Terminal Node"
- Add "✓ Save" and "✗ Cancel" buttons
- Process save/cancel actions outside the panel closure

#### 1.3 Integrate Node Editing Workflow

Update `show_dialogue_nodes_editor()` to:

- Track whether we're in "add mode" or "edit mode" for nodes
- Call `edit_node()` when Edit button is clicked
- Call `show_node_editor_panel()` after the node list
- Clear `selected_node` when save/cancel is clicked
- Update status messages appropriately

#### 1.4 Testing Requirements

- Manual testing: Create a dialogue, add nodes, edit node text, verify changes persist
- Manual testing: Try to delete root node, verify error message
- Manual testing: Edit speaker override and terminal status, verify changes
- Verify existing unit tests still pass: `cargo test dialogue_editor`

#### 1.5 Deliverables

- [ ] `show_node_editor_panel()` method added to `DialogueEditorState`
- [ ] Edit and Delete buttons added to node display
- [ ] Node editing workflow integrated and functional
- [ ] Status messages updated for node operations

#### 1.6 Success Criteria

- Users can click "Edit" on any node and modify its properties
- Changes to nodes are saved when clicking "Save"
- Cannot delete root node (error message shown)
- All existing dialogue editor tests pass

---

### Phase 2: Fix Add Node Functionality

#### 2.1 Auto-Generate Node IDs

Modify the Add Node workflow to automatically generate node IDs:

- Add method `next_available_node_id()` to `DialogueEditorState`
- Similar to `next_available_dialogue_id()` at [line 983](file:///Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder/src/dialogue_editor.rs#L983)
- Find max node ID in current dialogue and add 1
- Pre-populate `node_buffer.id` when showing the add node form

#### 2.2 Improve Add Node UI

Enhance the node creation interface:

- Remove manual Node ID field (auto-generated instead)
- Show "Adding node to: [Dialogue Name]" label for clarity
- Add validation that dialogue is selected before showing add form
- Consider making node text a multiline field for longer dialogue
- Add "Speaker Override" field to the add form (currently missing)

#### 2.3 Add Node Creation Feedback

Improve user feedback:

- Show success message with created node ID
- Automatically clear the node buffer after successful add
- Scroll to the newly added node in the list
- Highlight the new node briefly (if feasible with egui)

#### 2.4 Testing Requirements

- Manual testing: Enter editing mode for a dialogue, add multiple nodes, verify IDs increment
- Manual testing: Try to add node without selecting dialogue, verify error handling
- Manual testing: Add node with speaker override, verify it's saved correctly
- Verify unit test `test_add_node_to_dialogue` passes (if exists, otherwise create one)

#### 2.5 Deliverables

- [ ] `next_available_node_id()` method implemented
- [ ] Add Node UI updated with auto-generated IDs
- [ ] Speaker override field added to node creation form
- [ ] Improved validation and user feedback

#### 2.6 Success Criteria

- Node IDs are automatically generated sequentially
- Users can add nodes without manually entering IDs
- Clear feedback when nodes are created
- Speaker override can be set during node creation

---

### Phase 3: Enhance Dialog Tree Workflow

#### 3.1 Add Visual Node Hierarchy

Improve node display to show relationships:

- Indent choices under their parent nodes
- Show target node connections more clearly (e.g., "→ Node 2: [node text preview]")
- Highlight orphaned nodes (unreachable from root) in a different color
- Add node count and reachability stats to dialogue header

#### 3.2 Add Node Navigation

Implement navigation helpers:

- "Jump to Node" button on choices that navigates to target node
- "Show Root Node" button to scroll to root
- "Find Node by ID" search field
- Breadcrumb trail showing current node path from root

#### 3.3 Improve Validation Feedback

Enhance the validation display:

- Show validation errors inline with affected nodes
- Add "Validate Tree" button that runs validation and highlights issues
- Display unreachable nodes with warning icons
- Show broken choice targets with error icons

#### 3.4 Testing Requirements

- Manual testing: Create dialogue with unreachable nodes, verify they're highlighted
- Manual testing: Use "Jump to Node" feature, verify navigation works
- Manual testing: Validate a dialogue tree with errors, verify inline feedback
- Run existing validation tests: `cargo test find_unreachable_nodes`

#### 3.5 Deliverables

- [ ] Visual hierarchy improvements to node display
- [ ] Navigation helpers implemented
- [ ] Inline validation feedback added
- [ ] Unreachable node detection integrated into UI

#### 3.6 Success Criteria

- Users can easily navigate complex dialogue trees
- Validation errors are clearly visible
- Unreachable nodes are highlighted
- Choice targets show destination context

## Verification Plan

### Automated Tests

Run existing dialogue editor tests to ensure no regressions:

```bash
cd /Users/bsmith/go/src/github.com/xbcsmith/antares
cargo test dialogue_editor --lib
```

Expected tests to pass:
- `test_dialogue_editor_state_creation`
- `test_save_edited_node`
- `test_save_edited_choice`
- `test_find_unreachable_nodes`

### Manual Verification

After each phase, perform the following manual tests:

**Phase 1 - Node Editing:**
1. Run Campaign Builder: `cargo run --bin campaign_builder`
2. Navigate to Dialogues tab
3. Create or edit a dialogue
4. Add at least 3 nodes
5. Click "Edit" on node 2, change text to "Modified text"
6. Click "Save", verify node 2 shows "Modified text"
7. Try to delete root node, verify error message appears
8. Delete node 3, verify it's removed from list

**Phase 2 - Add Node:**
1. Edit a dialogue
2. Click to add a new node
3. Verify Node ID field is auto-populated
4. Enter node text and optional speaker override
5. Click "Add Node"
6. Verify new node appears in list with correct ID
7. Add another node, verify ID increments

**Phase 3 - Tree Navigation:**
1. Create a dialogue with 5+ nodes and multiple choice branches
2. Create at least one unreachable node
3. Verify unreachable node is highlighted
4. Click "Jump to Node" on a choice, verify scroll to target
5. Use "Find Node by ID" to locate a specific node
6. Run validation, verify errors are shown inline

### Build Verification

Ensure the SDK compiles without errors:

```bash
cd /Users/bsmith/go/src/github.com/xbcsmith/antares/sdk/campaign_builder
cargo check
cargo clippy -- -D warnings
cargo fmt --check
```

All commands should complete successfully with no errors or warnings.
