# Creature Templates Reference

Complete reference documentation for all available creature templates in Antares.

## Overview

Creature templates provide pre-built starting points for creating custom creatures. Each template is a carefully designed mesh structure that can be customized, scaled, colored, and modified to create unique creatures for your campaigns.

## Template Categories

Templates are organized by body type and theme:

- **Humanoid**: Two-legged, human-like creatures
- **Quadruped**: Four-legged animals and beasts
- **Dragon**: Winged, reptilian creatures
- **Robot**: Mechanical, modular creatures
- **Undead**: Skeletal and ghostly creatures
- **Beast**: Feral, predatory creatures
- **Custom**: User-created templates

## Template Index

| ID   | Name             | Category  | Difficulty   | Meshes | Vertices | Triangles |
|------|------------------|-----------|--------------|--------|----------|-----------|
| 1000 | Humanoid         | Humanoid  | Beginner     | 4      | 32       | 72        |
| 1001 | Quadruped        | Quadruped | Intermediate | 7      | 56       | 126       |
| 1002 | Dragon           | Dragon    | Advanced     | 10     | 80       | 180       |
| 1003 | Robot            | Robot     | Intermediate | 9      | 72       | 162       |
| 1004 | Undead           | Undead    | Intermediate | 9      | 72       | 162       |
| 1005 | Beast            | Beast     | Advanced     | 13     | 104      | 234       |

---

## Template Details

### Humanoid Template (ID: 1000)

**File**: `data/creature_templates/humanoid.ron`

**Category**: Humanoid
**Difficulty**: Beginner
**Author**: Antares Team

#### Description

Basic humanoid template with torso, head, and arms. Perfect starting point for creating human-like creatures such as knights, wizards, townsfolk, and humanoid monsters.

#### Mesh Structure

- **Torso**: Box-shaped body (main mass)
- **Head**: Cube on top of torso
- **Left Arm**: Thin box extending from left shoulder
- **Right Arm**: Thin box extending from right shoulder

#### Customization Options

- **Height**: Adjust torso and arm scale
- **Build**: Scale torso width for thin/muscular builds
- **Head Size**: Scale head independently
- **Arm Length**: Modify arm transforms

#### Example Uses

- Human warrior
- Elf archer
- Dwarf blacksmith
- Orc chieftain
- Goblin scout

#### Tags

`biped`, `basic`, `starter`, `humanoid`

---

### Quadruped Template (ID: 1001)

**File**: `data/creature_templates/quadruped.ron`

**Category**: Quadruped
**Difficulty**: Intermediate
**Author**: Antares Team

#### Description

Four-legged creature template with body, head, legs, and tail. Great for creating animals, mounts, and beasts. Features customizable leg length, body size, and tail length.

#### Mesh Structure

- **Body**: Elongated torso
- **Head**: Forward-facing with snout shape
- **Front-Left Leg**: Positioned at front left
- **Front-Right Leg**: Positioned at front right
- **Back-Left Leg**: Positioned at rear left
- **Back-Right Leg**: Positioned at rear right
- **Tail**: Tapered, extending backward

#### Customization Options

- **Leg Length**: Scale legs vertically
- **Body Size**: Adjust body scale for large/small animals
- **Tail Length**: Modify tail scale
- **Snout**: Adjust head shape for different species

#### Example Uses

- Wolf
- Horse
- Dog
- Cat
- Deer
- Bear

#### Tags

`four-legged`, `animal`, `beast`, `mount`

---

### Dragon Template (ID: 1002)

**File**: `data/creature_templates/dragon.ron`

**Category**: Dragon
**Difficulty**: Advanced
**Author**: Antares Team

#### Description

Majestic dragon template with elongated body, neck, head, wings, legs, and tail. Complex multi-part creature for experienced creators. Features customizable wing size, tail length, and scale colors.

#### Mesh Structure

- **Body**: Elongated torso (muscular)
- **Neck**: Tapered, extending upward
- **Head**: Dragon skull with snout
- **Left Wing**: Triangular wing membrane
- **Right Wing**: Triangular wing membrane (mirrored)
- **Front-Left Leg**: Clawed limb
- **Front-Right Leg**: Clawed limb
- **Back-Left Leg**: Clawed limb
- **Back-Right Leg**: Clawed limb
- **Tail**: Long, tapered tail

#### Customization Options

- **Wing Span**: Scale wings horizontally
- **Neck Length**: Adjust neck scale
- **Tail Length**: Modify tail for different dragon types
- **Scale Color**: Color variants (red, blue, green, gold)
- **Wing Transparency**: Adjust alpha for membrane effect

#### Example Uses

- Red dragon (fire)
- Blue dragon (ice)
- Green dragon (poison)
- Ancient dragon (scaled up)
- Young dragon (scaled down)
- Wyvern (remove front legs)

#### Tags

`flying`, `wings`, `mythical`, `advanced`, `boss`

---

### Robot Template (ID: 1003)

**File**: `data/creature_templates/robot.ron`

**Category**: Robot
**Difficulty**: Intermediate
**Author**: Antares Team

#### Description

Modular robot template with boxy chassis, cube head, antenna, and segmented limbs. Easy to customize and modify. Perfect for sci-fi or steampunk campaigns.

#### Mesh Structure

- **Chassis**: Boxy torso (main body)
- **Head**: Cube with flat surfaces
- **Antenna**: Thin rod on top of head
- **Left Upper Arm**: Segmented shoulder joint
- **Left Forearm**: Segmented elbow joint
- **Right Upper Arm**: Segmented shoulder joint
- **Right Forearm**: Segmented elbow joint
- **Left Leg**: Cylindrical leg
- **Right Leg**: Cylindrical leg

#### Customization Options

- **Modular Parts**: Easy to add/remove segments
- **Metallic Material**: High metallic, low roughness
- **Colors**: Metallic grays, or painted colors
- **Antenna Style**: Remove or replace with sensors

#### Example Uses

- Security robot
- Battle mech
- Service droid
- Steampunk automaton
- Golem variant

#### Tags

`mechanical`, `modular`, `sci-fi`, `steampunk`

---

### Undead Template (ID: 1004)

**File**: `data/creature_templates/undead.ron`

**Category**: Undead
**Difficulty**: Intermediate
**Author**: Antares Team

#### Description

Skeletal undead template with skull head, exposed ribcage, and bone limbs. Perfect for creating skeletons, liches, and spectral creatures. Features ghostly color tint option for spectral variants.

#### Mesh Structure

- **Ribcage**: Thin, exposed rib structure
- **Skull**: Cranium with eye sockets
- **Jaw**: Lower mandible (separate from skull)
- **Left Upper Arm**: Bone structure
- **Left Forearm**: Bone structure
- **Right Upper Arm**: Bone structure
- **Right Forearm**: Bone structure
- **Left Leg**: Bone structure
- **Right Leg**: Bone structure

#### Customization Options

- **Bone Color**: White, yellowed, or darkened
- **Ghostly Tint**: Blue/green transparency for specters
- **Missing Parts**: Remove meshes for damaged skeletons
- **Armor**: Add metallic meshes for armored undead

#### Example Uses

- Skeleton warrior
- Lich
- Skeleton archer
- Ghost/specter (with transparency)
- Bone golem (scaled up)

#### Tags

`skeleton`, `undead`, `bones`, `ghostly`, `horror`

---

### Beast Template (ID: 1005)

**File**: `data/creature_templates/beast.ron`

**Category**: Beast
**Difficulty**: Advanced
**Author**: Antares Team

#### Description

Muscular quadruped beast template with large jaw, claws, and optional horns. Perfect for creating predatory creatures and feral monsters. Features detailed claw meshes and horn options.

#### Mesh Structure

- **Body**: Muscular torso (wider at shoulders)
- **Head**: Large jaw with wide mouth
- **Lower Jaw**: Separate mandible
- **Front-Left Leg**: Muscular limb
- **Front-Left Claw**: Pointed claw mesh
- **Front-Right Leg**: Muscular limb
- **Front-Right Claw**: Pointed claw mesh
- **Back-Left Leg**: Muscular limb
- **Back-Left Claw**: Pointed claw mesh
- **Back-Right Leg**: Muscular limb
- **Back-Right Claw**: Pointed claw mesh
- **Left Horn**: Spike/horn (optional)
- **Right Horn**: Spike/horn (optional)

#### Customization Options

- **Muscle Definition**: Adjust body scale for bulk
- **Claw Size**: Scale claws for different predators
- **Horn Style**: Remove, scale, or reposition horns
- **Jaw Size**: Adjust head for different bite profiles

#### Example Uses

- Dire wolf
- Tiger/lion
- Manticore (add tail spike)
- Hellhound (add fire emissive)
- Demon beast

#### Tags

`predator`, `claws`, `fangs`, `muscular`, `feral`, `dangerous`

---

## Usage Guidelines

### Loading Templates

Templates can be loaded in two ways:

**Method 1: Campaign Builder UI**
1. Open Creature Editor
2. Click File → Load Template
3. Browse to `data/creature_templates/`
4. Select template file

**Method 2: Direct File Reference**
```rust
use antares::domain::visual::CreatureDefinition;

let template: CreatureDefinition =
    ron::from_str(include_str!("data/creature_templates/humanoid.ron"))?;
```

### Template Metadata

Each template has a companion `.meta.ron` file containing:
- Category classification
- Tags for searching
- Difficulty rating
- Author information
- Description
- Thumbnail path (optional)

**Example**:
```ron
TemplateMetadata(
    category: Humanoid,
    tags: ["biped", "basic", "starter"],
    difficulty: Beginner,
    author: "Antares Team",
    description: "Basic humanoid template",
    thumbnail_path: None,
)
```

### Customization Workflow

Recommended workflow for customizing templates:

1. **Load Template**: Start with the closest matching template
2. **Basic Customization**: Adjust color, scale, and transforms
3. **Mesh Modification**: Add/remove meshes as needed
4. **Advanced Features**: Apply materials, LOD, animations
5. **Validation**: Check mesh integrity
6. **Save**: Save as new creature definition

### Difficulty Ratings

**Beginner** (1-3 meshes):
- Simple structure
- Few customization points
- Easy to understand
- Quick to modify

**Intermediate** (4-8 meshes):
- Moderate complexity
- Multiple body parts
- More customization options
- Requires basic understanding

**Advanced** (9+ meshes):
- Complex multi-part structure
- Many customization points
- Advanced features (wings, etc.)
- Requires experience

---

## Performance Considerations

### Vertex Budgets

| Creature Type | Recommended Vertices | Maximum Vertices |
|---------------|---------------------|------------------|
| Common        | < 500               | 1000             |
| Uncommon      | < 1000              | 2000             |
| Rare/Boss     | < 2000              | 5000             |

### LOD Recommendations

All templates should have LOD levels generated:

- **LOD 0** (0-10 units): Full detail
- **LOD 1** (10-30 units): 50% reduction
- **LOD 2** (30+ units): 75% reduction

### Instancing

For creatures that appear in groups (e.g., goblins, skeletons), use instancing:
- One mesh definition
- Multiple instances with different transforms
- Significant performance improvement

---

## Template Compatibility

### Version Compatibility

Templates are compatible with:
- Antares Engine v0.1.0+
- Campaign Builder v0.1.0+
- RON format specification v0.8+

### Future Compatibility

The template format is designed to be forward-compatible. New features may be added without breaking existing templates:
- New optional fields default to `None`
- Existing templates continue to work
- Migration tools provided for major changes

---

## Creating Custom Templates

### Template Requirements

To create a custom template:

1. **Unique ID**: Use ID > 10000 for custom templates
2. **Valid Mesh**: All meshes must pass validation
3. **Metadata File**: Create companion `.meta.ron` file
4. **Naming Convention**: Use lowercase with underscores
5. **Documentation**: Add description and tags

### Publishing Templates

To share your templates with the community:

1. Ensure template passes validation
2. Create high-quality thumbnail (512x512 PNG)
3. Write clear description
4. Include usage examples
5. Submit to template gallery

---

## Example Creatures

The following example creatures are included in `data/creature_examples/`:

- **goblin.ron**: Small humanoid enemy
- **skeleton.ron**: Undead warrior
- **wolf.ron**: Wild animal
- **dragon.ron**: Boss creature
- **orc.ron**: Medium humanoid enemy
- **ogre.ron**: Large humanoid enemy
- **kobold.ron**: Small reptilian enemy
- **zombie.ron**: Slow undead
- **lich.ron**: Undead spellcaster
- **fire_elemental.ron**: Magical creature
- **giant_rat.ron**: Small beast

Each example demonstrates different techniques and customizations.

---

## See Also

- [Creature Creation Quickstart](../tutorials/creature_creation_quickstart.md)
- [How to Create Creatures](../how-to/create_creatures.md)
- [Procedural Mesh Implementation Plan](../explanation/procedural_mesh_implementation_plan.md)
- [Phase 8: Content Creation](../explanation/procedural_mesh_implementation_plan.md#phase-8-content-creation--templates)
