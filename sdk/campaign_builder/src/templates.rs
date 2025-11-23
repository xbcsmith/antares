// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Template System - Phase 15.2
//!
//! Pre-built templates for common game content:
//! - Items (weapons, armor, consumables)
//! - Monsters (classic RPG creatures)
//! - Quests (fetch, kill, escort)
//! - Dialogues (merchant, quest giver, guard)
//! - Maps (town, dungeon, wilderness)

use antares::domain::character::{AttributePair, Stats};
use antares::domain::combat::database::MonsterDefinition;
use antares::domain::combat::monster::{LootTable, MonsterResistances};
use antares::domain::combat::types::Attack;
use antares::domain::dialogue::{DialogueNode, DialogueTree};
use antares::domain::items::types::{
    ArmorData, ConsumableData, Disablement, Item, ItemType, WeaponData,
};

use antares::domain::quest::{Quest, QuestId, QuestObjective, QuestReward, QuestStage};
use antares::domain::types::{DiceRoll, MonsterId};
use antares::domain::world::Map;
use serde::{Deserialize, Serialize};

/// Template category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TemplateCategory {
    Item,
    Monster,
    Quest,
    Dialogue,
    Map,
}

impl TemplateCategory {
    pub fn all() -> &'static [TemplateCategory] {
        &[
            TemplateCategory::Item,
            TemplateCategory::Monster,
            TemplateCategory::Quest,
            TemplateCategory::Dialogue,
            TemplateCategory::Map,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            TemplateCategory::Item => "Items",
            TemplateCategory::Monster => "Monsters",
            TemplateCategory::Quest => "Quests",
            TemplateCategory::Dialogue => "Dialogues",
            TemplateCategory::Map => "Maps",
        }
    }
}

/// Template metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: TemplateCategory,
    pub tags: Vec<String>,
}

/// Template manager
#[derive(Debug, Clone, Default)]
pub struct TemplateManager {
    custom_items: Vec<Item>,
    custom_monsters: Vec<MonsterDefinition>,
    custom_quests: Vec<Quest>,
    custom_dialogues: Vec<DialogueTree>,
    custom_maps: Vec<Map>,
}

impl TemplateManager {
    /// Create a new template manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all available item templates
    pub fn item_templates(&self) -> Vec<TemplateInfo> {
        let mut templates = vec![
            TemplateInfo {
                id: "basic_sword".to_string(),
                name: "Basic Sword".to_string(),
                description: "Simple one-handed sword".to_string(),
                category: TemplateCategory::Item,
                tags: vec!["weapon".to_string(), "melee".to_string()],
            },
            TemplateInfo {
                id: "basic_dagger".to_string(),
                name: "Basic Dagger".to_string(),
                description: "Light one-handed dagger".to_string(),
                category: TemplateCategory::Item,
                tags: vec!["weapon".to_string(), "melee".to_string()],
            },
            TemplateInfo {
                id: "basic_bow".to_string(),
                name: "Basic Bow".to_string(),
                description: "Simple ranged bow".to_string(),
                category: TemplateCategory::Item,
                tags: vec!["weapon".to_string(), "ranged".to_string()],
            },
            TemplateInfo {
                id: "basic_staff".to_string(),
                name: "Basic Staff".to_string(),
                description: "Wooden staff for casters".to_string(),
                category: TemplateCategory::Item,
                tags: vec!["weapon".to_string(), "magic".to_string()],
            },
            TemplateInfo {
                id: "leather_armor".to_string(),
                name: "Leather Armor".to_string(),
                description: "Light leather armor".to_string(),
                category: TemplateCategory::Item,
                tags: vec!["armor".to_string(), "light".to_string()],
            },
            TemplateInfo {
                id: "chain_mail".to_string(),
                name: "Chain Mail".to_string(),
                description: "Medium chain mail armor".to_string(),
                category: TemplateCategory::Item,
                tags: vec!["armor".to_string(), "medium".to_string()],
            },
            TemplateInfo {
                id: "plate_mail".to_string(),
                name: "Plate Mail".to_string(),
                description: "Heavy plate armor".to_string(),
                category: TemplateCategory::Item,
                tags: vec!["armor".to_string(), "heavy".to_string()],
            },
            TemplateInfo {
                id: "healing_potion".to_string(),
                name: "Healing Potion".to_string(),
                description: "Restores HP".to_string(),
                category: TemplateCategory::Item,
                tags: vec!["consumable".to_string(), "healing".to_string()],
            },
            TemplateInfo {
                id: "mana_potion".to_string(),
                name: "Mana Potion".to_string(),
                description: "Restores SP".to_string(),
                category: TemplateCategory::Item,
                tags: vec!["consumable".to_string(), "magic".to_string()],
            },
        ];

        // Add custom templates
        for item in &self.custom_items {
            templates.push(TemplateInfo {
                id: format!("custom_{}", item.id),
                name: format!("{} (Custom)", item.name),
                description: item.name.clone(),
                category: TemplateCategory::Item,
                tags: vec!["custom".to_string()],
            });
        }

        templates
    }

    /// Create an item from a template
    pub fn create_item(&self, template_id: &str, id: u32) -> Option<Item> {
        let id: u8 = id.try_into().ok()?;
        match template_id {
            "basic_sword" => Some(Item {
                id,
                name: "Short Sword".to_string(),
                item_type: ItemType::Weapon(WeaponData {
                    damage: DiceRoll::new(1, 6, 0),
                    bonus: 0,
                    hands_required: 1,
                }),
                base_cost: 50,
                sell_cost: 25,
                disablements: Disablement::ALL,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
            }),
            "basic_dagger" => Some(Item {
                id,
                name: "Dagger".to_string(),
                item_type: ItemType::Weapon(WeaponData {
                    damage: DiceRoll::new(1, 4, 0),
                    bonus: 0,
                    hands_required: 1,
                }),
                base_cost: 20,
                sell_cost: 10,
                disablements: Disablement::ALL,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
            }),
            "basic_bow" => Some(Item {
                id,
                name: "Short Bow".to_string(),
                item_type: ItemType::Weapon(WeaponData {
                    damage: DiceRoll::new(1, 6, 0),
                    bonus: 0,
                    hands_required: 2,
                }),
                base_cost: 60,
                sell_cost: 30,
                disablements: Disablement::ALL,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
            }),
            "basic_staff" => Some(Item {
                id,
                name: "Wooden Staff".to_string(),
                item_type: ItemType::Weapon(WeaponData {
                    damage: DiceRoll::new(1, 4, 0),
                    bonus: 0,
                    hands_required: 2,
                }),
                base_cost: 30,
                sell_cost: 15,
                disablements: Disablement::ALL,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
            }),
            "leather_armor" => Some(Item {
                id,
                name: "Leather Armor".to_string(),
                item_type: ItemType::Armor(ArmorData {
                    ac_bonus: 2,
                    weight: 8,
                }),
                base_cost: 40,
                sell_cost: 20,
                disablements: Disablement::ALL,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
            }),
            "chain_mail" => Some(Item {
                id,
                name: "Chain Mail".to_string(),
                item_type: ItemType::Armor(ArmorData {
                    ac_bonus: 4,
                    weight: 20,
                }),
                base_cost: 150,
                sell_cost: 75,
                disablements: Disablement::ALL,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
            }),
            "plate_mail" => Some(Item {
                id,
                name: "Plate Mail".to_string(),
                item_type: ItemType::Armor(ArmorData {
                    ac_bonus: 6,
                    weight: 35,
                }),
                base_cost: 400,
                sell_cost: 200,
                disablements: Disablement::ALL,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 0,
                is_cursed: false,
            }),
            "healing_potion" => Some(Item {
                id,
                name: "Healing Potion".to_string(),
                item_type: ItemType::Consumable(ConsumableData {
                    effect: antares::domain::items::types::ConsumableEffect::HealHp(10),
                    is_combat_usable: true,
                }),
                base_cost: 50,
                sell_cost: 25,
                disablements: Disablement::ALL,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 1,
                is_cursed: false,
            }),
            "mana_potion" => Some(Item {
                id,
                name: "Mana Potion".to_string(),
                item_type: ItemType::Consumable(ConsumableData {
                    effect: antares::domain::items::types::ConsumableEffect::RestoreSp(10),
                    is_combat_usable: false,
                }),
                base_cost: 60,
                sell_cost: 30,
                disablements: Disablement::ALL,
                constant_bonus: None,
                temporary_bonus: None,
                spell_effect: None,
                max_charges: 1,
                is_cursed: false,
            }),
            _ => {
                // Check custom templates
                if let Some(stripped) = template_id.strip_prefix("custom_") {
                    if let Ok(custom_id) = stripped.parse::<u8>() {
                        self.custom_items
                            .iter()
                            .find(|item| item.id == custom_id)
                            .map(|item| {
                                let mut new_item = item.clone();
                                new_item.id = id;
                                new_item
                            })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    /// Get all available monster templates
    pub fn monster_templates(&self) -> Vec<TemplateInfo> {
        vec![
            TemplateInfo {
                id: "goblin".to_string(),
                name: "Goblin".to_string(),
                description: "Weak humanoid creature".to_string(),
                category: TemplateCategory::Monster,
                tags: vec!["humanoid".to_string(), "weak".to_string()],
            },
            TemplateInfo {
                id: "skeleton".to_string(),
                name: "Skeleton".to_string(),
                description: "Undead warrior".to_string(),
                category: TemplateCategory::Monster,
                tags: vec!["undead".to_string(), "common".to_string()],
            },
            TemplateInfo {
                id: "orc".to_string(),
                name: "Orc".to_string(),
                description: "Brutal warrior".to_string(),
                category: TemplateCategory::Monster,
                tags: vec!["humanoid".to_string(), "medium".to_string()],
            },
            TemplateInfo {
                id: "dragon".to_string(),
                name: "Dragon".to_string(),
                description: "Powerful flying beast".to_string(),
                category: TemplateCategory::Monster,
                tags: vec!["dragon".to_string(), "boss".to_string()],
            },
        ]
    }

    /// Create a monster from a template
    pub fn create_monster(&self, template_id: &str, id: u32) -> Option<MonsterDefinition> {
        let id: MonsterId = id.try_into().ok()?;
        match template_id {
            "goblin" => Some(MonsterDefinition {
                id,
                name: "Goblin".to_string(),
                stats: Stats {
                    might: AttributePair::new(6),
                    intellect: AttributePair::new(3),
                    personality: AttributePair::new(4),
                    endurance: AttributePair::new(5),
                    speed: AttributePair::new(8),
                    accuracy: AttributePair::new(6),
                    luck: AttributePair::new(4),
                },
                hp: 8,
                ac: 12,
                attacks: vec![Attack::physical(DiceRoll::new(1, 4, 0))],
                flee_threshold: 2,
                special_attack_threshold: 10,
                resistances: MonsterResistances::new(),
                can_regenerate: false,
                can_advance: false,
                is_undead: false,
                magic_resistance: 0,
                loot: LootTable::new(1, 10, 0, 0, 25),
            }),
            "skeleton" => Some(MonsterDefinition {
                id: id.try_into().ok()?,
                name: "Skeleton".to_string(),
                stats: Stats {
                    might: AttributePair::new(8),
                    intellect: AttributePair::new(1),
                    personality: AttributePair::new(1),
                    endurance: AttributePair::new(6),
                    speed: AttributePair::new(6),
                    accuracy: AttributePair::new(7),
                    luck: AttributePair::new(2),
                },
                hp: 10,
                ac: 13,
                attacks: vec![Attack::physical(DiceRoll::new(1, 6, 0))],
                flee_threshold: 0,
                special_attack_threshold: 5,
                resistances: MonsterResistances::new(),
                can_regenerate: false,
                can_advance: false,
                is_undead: true,
                magic_resistance: 5,
                loot: LootTable::new(5, 20, 0, 0, 50),
            }),
            "orc" => Some(MonsterDefinition {
                id: id.try_into().ok()?,
                name: "Orc".to_string(),
                stats: Stats {
                    might: AttributePair::new(14),
                    intellect: AttributePair::new(5),
                    personality: AttributePair::new(6),
                    endurance: AttributePair::new(12),
                    speed: AttributePair::new(8),
                    accuracy: AttributePair::new(9),
                    luck: AttributePair::new(5),
                },
                hp: 18,
                ac: 14,
                attacks: vec![Attack::physical(DiceRoll::new(1, 8, 1))],
                flee_threshold: 5,
                special_attack_threshold: 15,
                resistances: MonsterResistances::new(),
                can_regenerate: false,
                can_advance: true,
                is_undead: false,
                magic_resistance: 0,
                loot: LootTable::new(10, 50, 0, 0, 100),
            }),
            "dragon" => Some(MonsterDefinition {
                id: id.try_into().ok()?,
                name: "Dragon".to_string(),
                stats: Stats {
                    might: AttributePair::new(20),
                    intellect: AttributePair::new(16),
                    personality: AttributePair::new(18),
                    endurance: AttributePair::new(18),
                    speed: AttributePair::new(14),
                    accuracy: AttributePair::new(15),
                    luck: AttributePair::new(12),
                },
                hp: 120,
                ac: 18,
                attacks: vec![
                    Attack::physical(DiceRoll::new(2, 10, 5)),
                    Attack::physical(DiceRoll::new(1, 8, 3)),
                ],
                flee_threshold: 30,
                special_attack_threshold: 50,
                resistances: MonsterResistances::new(),
                can_regenerate: true,
                can_advance: true,
                is_undead: false,
                magic_resistance: 50,
                loot: LootTable::new(1000, 5000, 0, 0, 5000),
            }),
            _ => None,
        }
    }

    /// Get all available quest templates
    pub fn quest_templates(&self) -> Vec<TemplateInfo> {
        vec![
            TemplateInfo {
                id: "fetch_quest".to_string(),
                name: "Fetch Quest".to_string(),
                description: "Retrieve an item".to_string(),
                category: TemplateCategory::Quest,
                tags: vec!["fetch".to_string(), "simple".to_string()],
            },
            TemplateInfo {
                id: "kill_quest".to_string(),
                name: "Kill Quest".to_string(),
                description: "Defeat monsters".to_string(),
                category: TemplateCategory::Quest,
                tags: vec!["combat".to_string(), "simple".to_string()],
            },
            TemplateInfo {
                id: "escort_quest".to_string(),
                name: "Escort Quest".to_string(),
                description: "Escort NPC safely".to_string(),
                category: TemplateCategory::Quest,
                tags: vec!["escort".to_string(), "complex".to_string()],
            },
            TemplateInfo {
                id: "delivery_quest".to_string(),
                name: "Delivery Quest".to_string(),
                description: "Deliver item to NPC".to_string(),
                category: TemplateCategory::Quest,
                tags: vec!["delivery".to_string(), "simple".to_string()],
            },
        ]
    }

    /// Create a quest from a template
    pub fn create_quest(&self, template_id: &str, id: u32) -> Option<Quest> {
        let id: QuestId = id as u16;
        match template_id {
            "fetch_quest" => Some(Quest {
                id,
                name: "Fetch the Lost Amulet".to_string(),
                description: "Retrieve the ancient amulet from the dungeon".to_string(),
                stages: vec![QuestStage {
                    stage_number: 1,
                    name: "Find the Amulet".to_string(),
                    description: "Find the amulet in the Old Ruins".to_string(),
                    objectives: vec![QuestObjective::CollectItems {
                        item_id: 1,
                        quantity: 1,
                    }],
                    require_all_objectives: true,
                }],
                rewards: vec![QuestReward::Experience(50), QuestReward::Gold(100)],
                min_level: Some(1),
                max_level: None,
                required_quests: vec![],
                repeatable: false,
                is_main_quest: false,
                quest_giver_npc: None,
                quest_giver_map: None,
                quest_giver_position: None,
            }),
            "kill_quest" => Some(Quest {
                id: id.try_into().ok()?,
                name: "Goblin Extermination".to_string(),
                description: "Clear the goblin camp".to_string(),
                stages: vec![QuestStage {
                    stage_number: 1,
                    name: "Defeat Goblins".to_string(),
                    description: "Defeat 10 goblins".to_string(),
                    objectives: vec![QuestObjective::KillMonsters {
                        monster_id: 1,
                        quantity: 10,
                    }],
                    require_all_objectives: true,
                }],
                rewards: vec![QuestReward::Experience(150), QuestReward::Gold(200)],
                min_level: Some(2),
                max_level: None,
                required_quests: vec![],
                repeatable: false,
                is_main_quest: false,
                quest_giver_npc: None,
                quest_giver_map: None,
                quest_giver_position: None,
            }),
            "escort_quest" => Some(Quest {
                id: id.try_into().ok()?,
                name: "Escort the Merchant".to_string(),
                description: "Protect the merchant on the road".to_string(),
                stages: vec![QuestStage {
                    stage_number: 1,
                    name: "Escort Merchant".to_string(),
                    description: "Escort merchant to town".to_string(),
                    objectives: vec![QuestObjective::ReachLocation {
                        map_id: 2,
                        position: antares::domain::types::Position::new(10, 10),
                        radius: 3,
                    }],
                    require_all_objectives: true,
                }],
                rewards: vec![QuestReward::Experience(100), QuestReward::Gold(150)],
                min_level: Some(1),
                max_level: None,
                required_quests: vec![],
                repeatable: false,
                is_main_quest: false,
                quest_giver_npc: None,
                quest_giver_map: None,
                quest_giver_position: None,
            }),
            "delivery_quest" => Some(Quest {
                id: id.try_into().ok()?,
                name: "Deliver the Package".to_string(),
                description: "Take this package to the inn".to_string(),
                stages: vec![QuestStage {
                    stage_number: 1,
                    name: "Deliver Package".to_string(),
                    description: "Deliver package to innkeeper".to_string(),
                    objectives: vec![QuestObjective::CollectItems {
                        item_id: 2,
                        quantity: 1,
                    }],
                    require_all_objectives: true,
                }],
                rewards: vec![QuestReward::Experience(25), QuestReward::Gold(50)],
                min_level: Some(1),
                max_level: None,
                required_quests: vec![],
                repeatable: false,
                is_main_quest: false,
                quest_giver_npc: None,
                quest_giver_map: None,
                quest_giver_position: None,
            }),
            _ => None,
        }
    }

    /// Get all available dialogue templates
    pub fn dialogue_templates(&self) -> Vec<TemplateInfo> {
        vec![
            TemplateInfo {
                id: "merchant".to_string(),
                name: "Merchant".to_string(),
                description: "Standard merchant dialogue".to_string(),
                category: TemplateCategory::Dialogue,
                tags: vec!["merchant".to_string(), "shop".to_string()],
            },
            TemplateInfo {
                id: "quest_giver".to_string(),
                name: "Quest Giver".to_string(),
                description: "NPC offering a quest".to_string(),
                category: TemplateCategory::Dialogue,
                tags: vec!["quest".to_string(), "npc".to_string()],
            },
            TemplateInfo {
                id: "guard".to_string(),
                name: "Guard".to_string(),
                description: "Town guard dialogue".to_string(),
                category: TemplateCategory::Dialogue,
                tags: vec!["guard".to_string(), "town".to_string()],
            },
            TemplateInfo {
                id: "innkeeper".to_string(),
                name: "Innkeeper".to_string(),
                description: "Inn and rest dialogue".to_string(),
                category: TemplateCategory::Dialogue,
                tags: vec!["inn".to_string(), "rest".to_string()],
            },
        ]
    }

    /// Create a dialogue from a template
    pub fn create_dialogue(&self, template_id: &str, id: u32) -> Option<DialogueTree> {
        let id: u16 = id.try_into().ok()?;
        use antares::domain::dialogue::DialogueChoice;
        use std::collections::HashMap;

        match template_id {
            "merchant" => {
                let mut nodes = HashMap::new();
                nodes.insert(
                    0,
                    DialogueNode {
                        id: 0,
                        text: "Welcome to my shop! What can I get for you?".to_string(),
                        speaker_override: Some("Merchant".to_string()),
                        choices: vec![
                            DialogueChoice::new("I'd like to see your wares", Some(1)),
                            DialogueChoice::new("Just browsing", Some(2)),
                            DialogueChoice::new("Goodbye", None),
                        ],
                        conditions: vec![],
                        actions: vec![],
                        is_terminal: false,
                    },
                );
                Some(DialogueTree {
                    id,
                    name: "Merchant Dialogue".to_string(),
                    root_node: 0,
                    nodes,
                    speaker_name: Some("Merchant".to_string()),
                    repeatable: true,
                    associated_quest: None,
                })
            }
            "quest_giver" => {
                let mut nodes = HashMap::new();
                nodes.insert(
                    0,
                    DialogueNode {
                        id: 0,
                        text: "Greetings, adventurer! I have a task that needs doing.".to_string(),
                        speaker_override: Some("Elder".to_string()),
                        choices: vec![
                            DialogueChoice::new("Tell me more", Some(1)),
                            DialogueChoice::new("I'm busy right now", None),
                        ],
                        conditions: vec![],
                        actions: vec![],
                        is_terminal: false,
                    },
                );
                Some(DialogueTree {
                    id,
                    name: "Quest Giver Dialogue".to_string(),
                    root_node: 0,
                    nodes,
                    speaker_name: Some("Elder".to_string()),
                    repeatable: false,
                    associated_quest: None,
                })
            }
            "guard" => {
                let mut nodes = HashMap::new();
                nodes.insert(
                    0,
                    DialogueNode {
                        id: 0,
                        text: "Halt! State your business in this town.".to_string(),
                        speaker_override: Some("Guard".to_string()),
                        choices: vec![
                            DialogueChoice::new("I'm just passing through", Some(1)),
                            DialogueChoice::new("I'm looking for work", Some(2)),
                            DialogueChoice::new("None of your business", Some(3)),
                        ],
                        conditions: vec![],
                        actions: vec![],
                        is_terminal: false,
                    },
                );
                Some(DialogueTree {
                    id,
                    name: "Guard Dialogue".to_string(),
                    root_node: 0,
                    nodes,
                    speaker_name: Some("Guard".to_string()),
                    repeatable: true,
                    associated_quest: None,
                })
            }
            "innkeeper" => {
                let mut nodes = HashMap::new();
                nodes.insert(
                    0,
                    DialogueNode {
                        id: 0,
                        text: "Welcome to the inn! A bed for the night?".to_string(),
                        speaker_override: Some("Innkeeper".to_string()),
                        choices: vec![
                            DialogueChoice::new("Yes, I need rest (10 gold)", Some(1)),
                            DialogueChoice::new("Not right now", None),
                        ],
                        conditions: vec![],
                        actions: vec![],
                        is_terminal: false,
                    },
                );
                Some(DialogueTree {
                    id,
                    name: "Innkeeper Dialogue".to_string(),
                    root_node: 0,
                    nodes,
                    speaker_name: Some("Innkeeper".to_string()),
                    repeatable: true,
                    associated_quest: None,
                })
            }
            _ => None,
        }
    }

    /// Add a custom item template
    pub fn add_custom_item(&mut self, item: Item) {
        self.custom_items.push(item);
    }

    /// Add a custom monster template
    pub fn add_custom_monster(&mut self, monster: MonsterDefinition) {
        self.custom_monsters.push(monster);
    }

    /// Add a custom quest template
    pub fn add_custom_quest(&mut self, quest: Quest) {
        self.custom_quests.push(quest);
    }

    /// Add a custom dialogue template
    pub fn add_custom_dialogue(&mut self, dialogue: DialogueTree) {
        self.custom_dialogues.push(dialogue);
    }

    /// Get custom item count
    pub fn custom_item_count(&self) -> usize {
        self.custom_items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_manager_creation() {
        let manager = TemplateManager::new();
        assert!(!manager.item_templates().is_empty());
        assert!(!manager.monster_templates().is_empty());
        assert!(!manager.quest_templates().is_empty());
        assert!(!manager.dialogue_templates().is_empty());
    }

    #[test]
    fn test_create_item_from_template() {
        let manager = TemplateManager::new();
        let item = manager.create_item("basic_sword", 1);
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.id, 1);
        assert_eq!(item.name, "Short Sword");
    }

    #[test]
    fn test_create_monster_from_template() {
        let manager = TemplateManager::new();
        let monster = manager.create_monster("goblin", 1);
        assert!(monster.is_some());
        let monster = monster.unwrap();
        assert_eq!(monster.id, 1);
        assert_eq!(monster.name, "Goblin");
    }

    #[test]
    fn test_create_quest_from_template() {
        let manager = TemplateManager::new();
        let quest = manager.create_quest("fetch_quest", 1);
        assert!(quest.is_some());
        let quest = quest.unwrap();
        assert_eq!(quest.id, 1);
        assert_eq!(quest.name, "Fetch the Lost Amulet");
    }

    #[test]
    fn test_create_dialogue_from_template() {
        let manager = TemplateManager::new();
        let dialogue = manager.create_dialogue("merchant", 1);
        assert!(dialogue.is_some());
        let dialogue = dialogue.unwrap();
        assert_eq!(dialogue.id, 1);
        assert_eq!(dialogue.name, "Merchant Dialogue");
    }

    #[test]
    fn test_invalid_template_returns_none() {
        let manager = TemplateManager::new();
        assert!(manager.create_item("invalid", 1).is_none());
        assert!(manager.create_monster("invalid", 1).is_none());
        assert!(manager.create_quest("invalid", 1).is_none());
        assert!(manager.create_dialogue("invalid", 1).is_none());
    }

    #[test]
    fn test_custom_templates() {
        let mut manager = TemplateManager::new();
        let initial_count = manager.item_templates().len();

        let custom_item = Item {
            id: 99,
            name: "Custom Sword".to_string(),
            item_type: ItemType::Weapon(WeaponData {
                damage: DiceRoll::new(2, 8, 2),
                bonus: 2,
                hands_required: 1,
            }),
            base_cost: 500,
            sell_cost: 250,
            disablements: Disablement::ALL,
            constant_bonus: None,
            temporary_bonus: None,
            spell_effect: None,
            max_charges: 0,
            is_cursed: false,
        };

        manager.add_custom_item(custom_item.clone());
        assert_eq!(manager.item_templates().len(), initial_count + 1);
        assert_eq!(manager.custom_item_count(), 1);
    }

    #[test]
    fn test_template_categories() {
        let categories = TemplateCategory::all();
        assert_eq!(categories.len(), 5);
        assert!(categories.contains(&TemplateCategory::Item));
        assert!(categories.contains(&TemplateCategory::Monster));
    }

    #[test]
    fn test_template_category_names() {
        assert_eq!(TemplateCategory::Item.name(), "Items");
        assert_eq!(TemplateCategory::Monster.name(), "Monsters");
        assert_eq!(TemplateCategory::Quest.name(), "Quests");
    }

    #[test]
    fn test_item_template_variety() {
        let manager = TemplateManager::new();
        let templates = manager.item_templates();

        // Should have weapons, armor, and consumables
        assert!(templates
            .iter()
            .any(|t| t.tags.contains(&"weapon".to_string())));
        assert!(templates
            .iter()
            .any(|t| t.tags.contains(&"armor".to_string())));
        assert!(templates
            .iter()
            .any(|t| t.tags.contains(&"consumable".to_string())));
    }

    #[test]
    fn test_monster_template_variety() {
        let manager = TemplateManager::new();
        let templates = manager.monster_templates();

        // Should have different difficulty levels
        assert!(templates
            .iter()
            .any(|t| t.tags.contains(&"weak".to_string())));
        assert!(templates
            .iter()
            .any(|t| t.tags.contains(&"boss".to_string())));
    }

    #[test]
    fn test_quest_template_variety() {
        let manager = TemplateManager::new();
        let templates = manager.quest_templates();

        // Should have different quest types
        assert!(templates
            .iter()
            .any(|t| t.tags.contains(&"fetch".to_string())));
        assert!(templates
            .iter()
            .any(|t| t.tags.contains(&"combat".to_string())));
        assert!(templates
            .iter()
            .any(|t| t.tags.contains(&"escort".to_string())));
    }

    #[test]
    fn test_all_weapon_templates_create_successfully() {
        let manager = TemplateManager::new();
        assert!(manager.create_item("basic_sword", 1).is_some());
        assert!(manager.create_item("basic_dagger", 2).is_some());
        assert!(manager.create_item("basic_bow", 3).is_some());
        assert!(manager.create_item("basic_staff", 4).is_some());
    }

    #[test]
    fn test_all_armor_templates_create_successfully() {
        let manager = TemplateManager::new();
        assert!(manager.create_item("leather_armor", 10).is_some());
        assert!(manager.create_item("chain_mail", 11).is_some());
        assert!(manager.create_item("plate_mail", 12).is_some());
    }

    #[test]
    fn test_all_monster_templates_create_successfully() {
        let manager = TemplateManager::new();
        assert!(manager.create_monster("goblin", 1).is_some());
        assert!(manager.create_monster("skeleton", 2).is_some());
        assert!(manager.create_monster("orc", 3).is_some());
        assert!(manager.create_monster("dragon", 4).is_some());
    }

    #[test]
    fn test_all_quest_templates_create_successfully() {
        let manager = TemplateManager::new();
        assert!(manager.create_quest("fetch_quest", 1).is_some());
        assert!(manager.create_quest("kill_quest", 2).is_some());
        assert!(manager.create_quest("escort_quest", 3).is_some());
        assert!(manager.create_quest("delivery_quest", 4).is_some());
    }

    #[test]
    fn test_all_dialogue_templates_create_successfully() {
        let manager = TemplateManager::new();
        assert!(manager.create_dialogue("merchant", 1).is_some());
        assert!(manager.create_dialogue("quest_giver", 2).is_some());
        assert!(manager.create_dialogue("guard", 3).is_some());
        assert!(manager.create_dialogue("innkeeper", 4).is_some());
    }
}
