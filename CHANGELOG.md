# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### BREAKING CHANGES

- Inn System Migration: Inn identifiers migrated from numeric town IDs to string-based innkeeper NPC IDs.
  - Type removal: `TownId = u8` has been removed.
  - Type addition: `InnkeeperId = String` has been introduced.
  - `CharacterLocation`: `AtInn(u8)` → `AtInn(String)` (now uses `InnkeeperId`).
  - `MapEvent::EnterInn`: `inn_id: u8` → `innkeeper_id: String`.
  - Campaign configuration: `starting_inn: u8` → `starting_innkeeper: String`.
  - Save game format is breaking for the inn/character-location portion: older saves referencing numeric `TownId` values will not be compatible with the new format.
  - Campaign data format is breaking: authors must update any legacy numeric inn references to the new string `innkeeper_id`/`starting_innkeeper` fields.
  - Example conversion: `AtInn(1)` → `AtInn("tutorial_innkeeper_town")`.

**Rationale / Benefits**
- Validation: Innkeeper IDs are validated against campaign NPC definitions (the referenced NPC must have `is_innkeeper: true`).
- UX: Editors and UI can show innkeeper names and portraits directly.
- Authoring clarity: Campaign authors select an NPC ID, not an arbitrary numeric index.

### ADDED

- SDK and validation:
  - Validator checks for `starting_innkeeper` presence, resolves the NPC, and confirms `is_innkeeper == true`.
  - `CampaignLoader::validate_campaign()` surfaces errors/warnings about innkeeper references.
- Campaign Builder:
  - `CampaignMetadata` and editor updated to use `starting_innkeeper: String`.
  - UI updated to accept an innkeeper ID (text input / ComboBox in the editor).
- Tutorial campaign:
  - `campaigns/tutorial/campaign.ron` now includes `starting_innkeeper: "tutorial_innkeeper_town"`.
  - `campaigns/tutorial/README.md` added to satisfy validator structure checks.
  - All `EnterInn` events in tutorial maps reference `innkeeper_id` string IDs.
- Documentation:
  - `docs/reference/campaign_content_format.md` — Added "Inn and Innkeeper System" section.
  - `docs/reference/architecture.md` — Verified `InnkeeperId = String` and updated relevant sections where applicable.
  - `docs/explanation/implementations.md` — Updated with Phase 7 summary and final validation notes.

### CHANGED

- Code and tests updated to use `InnkeeperId` (String) end-to-end in party management, map event handling, campaign loading, and save/load.
- Tests updated to assert the tutorial campaign validates cleanly (no errors/warnings).
- Error messages and SDK suggestions improved to give actionable guidance for invalid innkeeper references.

### MIGRATION NOTES (For Campaign Authors & Tooling)

- Manually update campaign/map data:
  - Map events: change `inn_id: 1` → `innkeeper_id: "tutorial_innkeeper_town"`.
  - Campaign metadata: change `starting_inn: 1` → `starting_innkeeper: "tutorial_innkeeper_town"`.
  - Ensure referenced NPCs exist in `data/npcs.ron` and have `is_innkeeper: true`.
- No automatic migration tool was shipped as part of this change. If you need help migrating many campaigns or save files, a helper that maps numeric `starting_inn` values to canonical `starting_innkeeper` IDs can be implemented and tested.
- Recommended workflow for campaign authors:
  1. Update campaign files as above.
  2. Run the SDK validator: `cargo run --bin campaign_validator -- campaigns/<your_campaign>`
  3. Fix any missing or non-innkeeper NPC references reported by the validator.

### TESTING & QUALITY

- Local checks performed:
  - `cargo fmt --all` — OK
  - `cargo check --all-targets --all-features` — OK
  - `cargo clippy --all-targets --all-features -- -D warnings` — OK
  - Full test suite (`cargo test --lib` / `cargo nextest run --all-features`) — OK
- `campaigns/tutorial` validates cleanly with zero errors and zero warnings after adding the README and updating inn references.

---

If you want, I can:
- Implement a migration helper to map legacy numeric `starting_inn` values to string innkeeper IDs and add tests for it.
- Help batch-update campaign/map files or provide a small script to assist campaign authors.
