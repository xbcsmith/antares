# Antares RPG Documentation

Welcome to the Antares RPG documentation. This guide will help you  find what you need.

---

## For Players

**Getting Started**:
- [Getting Started with Antares](tutorials/getting_started.md) - Install and play the game
- [Playing Campaigns](how-to/playing_campaigns.md) - How to play custom campaigns

---

## For Modders and Campaign Creators

**Tutorials** (Learning-oriented - Start here):
- [Getting Started with Campaign Creation](tutorials/getting_started_campaign_creation.md) - First campaign tutorial
- [Creating Campaigns](tutorials/creating_campaigns.md) - Comprehensive campaign tutorial

**How-To Guides** (Task-oriented - Solve specific problems):
- [Creating Maps](how-to/creating_maps.md) - Map creation guide
- [Using Quest and Dialogue Tools](how-to/using_quest_and_dialogue_tools.md) - Quest/dialogue editing
- [Using SDK Tools](how-to/using_sdk_tools.md) - SDK command-line tools
- [Creating and Validating Campaigns](how-to/creating_and_validating_campaigns.md) - Campaign workflow
- [Package and Test Campaigns](how-to/package_and_test_campaigns.md) - Distribution preparation
- [Play Custom Campaigns](how-to/play_custom_campaigns.md) - Loading and testing
- [Using Map Builder](how-to/using_map_builder.md) - Map builder tool guide
- [Using SDK Map Editor](how-to/using_sdk_map_editor.md) - SDK map editor
- [Using Item Editor](how-to/using_item_editor.md) - Item editing tool

**Explanation** (Understanding-oriented - Learn concepts):
- [Modding Guide](explanation/modding_guide.md) - Comprehensive modding concepts and patterns
- [SDK and Campaign Architecture](explanation/sdk_and_campaign_architecture.md) - SDK design overview
- [Engine Options](explanation/engine_options.md) - Why Bevy was chosen
- [Map to SDK Bridge](explanation/map_to_sdk_bridge.md) - Technical bridge explanation
- [Testing Philosophy](explanation/testing_philosophy.md) - Automated vs manual testing

**Reference** (Technical specifications):
- [Architecture](reference/architecture.md) - System architecture (**READ THIS FIRST** for development)
- [SDK API Reference](reference/sdk_api.md) - Complete SDK API
- [Map RON Format](reference/map_ron_format.md) - Map file format specification
- [Data Dependencies](reference/data_dependencies.md) - Content relationship reference
- [World Layout](reference/world_layout.md) - World structure reference

---

## For Developers

**Setup and Contributing**:
- See **AGENTS.md** in project root for mandatory development rules
- [Architecture](reference/architecture.md) - **CRITICAL**: Read before coding

**Development How-Tos**:
- [Creating Maps](how-to/creating_maps.md) - Map creation workflows
- [Testing Philosophy](explanation/testing_philosophy.md) - Testing approaches

---

## Quick Links

### Common Tasks
- **Install and play** → [Getting Started](tutorials/getting_started.md)
- **Create a campaign** → [Campaign Tutorial](tutorials/creating_campaigns.md)
- **Edit maps** → [Creating Maps](how-to/creating_maps.md)
- **Validate campaign** → [Package and Test](how-to/package_and_test_campaigns.md)
- **Learn modding concepts** → [Modding Guide](explanation/modding_guide.md)

### Reference
- **Game architecture** → [Architecture](reference/architecture.md)
- **RON formats** → [Map RON Format](reference/map_ron_format.md)
- **SDK API** → [SDK API](reference/sdk_api.md)

---

## Documentation Organization

This documentation follows the [Diataxis Framework](https://diataxis.fr/):

- **Tutorials**: Step-by-step learning guides for beginners
- **How-To Guides**: Task-oriented recipes for specific problems
- **Explanation**: Conceptual discussions and design decisions
- **Reference**: Technical specifications and API documentation

---

## About Antares RPG

Antares is a classic turn-based RPG inspired by Might and Magic 1, built with:
- **Engine**: Rust + Bevy game engine
- **Data Format**: RON (Rusty Object Notation)
- **Modding**: Campaign SDK for creating custom content

---

**Need Help?** Check the relevant section above or see AGENTS.md for development guidelines.
