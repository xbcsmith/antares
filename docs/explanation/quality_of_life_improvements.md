## QOL Improvements

## SDK Campaign Builder main.rs clean up

Let's write a plan to refactor `main.rs` and move UI code for the remaining editors into their own `<foo>_editors.rs` files for consistency with `items_editor.rs`, `monsters_editor.rs`, `conditions_editor.rs`, etc... Also, to reduce the cognitive complexity of `main.rs`, create a method `<Foo>EditorState::show(...)` that takes the same set of parameters as the other editors and renders the UI there. This will make tests and future refactors simpler: tests can search editor-specific files (like we already updated in `bug_verification.rs` for Items/Monsters). Make sure as part of the plan that all the editors auto load data from the campaign.

## Editor Layout Continuity

The monsters_editor, items_editor, and spells_editor all have the same layout and workflow. In the display panel there are buttons to manipulate the data. The buttons are Edit, Delete, Duplicate, and Export. All the editors should follow this pattern.

Editors that need to be fixed are the conditions_editor, quests_editor, dialog_editor, and maps_editor should all follow the same layout and workflow as monsters_editor, items_editor, and spells_editor. This will include a massive refactor for maps_editor

Top level editor rows should scale to fit the screen width. They should all be consistent. Use the layout for quests_editor. Having the name in each button is redundant. The buttons should be New, Save, Load, Import (w/ Merge Checkbox), Export. Import should Load from a File and Export should Save to a File.

EXAMPLE in ASCII:

┌─────────────────────────────────────────────────────────────────┐
│ Foo Editor                                               [X]    │
├─────────────────────────────────────────────────────────────────┤
│ ➕New  💾Save  📂Load  📥Import [x] Merge  📋Export            │
├─────────────────────────────────────────────────────────────────┤
│ 🔍 Search: [________]                                           │
├──────────────────┬──────────────────────────────────────────────┤
│ Foo              │ Foo                                          │
│ ┌──────────────┐ ├──────────────────────────────────────────────│
│ │ 🔍 [filter]  │ │ Edit Delete Dupliate Export                  │
│ ├──────────────┤ ├──────────────────────────────────────────────│
│ │ • Blind      │ │ ┌──────────────────────────────────────────┐ │
│ │ ► Poison     │ │ │ ID: poison (read-only)                   │ │
│ │ • Sleep      │ │ │ Name: [Foo_____________]                 │ │
│ │ • Bless      │ │ │ Description:                             │ │
│ │              │ │ │ [Takes damage over time from poison___]  │ │
│ │              │ │ │                                          │ │
│ │              │ │ │ Duration: [Permanent ▼]                  │ │
│ │              │ │ │ Icon: [icon_poison___________]           │ │
│ │              │ │ │                                          │ │
│ │              │ │ │ Effects:                                 │ │
│ │              │ │ │ ┌──────────────────────────────────────┐ │ │
│ │              │ │ │ │ 1. DOT: 1d4 poison [✏️][🗑️][⬆️][⬇️] │ │ │
│ │              │ │ │ │    ➕ Add Effect                     │ │ │
│ │              │ │ │ └──────────────────────────────────────┘ │ │
│ │              │ │ └──────────────────────────────────────────┘ │
│ └──────────────┘ │                                              │
└──────────────────┴──────────────────────────────────────────────┘

## Campaign Validation

The Validation UI should list all the things it validated with a Green Check or Red X icon next to each file. Use a table like layout to make sure all the CHecks/Errors line up nicely.

## Assets

Assets panel reports 13 unreferenced assets found and lists all the files in the campaign data folder as Unused. I am not sure what we are accomplishing with the Assets panel. It should report the files that are loaded as Loaded or something similar to indicate all the files in the Campaign loaded. If an asset failed to load it should show Error.

This is lower priority than UI improvements.

## Improve Debugging Logging to Console

We should add a verbose logging level to the console logger. This will allow us to see more detailed information about what is happening in the application. We can achieve this by adding a new logging level called `VERBOSE` and setting it to the console logger.

## Testing Improvements

Notes and follow-up suggestions
- The tests currently scan source files via simple string matching. That's a brittle approach (it can break if function signatures or file structure change). Consider one of the following for greater robustness:
  - Make the tests look for the presence/absence of widget ID patterns across the entire SDK source folder rather than limiting it to `main.rs` (i.e., check all `src/*.rs` in the sdk/campaign_builder crate).
  - Use a lightweight Rust source parser (e.g., `syn`) in test code to locate the `pub fn show` method AST and inspect specific method bodies (safer but more complexity).
  - Add a unit test or a small compile-time test API to assert usage of `ComboBox::from_id_salt` vs `ComboBox::from_label` where widgets are created (this would avoid brittle string parsing).
  - Make the test even more resilient (e.g., search all editor files for `from_id_salt` usage),
  - Add a test that ensures all `ComboBox` uses across the SDK use `from_id_salt` rather than any `from_label`,
  - Convert string-check tests into AST-based checks for future-proofing.
