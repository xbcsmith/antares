# How to Create Creatures

This guide documents the creature-creation workflow that is currently shipped in
the Campaign Builder UI.

## Opening the Editor

Open a campaign, then use either entry path:

1. `Tools -> Creature Editor`
2. Left sidebar `Creatures` tab

You can also open the template browser directly from `Tools -> Creature Templates...`.

## Registry Mode

When you first open the Creatures editor, it starts in registry mode.

Available controls:

- Search box (name or ID filter)
- Category filter (`All`, `Monsters`, `NPCs`, `Templates`, `Variants`, `Custom`)
- Sort selector (`By ID`, `By Name`, `By Category`)
- `Revalidate`
- `Register Asset`
- `Browse Templates`

Registry behavior:

- Single-click selects a creature.
- Double-click opens the selected creature in edit mode.
- Right preview panel shows selected creature metadata.
- Preview panel actions: `Edit`, `Duplicate`, `Delete` (with confirmation).

## Registering Existing Creature Assets

Use `Register Asset` in registry mode to add an existing file into the campaign
registry.

1. Click `Register Asset`.
2. Enter a relative path such as `assets/creatures/goblin.ron`.
3. Click `Validate`.
4. Review the parsed creature summary.
5. Click `Register`.

Notes:

- Paths must be relative and use forward slashes.
- Duplicate IDs are blocked before registration.

## Creating From Templates

You can open templates from two places:

1. `Tools -> Creature Templates...`
2. `Browse Templates` button inside the Creatures editor

Template actions:

- `Create New`: generates a new creature entry and opens it in edit mode.
- `Apply to Current`: replaces mesh data for the currently open creature in edit mode.

## Edit Mode Layout

Edit mode uses a three-panel layout plus a bottom properties area:

- Left: Mesh list
- Center: Live 3D preview
- Right: Mesh properties
- Bottom: Creature-level properties and validation/issues

Top-row actions in edit mode:

- `Save`
- `Cancel`
- `Browse Templates`

## Mesh Editing

### Mesh List Panel

Available actions:

- `Add Primitive`
- `Duplicate` selected mesh
- `Delete` selected mesh
- Visibility toggle per mesh
- Mesh selection for editing

### Mesh Properties Panel

For a selected mesh, you can edit:

- Name
- Color
- Transform:
  translation, rotation (degrees), scale
- Validation and utility actions:
  `Replace with Primitive`, `Validate Mesh`, `Reset Transform`

## Preview Controls

The preview panel supports:

- `Grid`, `Wireframe`, `Normals`, `Axes` toggles
- `Reset Camera`
- Camera distance slider
- Background color picker

Preview updates as you edit transforms, mesh colors, visibility, and selection.

## Creature Properties and Output Actions

Bottom panel fields:

- ID (read-only)
- Name
- Scale
- Optional color tint

Validation panel:

- `Show Issues` toggles validation errors and warnings.

Output actions:

- `Save As...`: writes a new creature asset and appends a new registry entry.
- `Export RON`: copies current creature RON to the clipboard.
- `Revert Changes`: restores edit buffer from registry state.

## Current Scope and Planned Work

The following workflows are not currently wired in the active creature-editor UI
flow and should be treated as planned work:

- Variations workflow
- LOD authoring workflow
- Animation authoring workflow
- Material editing workflow

Do not rely on those paths for production campaign authoring until they are
explicitly documented as shipped.

## Related References

- `sdk/campaign_builder/src/lib.rs`
- `sdk/campaign_builder/src/creatures_editor.rs`
- `docs/explanation/creature_editor_findings_remediation_implementation_plan.md`
