// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Undo/Redo System - Phase 15.1
//!
//! Command pattern implementation for reversible operations in the Campaign Builder.
//! Supports undo/redo for:
//! - Campaign metadata changes
//! - Data editor operations (add/edit/delete items, spells, monsters)
//! - Map editor tile placement
//! - Quest and dialogue modifications

use antares::domain::combat::database::MonsterDefinition;
use antares::domain::dialogue::DialogueTree;
use antares::domain::items::types::Item;
use antares::domain::magic::types::Spell;
use antares::domain::quest::Quest;
use antares::domain::world::Map;
use serde::{Deserialize, Serialize};

/// Maximum number of actions in the undo/redo history
pub const MAX_HISTORY_SIZE: usize = 50;

/// Command trait for reversible operations
pub trait Command: std::fmt::Debug {
    /// Execute the command
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String>;

    /// Undo the command
    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String>;

    /// Get a human-readable description of this command
    fn description(&self) -> String;
}

/// Global state that commands operate on
#[derive(Debug, Clone, Default)]
pub struct UndoRedoState {
    pub metadata_changed: bool,
    pub items: Vec<Item>,
    pub spells: Vec<Spell>,
    pub monsters: Vec<MonsterDefinition>,
    pub maps: Vec<Map>,
    pub quests: Vec<Quest>,
    pub dialogues: Vec<DialogueTree>,
}

/// Undo/Redo manager
#[derive(Debug)]
pub struct UndoRedoManager {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    state: UndoRedoState,
}

impl Default for UndoRedoManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoRedoManager {
    /// Create a new undo/redo manager
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            state: UndoRedoState::default(),
        }
    }

    /// Execute a command and add it to the undo stack
    pub fn execute(&mut self, command: Box<dyn Command>) -> Result<(), String> {
        command.execute(&mut self.state)?;

        // Clear redo stack when new command is executed
        self.redo_stack.clear();

        // Add to undo stack
        self.undo_stack.push(command);

        // Limit stack size
        if self.undo_stack.len() > MAX_HISTORY_SIZE {
            self.undo_stack.remove(0);
        }

        Ok(())
    }

    /// Undo the last command
    pub fn undo(&mut self) -> Result<String, String> {
        if let Some(command) = self.undo_stack.pop() {
            let description = command.description();
            command.undo(&mut self.state)?;
            self.redo_stack.push(command);
            Ok(description)
        } else {
            Err("Nothing to undo".to_string())
        }
    }

    /// Redo the last undone command
    pub fn redo(&mut self) -> Result<String, String> {
        if let Some(command) = self.redo_stack.pop() {
            let description = command.description();
            command.execute(&mut self.state)?;
            self.undo_stack.push(command);
            Ok(description)
        } else {
            Err("Nothing to redo".to_string())
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the number of actions that can be undone
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of actions that can be redone
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Get description of the next undo action
    pub fn next_undo_description(&self) -> Option<String> {
        self.undo_stack.last().map(|cmd| cmd.description())
    }

    /// Get description of the next redo action
    pub fn next_redo_description(&self) -> Option<String> {
        self.redo_stack.last().map(|cmd| cmd.description())
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Get read-only access to state
    pub fn state(&self) -> &UndoRedoState {
        &self.state
    }

    /// Get mutable access to state (use with caution)
    pub fn state_mut(&mut self) -> &mut UndoRedoState {
        &mut self.state
    }
}

// ============================================================================
// Item Commands
// ============================================================================

/// Command to add an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddItemCommand {
    item: Item,
}

impl AddItemCommand {
    pub fn new(item: Item) -> Box<Self> {
        Box::new(Self { item })
    }
}

impl Command for AddItemCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        state.items.push(self.item.clone());
        Ok(())
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if let Some(pos) = state.items.iter().position(|i| i.id == self.item.id) {
            state.items.remove(pos);
            Ok(())
        } else {
            Err(format!("Item {} not found for undo", self.item.id))
        }
    }

    fn description(&self) -> String {
        format!("Add item: {}", self.item.name)
    }
}

/// Command to delete an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteItemCommand {
    item: Item,
    index: usize,
}

impl DeleteItemCommand {
    pub fn new(item: Item, index: usize) -> Box<Self> {
        Box::new(Self { item, index })
    }
}

impl Command for DeleteItemCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index < state.items.len() {
            state.items.remove(self.index);
            Ok(())
        } else {
            Err(format!("Invalid item index: {}", self.index))
        }
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index <= state.items.len() {
            state.items.insert(self.index, self.item.clone());
            Ok(())
        } else {
            Err(format!("Invalid item index for undo: {}", self.index))
        }
    }

    fn description(&self) -> String {
        format!("Delete item: {}", self.item.name)
    }
}

/// Command to edit an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditItemCommand {
    index: usize,
    old_item: Item,
    new_item: Item,
}

impl EditItemCommand {
    pub fn new(index: usize, old_item: Item, new_item: Item) -> Box<Self> {
        Box::new(Self {
            index,
            old_item,
            new_item,
        })
    }
}

impl Command for EditItemCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index < state.items.len() {
            state.items[self.index] = self.new_item.clone();
            Ok(())
        } else {
            Err(format!("Invalid item index: {}", self.index))
        }
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index < state.items.len() {
            state.items[self.index] = self.old_item.clone();
            Ok(())
        } else {
            Err(format!("Invalid item index for undo: {}", self.index))
        }
    }

    fn description(&self) -> String {
        format!("Edit item: {}", self.new_item.name)
    }
}

// ============================================================================
// Spell Commands
// ============================================================================

/// Command to add a spell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddSpellCommand {
    spell: Spell,
}

impl AddSpellCommand {
    pub fn new(spell: Spell) -> Box<Self> {
        Box::new(Self { spell })
    }
}

impl Command for AddSpellCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        state.spells.push(self.spell.clone());
        Ok(())
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if let Some(pos) = state.spells.iter().position(|s| s.id == self.spell.id) {
            state.spells.remove(pos);
            Ok(())
        } else {
            Err(format!("Spell {} not found for undo", self.spell.id))
        }
    }

    fn description(&self) -> String {
        format!("Add spell: {}", self.spell.name)
    }
}

/// Command to delete a spell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteSpellCommand {
    spell: Spell,
    index: usize,
}

impl DeleteSpellCommand {
    pub fn new(spell: Spell, index: usize) -> Box<Self> {
        Box::new(Self { spell, index })
    }
}

impl Command for DeleteSpellCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index < state.spells.len() {
            state.spells.remove(self.index);
            Ok(())
        } else {
            Err(format!("Invalid spell index: {}", self.index))
        }
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index <= state.spells.len() {
            state.spells.insert(self.index, self.spell.clone());
            Ok(())
        } else {
            Err(format!("Invalid spell index for undo: {}", self.index))
        }
    }

    fn description(&self) -> String {
        format!("Delete spell: {}", self.spell.name)
    }
}

/// Command to edit a spell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditSpellCommand {
    index: usize,
    old_spell: Spell,
    new_spell: Spell,
}

impl EditSpellCommand {
    pub fn new(index: usize, old_spell: Spell, new_spell: Spell) -> Box<Self> {
        Box::new(Self {
            index,
            old_spell,
            new_spell,
        })
    }
}

impl Command for EditSpellCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index < state.spells.len() {
            state.spells[self.index] = self.new_spell.clone();
            Ok(())
        } else {
            Err(format!("Invalid spell index: {}", self.index))
        }
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index < state.spells.len() {
            state.spells[self.index] = self.old_spell.clone();
            Ok(())
        } else {
            Err(format!("Invalid spell index for undo: {}", self.index))
        }
    }

    fn description(&self) -> String {
        format!("Edit spell: {}", self.new_spell.name)
    }
}

// ============================================================================
// Monster Commands
// ============================================================================

/// Command to add a monster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMonsterCommand {
    monster: MonsterDefinition,
}

impl AddMonsterCommand {
    pub fn new(monster: MonsterDefinition) -> Box<Self> {
        Box::new(Self { monster })
    }
}

impl Command for AddMonsterCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        state.monsters.push(self.monster.clone());
        Ok(())
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if let Some(pos) = state.monsters.iter().position(|m| m.id == self.monster.id) {
            state.monsters.remove(pos);
            Ok(())
        } else {
            Err(format!("Monster {} not found for undo", self.monster.id))
        }
    }

    fn description(&self) -> String {
        format!("Add monster: {}", self.monster.name)
    }
}

/// Command to delete a monster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMonsterCommand {
    monster: MonsterDefinition,
    index: usize,
}

impl DeleteMonsterCommand {
    pub fn new(monster: MonsterDefinition, index: usize) -> Box<Self> {
        Box::new(Self { monster, index })
    }
}

impl Command for DeleteMonsterCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index < state.monsters.len() {
            state.monsters.remove(self.index);
            Ok(())
        } else {
            Err(format!("Invalid monster index: {}", self.index))
        }
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index <= state.monsters.len() {
            state.monsters.insert(self.index, self.monster.clone());
            Ok(())
        } else {
            Err(format!("Invalid monster index for undo: {}", self.index))
        }
    }

    fn description(&self) -> String {
        format!("Delete monster: {}", self.monster.name)
    }
}

/// Command to edit a monster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditMonsterCommand {
    index: usize,
    old_monster: MonsterDefinition,
    new_monster: MonsterDefinition,
}

impl EditMonsterCommand {
    pub fn new(
        index: usize,
        old_monster: MonsterDefinition,
        new_monster: MonsterDefinition,
    ) -> Box<Self> {
        Box::new(Self {
            index,
            old_monster,
            new_monster,
        })
    }
}

impl Command for EditMonsterCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index < state.monsters.len() {
            state.monsters[self.index] = self.new_monster.clone();
            Ok(())
        } else {
            Err(format!("Invalid monster index: {}", self.index))
        }
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index < state.monsters.len() {
            state.monsters[self.index] = self.old_monster.clone();
            Ok(())
        } else {
            Err(format!("Invalid monster index for undo: {}", self.index))
        }
    }

    fn description(&self) -> String {
        format!("Edit monster: {}", self.new_monster.name)
    }
}

// ============================================================================
// Quest Commands
// ============================================================================

/// Command to add a quest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddQuestCommand {
    quest: Quest,
}

impl AddQuestCommand {
    pub fn new(quest: Quest) -> Box<Self> {
        Box::new(Self { quest })
    }
}

impl Command for AddQuestCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        state.quests.push(self.quest.clone());
        Ok(())
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if let Some(pos) = state.quests.iter().position(|q| q.id == self.quest.id) {
            state.quests.remove(pos);
            Ok(())
        } else {
            Err(format!("Quest {} not found for undo", self.quest.id))
        }
    }

    fn description(&self) -> String {
        format!("Add quest: {}", self.quest.name)
    }
}

/// Command to delete a quest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteQuestCommand {
    quest: Quest,
    index: usize,
}

impl DeleteQuestCommand {
    pub fn new(quest: Quest, index: usize) -> Box<Self> {
        Box::new(Self { quest, index })
    }
}

impl Command for DeleteQuestCommand {
    fn execute(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index < state.quests.len() {
            state.quests.remove(self.index);
            Ok(())
        } else {
            Err(format!("Invalid quest index: {}", self.index))
        }
    }

    fn undo(&self, state: &mut UndoRedoState) -> Result<(), String> {
        if self.index <= state.quests.len() {
            state.quests.insert(self.index, self.quest.clone());
            Ok(())
        } else {
            Err(format!("Invalid quest index for undo: {}", self.index))
        }
    }

    fn description(&self) -> String {
        format!("Delete quest: {}", self.quest.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use antares::domain::items::types::{Disablement, ItemType, WeaponData};
    use antares::domain::types::DiceRoll;

    fn create_test_item(id: u32, name: &str) -> Item {
        Item {
            id: id.try_into().expect("ItemId out of range"),
            name: name.to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
            }),
            base_cost: 100,
            sell_cost: 50,
            disablements: Disablement::ALL,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
        }
    }

    #[test]
    fn test_undo_redo_manager_creation() {
        let manager = UndoRedoManager::new();
        assert!(!manager.can_undo());
        assert!(!manager.can_redo());
        assert_eq!(manager.undo_count(), 0);
        assert_eq!(manager.redo_count(), 0);
    }

    #[test]
    fn test_add_item_command() {
        let mut manager = UndoRedoManager::new();
        let item = create_test_item(1, "Sword");

        let cmd = AddItemCommand::new(item.clone());
        manager.execute(cmd).unwrap();

        assert_eq!(manager.state().items.len(), 1);
        assert_eq!(manager.state().items[0].name, "Sword");
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_undo_add_item() {
        let mut manager = UndoRedoManager::new();
        let item = create_test_item(1, "Sword");

        let cmd = AddItemCommand::new(item.clone());
        manager.execute(cmd).unwrap();
        assert_eq!(manager.state().items.len(), 1);

        manager.undo().unwrap();
        assert_eq!(manager.state().items.len(), 0);
        assert!(!manager.can_undo());
        assert!(manager.can_redo());
    }

    #[test]
    fn test_redo_add_item() {
        let mut manager = UndoRedoManager::new();
        let item = create_test_item(1, "Sword");

        let cmd = AddItemCommand::new(item.clone());
        manager.execute(cmd).unwrap();
        manager.undo().unwrap();

        manager.redo().unwrap();
        assert_eq!(manager.state().items.len(), 1);
        assert_eq!(manager.state().items[0].name, "Sword");
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_delete_item_command() {
        let mut manager = UndoRedoManager::new();
        let item = create_test_item(1, "Sword");
        manager.state_mut().items.push(item.clone());

        let cmd = DeleteItemCommand::new(item, 0);
        manager.execute(cmd).unwrap();

        assert_eq!(manager.state().items.len(), 0);
    }

    #[test]
    fn test_undo_delete_item() {
        let mut manager = UndoRedoManager::new();
        let item = create_test_item(1, "Sword");
        manager.state_mut().items.push(item.clone());

        let cmd = DeleteItemCommand::new(item.clone(), 0);
        manager.execute(cmd).unwrap();
        assert_eq!(manager.state().items.len(), 0);

        manager.undo().unwrap();
        assert_eq!(manager.state().items.len(), 1);
        assert_eq!(manager.state().items[0].name, "Sword");
    }

    #[test]
    fn test_edit_item_command() {
        let mut manager = UndoRedoManager::new();
        let old_item = create_test_item(1, "Sword");
        let new_item = create_test_item(1, "Magic Sword");
        manager.state_mut().items.push(old_item.clone());

        let cmd = EditItemCommand::new(0, old_item.clone(), new_item.clone());
        manager.execute(cmd).unwrap();

        assert_eq!(manager.state().items[0].name, "Magic Sword");
    }

    #[test]
    fn test_undo_edit_item() {
        let mut manager = UndoRedoManager::new();
        let old_item = create_test_item(1, "Sword");
        let new_item = create_test_item(1, "Magic Sword");
        manager.state_mut().items.push(old_item.clone());

        let cmd = EditItemCommand::new(0, old_item.clone(), new_item.clone());
        manager.execute(cmd).unwrap();
        assert_eq!(manager.state().items[0].name, "Magic Sword");

        manager.undo().unwrap();
        assert_eq!(manager.state().items[0].name, "Sword");
    }

    #[test]
    fn test_multiple_undo_redo() {
        let mut manager = UndoRedoManager::new();

        // Add three items
        for i in 1..=3 {
            let item = create_test_item(i, &format!("Item {}", i));
            let cmd = AddItemCommand::new(item);
            manager.execute(cmd).unwrap();
        }

        assert_eq!(manager.state().items.len(), 3);
        assert_eq!(manager.undo_count(), 3);

        // Undo all
        manager.undo().unwrap();
        manager.undo().unwrap();
        manager.undo().unwrap();

        assert_eq!(manager.state().items.len(), 0);
        assert_eq!(manager.redo_count(), 3);

        // Redo all
        manager.redo().unwrap();
        manager.redo().unwrap();
        manager.redo().unwrap();

        assert_eq!(manager.state().items.len(), 3);
    }

    #[test]
    fn test_new_command_clears_redo_stack() {
        let mut manager = UndoRedoManager::new();

        let item1 = create_test_item(1, "Item 1");
        let cmd1 = AddItemCommand::new(item1);
        manager.execute(cmd1).unwrap();

        manager.undo().unwrap();
        assert!(manager.can_redo());

        // New command should clear redo stack
        let item2 = create_test_item(2, "Item 2");
        let cmd2 = AddItemCommand::new(item2);
        manager.execute(cmd2).unwrap();

        assert!(!manager.can_redo());
    }

    #[test]
    fn test_max_history_size() {
        let mut manager = UndoRedoManager::new();

        // Add more than MAX_HISTORY_SIZE commands
        for i in 0..(MAX_HISTORY_SIZE + 10) {
            let item = create_test_item(i as u32, &format!("Item {}", i));
            let cmd = AddItemCommand::new(item);
            manager.execute(cmd).unwrap();
        }

        // Should be limited to MAX_HISTORY_SIZE
        assert_eq!(manager.undo_count(), MAX_HISTORY_SIZE);
    }

    #[test]
    fn test_command_descriptions() {
        let item = create_test_item(1, "Test Sword");

        let add_cmd = AddItemCommand::new(item.clone());
        assert_eq!(add_cmd.description(), "Add item: Test Sword");

        let del_cmd = DeleteItemCommand::new(item.clone(), 0);
        assert_eq!(del_cmd.description(), "Delete item: Test Sword");

        let edit_cmd = EditItemCommand::new(0, item.clone(), item.clone());
        assert_eq!(edit_cmd.description(), "Edit item: Test Sword");
    }

    #[test]
    fn test_next_undo_redo_descriptions() {
        let mut manager = UndoRedoManager::new();
        assert_eq!(manager.next_undo_description(), None);
        assert_eq!(manager.next_redo_description(), None);

        let item = create_test_item(1, "Sword");
        let cmd = AddItemCommand::new(item);
        manager.execute(cmd).unwrap();

        assert_eq!(
            manager.next_undo_description(),
            Some("Add item: Sword".to_string())
        );

        manager.undo().unwrap();
        assert_eq!(
            manager.next_redo_description(),
            Some("Add item: Sword".to_string())
        );
    }

    #[test]
    fn test_clear_history() {
        let mut manager = UndoRedoManager::new();

        let item = create_test_item(1, "Sword");
        let cmd = AddItemCommand::new(item);
        manager.execute(cmd).unwrap();

        assert!(manager.can_undo());
        manager.clear();
        assert!(!manager.can_undo());
        assert!(!manager.can_redo());
    }
}
