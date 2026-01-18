# Campaign Data Structure Documentation Plan

## Goal Description
Create a comprehensive reference document (`docs/explanation/campaign_data_structures.md`) describing the current Campaign Data Structures. This document will serve as a specification for creating external tools (e.g., a Python script) to manipulate campaign data.

## User Review Required
> [!NOTE]
> This plan focuses on *documentation* of existing structures, not implementation of new features.

## Proposed Changes

### Documentation Artifact
Create `docs/explanation/campaign_data_structures.md` with the following sections:

#### 1. Data Types & Serialization
-   **Format**: Explain RON (Rusty Object Notation) quirks relevant to Python (tuples, enums, `Option` handling).
-   **Primitives**: Define custom primitives like `AttributePair` and `AttributePair16`.
    -   `AttributePair`: `{ base: int, current: int }` (often serialized as tuple `(base, current)` or just `base` if simplified).
    -   `AttributePair16`: Same as above but using `u16`.

#### 2. Core Structures
For each structure, provide:
-   **Rust Definition**: Brief overview or link to source.
-   **Schema**: Pseudo-code or JSON-like schema definition.
-   **Example**: Valid RON snippet.

**Structures to Document:**
1.  **CharacterDefinition** (`src/domain/character_definition.rs`)
    -   Highlight `base_stats` using `AttributePair` (nested structs/tuples).
    -   Highlight `hp_override` (`Option<AttributePair16>`).
    -   Explain backward compatibility fields (`hp_base` etc.) but emphasize the *target* format for new tools.
2.  **NpcDefinition** (`src/domain/world/npc.rs`)
    -   Explain `id` (String) vs internal numeric IDs.
    -   Explain `portrait_id` (String path/stem).
    -   References: `dialogue_id` (u16), `quest_ids` (List[u16]).
3.  **MapBlueprint** (`src/domain/world/blueprint.rs`)
    -   Explain `NpcPlacementBlueprint` decoupling (referencing NPC by String ID).
    -   Explain `events` and `tiles` arrays.
4.  **DialogueTree** (`src/domain/dialogue.rs`) & **Quest** (`src/domain/quest.rs`)
    -   Brief overview of their schemas as they are referenced by ID.

### Validation Rules
-   Document data integrity rules (e.g., `hp_override.current` cannot exceed `base`).
-   Document ID uniqueness constraints (CharacterDefinition IDs, NPC IDs).

## Verification Plan

### Manual Verification
-   Create a sample Python script block in the documentation that parses a snippet using a RON library (or regex/simple parser logic) to demonstrate viability.
