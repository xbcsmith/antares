# Tutorial Campaign

A minimal tutorial campaign used by the Antares project for examples, tests,
and developer onboarding. This campaign demonstrates the innkeeper-based inn
system and provides a small set of maps, NPCs, and premade characters.

## Overview

- Campaign ID: `tutorial` (directory name)
- Purpose: Example/tutorial content and test fixture for campaign tooling
- Format: RON data files under `data/` (maps, npcs, characters, etc.)
- Default starting innkeeper: `tutorial_innkeeper_town` (`starting_innkeeper: "tutorial_innkeeper_town"` in `campaign.ron`)

This campaign intentionally uses string-based innkeeper NPC identifiers (NpcId)
for inn references and map `EnterInn` events (e.g., `innkeeper_id: "tutorial_innkeeper_town"`).
NPCs that act as innkeepers are defined in `data/npcs.ron` and have `is_innkeeper: true`.

## Innkeeper Requirements

**MANDATORY**: All NPCs with `is_innkeeper: true` MUST have a `dialogue_id` configured.

- Default template: Dialogue ID `999` (use this template for campaigns under construction).
- Custom dialogues: Must include a party-management option. This can be implemented using either:
  - `OpenInnManagement { innkeeper_id: "<your_innkeeper_id>" }` action on a terminal node, or
  - a node that triggers `TriggerEvent(event_name: "open_inn_party_management")` (the dialogue runtime will open the inn management UI using the dialogue's speaker NPC ID).
- Validation: The SDK validator will report an error if an innkeeper NPC lacks a `dialogue_id`. Use the validator after edits to verify compliance.

### Example Innkeeper Dialogue

See dialogue ID `4` or `9` in `data/dialogues.ron` for reference implementations. The default template (ID `999`) is also provided in this campaign as a starting point.

## Included Content

- `campaign.ron` — campaign metadata (includes `starting_innkeeper`)
- `config.ron` — game configuration for this campaign
- `data/` — campaign data files:
  - `npcs.ron` — NPC definitions (includes `tutorial_innkeeper_town`, `tutorial_innkeeper_town2`)
  - `maps/` — map files with `EnterInn` events referencing innkeeper IDs
  - other data files as needed by the campaign

## How to Run & Validate

- Run the game with this campaign:
  `cargo run --bin antares -- --campaign campaigns/tutorial`

- Validate campaign structure and content:
  `cargo run --bin campaign_validator -- campaigns/tutorial`

The validator checks:

- Required files and directories exist (including this README)
- `starting_innkeeper` is non-empty and references an NPC that has `is_innkeeper: true`
- Map `EnterInn` events reference valid innkeeper NPC IDs
- Cross-file references (maps, NPCs, characters) are consistent

## Notes for Editors

- If you change innkeeper identifiers, update:

  - `campaign.ron` (`starting_innkeeper`)
  - `data/npcs.ron` (ensure the NPC exists and `is_innkeeper: true`)
  - Any `EnterInn` events in `data/maps/` to use the new `innkeeper_id`

- Follow RON formatting conventions and run the SDK validator after edits.

## Changelog

- Unreleased
  - Added `README.md` to satisfy campaign validation and document `starting_innkeeper` usage.

## License & Credits

- See the project top-level `LICENSE` file for license terms.
- Maintainer / Contact: Brett Smith <xbcsmith@gmail.com>
