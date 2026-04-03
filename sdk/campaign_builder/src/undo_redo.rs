// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Undo/Redo System
//!
//! Command pattern implementation for reversible operations in the Campaign Builder.
//! Supports undo/redo for:
//! - Campaign metadata changes
//! - Data editor operations (add/edit/delete items, spells, monsters)
//! - Map editor tile placement
//! - Quest and dialogue modifications

use crate::editor_state::CampaignData;
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::items::types::Item;
use antares::domain::magic::types::Spell;
use antares::domain::quest::Quest;
use serde::{Deserialize, Serialize};

/// Maximum number of actions in the undo/redo history
pub const MAX_HISTORY_SIZE: usize = 50;

/// Command trait for reversible operations
pub(crate) trait Command: std::fmt::Debug {
    /// Execute the command
    fn execute(&self, data: &mut CampaignData) -> Result<(), String>;

    /// Undo the command
    fn undo(&self, data: &mut CampaignData) -> Result<(), String>;

    /// Get a human-readable description of this command
    fn description(&self) -> String;
}

/// A generic push-down undo/redo stack.
///
/// `UndoRedoStack<C>` provides the core mechanics for maintaining separate undo
/// and redo histories for any command type `C`. It enforces a maximum history
/// size on the undo stack, automatically evicting the oldest entry when the
/// limit is exceeded.
///
/// # Type Parameters
///
/// * `C` – The command type stored in the stack. Typically a trait object like
///   `Box<dyn Command>` or a concrete action enum.
///
/// # Examples
///
/// ```
/// use campaign_builder::undo_redo::UndoRedoStack;
///
/// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
/// stack.push_new("first".to_string());
/// stack.push_new("second".to_string());
///
/// assert_eq!(stack.undo_count(), 2);
/// assert!(!stack.can_redo());
///
/// let cmd = stack.pop_undo().unwrap();
/// stack.push_to_redo(cmd);
///
/// assert_eq!(stack.undo_count(), 1);
/// assert!(stack.can_redo());
/// ```
#[derive(Debug)]
pub struct UndoRedoStack<C> {
    /// Commands available to undo (oldest at index 0, newest at the end)
    undo_stack: Vec<C>,
    /// Commands available to redo (oldest at index 0, newest at the end)
    redo_stack: Vec<C>,
    /// Maximum number of entries the undo stack may hold
    max_history: usize,
}

impl<C> UndoRedoStack<C> {
    /// Creates a new stack with the given maximum undo-history size.
    ///
    /// When `max_history` is `usize::MAX` the stack behaves as unbounded.
    ///
    /// # Arguments
    ///
    /// * `max_history` – The maximum number of commands stored in the undo stack.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let stack: UndoRedoStack<String> = UndoRedoStack::new(50);
    /// assert_eq!(stack.undo_count(), 0);
    /// ```
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Pushes a new command onto the undo stack, clearing the redo stack.
    ///
    /// Any forward (redo) history is discarded because a new command branches
    /// the timeline. If the undo stack exceeds `max_history` the oldest entry
    /// is evicted.
    ///
    /// # Arguments
    ///
    /// * `cmd` – The command to push.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(2);
    /// stack.push_new("a".to_string());
    /// stack.push_new("b".to_string());
    /// stack.push_new("c".to_string()); // "a" is evicted
    ///
    /// assert_eq!(stack.undo_count(), 2);
    /// assert_eq!(stack.last_undo(), Some(&"c".to_string()));
    /// ```
    pub fn push_new(&mut self, cmd: C) {
        self.redo_stack.clear();
        self.undo_stack.push(cmd);
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Pops the most recent command from the undo stack.
    ///
    /// Returns `None` if the undo stack is empty. The caller is responsible
    /// for executing the undo logic and then calling
    /// [`push_to_redo`](Self::push_to_redo) with the command.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// stack.push_new("cmd".to_string());
    ///
    /// let cmd = stack.pop_undo().unwrap();
    /// assert_eq!(cmd, "cmd");
    /// assert!(!stack.can_undo());
    /// ```
    pub fn pop_undo(&mut self) -> Option<C> {
        self.undo_stack.pop()
    }

    /// Pushes a command onto the redo stack.
    ///
    /// Typically called after a successful undo operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// stack.push_new("cmd".to_string());
    /// let cmd = stack.pop_undo().unwrap();
    /// stack.push_to_redo(cmd);
    ///
    /// assert!(stack.can_redo());
    /// ```
    pub fn push_to_redo(&mut self, cmd: C) {
        self.redo_stack.push(cmd);
    }

    /// Pops the most recent command from the redo stack.
    ///
    /// Returns `None` if the redo stack is empty. The caller is responsible
    /// for re-executing the command and then calling
    /// [`push_to_undo`](Self::push_to_undo) with the command.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// stack.push_new("cmd".to_string());
    /// let cmd = stack.pop_undo().unwrap();
    /// stack.push_to_redo(cmd);
    ///
    /// let cmd = stack.pop_redo().unwrap();
    /// assert_eq!(cmd, "cmd");
    /// assert!(!stack.can_redo());
    /// ```
    pub fn pop_redo(&mut self) -> Option<C> {
        self.redo_stack.pop()
    }

    /// Pushes a command onto the undo stack **without** clearing the redo stack.
    ///
    /// This is used when a redo operation re-executes a command and needs to
    /// push it back to the undo stack while preserving any remaining redo
    /// entries. Enforces `max_history` by evicting the oldest entry when needed.
    ///
    /// # Arguments
    ///
    /// * `cmd` – The command to push.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// stack.push_new("cmd1".to_string());
    /// let cmd = stack.pop_undo().unwrap();
    /// stack.push_to_redo(cmd);
    ///
    /// // Redo: re-push to undo without clearing redo
    /// stack.push_to_undo("cmd1".to_string());
    /// assert!(stack.can_undo());
    /// // Redo stack is untouched by push_to_undo
    /// assert!(stack.can_redo());
    /// ```
    pub fn push_to_undo(&mut self, cmd: C) {
        self.undo_stack.push(cmd);
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Returns `true` if there is at least one command that can be undone.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// assert!(!stack.can_undo());
    /// stack.push_new("cmd".to_string());
    /// assert!(stack.can_undo());
    /// ```
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns `true` if there is at least one command that can be redone.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// assert!(!stack.can_redo());
    /// stack.push_new("cmd".to_string());
    /// let cmd = stack.pop_undo().unwrap();
    /// stack.push_to_redo(cmd);
    /// assert!(stack.can_redo());
    /// ```
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Returns the number of commands currently on the undo stack.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// assert_eq!(stack.undo_count(), 0);
    /// stack.push_new("cmd".to_string());
    /// assert_eq!(stack.undo_count(), 1);
    /// ```
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the number of commands currently on the redo stack.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// stack.push_new("cmd".to_string());
    /// let cmd = stack.pop_undo().unwrap();
    /// stack.push_to_redo(cmd);
    /// assert_eq!(stack.redo_count(), 1);
    /// ```
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Returns a reference to the most recently pushed undo command (top of the undo stack).
    ///
    /// Returns `None` if the undo stack is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// assert!(stack.last_undo().is_none());
    /// stack.push_new("cmd".to_string());
    /// assert_eq!(stack.last_undo(), Some(&"cmd".to_string()));
    /// ```
    pub fn last_undo(&self) -> Option<&C> {
        self.undo_stack.last()
    }

    /// Returns a reference to the most recently undone command (top of the redo stack).
    ///
    /// Returns `None` if the redo stack is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// stack.push_new("cmd".to_string());
    /// let cmd = stack.pop_undo().unwrap();
    /// stack.push_to_redo(cmd);
    /// assert_eq!(stack.last_redo(), Some(&"cmd".to_string()));
    /// ```
    pub fn last_redo(&self) -> Option<&C> {
        self.redo_stack.last()
    }

    /// Returns an iterator over the undo stack from oldest to newest.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// stack.push_new("first".to_string());
    /// stack.push_new("second".to_string());
    ///
    /// let items: Vec<&String> = stack.undo_iter().collect();
    /// assert_eq!(items, vec!["first", "second"]);
    /// ```
    pub fn undo_iter(&self) -> impl DoubleEndedIterator<Item = &C> {
        self.undo_stack.iter()
    }

    /// Returns an iterator over the redo stack from oldest to newest.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// stack.push_new("first".to_string());
    /// let cmd = stack.pop_undo().unwrap();
    /// stack.push_to_redo(cmd);
    ///
    /// let items: Vec<&String> = stack.redo_iter().collect();
    /// assert_eq!(items, vec!["first"]);
    /// ```
    pub fn redo_iter(&self) -> impl DoubleEndedIterator<Item = &C> {
        self.redo_stack.iter()
    }

    /// Clears both the undo and redo stacks.
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::undo_redo::UndoRedoStack;
    ///
    /// let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
    /// stack.push_new("cmd".to_string());
    /// stack.clear();
    ///
    /// assert!(!stack.can_undo());
    /// assert!(!stack.can_redo());
    /// ```
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

/// Undo/Redo manager
#[derive(Debug)]
pub struct UndoRedoManager {
    stack: UndoRedoStack<Box<dyn Command>>,
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
            stack: UndoRedoStack::new(MAX_HISTORY_SIZE),
        }
    }

    /// Execute a command, applying it to `data` and pushing it to the undo stack.
    ///
    /// The redo stack is cleared because a new command branches the history.
    ///
    /// # Note
    ///
    /// This method is part of the planned undo/redo API and will be wired to
    /// editor actions in a future milestone.
    #[allow(dead_code)]
    pub(crate) fn execute(
        &mut self,
        command: Box<dyn Command>,
        data: &mut CampaignData,
    ) -> Result<(), String> {
        command.execute(data)?;
        self.stack.push_new(command);
        Ok(())
    }

    /// Undo the most recent command, restoring `data` to its previous state.
    ///
    /// The undone command is pushed to the redo stack so it can be re-applied.
    pub(crate) fn undo(&mut self, data: &mut CampaignData) -> Result<String, String> {
        if let Some(command) = self.stack.pop_undo() {
            let description = command.description();
            command.undo(data)?;
            self.stack.push_to_redo(command);
            Ok(description)
        } else {
            Err("Nothing to undo".to_string())
        }
    }

    /// Redo the most recently undone command, re-applying it to `data`.
    ///
    /// The redone command is pushed back to the undo stack.
    pub(crate) fn redo(&mut self, data: &mut CampaignData) -> Result<String, String> {
        if let Some(command) = self.stack.pop_redo() {
            let description = command.description();
            command.execute(data)?;
            self.stack.push_to_undo(command);
            Ok(description)
        } else {
            Err("Nothing to redo".to_string())
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        self.stack.can_undo()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        self.stack.can_redo()
    }

    /// Get the number of actions that can be undone
    pub fn undo_count(&self) -> usize {
        self.stack.undo_count()
    }

    /// Get the number of actions that can be redone
    pub fn redo_count(&self) -> usize {
        self.stack.redo_count()
    }

    /// Get description of the next undo action
    pub fn next_undo_description(&self) -> Option<String> {
        self.stack.last_undo().map(|cmd| cmd.description())
    }

    /// Get description of the next redo action
    pub fn next_redo_description(&self) -> Option<String> {
        self.stack.last_redo().map(|cmd| cmd.description())
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.stack.clear();
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        data.items.push(self.item.clone());
        Ok(())
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if let Some(pos) = data.items.iter().position(|i| i.id == self.item.id) {
            data.items.remove(pos);
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index < data.items.len() {
            data.items.remove(self.index);
            Ok(())
        } else {
            Err(format!("Invalid item index: {}", self.index))
        }
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index <= data.items.len() {
            data.items.insert(self.index, self.item.clone());
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index < data.items.len() {
            data.items[self.index] = self.new_item.clone();
            Ok(())
        } else {
            Err(format!("Invalid item index: {}", self.index))
        }
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index < data.items.len() {
            data.items[self.index] = self.old_item.clone();
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        data.spells.push(self.spell.clone());
        Ok(())
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if let Some(pos) = data.spells.iter().position(|s| s.id == self.spell.id) {
            data.spells.remove(pos);
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index < data.spells.len() {
            data.spells.remove(self.index);
            Ok(())
        } else {
            Err(format!("Invalid spell index: {}", self.index))
        }
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index <= data.spells.len() {
            data.spells.insert(self.index, self.spell.clone());
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index < data.spells.len() {
            data.spells[self.index] = self.new_spell.clone();
            Ok(())
        } else {
            Err(format!("Invalid spell index: {}", self.index))
        }
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index < data.spells.len() {
            data.spells[self.index] = self.old_spell.clone();
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        data.monsters.push(self.monster.clone());
        Ok(())
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if let Some(pos) = data.monsters.iter().position(|m| m.id == self.monster.id) {
            data.monsters.remove(pos);
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index < data.monsters.len() {
            data.monsters.remove(self.index);
            Ok(())
        } else {
            Err(format!("Invalid monster index: {}", self.index))
        }
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index <= data.monsters.len() {
            data.monsters.insert(self.index, self.monster.clone());
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index < data.monsters.len() {
            data.monsters[self.index] = self.new_monster.clone();
            Ok(())
        } else {
            Err(format!("Invalid monster index: {}", self.index))
        }
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index < data.monsters.len() {
            data.monsters[self.index] = self.old_monster.clone();
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        data.quests.push(self.quest.clone());
        Ok(())
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if let Some(pos) = data.quests.iter().position(|q| q.id == self.quest.id) {
            data.quests.remove(pos);
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
    fn execute(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index < data.quests.len() {
            data.quests.remove(self.index);
            Ok(())
        } else {
            Err(format!("Invalid quest index: {}", self.index))
        }
    }

    fn undo(&self, data: &mut CampaignData) -> Result<(), String> {
        if self.index <= data.quests.len() {
            data.quests.insert(self.index, self.quest.clone());
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
    use antares::domain::items::types::{ItemType, WeaponClassification, WeaponData};
    use antares::domain::types::DiceRoll;

    fn create_test_item(id: u32, name: &str) -> Item {
        Item {
            id: id.try_into().expect("ItemId out of range"),
            name: name.to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(1, 6, 0),
                bonus: 0,
                hands_required: 1,
                classification: WeaponClassification::Simple,
            }),
            base_cost: 100,
            sell_cost: 50,
            alignment_restriction: None,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
            icon_path: None,
            tags: vec![],
            mesh_descriptor_override: None,
            mesh_id: None,
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
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();
        let item = create_test_item(1, "Sword");

        let cmd = AddItemCommand::new(item.clone());
        manager.execute(cmd, &mut data).unwrap();

        assert_eq!(data.items.len(), 1);
        assert_eq!(data.items[0].name, "Sword");
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_undo_add_item() {
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();
        let item = create_test_item(1, "Sword");

        let cmd = AddItemCommand::new(item.clone());
        manager.execute(cmd, &mut data).unwrap();
        assert_eq!(data.items.len(), 1);

        manager.undo(&mut data).unwrap();
        assert_eq!(data.items.len(), 0);
        assert!(!manager.can_undo());
        assert!(manager.can_redo());
    }

    #[test]
    fn test_redo_add_item() {
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();
        let item = create_test_item(1, "Sword");

        let cmd = AddItemCommand::new(item.clone());
        manager.execute(cmd, &mut data).unwrap();
        manager.undo(&mut data).unwrap();

        manager.redo(&mut data).unwrap();
        assert_eq!(data.items.len(), 1);
        assert_eq!(data.items[0].name, "Sword");
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_delete_item_command() {
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();
        let item = create_test_item(1, "Sword");
        data.items.push(item.clone());

        let cmd = DeleteItemCommand::new(item, 0);
        manager.execute(cmd, &mut data).unwrap();

        assert_eq!(data.items.len(), 0);
    }

    #[test]
    fn test_undo_delete_item() {
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();
        let item = create_test_item(1, "Sword");
        data.items.push(item.clone());

        let cmd = DeleteItemCommand::new(item.clone(), 0);
        manager.execute(cmd, &mut data).unwrap();
        assert_eq!(data.items.len(), 0);

        manager.undo(&mut data).unwrap();
        assert_eq!(data.items.len(), 1);
        assert_eq!(data.items[0].name, "Sword");
    }

    #[test]
    fn test_edit_item_command() {
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();
        let old_item = create_test_item(1, "Sword");
        let new_item = create_test_item(1, "Magic Sword");
        data.items.push(old_item.clone());

        let cmd = EditItemCommand::new(0, old_item.clone(), new_item.clone());
        manager.execute(cmd, &mut data).unwrap();

        assert_eq!(data.items[0].name, "Magic Sword");
    }

    #[test]
    fn test_undo_edit_item() {
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();
        let old_item = create_test_item(1, "Sword");
        let new_item = create_test_item(1, "Magic Sword");
        data.items.push(old_item.clone());

        let cmd = EditItemCommand::new(0, old_item.clone(), new_item.clone());
        manager.execute(cmd, &mut data).unwrap();
        assert_eq!(data.items[0].name, "Magic Sword");

        manager.undo(&mut data).unwrap();
        assert_eq!(data.items[0].name, "Sword");
    }

    #[test]
    fn test_multiple_undo_redo() {
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();

        // Add three items
        for i in 1..=3 {
            let item = create_test_item(i, &format!("Item {}", i));
            let cmd = AddItemCommand::new(item);
            manager.execute(cmd, &mut data).unwrap();
        }

        assert_eq!(data.items.len(), 3);
        assert_eq!(manager.undo_count(), 3);

        // Undo all
        manager.undo(&mut data).unwrap();
        manager.undo(&mut data).unwrap();
        manager.undo(&mut data).unwrap();

        assert_eq!(data.items.len(), 0);
        assert_eq!(manager.redo_count(), 3);

        // Redo all
        manager.redo(&mut data).unwrap();
        manager.redo(&mut data).unwrap();
        manager.redo(&mut data).unwrap();

        assert_eq!(data.items.len(), 3);
    }

    #[test]
    fn test_new_command_clears_redo_stack() {
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();

        let item1 = create_test_item(1, "Item 1");
        let cmd1 = AddItemCommand::new(item1);
        manager.execute(cmd1, &mut data).unwrap();

        manager.undo(&mut data).unwrap();
        assert!(manager.can_redo());

        // New command should clear redo stack
        let item2 = create_test_item(2, "Item 2");
        let cmd2 = AddItemCommand::new(item2);
        manager.execute(cmd2, &mut data).unwrap();

        assert!(!manager.can_redo());
    }

    #[test]
    fn test_max_history_size() {
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();

        // Add more than MAX_HISTORY_SIZE commands
        for i in 0..(MAX_HISTORY_SIZE + 10) {
            let item = create_test_item(i as u32, &format!("Item {}", i));
            let cmd = AddItemCommand::new(item);
            manager.execute(cmd, &mut data).unwrap();
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
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();
        assert_eq!(manager.next_undo_description(), None);
        assert_eq!(manager.next_redo_description(), None);

        let item = create_test_item(1, "Sword");
        let cmd = AddItemCommand::new(item);
        manager.execute(cmd, &mut data).unwrap();

        assert_eq!(
            manager.next_undo_description(),
            Some("Add item: Sword".to_string())
        );

        manager.undo(&mut data).unwrap();
        assert_eq!(
            manager.next_redo_description(),
            Some("Add item: Sword".to_string())
        );
    }

    #[test]
    fn test_clear_history() {
        let mut data = CampaignData::default();
        let mut manager = UndoRedoManager::new();

        let item = create_test_item(1, "Sword");
        let cmd = AddItemCommand::new(item);
        manager.execute(cmd, &mut data).unwrap();

        assert!(manager.can_undo());
        manager.clear();
        assert!(!manager.can_undo());
        assert!(!manager.can_redo());
    }

    // ── UndoRedoStack<C> tests ────────────────────────────────────────────────

    #[test]
    fn test_undo_redo_stack_new() {
        let stack: UndoRedoStack<String> = UndoRedoStack::new(10);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 0);
        assert!(stack.last_undo().is_none());
        assert!(stack.last_redo().is_none());
    }

    #[test]
    fn test_undo_redo_stack_push_new() {
        let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
        stack.push_new("cmd1".to_string());
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(stack.undo_count(), 1);
        assert_eq!(stack.last_undo(), Some(&"cmd1".to_string()));
    }

    #[test]
    fn test_undo_redo_stack_push_new_clears_redo() {
        let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
        stack.push_new("cmd1".to_string());
        let cmd = stack.pop_undo().unwrap();
        stack.push_to_redo(cmd);
        assert!(stack.can_redo());
        stack.push_new("cmd2".to_string());
        assert!(!stack.can_redo(), "push_new must clear redo stack");
    }

    #[test]
    fn test_undo_redo_stack_max_history() {
        let mut stack: UndoRedoStack<String> = UndoRedoStack::new(3);
        stack.push_new("a".to_string());
        stack.push_new("b".to_string());
        stack.push_new("c".to_string());
        stack.push_new("d".to_string()); // "a" is evicted
        assert_eq!(stack.undo_count(), 3);
        assert_eq!(stack.last_undo(), Some(&"d".to_string()));
        let items: Vec<&String> = stack.undo_iter().collect();
        assert_eq!(items, vec!["b", "c", "d"]);
    }

    #[test]
    fn test_undo_redo_stack_pop_undo_and_redo() {
        let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
        stack.push_new("cmd".to_string());
        let cmd = stack.pop_undo().unwrap();
        assert_eq!(cmd, "cmd");
        assert!(!stack.can_undo());
        stack.push_to_redo(cmd);
        assert!(stack.can_redo());
        assert_eq!(stack.last_redo(), Some(&"cmd".to_string()));
        let cmd = stack.pop_redo().unwrap();
        assert_eq!(cmd, "cmd");
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_undo_redo_stack_push_to_undo_preserves_redo() {
        let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
        stack.push_new("cmd1".to_string());
        let cmd1 = stack.pop_undo().unwrap();
        stack.push_to_redo(cmd1);
        // push_to_undo should NOT clear redo
        stack.push_to_undo("cmd1".to_string());
        assert!(stack.can_undo());
        assert!(
            stack.can_redo(),
            "redo stack must be preserved by push_to_undo"
        );
    }

    #[test]
    fn test_undo_redo_stack_push_to_undo_enforces_max_history() {
        let mut stack: UndoRedoStack<String> = UndoRedoStack::new(2);
        stack.push_to_undo("a".to_string());
        stack.push_to_undo("b".to_string());
        stack.push_to_undo("c".to_string()); // "a" evicted
        assert_eq!(stack.undo_count(), 2);
        assert_eq!(stack.last_undo(), Some(&"c".to_string()));
    }

    #[test]
    fn test_undo_redo_stack_iterators() {
        let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
        stack.push_new("first".to_string());
        stack.push_new("second".to_string());
        stack.push_new("third".to_string());
        let undo_items: Vec<&String> = stack.undo_iter().collect();
        assert_eq!(undo_items, vec!["first", "second", "third"]);
        let cmd3 = stack.pop_undo().unwrap();
        stack.push_to_redo(cmd3);
        let cmd2 = stack.pop_undo().unwrap();
        stack.push_to_redo(cmd2);
        // redo_stack oldest→newest: third was pushed first, second second
        let redo_items: Vec<&String> = stack.redo_iter().collect();
        assert_eq!(redo_items, vec!["third", "second"]);
    }

    #[test]
    fn test_undo_redo_stack_clear() {
        let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
        stack.push_new("cmd".to_string());
        let cmd = stack.pop_undo().unwrap();
        stack.push_to_redo(cmd);
        stack.clear();
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(stack.undo_count(), 0);
        assert_eq!(stack.redo_count(), 0);
    }

    #[test]
    fn test_undo_redo_stack_pop_empty_returns_none() {
        let mut stack: UndoRedoStack<String> = UndoRedoStack::new(10);
        assert!(stack.pop_undo().is_none());
        assert!(stack.pop_redo().is_none());
    }
}
