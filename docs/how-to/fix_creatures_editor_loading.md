# How to Fix Creatures Editor Loading

## Quick Summary

The creatures editor fails to load creatures because `campaigns/tutorial/data/creatures.ron` contains `CreatureReference` entries (lightweight registry), but the campaign builder tries to parse them as `CreatureDefinition` entries (full definitions).

**Solution**: Implement registry-based loading that reads the registry file, then loads individual creature files from `assets/creatures/`.

## Step-by-Step Fix

### Step 1: Update load_creatures() in lib.rs

**File**: `sdk/campaign_builder/src/lib.rs`

**Location**: Find `fn load_creatures(&mut self)` around line 1961

**Change**: Replace the entire function with:

```rust
fn load_creatures(&mut self) {
    let creatures_file = self.campaign.creatures_file.clone();
    if let Some(ref dir) = self.campaign_dir {
        let creatures_path = dir.join(&creatures_file);
        if creatures_path.exists() {
            match fs::read_to_string(&creatures_path) {
                Ok(contents) => {
                    // Step 1: Parse registry file as Vec<CreatureReference>
                    match ron::from_str::<Vec<antares::domain::visual::CreatureReference>>(
                        &contents,
                    ) {
                        Ok(references) => {
                            // Step 2: Load full definitions for each reference
                            let mut creatures = Vec::new();
                            let mut load_errors = Vec::new();
                            
                            for reference in references {
                                let creature_path = dir.join(&reference.filepath);
                                
                                match fs::read_to_string(&creature_path) {
                                    Ok(creature_contents) => {
                                        match ron::from_str::<
                                            antares::domain::visual::CreatureDefinition
                                        >(&creature_contents) {
                                            Ok(creature) => {
                                                // Validate ID match
                                                if creature.id != reference.id {
                                                    load_errors.push(
                                                        format!(
                                                            "ID mismatch for {}: registry={}, file={}",
                                                            reference.filepath,
                                                            reference.id,
                                                            creature.id
                                                        )
                                                    );
                                                } else {
                                                    creatures.push(creature);
                                                }
                                            }
                                            Err(e) => {
                                                load_errors.push(
                                                    format!(
                                                        "Failed to parse {}: {}",
                                                        reference.filepath, e
                                                    )
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        load_errors.push(
                                            format!(
                                                "Failed to read {}: {}",
                                                reference.filepath, e
                                            )
                                        );
                                    }
                                }
                            }
                            
                            if load_errors.is_empty() {
                                let count = creatures.len();
                                self.creatures = creatures;
                                
                                if let Some(ref mut manager) = self.asset_manager {
                                    manager.mark_data_file_loaded(&creatures_file, count);
                                }
                                
                                self.status_message =
                                    format!("Loaded {} creatures", count);
                            } else {
                                if let Some(ref mut manager) = self.asset_manager {
                                    manager.mark_data_file_error(
                                        &creatures_file,
                                        &format!("{} errors loading creatures", load_errors.len()),
                                    );
                                }
                                
                                self.status_message = format!(
                                    "Loaded {} creatures with {} errors:\n{}",
                                    creatures.len(),
                                    load_errors.len(),
                                    load_errors.join("\n")
                                );
                                eprintln!("Creature loading errors: {}", load_errors.join("\n"));
                            }
                        }
                        Err(e) => {
                            if let Some(ref mut manager) = self.asset_manager {
                                manager.mark_data_file_error(&creatures_file, &e.to_string());
                            }
                            self.status_message = 
                                format!("Failed to parse creatures registry: {}", e);
                            eprintln!(
                                "Failed to parse creatures registry {:?}: {}",
                                creatures_path, e
                            );
                        }
                    }
                }
                Err(e) => {
                    if let Some(ref mut manager) = self.asset_manager {
                        manager.mark_data_file_error(&creatures_file, &e.to_string());
                    }
                    self.status_message = format!("Failed to read creatures registry: {}", e);
                    eprintln!("Failed to read creatures registry {:?}: {}", creatures_path, e);
                }
            }
        } else {
            eprintln!("Creatures file does not exist: {:?}", creatures_path);
        }
    } else {
        eprintln!("No campaign directory set when trying to load creatures");
    }
}
```

### Step 2: Update save_creatures() in lib.rs

**File**: `sdk/campaign_builder/src/lib.rs`

**Location**: Find `fn save_creatures(&mut self) -> Result<(), String>` around line 2010

**Change**: Replace with:

```rust
fn save_creatures(&mut self) -> Result<(), String> {
    if let Some(ref dir) = self.campaign_dir {
        // Step 1: Create registry entries from creatures
        let references: Vec<antares::domain::visual::CreatureReference> = self.creatures
            .iter()
            .map(|creature| {
                let filename = creature.name
                    .to_lowercase()
                    .replace(" ", "_")
                    .replace("'", "")
                    .replace("-", "_");
                    
                antares::domain::visual::CreatureReference {
                    id: creature.id,
                    name: creature.name.clone(),
                    filepath: format!("assets/creatures/{}.ron", filename),
                }
            })
            .collect();

        // Step 2: Save registry file (creatures.ron)
        let creatures_path = dir.join(&self.campaign.creatures_file);
        if let Some(parent) = creatures_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create creatures directory: {}", e))?;
        }

        let registry_ron_config = ron::ser::PrettyConfig::new()
            .struct_names(false)
            .enumerate_arrays(true)
            .separate_tuple_members(true)
            .depth_limit(2);

        let registry_contents = ron::ser::to_string_pretty(&references, registry_ron_config)
            .map_err(|e| format!("Failed to serialize creatures registry: {}", e))?;

        fs::write(&creatures_path, registry_contents)
            .map_err(|e| format!("Failed to write creatures registry: {}", e))?;

        // Step 3: Save individual creature files (assets/creatures/*.ron)
        let creatures_dir = dir.join("assets/creatures");
        fs::create_dir_all(&creatures_dir)
            .map_err(|e| format!("Failed to create creatures assets directory: {}", e))?;

        let creature_ron_config = ron::ser::PrettyConfig::new()
            .struct_names(false)
            .enumerate_arrays(false)
            .depth_limit(3);

        for (reference, creature) in references.iter().zip(self.creatures.iter()) {
            let creature_path = dir.join(&reference.filepath);
            
            let creature_contents = ron::ser::to_string_pretty(creature, creature_ron_config.clone())
                .map_err(|e| {
                    format!("Failed to serialize creature {}: {}", reference.name, e)
                })?;

            fs::write(&creature_path, creature_contents)
                .map_err(|e| {
                    format!("Failed to write creature file {}: {}", reference.filepath, e)
                })?;
        }

        self.status_message = format!(
            "Saved {} creatures (registry + {} individual files)",
            self.creatures.len(),
            self.creatures.len()
        );
        self.unsaved_changes = true;
        Ok(())
    } else {
        Err("No campaign directory set".to_string())
    }
}
```

### Step 3: Add CreatureReference Import

**File**: `sdk/campaign_builder/src/lib.rs`

**Location**: Find the imports at the top of the file (around line 1-50)

**Check**: Ensure you have this import:
```rust
use antares::domain::visual::CreatureReference;
```

If not present, add it to the existing imports from `antares::domain::visual`.

### Step 4: Test the Fix

1. **Open Tutorial Campaign**
   - File → Open Campaign
   - Navigate to `campaigns/tutorial`
   - Select and open

2. **Navigate to Creatures Tab**
   - Click on the "Creatures" tab
   - Should see list of ~40 creatures (Goblin, Dragon, Skeleton, etc.)

3. **Verify Creatures Loaded**
   - Check status message: "Loaded 40 creatures" (or similar count)
   - Scroll through the list
   - Double-click a creature to edit and verify meshes load

4. **Test Save**
   - Make a small edit (rename a creature)
   - Click Save
   - Verify status: "Saved 40 creatures (registry + 40 individual files)"
   - Close and reopen campaign to verify persistence

## What This Fix Does

### Before
- Tries to parse `creatures.ron` as `Vec<CreatureDefinition>`
- Fails because file contains `Vec<CreatureReference>` (lightweight registry)
- Creatures editor shows empty list
- Error: "Failed to parse creatures: unknown field `filepath`"

### After
- Parses `creatures.ron` as `Vec<CreatureReference>` (registry)
- For each reference, loads individual creature file from `assets/creatures/`
- Parses each file as `CreatureDefinition`
- Combines all creatures into the editor
- Saves both registry and individual files

## File Structure After Fix

```
campaigns/tutorial/
├── data/
│   └── creatures.ron                    ← Registry (Vec<CreatureReference>)
└── assets/creatures/
    ├── goblin.ron                       ← Individual definitions
    ├── dragon.ron
    ├── skeleton.ron
    ├── wolf.ron
    ├── orc.ron
    └── ... (35+ more)
```

## Troubleshooting

### "Failed to parse creatures registry"
- Check if `creatures.ron` exists and is readable
- Ensure it's valid RON syntax
- Look at error message for specific parse failure

### "Failed to read assets/creatures/goblin.ron"
- File doesn't exist in the expected location
- Check that filename matches the filepath in registry
- Verify file is readable and contains valid RON

### "ID mismatch for assets/creatures/goblin.ron"
- Creature ID in the file doesn't match registry entry
- Update either the file or the registry to match
- Game requires these to match for validation

### Status shows "with N errors" but creatures still load
- Some creature files had issues but others loaded
- Check the detailed error messages in console
- Fix the problematic files individually

## Code Quality Check

After making changes, run:

```bash
cargo fmt --all
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
```

All should pass with no errors or warnings.
