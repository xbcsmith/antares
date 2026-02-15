# Creature Creation Quickstart

Get started creating your first custom creature in under 5 minutes!

## What You'll Create

In this quickstart, you'll:
1. Load the humanoid template
2. Change its color to blue
3. Scale it to 2x size
4. Save it as "Blue Giant"
5. Preview it in the game

## Prerequisites

- Campaign Builder installed and running
- Basic familiarity with the UI

## Step 1: Open the Creature Editor

1. Launch the Campaign Builder
2. Click **Tools** → **Creature Editor**
3. The creature editor window will open

## Step 2: Load the Humanoid Template

1. In the creature editor, click **File** → **Load Template**
2. Browse to `data/creature_templates/`
3. Select `humanoid.ron`
4. Click **Open**

You should now see a basic humanoid creature in the preview pane.

## Step 3: Change the Color

1. In the **Properties** panel on the right, find **Color Tint**
2. Click the color picker button
3. Select a bright blue color (e.g., RGB: 0.2, 0.4, 1.0)
4. Click **Apply**

The creature should now appear blue in the preview.

## Step 4: Scale to 2x Size

1. In the **Properties** panel, find **Scale**
2. Change the value from `1.0` to `2.0`
3. Press **Enter** or click outside the field

The creature should now be twice as large.

## Step 5: Save Your Creature

1. Click **File** → **Save As**
2. Enter the name: `blue_giant`
3. Click **Save**

Your creature is now saved to `data/creatures/blue_giant.ron`

## Step 6: Preview in Game (Optional)

1. In the Campaign Builder, go to **Map Editor**
2. Load any test map
3. Click **Place Creature**
4. Select your `blue_giant` from the list
5. Click on the map to place it

Congratulations! You've created your first custom creature.

## What's Next?

Now that you've created a basic creature, try:

- **Customize meshes**: Add or remove body parts in the Mesh Editor
- **Create variations**: Use the Variation Editor to create color/size variants
- **Add animations**: Use the Animation Editor to create simple movement
- **Try other templates**: Explore the dragon, robot, and beast templates

## Common Issues

### Preview is all black
- Check that your color values are between 0.0 and 1.0
- Make sure the preview camera is positioned correctly (use mouse wheel to zoom)

### Changes don't appear
- Click **Refresh Preview** in the toolbar
- Make sure you clicked **Apply** after making changes

### Can't find the template
- Templates are in `data/creature_templates/`
- Make sure you selected a `.ron` file, not a `.meta.ron` file

## Learn More

- Full tutorial: [Create Creatures Guide](../how-to/create_creatures.md)
- Template reference: [Creature Templates](../reference/creature_templates.md)
- Community templates: [Template Gallery](https://antares-rpg.org/templates)
