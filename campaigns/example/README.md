# Example Campaign

A simple starter campaign for Antares RPG demonstrating the basic campaign structure and gameplay mechanics.

## Description

This is a minimal example campaign that serves as a template for creating new campaigns. It includes the basic file structure, configuration, and demonstrates how to organize campaign content.

## Story

Welcome to the Example Campaign! This is a simple adventure designed to help you learn the basics of Antares and serve as a starting point for creating your own campaigns.

## Features

- Basic campaign structure
- Simple starting configuration
- Template for data files
- Example directory layout

## Requirements

- Antares Engine v0.1.0 or later
- No special features required

## Installation

1. Copy this campaign directory to your `campaigns/` folder
2. The campaign will appear in the campaign list
3. Select it when starting a new game

## Campaign Structure

```
example/
├── campaign.ron          # Campaign metadata and configuration
├── README.md            # This file
├── data/                # Game content data files
│   ├── items.ron
│   ├── spells.ron
│   ├── monsters.ron
│   ├── classes.ron
│   ├── races.ron
│   ├── quests.ron
│   ├── dialogues.ron
│   └── maps/
└── assets/              # Graphics and audio (optional)
    ├── tilesets/
    ├── music/
    ├── sounds/
    └── images/
```

## Creating Your Own Campaign

Use this campaign as a template:

1. Copy the `example/` directory
2. Rename it to your campaign name
3. Edit `campaign.ron` with your campaign details
4. Add your own content in the `data/` directory
5. Validate with `campaign_validator campaigns/your_campaign`

## Credits

Created by the Antares Team as an example and template for campaign creation.

## License

This example campaign is released under the Apache 2.0 License and can be freely used as a template for your own campaigns.
