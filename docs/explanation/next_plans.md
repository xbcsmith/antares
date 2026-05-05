# Next Plans

## Generic

## SDK

## Game Engine

### Custom Fonts

Supporting custom fonts requires updates to the campaign config to allow specify a custom Dialogue Font, a Custom Game Menu font. I would expect it to work like this. Default Dialogue Font --> Custom Font in Campaign. The custom Font path should be ./campaigns/<campaign name>/fonts/<font-name>.ttf and it should be configurable by the Campaign Config RON file. If no custom font is specified in the Campaign Config RON file, the default font should be used.

Write a plan with a phased approach to implementing custom fonts. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [custom fonts](./custom_fonts_plan.md)

### Game Tray Icon Implementation Plan

We need to add a tray icon for the game like the ones we added for the SDK.

✅ PLAN WRITTEN - [Game Tray Icon Implementation Plan](./game_tray_icon_implementation_plan.md)

### SKill System Level Scaling

Next task will be to tackle skill system level scaling. Currently the skill system does not scale with character level. We should just do an Auto Skills system to start then follow the route training went with NPC Train Skills.

Write a plan with a phased approach to implementing skill system level scaling in the game engine. THINK HARD and follow the rules in @PLAN.md

✅ PLAN WRITTEN - [Skill System Level Scaling Implementation Plan](./skill_system_level_scaling_implementation_plan.md)


## Notes

Month Year Date in Game Engine View looks horrible.
