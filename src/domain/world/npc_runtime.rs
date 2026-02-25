// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! NPC runtime state - mutable per-session NPC data
//!
//! This module provides the runtime state that tracks NPC stock and consumed
//! services during a play session. Unlike `NpcDefinition` (static data loaded
//! from RON files), these types are mutated as the player interacts with NPCs.
//!
//! # Architecture
//!
//! - `NpcRuntimeState` - per-NPC mutable state (stock quantities, consumed services)
//! - `NpcRuntimeStore` - session-wide map of all NPC runtime states
//! - `MerchantStockTemplate` - immutable template used to initialize merchant stock
//! - `MerchantStockTemplateDatabase` - loads and indexes all stock templates
//!
//! Runtime states are serialized into save data so stock changes persist across
//! save/load cycles.
//!
//! # Architecture Reference
//!
//! See `docs/reference/architecture.md` Section 4.9 (Campaign System) and the
//! inventory system implementation plan Phase 2 for complete specifications.

use crate::domain::inventory::{MerchantStock, StockEntry};
use crate::domain::world::npc::{NpcDefinition, NpcId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

// ===== MerchantStockTemplate =====

/// A single item entry in a merchant stock template
///
/// Template entries hold the *initial* quantities used to seed runtime stock
/// when the game session begins. They are never mutated during play.
///
/// # Examples
///
/// ```
/// use antares::domain::world::npc_runtime::TemplateStockEntry;
///
/// let entry = TemplateStockEntry { item_id: 1, quantity: 5, override_price: None };
/// assert_eq!(entry.item_id, 1);
/// assert_eq!(entry.quantity, 5);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateStockEntry {
    /// Item being sold (references `ItemDatabase`)
    pub item_id: crate::domain::types::ItemId,
    /// Initial quantity at session start
    pub quantity: u8,
    /// Optional fixed price override (None = use item base_cost × sell_rate)
    pub override_price: Option<u32>,
}

/// Immutable stock template for a merchant NPC
///
/// Templates are loaded from `npc_stock_templates.ron` at game start. Each
/// template is identified by a string ID that matches `NpcDefinition::stock_template`.
/// When a merchant is initialised for the session, a `MerchantStock` is created
/// by copying the template quantities into mutable `StockEntry` values.
///
/// Templates are **never** mutated during play; all mutations happen on the
/// `MerchantStock` stored inside `NpcRuntimeState`.
///
/// # Examples
///
/// ```
/// use antares::domain::world::npc_runtime::{MerchantStockTemplate, TemplateStockEntry};
///
/// let template = MerchantStockTemplate {
///     id: "blacksmith_basic".to_string(),
///     entries: vec![
///         TemplateStockEntry { item_id: 1, quantity: 3, override_price: None },
///         TemplateStockEntry { item_id: 2, quantity: 1, override_price: Some(500) },
///     ],
/// };
///
/// assert_eq!(template.id, "blacksmith_basic");
/// assert_eq!(template.entries.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantStockTemplate {
    /// Unique identifier – matches `NpcDefinition::stock_template`
    pub id: String,
    /// Template entries (initial quantities; not mutated during play)
    pub entries: Vec<TemplateStockEntry>,
}

impl MerchantStockTemplate {
    /// Creates a `MerchantStock` runtime instance from this template
    ///
    /// Copies all template entries into mutable `StockEntry` values so that
    /// quantities can be decremented during the session without affecting the
    /// original template.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{MerchantStockTemplate, TemplateStockEntry};
    ///
    /// let template = MerchantStockTemplate {
    ///     id: "general_goods".to_string(),
    ///     entries: vec![
    ///         TemplateStockEntry { item_id: 10, quantity: 5, override_price: None },
    ///     ],
    /// };
    ///
    /// let stock = template.to_runtime_stock();
    /// assert_eq!(stock.get_entry(10).unwrap().quantity, 5);
    /// assert_eq!(stock.restock_template, Some("general_goods".to_string()));
    /// ```
    pub fn to_runtime_stock(&self) -> MerchantStock {
        let entries = self
            .entries
            .iter()
            .map(|t| StockEntry {
                item_id: t.item_id,
                quantity: t.quantity,
                override_price: t.override_price,
            })
            .collect();

        MerchantStock {
            entries,
            restock_template: Some(self.id.clone()),
        }
    }
}

// ===== NpcRuntimeState =====

/// Per-NPC mutable runtime state for a play session
///
/// This holds all data about an NPC that can change during play:
/// - Current merchant stock (quantities decrease as items are purchased)
/// - Services consumed this session (for future per-session limits)
///
/// `NpcRuntimeState` is serialised into save data so that stock levels and
/// consumed services persist across save/load cycles.
///
/// # Examples
///
/// ```
/// use antares::domain::world::npc_runtime::NpcRuntimeState;
///
/// let state = NpcRuntimeState::new("merchant_tom".to_string());
/// assert_eq!(state.npc_id, "merchant_tom");
/// assert!(state.stock.is_none());
/// assert!(state.services_consumed.is_empty());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRuntimeState {
    /// Which NPC this runtime state belongs to
    pub npc_id: NpcId,

    /// Current mutable merchant stock.
    ///
    /// `None` if the NPC is not a merchant or has no stock template. Initialised
    /// from a `MerchantStockTemplate` at session start via
    /// `NpcRuntimeStore::initialize_merchant`.
    pub stock: Option<MerchantStock>,

    /// Service IDs consumed by the party this session.
    ///
    /// Appended by `consume_service` in `transactions.rs`. May be used in
    /// future phases to enforce per-session service limits (e.g. one
    /// resurrection per session).
    pub services_consumed: Vec<String>,
}

impl NpcRuntimeState {
    /// Creates a new runtime state with no stock and no consumed services
    ///
    /// # Arguments
    ///
    /// * `npc_id` - The NPC this state belongs to
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::NpcRuntimeState;
    ///
    /// let state = NpcRuntimeState::new("village_elder".to_string());
    /// assert_eq!(state.npc_id, "village_elder");
    /// assert!(state.stock.is_none());
    /// assert!(state.services_consumed.is_empty());
    /// ```
    pub fn new(npc_id: NpcId) -> Self {
        Self {
            npc_id,
            stock: None,
            services_consumed: Vec::new(),
        }
    }

    /// Creates a runtime state with stock initialised from the given template
    ///
    /// # Arguments
    ///
    /// * `npc_id` - The NPC this state belongs to
    /// * `template` - The stock template to copy quantities from
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{NpcRuntimeState, MerchantStockTemplate, TemplateStockEntry};
    ///
    /// let template = MerchantStockTemplate {
    ///     id: "basic_shop".to_string(),
    ///     entries: vec![TemplateStockEntry { item_id: 1, quantity: 3, override_price: None }],
    /// };
    ///
    /// let state = NpcRuntimeState::initialize_stock_from_template(
    ///     "blacksmith".to_string(),
    ///     &template,
    /// );
    ///
    /// assert_eq!(state.npc_id, "blacksmith");
    /// assert!(state.stock.is_some());
    /// assert_eq!(state.stock.as_ref().unwrap().get_entry(1).unwrap().quantity, 3);
    /// ```
    pub fn initialize_stock_from_template(npc_id: NpcId, template: &MerchantStockTemplate) -> Self {
        Self {
            npc_id,
            stock: Some(template.to_runtime_stock()),
            services_consumed: Vec::new(),
        }
    }
}

// ===== NpcRuntimeStore =====

/// Session-wide store of all NPC runtime states
///
/// Maps NPC IDs to their mutable runtime state. The store is held on `GameState`
/// and serialised into save data so that merchant stock levels persist across
/// save/load cycles.
///
/// # Examples
///
/// ```
/// use antares::domain::world::npc_runtime::{NpcRuntimeStore, NpcRuntimeState};
///
/// let mut store = NpcRuntimeStore::new();
/// let state = NpcRuntimeState::new("merchant_tom".to_string());
/// store.insert(state);
///
/// assert!(store.get(&"merchant_tom".to_string()).is_some());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NpcRuntimeStore {
    /// Map of NPC ID → runtime state
    npcs: HashMap<NpcId, NpcRuntimeState>,
}

impl NpcRuntimeStore {
    /// Creates a new empty store
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::NpcRuntimeStore;
    ///
    /// let store = NpcRuntimeStore::new();
    /// ```
    pub fn new() -> Self {
        Self {
            npcs: HashMap::new(),
        }
    }

    /// Returns an immutable reference to the runtime state for the given NPC, if present
    ///
    /// # Arguments
    ///
    /// * `npc_id` - The NPC ID to look up
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{NpcRuntimeStore, NpcRuntimeState};
    ///
    /// let mut store = NpcRuntimeStore::new();
    /// store.insert(NpcRuntimeState::new("guard".to_string()));
    ///
    /// assert!(store.get(&"guard".to_string()).is_some());
    /// assert!(store.get(&"nobody".to_string()).is_none());
    /// ```
    pub fn get(&self, npc_id: &NpcId) -> Option<&NpcRuntimeState> {
        self.npcs.get(npc_id)
    }

    /// Returns a mutable reference to the runtime state for the given NPC, if present
    ///
    /// # Arguments
    ///
    /// * `npc_id` - The NPC ID to look up
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{NpcRuntimeStore, NpcRuntimeState};
    ///
    /// let mut store = NpcRuntimeStore::new();
    /// store.insert(NpcRuntimeState::new("merchant_bob".to_string()));
    ///
    /// if let Some(state) = store.get_mut(&"merchant_bob".to_string()) {
    ///     state.services_consumed.push("heal_all".to_string());
    /// }
    ///
    /// assert_eq!(
    ///     store.get(&"merchant_bob".to_string()).unwrap().services_consumed,
    ///     vec!["heal_all".to_string()]
    /// );
    /// ```
    pub fn get_mut(&mut self, npc_id: &NpcId) -> Option<&mut NpcRuntimeState> {
        self.npcs.get_mut(npc_id)
    }

    /// Inserts (or replaces) the runtime state for an NPC
    ///
    /// # Arguments
    ///
    /// * `state` - Runtime state to insert; keyed by `state.npc_id`
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{NpcRuntimeStore, NpcRuntimeState};
    ///
    /// let mut store = NpcRuntimeStore::new();
    /// store.insert(NpcRuntimeState::new("priest_anna".to_string()));
    ///
    /// assert!(store.get(&"priest_anna".to_string()).is_some());
    /// ```
    pub fn insert(&mut self, state: NpcRuntimeState) {
        self.npcs.insert(state.npc_id.clone(), state);
    }

    /// Initialises (or re-initialises) the runtime state for a merchant NPC
    ///
    /// If `npc.stock_template` is `Some` and the template exists in `templates`,
    /// creates a `NpcRuntimeState` with stock quantities copied from the template
    /// and inserts it into the store, replacing any pre-existing state for that NPC.
    ///
    /// If the NPC has no `stock_template` (e.g. a plain NPC or a priest), inserts
    /// a bare `NpcRuntimeState` with no stock so that service tracking still works.
    ///
    /// # Arguments
    ///
    /// * `npc` - The NPC definition to initialise
    /// * `templates` - Database of stock templates to look up the template from
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{
    ///     NpcRuntimeStore, MerchantStockTemplate, MerchantStockTemplateDatabase, TemplateStockEntry,
    /// };
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let mut store = NpcRuntimeStore::new();
    ///
    /// let mut templates = MerchantStockTemplateDatabase::new();
    /// templates.add(MerchantStockTemplate {
    ///     id: "weapons_basic".to_string(),
    ///     entries: vec![TemplateStockEntry { item_id: 1, quantity: 3, override_price: None }],
    /// });
    ///
    /// let mut merchant = NpcDefinition::merchant("smith", "The Smith", "smith.png");
    /// merchant.stock_template = Some("weapons_basic".to_string());
    ///
    /// store.initialize_merchant(&merchant, &templates);
    ///
    /// let state = store.get(&"smith".to_string()).unwrap();
    /// assert!(state.stock.is_some());
    /// assert_eq!(state.stock.as_ref().unwrap().get_entry(1).unwrap().quantity, 3);
    /// ```
    pub fn initialize_merchant(
        &mut self,
        npc: &NpcDefinition,
        templates: &MerchantStockTemplateDatabase,
    ) {
        let state = match &npc.stock_template {
            Some(template_id) => match templates.get(template_id) {
                Some(template) => {
                    NpcRuntimeState::initialize_stock_from_template(npc.id.clone(), template)
                }
                None => NpcRuntimeState::new(npc.id.clone()),
            },
            None => NpcRuntimeState::new(npc.id.clone()),
        };
        self.insert(state);
    }

    /// Returns the total number of NPC runtime states in this store
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{NpcRuntimeStore, NpcRuntimeState};
    ///
    /// let mut store = NpcRuntimeStore::new();
    /// assert_eq!(store.len(), 0);
    ///
    /// store.insert(NpcRuntimeState::new("a".to_string()));
    /// store.insert(NpcRuntimeState::new("b".to_string()));
    /// assert_eq!(store.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.npcs.len()
    }

    /// Returns `true` if the store contains no runtime states
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::NpcRuntimeStore;
    ///
    /// let store = NpcRuntimeStore::new();
    /// assert!(store.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.npcs.is_empty()
    }
}

// ===== MerchantStockTemplateDatabase =====

/// Errors that can occur when loading stock template data
#[derive(Error, Debug)]
pub enum MerchantStockTemplateDatabaseError {
    #[error("Failed to read stock template file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse RON stock template data: {0}")]
    ParseError(#[from] ron::error::SpannedError),

    #[error("Duplicate stock template ID: {0}")]
    DuplicateId(String),
}

/// Database of all merchant stock templates
///
/// Loaded from `npc_stock_templates.ron` at game start. Templates are indexed
/// by their string ID so they can be looked up quickly when initialising
/// merchant runtime states.
///
/// # Examples
///
/// ```
/// use antares::domain::world::npc_runtime::{MerchantStockTemplateDatabase, MerchantStockTemplate};
///
/// let mut db = MerchantStockTemplateDatabase::new();
/// db.add(MerchantStockTemplate { id: "basic".to_string(), entries: vec![] });
///
/// assert!(db.get("basic").is_some());
/// assert!(db.get("missing").is_none());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MerchantStockTemplateDatabase {
    /// All templates indexed by ID
    templates: HashMap<String, MerchantStockTemplate>,
}

impl MerchantStockTemplateDatabase {
    /// Creates a new empty database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::MerchantStockTemplateDatabase;
    ///
    /// let db = MerchantStockTemplateDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Adds a template to the database
    ///
    /// Silently replaces any existing template with the same ID.
    ///
    /// # Arguments
    ///
    /// * `template` - The template to add
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{MerchantStockTemplateDatabase, MerchantStockTemplate};
    ///
    /// let mut db = MerchantStockTemplateDatabase::new();
    /// db.add(MerchantStockTemplate { id: "shop_a".to_string(), entries: vec![] });
    /// assert_eq!(db.len(), 1);
    /// ```
    pub fn add(&mut self, template: MerchantStockTemplate) {
        self.templates.insert(template.id.clone(), template);
    }

    /// Returns an immutable reference to the template with the given ID, if present
    ///
    /// # Arguments
    ///
    /// * `id` - The template ID to look up
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{MerchantStockTemplateDatabase, MerchantStockTemplate};
    ///
    /// let mut db = MerchantStockTemplateDatabase::new();
    /// db.add(MerchantStockTemplate { id: "basic".to_string(), entries: vec![] });
    ///
    /// assert!(db.get("basic").is_some());
    /// assert!(db.get("nope").is_none());
    /// ```
    pub fn get(&self, id: &str) -> Option<&MerchantStockTemplate> {
        self.templates.get(id)
    }

    /// Loads a template database from a RON file
    ///
    /// The file must contain a `Vec<MerchantStockTemplate>`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the `.ron` file
    ///
    /// # Errors
    ///
    /// Returns `MerchantStockTemplateDatabaseError::ReadError` if the file cannot be read.
    /// Returns `MerchantStockTemplateDatabaseError::ParseError` if RON parsing fails.
    /// Returns `MerchantStockTemplateDatabaseError::DuplicateId` if two templates share the same ID.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use antares::domain::world::npc_runtime::MerchantStockTemplateDatabase;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = MerchantStockTemplateDatabase::load_from_file("data/npc_stock_templates.ron")?;
    /// println!("Loaded {} templates", db.len());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<Self, MerchantStockTemplateDatabaseError> {
        let contents = std::fs::read_to_string(path)?;
        Self::load_from_string(&contents)
    }

    /// Loads a template database from a RON string
    ///
    /// # Arguments
    ///
    /// * `ron_data` - RON-formatted string containing a `Vec<MerchantStockTemplate>`
    ///
    /// # Errors
    ///
    /// Returns `MerchantStockTemplateDatabaseError::ParseError` if RON parsing fails.
    /// Returns `MerchantStockTemplateDatabaseError::DuplicateId` if two templates share the same ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::MerchantStockTemplateDatabase;
    ///
    /// let ron = r#"[
    ///     (id: "weapons_basic", entries: [
    ///         (item_id: 1, quantity: 3, override_price: None),
    ///     ]),
    /// ]"#;
    ///
    /// let db = MerchantStockTemplateDatabase::load_from_string(ron).unwrap();
    /// assert_eq!(db.len(), 1);
    /// ```
    pub fn load_from_string(ron_data: &str) -> Result<Self, MerchantStockTemplateDatabaseError> {
        let templates: Vec<MerchantStockTemplate> = ron::from_str(ron_data)?;
        let mut db = Self::new();
        for template in templates {
            if db.templates.contains_key(&template.id) {
                return Err(MerchantStockTemplateDatabaseError::DuplicateId(template.id));
            }
            db.templates.insert(template.id.clone(), template);
        }
        Ok(db)
    }

    /// Returns the number of templates in the database
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::MerchantStockTemplateDatabase;
    ///
    /// let db = MerchantStockTemplateDatabase::new();
    /// assert_eq!(db.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Returns `true` if the database contains no templates
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::MerchantStockTemplateDatabase;
    ///
    /// let db = MerchantStockTemplateDatabase::new();
    /// assert!(db.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::world::npc::NpcDefinition;

    // ===== NpcRuntimeState tests =====

    #[test]
    fn test_npc_runtime_state_new() {
        let state = NpcRuntimeState::new("guard_01".to_string());

        assert_eq!(state.npc_id, "guard_01");
        assert!(state.stock.is_none());
        assert!(state.services_consumed.is_empty());
    }

    #[test]
    fn test_npc_runtime_state_initialize_stock_from_template() {
        let template = MerchantStockTemplate {
            id: "test_template".to_string(),
            entries: vec![
                TemplateStockEntry {
                    item_id: 1,
                    quantity: 5,
                    override_price: None,
                },
                TemplateStockEntry {
                    item_id: 2,
                    quantity: 2,
                    override_price: Some(300),
                },
            ],
        };

        let state =
            NpcRuntimeState::initialize_stock_from_template("merchant_bob".to_string(), &template);

        assert_eq!(state.npc_id, "merchant_bob");
        assert!(state.services_consumed.is_empty());

        let stock = state.stock.as_ref().expect("stock must be Some");
        assert_eq!(stock.get_entry(1).expect("item 1 must exist").quantity, 5);
        assert_eq!(stock.get_entry(1).unwrap().override_price, None);
        assert_eq!(stock.get_entry(2).expect("item 2 must exist").quantity, 2);
        assert_eq!(stock.get_entry(2).unwrap().override_price, Some(300));
        assert_eq!(stock.restock_template, Some("test_template".to_string()));
    }

    #[test]
    fn test_npc_runtime_state_stock_independent_of_template() {
        // Mutating runtime stock must not affect the template
        let template = MerchantStockTemplate {
            id: "goods".to_string(),
            entries: vec![TemplateStockEntry {
                item_id: 10,
                quantity: 4,
                override_price: None,
            }],
        };

        let mut state =
            NpcRuntimeState::initialize_stock_from_template("vendor".to_string(), &template);

        // Decrement runtime stock
        let stock = state.stock.as_mut().unwrap();
        assert!(stock.decrement(10));
        assert_eq!(stock.get_entry(10).unwrap().quantity, 3);

        // Template entry quantity is unchanged
        assert_eq!(template.entries[0].quantity, 4);
    }

    #[test]
    fn test_npc_runtime_state_serialization_roundtrip() {
        let mut state = NpcRuntimeState::new("priest_anna".to_string());
        state.services_consumed.push("heal_all".to_string());

        let serialized = ron::to_string(&state).expect("serialization must succeed");
        let deserialized: NpcRuntimeState =
            ron::from_str(&serialized).expect("deserialization must succeed");

        assert_eq!(deserialized.npc_id, state.npc_id);
        assert_eq!(deserialized.services_consumed, state.services_consumed);
        assert!(deserialized.stock.is_none());
    }

    // ===== NpcRuntimeStore tests =====

    #[test]
    fn test_npc_runtime_store_insert_and_get() {
        let mut store = NpcRuntimeStore::new();
        let state = NpcRuntimeState::new("merchant_tom".to_string());
        store.insert(state);

        let retrieved = store.get(&"merchant_tom".to_string());
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().npc_id, "merchant_tom");
    }

    #[test]
    fn test_npc_runtime_store_get_absent() {
        let store = NpcRuntimeStore::new();
        assert!(store.get(&"nobody".to_string()).is_none());
    }

    #[test]
    fn test_npc_runtime_store_get_mut() {
        let mut store = NpcRuntimeStore::new();
        store.insert(NpcRuntimeState::new("innkeeper_mary".to_string()));

        // Mutate via get_mut
        {
            let state = store
                .get_mut(&"innkeeper_mary".to_string())
                .expect("state must exist");
            state.services_consumed.push("rest".to_string());
        }

        // Assert mutation persisted
        let state = store
            .get(&"innkeeper_mary".to_string())
            .expect("state must exist after mutation");
        assert_eq!(state.services_consumed, vec!["rest".to_string()]);
    }

    #[test]
    fn test_npc_runtime_store_insert_replaces_existing() {
        let mut store = NpcRuntimeStore::new();
        store.insert(NpcRuntimeState::new("guard".to_string()));

        // Insert replacement with consumed service recorded
        let mut replacement = NpcRuntimeState::new("guard".to_string());
        replacement.services_consumed.push("something".to_string());
        store.insert(replacement);

        let state = store.get(&"guard".to_string()).unwrap();
        assert_eq!(state.services_consumed.len(), 1);
    }

    #[test]
    fn test_npc_runtime_store_len_and_is_empty() {
        let mut store = NpcRuntimeStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        store.insert(NpcRuntimeState::new("a".to_string()));
        store.insert(NpcRuntimeState::new("b".to_string()));

        assert!(!store.is_empty());
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_npc_runtime_store_initialize_merchant_with_template() {
        let mut store = NpcRuntimeStore::new();
        let mut templates = MerchantStockTemplateDatabase::new();
        templates.add(MerchantStockTemplate {
            id: "weapons_basic".to_string(),
            entries: vec![TemplateStockEntry {
                item_id: 1,
                quantity: 3,
                override_price: None,
            }],
        });

        let mut merchant = NpcDefinition::merchant("smith", "The Smith", "smith.png");
        merchant.stock_template = Some("weapons_basic".to_string());

        store.initialize_merchant(&merchant, &templates);

        let state = store
            .get(&"smith".to_string())
            .expect("state must be inserted");
        assert_eq!(state.npc_id, "smith");
        let stock = state.stock.as_ref().expect("stock must be Some");
        assert_eq!(stock.get_entry(1).unwrap().quantity, 3);
    }

    #[test]
    fn test_npc_runtime_store_initialize_merchant_missing_template() {
        let mut store = NpcRuntimeStore::new();
        let templates = MerchantStockTemplateDatabase::new(); // empty

        let mut merchant = NpcDefinition::merchant("trader", "Trader Joe", "trader.png");
        merchant.stock_template = Some("nonexistent_template".to_string());

        store.initialize_merchant(&merchant, &templates);

        // State inserted but stock is None (template not found)
        let state = store
            .get(&"trader".to_string())
            .expect("state must still be inserted");
        assert!(state.stock.is_none());
    }

    #[test]
    fn test_npc_runtime_store_initialize_merchant_no_stock_template() {
        let mut store = NpcRuntimeStore::new();
        let templates = MerchantStockTemplateDatabase::new();

        let priest = NpcDefinition::priest("healer", "High Healer", "healer.png");
        // stock_template is None by default

        store.initialize_merchant(&priest, &templates);

        let state = store
            .get(&"healer".to_string())
            .expect("state must be inserted");
        assert!(state.stock.is_none());
        assert!(state.services_consumed.is_empty());
    }

    // ===== MerchantStockTemplateDatabase tests =====

    #[test]
    fn test_merchant_stock_template_database_new_is_empty() {
        let db = MerchantStockTemplateDatabase::new();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
    }

    #[test]
    fn test_merchant_stock_template_database_add_and_get() {
        let mut db = MerchantStockTemplateDatabase::new();
        db.add(MerchantStockTemplate {
            id: "general_goods".to_string(),
            entries: vec![],
        });

        assert!(db.get("general_goods").is_some());
        assert!(db.get("other").is_none());
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn test_merchant_stock_template_database_load_from_string_success() {
        let ron = r#"[
            (id: "shop_a", entries: [
                (item_id: 1, quantity: 5, override_price: None),
                (item_id: 2, quantity: 1, override_price: Some(999)),
            ]),
            (id: "shop_b", entries: []),
        ]"#;

        let db =
            MerchantStockTemplateDatabase::load_from_string(ron).expect("must parse successfully");

        assert_eq!(db.len(), 2);

        let shop_a = db.get("shop_a").expect("shop_a must exist");
        assert_eq!(shop_a.entries.len(), 2);
        assert_eq!(shop_a.entries[0].item_id, 1);
        assert_eq!(shop_a.entries[0].quantity, 5);
        assert_eq!(shop_a.entries[1].override_price, Some(999));

        let shop_b = db.get("shop_b").expect("shop_b must exist");
        assert!(shop_b.entries.is_empty());
    }

    #[test]
    fn test_merchant_stock_template_database_load_from_string_duplicate_id_error() {
        let ron = r#"[
            (id: "dup", entries: []),
            (id: "dup", entries: []),
        ]"#;

        let result = MerchantStockTemplateDatabase::load_from_string(ron);
        assert!(matches!(
            result,
            Err(MerchantStockTemplateDatabaseError::DuplicateId(_))
        ));
    }

    #[test]
    fn test_merchant_stock_template_database_load_from_string_invalid_ron_error() {
        let bad_ron = "this is not valid ron }{";
        let result = MerchantStockTemplateDatabase::load_from_string(bad_ron);
        assert!(matches!(
            result,
            Err(MerchantStockTemplateDatabaseError::ParseError(_))
        ));
    }

    #[test]
    fn test_merchant_stock_template_to_runtime_stock() {
        let template = MerchantStockTemplate {
            id: "forge_stock".to_string(),
            entries: vec![
                TemplateStockEntry {
                    item_id: 5,
                    quantity: 10,
                    override_price: None,
                },
                TemplateStockEntry {
                    item_id: 6,
                    quantity: 2,
                    override_price: Some(1000),
                },
            ],
        };

        let stock = template.to_runtime_stock();
        assert_eq!(stock.restock_template, Some("forge_stock".to_string()));
        assert_eq!(stock.get_entry(5).unwrap().quantity, 10);
        assert_eq!(stock.get_entry(5).unwrap().override_price, None);
        assert_eq!(stock.get_entry(6).unwrap().quantity, 2);
        assert_eq!(stock.get_entry(6).unwrap().override_price, Some(1000));
    }

    #[test]
    fn test_npc_runtime_store_serialization_roundtrip() {
        let mut store = NpcRuntimeStore::new();
        let mut state = NpcRuntimeState::new("merchant".to_string());
        state.services_consumed.push("heal_all".to_string());
        store.insert(state);

        let serialized = ron::to_string(&store).expect("serialization must succeed");
        let deserialized: NpcRuntimeStore =
            ron::from_str(&serialized).expect("deserialization must succeed");

        let recovered = deserialized.get(&"merchant".to_string()).unwrap();
        assert_eq!(recovered.services_consumed, vec!["heal_all".to_string()]);
    }
}
