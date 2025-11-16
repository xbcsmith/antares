# How to Use the Item Editor

This guide explains how to use the `item_editor` CLI tool to create and manage
item definitions for Antares RPG.

## Overview

The Item Editor is an interactive command-line tool for creating and editing
game items including weapons, armor, accessories, consumables, ammunition, and
quest items. It saves data in RON (Rusty Object Notation) format.

## Starting the Editor

### Default Location

Edit items in the default `data/items.ron` file:

```bash
cargo run --bin item_editor
```

Or use the compiled binary:

```bash
./target/release/item_editor
```

### Custom File Location

Edit items in a specific file:

```bash
cargo run --bin item_editor -- campaigns/my_campaign/data/items.ron
```

### Creating New Item Files

The editor will create a new file if it doesn't exist:

```bash
cargo run --bin item_editor -- campaigns/new_campaign/data/items.ron
```

The editor automatically creates parent directories as needed.

## Main Menu

When you start the editor, you'll see:

```
╔════════════════════════════════════════╗
║       ANTARES ITEM EDITOR              ║
╚════════════════════════════════════════╝
  File: data/items.ron
  Items: 42

  [1] List Items
  [2] Add Item
  [3] Edit Item
  [4] Delete Item
  [5] Preview Item
  [6] Save & Exit
  [Q] Quit (discard changes)
```

## Listing Items

Select option `[1]` to view all items:

```
════════════════════════════════════════
  ITEM LIST
════════════════════════════════════════
  [1] Club - Weapon
  [2] Dagger - Weapon
  [3] Leather Armor - Armor
  [4] Healing Potion - Consumable
  [5] Ring of Protection - Accessory ✨
  [6] Cursed Mace - Weapon 💀
```

Symbols:
- ✨ = Magical item (has bonuses, charges, or spell effects)
- 💀 = Cursed (cannot be unequipped)

## Adding Items

Select option `[2]` to add a new item. The editor will guide you through:

### Step 1: Basic Information

```
════════════════════════════════════════
  ADD NEW ITEM
════════════════════════════════════════
  Auto-assigned ID: 43
Item name: Flaming Sword
```

### Step 2: Select Item Type

```
Item Type:
  [1] Weapon
  [2] Armor
  [3] Accessory
  [4] Consumable
  [5] Ammunition
  [6] Quest Item
Type: 1
```

### Step 3: Type-Specific Configuration

#### Weapons

```
  Weapon Configuration:
Damage dice (format: 1d8 or 2d6+1): 2d6+2
To-hit/damage bonus: 3
Hands required (1 or 2): 1
```

Creates a weapon with:
- 2d6+2 base damage
- +3 to-hit and damage bonus
- 1-handed

#### Armor

```
  Armor Configuration:
AC bonus: 8
Weight (pounds): 50
```

#### Accessories

```
  Accessory Type:
    [1] Ring
    [2] Amulet
    [3] Belt
    [4] Cloak
  Slot: 1
```

#### Consumables

```
  Consumable Effect:
    [1] Heal HP
    [2] Restore SP
    [3] Cure Condition
    [4] Boost Attribute
  Effect: 1
HP to heal: 50
Usable in combat? (y/n): y
```

#### Ammunition

```
  Ammunition Type:
    [1] Arrow
    [2] Bolt
    [3] Stone
  Type: 1
Quantity per bundle: 20
```

#### Quest Items

```
  Quest Item Configuration:
Quest ID: elder_scroll_quest
Is key item (cannot drop/sell)? (y/n): y
```

### Step 4: Economic Properties

```
Base cost (gold): 5000
Sell cost (gold): 2500
```

### Step 5: Class Restrictions

```
  Class Restrictions:
    [1] All classes can use (0xFF)
    [2] No classes (quest item)
    [3] Custom selection
  Choice: 3

    Select classes that CAN use this item:
    Knight? (y/n): y
    Paladin? (y/n): y
    Archer? (y/n): y
    Cleric? (y/n): n
    Sorcerer? (y/n): n
    Robber? (y/n): y
    Good alignment only? (y/n): n
    Evil alignment only? (y/n): n
```

### Step 6: Magical Properties (Optional)

```
  Add Constant bonus (passive)? (y/n): y

  Bonus Attribute:
    [1] Might
    [2] Intellect
    [3] Personality
    [4] Endurance
    [5] Speed
    [6] Accuracy
    [7] Luck
    [8] Fire Resistance
    [9] Cold Resistance
    [10] Electricity Resistance
    [11] Acid Resistance
    [12] Poison Resistance
    [13] Magic Resistance
    [14] Armor Class
  Attribute: 8
  Value: 20

  Add Temporary bonus (on use)? (y/n): n
```

### Step 7: Spell Effects and Charges

```
Spell effect ID (0 for none): 260
Max charges: 30
```

Spell effect encoding: `high_byte = school, low_byte = spell`
- Example: 0x0104 (260) = Fire school, spell 4

### Step 8: Cursed Status

```
Is cursed? (y/n): n
```

### Completion

```
✅ Item added successfully!
```

The item is now in memory. Remember to save before exiting!

## Previewing Items

Select option `[5]` and enter the item ID:

```
Enter item ID to preview: 43

════════════════════════════════════════
  ITEM PREVIEW
════════════════════════════════════════
  ID: 43
  Name: Flaming Sword
  Type: Weapon
  Damage: 2d6+2
  Bonus: 3
  Hands: 1
  Base Cost: 5000 gp
  Sell Cost: 2500 gp
  Disablement Flags: 0x2B
  Constant Bonus: ResistFire +20
  Spell Effect: 0x0104
  Max Charges: 30
  ✨ MAGICAL

  Press Enter to return...
```

## Editing Items

Select option `[3]` to edit existing items:

```
Enter item ID to edit: 5

  Editing: Ring of Protection
  Note: For now, delete and re-add to change item data.
        This preserves structural integrity.
```

**Note**: Full editing is not yet implemented. To modify an item:
1. Preview the item to note its properties
2. Delete the item
3. Add it again with updated properties

## Deleting Items

Select option `[4]`:

```
Enter item ID to delete: 99
Delete "Broken Sword"? (y/n): y
✅ Item deleted.
```

## Saving Changes

### Save and Exit

Select option `[6]` to save and exit:

```
💾 Saving to data/items.ron...
✅ Saved successfully. Exiting.
```

### Quit Without Saving

Select option `[Q]`:

```
⚠️  You have unsaved changes!
Discard changes and exit? (y/n): n
```

Press `n` to return to the menu, or `y` to discard changes and quit.

## Output Format

Items are saved in RON format with pretty-printing:

```ron
// items.ron - Item definitions
//
// Generated by item_editor
// Total items: 43

[
    (
        id: 1,
        name: "Club",
        item_type: Weapon((
            damage: (count: 1, sides: 3, bonus: 0),
            bonus: 0,
            hands_required: 1,
        )),
        base_cost: 1,
        sell_cost: 0,
        disablements: (255),
        constant_bonus: None,
        temporary_bonus: None,
        spell_effect: None,
        max_charges: 0,
        is_cursed: false,
    ),
    // ... more items ...
]
```

## Common Workflows

### Creating a Basic Weapon

1. Add item → Weapon type
2. Set damage dice (e.g., `1d8`)
3. Set bonus to 0 for non-magical
4. Choose 1 or 2 hands
5. Set costs (base and sell)
6. Select class restrictions (usually "All classes")
7. Skip magical properties
8. Not cursed

### Creating a Magical Item

1. Follow basic item creation
2. When prompted for constant bonus, add attribute boost
3. Add spell effect ID if it casts spells
4. Set max charges (10-50 typical)
5. Not cursed (unless intentionally cursed)

### Creating a Cursed Item

1. Create item normally
2. Usually has attractive bonuses to tempt players
3. Answer "y" to "Is cursed?" question
4. Once equipped, character cannot remove it

### Creating a Quest Item

1. Select "Quest Item" type
2. Enter quest ID (matches quest definition)
3. Mark as key item if required for quest
4. Set "No classes" restriction (quest items aren't equippable)

## Tips and Best Practices

### Item ID Management

- IDs are auto-assigned sequentially
- Don't reuse deleted IDs (causes validation issues)
- Keep IDs sequential for easier maintenance

### Class Restrictions

- Use "All classes" (0xFF) for common items
- Use "No classes" (0x00) for quest items only
- Custom selection for class-specific gear

### Disablement Bit Flags

The disablement system uses bit flags:
- Bit 0 (0x01): Knight
- Bit 1 (0x02): Paladin
- Bit 2 (0x04): Archer
- Bit 3 (0x08): Cleric
- Bit 4 (0x10): Sorcerer
- Bit 5 (0x20): Robber
- Bit 6 (0x40): Good alignment
- Bit 7 (0x80): Evil alignment

### Pricing Guidelines

- Sell cost typically 50% of base cost
- Magical items: base cost increases exponentially with power
- Common weapons: 1-100 gp
- Magic weapons: 500-10000 gp
- Artifacts: 10000+ gp

### Magical Item Design

**Constant Bonus** (passive):
- Always active while equipped/carried
- +stat bonuses, resistances, AC
- No charges consumed

**Temporary Bonus** (active):
- Requires charges
- Activated by using the item
- Limited duration (implementation-dependent)

**Spell Effects**:
- Requires charges
- Casts a spell when used
- Useful for utility items (wands, staves)

### Balance Considerations

- Two-handed weapons should have higher damage than one-handed
- Heavy armor should provide more AC but reduce speed/accuracy
- Cursed items should have apparent benefits to trap unwary players
- Quest items should have no combat value (prevents exploits)

## Troubleshooting

### File Not Found Error

**Problem**: Editor can't find the file

**Solution**: Check the file path. The editor will create a new file if it
doesn't exist, but verify the directory structure is correct.

### Invalid RON Syntax

**Problem**: Manually edited file won't load

**Solution**: Use `campaign_validator` to check syntax:

```bash
cargo run --bin campaign_validator -- campaigns/my_campaign
```

### Duplicate Item IDs

**Problem**: Two items have the same ID

**Solution**: Delete one and re-add with auto-assigned ID.

### Can't Save Changes

**Problem**: Permission denied when saving

**Solution**: Check file/directory permissions:

```bash
ls -la data/
```

Ensure write permissions are set.

## Integration with Other Tools

### With Campaign Validator

Validate items after editing:

```bash
cargo run --bin item_editor
# Make changes...
cargo run --bin campaign_validator -- campaigns/my_campaign
```

### With Map Builder

Items referenced in maps must exist:

```bash
# Edit items first
cargo run --bin item_editor

# Then edit maps
cargo run --bin map_builder
```

### With Class Editor

Ensure class restrictions match available classes:

```bash
# Check classes
cargo run --bin class_editor -- data/classes.ron

# Update item restrictions
cargo run --bin item_editor -- data/items.ron
```

## Advanced Usage

### Batch Item Creation

For creating many similar items, consider:

1. Create one template item in the editor
2. Save and exit
3. Manually copy/edit the RON file for variations
4. Validate with `campaign_validator`

### Custom Spell Effects

Spell effect IDs are encoded as `school * 256 + spell_number`:

- Fire (school 1): 0x0100 - 0x01FF (256-511)
- Cold (school 2): 0x0200 - 0x02FF (512-767)
- Lightning (school 3): 0x0300 - 0x03FF (768-1023)

Example: Fire Bolt (school 1, spell 4) = 0x0104 = 260

### Multi-Campaign Management

Use separate item files per campaign:

```bash
# Core game items
cargo run --bin item_editor -- data/items.ron

# Campaign-specific items
cargo run --bin item_editor -- campaigns/desert_quest/data/items.ron
cargo run --bin item_editor -- campaigns/ice_realm/data/items.ron
```

## See Also

- [Architecture Reference](../reference/architecture.md) - Item data structures
- [Using Class Editor](using_class_editor.md) - Class restriction setup
- [Using Campaign Validator](validating_campaigns.md) - Item validation
- [SDK Implementation Plan](../explanation/sdk_implementation_plan.md) - Phase 7 details
