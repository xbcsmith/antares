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

✅ COMPLETED - [Skill System Level Scaling Implementation Plan](./skill_system_level_scaling_implementation_plan.md)

## Notes

Month Year Date in Game Engine View looks horrible.

Campaign Builder --> Importer --> Import Obj with an MTL file that has a tecture map. The texture map is not applied so all the models are white. The importer should import the texture map and apply it to the model. This will allow us to use custom textures for our models in the campaign builder. We should also add support for importing models with multiple texture maps, so that we can have more complex models with different textures for different parts of the model. This will be a big improvement for the campaign builder and will allow us to create more visually appealing campaigns.

Campaign Builder --> Importer --> GLB files should be supported. A single file is easier to deal with than multiple files.This way the teture maps are part of the file upload and we can apply the texture to the Creatures RON file and the Furniture and Item RON files. This will allow us to use custom textures for our models in the campaign builder and will make it easier to create visually appealing campaigns.

Write a plan to implement GLB file support in the campaign builder importer. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETE (Phases 1–7) — [GLB File Support in Campaign Builder Importer](./glb_file_support_in_campaign_builder_importer_plan.md)

In the game the new imported models are white washed. I think the color system in the Campaign Builder SDK Model Importer is not doing what we want it to. THe model in the game is imported from a fully textured .glb that has all the colors. Do we really need the color stuff in the Importer? How do we fix the textures getting washed out?

I checked the importer/runtime path, and your suspicion is mostly right: for a fully textured `.glb`, the Campaign Builder’s color system should **not** be driving the final look by default.

## Short answer

- **Keep the color system for OBJ / untextured meshes.**
- **Do not let it tint textured GLB imports by default.**
- The washed-out look is most likely coming from exported material fields, especially:
  - `emissive: Some((1.0, 1.0, 1.0))`
  - `metallic: 1.0`
  - and potentially non-neutral mesh vertex color/tint data.

Your current `whisper_new.ron` has this on the imported mesh:

- `color: (0.0, 0.0, 0.0, 0.0)`
- `material.base_color: (1.0, 1.0, 1.0, 1.0)`
- `material.metallic: 1.0`
- `material.roughness: 1.0`
- `material.emissive: Some((1.0, 1.0, 1.0))`
- `texture_path: Some("assets/textures/imported/whisper_new/baked_basecolor.png")`

That `emissive: Some((1.0, 1.0, 1.0))` is a major red flag: it makes the material glow white and can absolutely make a textured model look white-washed. `metallic: 1.0` can also make a character/creature look wrong under the game’s lighting.

## Immediate manual fix for the exported model

For the affected exported creature RON, set the textured mesh material to something neutral:

- `color: (1.0, 1.0, 1.0, 1.0)`
- `material.base_color: (1.0, 1.0, 1.0, 1.0)`
- `material.metallic: 0.0`
- `material.roughness: 0.8` or `1.0`
- `material.emissive: None`

That should make the texture show normally instead of being brightened/glowed.

## Proper code fix

The importer should treat textured GLB imports differently from OBJ imports:

### 1. GLB textured meshes should default to neutral color

When a GLB primitive has a base-color texture, the importer should export:

- `MeshDefinition.color = [1.0, 1.0, 1.0, 1.0]`
- `MaterialDefinition.base_color = [1.0, 1.0, 1.0, 1.0]`

That way the texture is the source of truth.

The color editor can still exist, but for textured GLB it should be labeled as an optional **tint override**, not normal import color.

### 2. Disable imported emissive by default

For normal textured character/creature assets, importing `emissiveFactor` directly is risky. A GLB with emissive white becomes a glowing white material in Bevy.

Recommended default:

- Ignore GLB emissive unless the user explicitly enables “Import emissive”.
- Or only preserve emissive when there is a real emissive workflow/flag.

### 3. Default metallic to `0.0` for baked texture imports

A baked base-color texture is usually not enough to reproduce PBR correctly. Without metallic/roughness maps, carrying `metallic: 1.0` over from the GLB can make characters look wrong.

Recommended default for textured creature imports:

- `metallic: 0.0`
- `roughness: 0.8` or `1.0`
- `emissive: None`

### 4. Avoid auto-assign colors for GLB textures

The `Auto-Assign All` palette behavior is useful for OBJ meshes with no textures, but it should not apply to textured GLB meshes by default. For GLB, the UI should probably either:

- Hide `Auto-Assign All`, or
- Rename it to “Apply Palette Tint” and warn that it will tint textures.

## Why this is not really a “texture extraction” problem

The importer appears to be exporting the embedded GLB texture to:

`assets/textures/imported/whisper_new/baked_basecolor.png`

And the runtime loads `MeshDefinition.texture_path` through Bevy’s `AssetServer`. So the issue is probably not that the PNG bytes are bad.

The issue is that the exported material is telling Bevy:

> “Use this texture, but also make the surface metallic and emit white light.”

That causes the visible washed-out result.

## Recommended fix order

1. Patch existing bad exported assets:

   - set `color` to white/opaque
   - set `metallic` to `0.0`
   - set `emissive` to `None`

2. Update `sdk/campaign_builder/src/mesh_glb_io.rs`:

   - textured GLB meshes export neutral color/base color
   - no emissive by default
   - metallic defaults to `0.0` for texture-backed importer output

3. Update `sdk/campaign_builder/src/obj_importer_ui.rs`:
   - keep palette/color editing for OBJ
   - make it optional tinting for GLB
   - prevent `Auto-Assign All` from silently tinting textured GLB meshes

✅ COMPLETED - So: \*\*yes, keep the color stuff for OBJ/untextured imports, but don’t let it drive fully textured GLB imports unless the user explicitly asks for tinting.

✅ COMPLETED - Walking over a Recruitable Character mesh makes them disappear from the screen without triggering a recruiting dialogue. The Recruitable Character should remain on the map until recruited. Once recruited then the mesh should disapear.

Why are the shadows on the models so bad? The new whisper model has a shadow under her chin that is so black you can not see anything. Very unrealistic

Here is what you already know

Inspecting creature spawning\*\*

I need to check the timing of creature spawning and the materials involved. I'm also considering whether there's a new whisper. My potential answer could be, "The current lighting likely uses just a single high contrast directional sun without any ambient or fill lights, and the model has baked Ambient Occlusion (AO)." So I need to inspect these elements to confirm and clarify. Let's explore this further!

Inspecting texture and lighting\*\*

I need to get a specific answer regarding the texture loading. I think I should check if the texture has been applied without an ambient occlusion map. The black area under the chin could be due to self-shadowing from dynamic shadows. To clarify, I need to know the actual light's position. The camera sets the point light at (0, 5, 0) during startup, but I'm wondering if the update moves the light. I should read the update lines to confirm.

I need to come up with potential fixes and ensure I mention that shadow quality isn’t being used anywhere in the configuration. This suggests the shadow quality setting isn't impacting rendering. I could explain that the graphics are currently set to a single camera-following point light and lack fill, HDR, and contact shadow tuning. If the user wants to fix it, I could point out specifics in the configuration files and provide relevant line numbers.**Analyzing light and texture issues**

If the Whisper model is close to the viewer, the point light positioned above or behind can cause chin and lip occlusion of the neck, without fill light or bounce. This results in hard self-shadows. The new model includes facial geometry under the chin, which catches occlusion. I should check the RON texture named baked_basecolor in case it contains baked shadows. It might be a large PNG file, so I'll find the path and inspect it closely.

## Art Notes

Art note (actionable, but not a code fix)

The `baked_basecolor.png` texture file in `whisper_new.ron` was likely exported from Blender with Ambient Occlusion pre-multiplied into the base colour channel. That pre-baked darkness in the concave chin/neck area was compounding with the dynamic shadow. The two code fixes above are enough to make those areas visible. If you want the model to look its best long-term, re-export the texture with AO in a separate channel (`occlusion_texture`) rather than baked into albedo — but that's an artist task, not a code task.

## Creature Mesh Selector

Campaing Builder --> Monster --> Edit --> Visual Asset hangs constantly when trying to change the creature mesh. It hangs when trying to scroll the window and it constantly hangs trying to select the mesh. It should work like the Character or NPC mesh selection wich is a text box with autocomplete.

✅ COMPLETED -

## Game Engine Sky

The Sky is Grey in the game. We have no concept of weather or if we are in a building. We should be able to mark an area inside or outside on the map ron. The SDK Map Editor should be able to mark an area inside or outside. When outside the sky should be whatever color that worlds sky is with as many suns as that world has. It should be part of the map ron to control the sky color and number of suns. We should also have a concept of night and day based on the clock time of day. Day has suns and night has stars. We should also be able to set star density based on map ron data. The SDK Map Editor should be updated to support setting the day sky color, number of suns, night sky color, and number of stars visible. We should have cloud coverage and color and density settings.

Write a plan with a phased approach to add this support to the game engine and SDK. THINK HARD and follow the rules in @PLAN.md

✅ COMPLETED - [Sky System Implementation Plan](./sky_system_implementation_plan.md)

## Landscape Category

Status: completed. Landscape is now a first-class importer, SDK, map-editor, runtime, and campaign-data feature. Current campaign-facing assets live under `assets/meshes/landscape/`, landscape definitions live in `data/landscape.ron`, and landscape mesh registries live in `data/landscape_mesh_registry.ron`. See `docs/explanation/landscape_implementation_plan.md` for the historical plan and `docs/explanation/implementations.md` for implementation summaries.

✅ COMPLETED - [landscape implementation plan](./landscape_implementation_plan.md)

## BUGS

### Game Bugs

✅ COMPLETED - When leaving player inventory with ESC button the game menu pops up immediately. ESC in any inventory screen should just close the inventory.

Party Inventory screen Mouse does not work on the Next Back or Party Overview buttons in the inventory screen.

Party Overview only shows the character that opened the inventory. It should show the whole party.

Recruitable Characters 3d models turn there back on the party when talking to them. There is no way to turn off auto rotate towards party in the SDK. For some reason the models seem to be oriented backwards causing the user to set which direction they face in the SDK. This causes the model to turn the other direction when facing the party and dialogue is engaged.

Sorcerer Spell point bar does not go down as spell points are used.

Can not select a Rest option of (1) (2) or (3) because it opens the 1,2, or 3 character sheet. Remove triggering character sheet by number when in the rest menu. The rest menu should be navigable by number but it should not trigger the character sheet.

✅ COMPLETED - When playing tutorial campaign ./campaigns/tutorial The oak and pine trees in the middle of that map_1 are not rendering. See the screenshot ./screenshots/trees_map_1.png The forrest of multiple different types of trees in map_1 are not using the new tree models. They look weird as you can see in the screenshot ./screenshots/forrest_map_1.png None of the trees on map_2 are rendering. I am assuming something changed in the tree rendering code and it broke the trees in the tutorial campaign. We should fix the tree rendering code so that the trees in the tutorial campaign render correctly.

Hit A Trap. Everyone died. Mode is Gamever but I can keep moving around and the game is still responsive. I should not be able to move around when the mode is GameOver. The game should be frozen and I should only be able to click on the Game Over menu options.
2026-06-11T20:14:50.026220Z  INFO antares::game::systems::input::global_toggles: Rest key pressed but mode is GameOver — ignoring

Open a Chest. Select item aad click take. Item modes to inventory. TAB back to chest Item reapears in the chest. It should be removed from the chest when taken. Once item is taken there is no way to exit the Chest. ESC does not work. The chest menu should be exited when pressing ESC.

### SDK Bugs

✅ COMPLETED - Campaign Builder --> Monster Editor --> Edit save button at the bottom of the editor does not always Save the monster when it returns to the list

Campaign Builder --> Importer --> Items and Furniture do not have category drop down like Creatures. Importing Furnture does not update the furniture.ron

✅ COMPLETED - Campaign Builder --> Landscape Editor layout is broken. The left hand list is horizontal and it does not follow the patterns in @sdk/AGENTS.md for how to layout an editor. The list should be vertical and the editor should follow the same patterns as the other editors in the Campaign Builder.

✅ COMPLETED - Campaign Builder --> Landscape Editor --> Delete does nothing. It should delete the landscape and update the landscape.ron file and the landscape_mesh_registry.ron file.
