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
//! - `NpcRuntimeState` - per-NPC mutable state (stock quantities, consumed services,
//!   restock tracking)
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
//! Phase 6 adds daily restock and magic-item rotation driven by `GameState::advance_time`.

use crate::domain::inventory::{MerchantStock, StockEntry};
use crate::domain::types::ItemId;
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
    pub item_id: ItemId,
    /// Initial quantity at session start
    pub quantity: u8,
    /// Optional fixed price override (None = use item base_cost × sell_rate)
    pub override_price: Option<u32>,
}

/// Returns the default number of in-game days between magic-item slot refreshes.
fn default_magic_refresh_days() -> u32 {
    7
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
/// The three Phase-6 fields (`magic_item_pool`, `magic_slot_count`,
/// `magic_refresh_days`) all carry `#[serde(default)]` so that existing `.ron`
/// files without those fields deserialise without error — the effect is that
/// magic-item rotation is simply disabled for any template that omits them.
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
///     magic_item_pool: vec![],
///     magic_slot_count: 0,
///     magic_refresh_days: 7,
/// };
///
/// assert_eq!(template.id, "blacksmith_basic");
/// assert_eq!(template.entries.len(), 2);
/// assert_eq!(template.magic_slot_count, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MerchantStockTemplate {
    /// Unique identifier – matches `NpcDefinition::stock_template`
    pub id: String,
    /// Template entries (initial quantities; not mutated during play)
    pub entries: Vec<TemplateStockEntry>,

    /// Pool of item IDs that may appear in the merchant's magic-item slots.
    ///
    /// At each magic refresh the engine picks `magic_slot_count` distinct
    /// items at random from this list. Duplicates in the list act as
    /// weighted entries (a doubled entry is twice as likely to be chosen).
    /// If the pool is empty, no magic slots are generated.
    #[serde(default)]
    pub magic_item_pool: Vec<ItemId>,

    /// How many random magic items appear in the shop at once.
    ///
    /// Defaults to 0 (no magic slots). Values above 0 activate the rotation.
    #[serde(default)]
    pub magic_slot_count: u8,

    /// Number of in-game days between magic-item slot refreshes.
    ///
    /// Defaults to 7. Values of 0 are treated as 1 at runtime.
    #[serde(default = "default_magic_refresh_days")]
    pub magic_refresh_days: u32,
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
    ///     magic_item_pool: vec![],
    ///     magic_slot_count: 0,
    ///     magic_refresh_days: 7,
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
/// - Restock tracking fields added in Phase 6 (all `#[serde(default)]` so
///   pre-Phase-6 saves load correctly; the sentinel value `0` causes an
///   immediate restock on the first `tick_restock` call).
///
/// `NpcRuntimeState` is serialised into save data so that stock levels,
/// consumed services, and restock timestamps persist across save/load cycles.
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
/// assert_eq!(state.last_restock_day, 0);
/// assert!(state.magic_slots.is_empty());
/// assert_eq!(state.last_magic_refresh_day, 0);
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

    /// The in-game day on which `stock` was last fully restocked from its
    /// template.  `0` means "never restocked this session" (forces an
    /// immediate restock on the first `tick_restock` call, which is the
    /// desired behaviour for a fresh or legacy-loaded save).
    #[serde(default)]
    pub last_restock_day: u32,

    /// Current magic-item slots: item IDs chosen at the last magic refresh.
    ///
    /// Each entry represents one unit of that magic item available for
    /// purchase. Entries are removed as items are bought (via the normal
    /// `MerchantStock` path — magic slots are injected into `stock.entries`
    /// at refresh time).
    #[serde(default)]
    pub magic_slots: Vec<ItemId>,

    /// The in-game day on which `magic_slots` was last refreshed.
    /// `0` means "never refreshed" (forces a refresh on first tick).
    #[serde(default)]
    pub last_magic_refresh_day: u32,
}

impl NpcRuntimeState {
    /// Creates a new runtime state with no stock and no consumed services
    ///
    /// All Phase-6 restock tracking fields are initialised to their sentinel
    /// values (`0` / empty) so the first `tick_restock` call triggers an
    /// immediate restock.
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
    /// assert_eq!(state.last_restock_day, 0);
    /// assert!(state.magic_slots.is_empty());
    /// assert_eq!(state.last_magic_refresh_day, 0);
    /// ```
    pub fn new(npc_id: NpcId) -> Self {
        Self {
            npc_id,
            stock: None,
            services_consumed: Vec::new(),
            last_restock_day: 0,
            magic_slots: Vec::new(),
            last_magic_refresh_day: 0,
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
    ///     magic_item_pool: vec![],
    ///     magic_slot_count: 0,
    ///     magic_refresh_days: 7,
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
    /// assert_eq!(state.last_restock_day, 0);
    /// ```
    pub fn initialize_stock_from_template(npc_id: NpcId, template: &MerchantStockTemplate) -> Self {
        Self {
            npc_id,
            stock: Some(template.to_runtime_stock()),
            services_consumed: Vec::new(),
            last_restock_day: 0,
            magic_slots: Vec::new(),
            last_magic_refresh_day: 0,
        }
    }

    /// Replenishes all regular stock entries back to the quantities defined in
    /// `template`.
    ///
    /// This method replaces each `StockEntry` quantity with the corresponding
    /// template entry quantity.  Any entry whose `item_id` is not present in the
    /// template (e.g. an item the player sold *to* the merchant) is left
    /// unchanged — the merchant keeps what they were given.
    ///
    /// If the NPC has no stock (`stock: None`) this is a no-op.
    ///
    /// # Arguments
    ///
    /// * `template` - The template this merchant was initialised from.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{
    ///     NpcRuntimeState, MerchantStockTemplate, TemplateStockEntry,
    /// };
    ///
    /// let template = MerchantStockTemplate {
    ///     id: "basic".to_string(),
    ///     entries: vec![TemplateStockEntry { item_id: 1, quantity: 5, override_price: None }],
    ///     magic_item_pool: vec![],
    ///     magic_slot_count: 0,
    ///     magic_refresh_days: 7,
    /// };
    ///
    /// let mut state = NpcRuntimeState::initialize_stock_from_template(
    ///     "merchant_bob".to_string(), &template,
    /// );
    /// // Buy all stock
    /// state.stock.as_mut().unwrap().entries[0].quantity = 0;
    ///
    /// state.restock_daily(&template);
    /// assert_eq!(state.stock.as_ref().unwrap().get_entry(1).unwrap().quantity, 5);
    /// ```
    pub fn restock_daily(&mut self, template: &MerchantStockTemplate) {
        let Some(stock) = self.stock.as_mut() else {
            return;
        };
        for tmpl_entry in &template.entries {
            match stock.get_entry_mut(tmpl_entry.item_id) {
                Some(entry) => entry.quantity = tmpl_entry.quantity,
                None => stock.entries.push(StockEntry {
                    item_id: tmpl_entry.item_id,
                    quantity: tmpl_entry.quantity,
                    override_price: tmpl_entry.override_price,
                }),
            }
        }
    }

    /// Replaces the merchant's random magic-item slots with a freshly chosen
    /// selection drawn from `template.magic_item_pool`.
    ///
    /// **Selection algorithm**
    ///
    /// 1. Remove any existing magic-slot entries from `stock.entries` (identified
    ///    by matching their `item_id` against the old `self.magic_slots` list).
    /// 2. Choose `template.magic_slot_count` item IDs at random from
    ///    `template.magic_item_pool` without replacement within a single draw
    ///    (but duplicates in the pool increase selection probability).
    /// 3. Add one `StockEntry` (quantity = 1, no price override) per chosen item
    ///    to `stock.entries`.
    /// 4. Update `self.magic_slots` with the newly chosen item IDs.
    ///
    /// A deterministic `seed` is accepted so tests can produce reproducible
    /// results without depending on OS randomness.
    ///
    /// If `stock` is `None`, `magic_slot_count` is `0`, or `magic_item_pool` is
    /// empty, this method is a no-op (no panic).
    ///
    /// # Arguments
    ///
    /// * `template` - The template defining the magic-item pool and slot count.
    /// * `seed`     - PRNG seed for reproducible selection (use `game_time.day`
    ///   combined with a stable NPC-specific value in production).
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::world::npc_runtime::{
    ///     NpcRuntimeState, MerchantStockTemplate, TemplateStockEntry,
    /// };
    /// use antares::domain::inventory::MerchantStock;
    ///
    /// let template = MerchantStockTemplate {
    ///     id: "wizard_shop".to_string(),
    ///     entries: vec![],
    ///     magic_item_pool: vec![100, 101, 102, 103, 104],
    ///     magic_slot_count: 2,
    ///     magic_refresh_days: 7,
    /// };
    ///
    /// let mut state = NpcRuntimeState::new("wizard_zara".to_string());
    /// state.stock = Some(MerchantStock::new());
    /// state.refresh_magic_slots(&template, 42);
    ///
    /// assert_eq!(state.magic_slots.len(), 2);
    /// assert_eq!(
    ///     state.stock.as_ref().unwrap().entries.len(),
    ///     2,
    ///     "one stock entry per magic slot"
    /// );
    /// ```
    pub fn refresh_magic_slots(&mut self, template: &MerchantStockTemplate, seed: u64) {
        let Some(stock) = self.stock.as_mut() else {
            return;
        };

        // Step 1 — remove stale magic-slot entries from stock
        let old_slots = std::mem::take(&mut self.magic_slots);
        for old_id in &old_slots {
            stock.entries.retain(|e| e.item_id != *old_id);
        }

        // Step 2 — pick new items from pool using a minimal LCG PRNG
        let count = template.magic_slot_count as usize;
        let pool = &template.magic_item_pool;
        if count == 0 || pool.is_empty() {
            return;
        }

        let mut rng_state = seed;
        let mut chosen: Vec<ItemId> = Vec::with_capacity(count);
        // Work from a mutable copy so we can remove chosen items (sampling without
        // replacement within a single draw).
        let mut available: Vec<ItemId> = pool.clone();

        for _ in 0..count {
            if available.is_empty() {
                break;
            }
            // LCG step: constants from Knuth TAOCP Vol.2
            rng_state = rng_state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let idx = (rng_state >> 33) as usize % available.len();
            chosen.push(available.remove(idx));
        }

        // Step 3 — add one stock entry per chosen item
        for &item_id in &chosen {
            stock.entries.push(StockEntry {
                item_id,
                quantity: 1,
                override_price: None,
            });
        }

        // Step 4 — persist the new slot list
        self.magic_slots = chosen;
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
    ///     magic_item_pool: vec![],
    ///     magic_slot_count: 0,
    ///     magic_refresh_days: 7,
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

    /// Advances the restock clock for all merchants in the store.
    ///
    /// Call this once per `advance_time` invocation, passing the **new**
    /// `GameTime` (after the time advance has been applied).
    ///
    /// For each NPC that has an active `MerchantStock`:
    ///
    /// 1. **Daily restock** — if `new_day > last_restock_day` (or
    ///    `last_restock_day == 0`), replenish all regular stock entries back
    ///    to their template quantities.
    /// 2. **Magic-slot refresh** — if the number of days elapsed since
    ///    `last_magic_refresh_day` meets or exceeds `template.magic_refresh_days`,
    ///    replace the magic slots with a freshly seeded selection.
    ///
    /// Both operations are skipped for NPCs without a `restock_template`
    /// or whose template cannot be found in `templates`.
    ///
    /// # Arguments
    ///
    /// * `new_time`  - The game time **after** the time advance.
    /// * `templates` - The loaded template database.
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::types::GameTime;
    /// use antares::domain::world::npc_runtime::{
    ///     NpcRuntimeStore, NpcRuntimeState, MerchantStockTemplate,
    ///     MerchantStockTemplateDatabase, TemplateStockEntry,
    /// };
    /// use antares::domain::world::npc::NpcDefinition;
    ///
    /// let mut templates = MerchantStockTemplateDatabase::new();
    /// templates.add(MerchantStockTemplate {
    ///     id: "daily_shop".to_string(),
    ///     entries: vec![TemplateStockEntry { item_id: 1, quantity: 3, override_price: None }],
    ///     magic_item_pool: vec![],
    ///     magic_slot_count: 0,
    ///     magic_refresh_days: 7,
    /// });
    ///
    /// let mut merchant = NpcDefinition::merchant("bob", "Bob", "bob.png");
    /// merchant.stock_template = Some("daily_shop".to_string());
    ///
    /// let mut store = NpcRuntimeStore::new();
    /// store.initialize_merchant(&merchant, &templates);
    ///
    /// // Deplete stock
    /// store.get_mut(&"bob".to_string()).unwrap()
    ///     .stock.as_mut().unwrap().entries[0].quantity = 0;
    ///
    /// // Advance to day 2
    /// let day2 = GameTime::new(2, 6, 0);
    /// store.tick_restock(&day2, &templates);
    ///
    /// // Stock should be replenished
    /// assert_eq!(
    ///     store.get(&"bob".to_string()).unwrap()
    ///         .stock.as_ref().unwrap().get_entry(1).unwrap().quantity,
    ///     3
    /// );
    /// ```
    pub fn tick_restock(
        &mut self,
        new_time: &crate::domain::types::GameTime,
        templates: &MerchantStockTemplateDatabase,
    ) {
        let new_day = new_time.day;

        // Collect NPC IDs first to avoid simultaneous mutable borrow issues.
        let npc_ids: Vec<NpcId> = self.npcs.keys().cloned().collect();

        for npc_id in npc_ids {
            // Retrieve the template ID from the stock — done in a narrow borrow scope.
            let template_id: Option<String> = self
                .npcs
                .get(&npc_id)
                .and_then(|s| s.stock.as_ref())
                .and_then(|stock| stock.restock_template.clone());

            let template_id = match template_id {
                Some(id) => id,
                None => continue,
            };

            let template = match templates.get(&template_id) {
                Some(t) => t.clone(),
                None => continue,
            };

            let state = match self.npcs.get_mut(&npc_id) {
                Some(s) => s,
                None => continue,
            };

            // --- Daily restock ---
            // Trigger on day 1 when last_restock_day == 0 (never ticked) OR whenever
            // a new day has begun.
            if new_day > state.last_restock_day {
                state.restock_daily(&template);
                state.last_restock_day = new_day;
            }

            // --- Magic-slot refresh ---
            if template.magic_slot_count > 0 && !template.magic_item_pool.is_empty() {
                let refresh_interval = template.magic_refresh_days.max(1);
                let days_since_refresh = new_day.saturating_sub(state.last_magic_refresh_day);
                // Trigger on first tick (last_magic_refresh_day == 0) or when the
                // refresh interval has elapsed.
                if state.last_magic_refresh_day == 0 || days_since_refresh >= refresh_interval {
                    // Build a deterministic seed from the current day and NPC ID.
                    let npc_hash: u64 = npc_id
                        .bytes()
                        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
                    let seed = (new_day as u64)
                        .wrapping_mul(2_654_435_761)
                        .wrapping_add(npc_hash);
                    state.refresh_magic_slots(&template, seed);
                    state.last_magic_refresh_day = new_day;
                }
            }
        }
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
/// db.add(MerchantStockTemplate {
///     id: "basic".to_string(),
///     entries: vec![],
///     magic_item_pool: vec![],
///     magic_slot_count: 0,
///     magic_refresh_days: 7,
/// });
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
    /// db.add(MerchantStockTemplate { id: "shop_a".to_string(), entries: vec![], magic_item_pool: vec![], magic_slot_count: 0, magic_refresh_days: 7 });
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
    /// db.add(MerchantStockTemplate { id: "basic".to_string(), entries: vec![], magic_item_pool: vec![], magic_slot_count: 0, magic_refresh_days: 7 });
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
    use crate::domain::inventory::MerchantStock;
    use crate::domain::types::GameTime;
    use crate::domain::world::npc::NpcDefinition;

    // ===== Helper builders =====

    /// Build a minimal template with regular stock only (no magic items).
    fn make_basic_template(id: &str, item_id: ItemId, quantity: u8) -> MerchantStockTemplate {
        MerchantStockTemplate {
            id: id.to_string(),
            entries: vec![TemplateStockEntry {
                item_id,
                quantity,
                override_price: None,
            }],
            magic_item_pool: vec![],
            magic_slot_count: 0,
            magic_refresh_days: 7,
        }
    }

    /// Build a template with a magic-item pool configured.
    fn make_magic_template(
        id: &str,
        pool: Vec<ItemId>,
        slot_count: u8,
        refresh_days: u32,
    ) -> MerchantStockTemplate {
        MerchantStockTemplate {
            id: id.to_string(),
            entries: vec![],
            magic_item_pool: pool,
            magic_slot_count: slot_count,
            magic_refresh_days: refresh_days,
        }
    }

    /// Returns an `NpcRuntimeState` with stock seeded from `template` and
    /// then the quantity of `item_id` zeroed out (simulating a buy-out).
    fn make_depleted_state(
        npc_id: &str,
        template: &MerchantStockTemplate,
        item_id: ItemId,
    ) -> NpcRuntimeState {
        let mut state =
            NpcRuntimeState::initialize_stock_from_template(npc_id.to_string(), template);
        if let Some(entry) = state.stock.as_mut().unwrap().get_entry_mut(item_id) {
            entry.quantity = 0;
        }
        state
    }

    // ===== NpcRuntimeState::new =====

    #[test]
    fn test_npc_runtime_state_new() {
        let state = NpcRuntimeState::new("guard_01".to_string());

        assert_eq!(state.npc_id, "guard_01");
        assert!(state.stock.is_none());
        assert!(state.services_consumed.is_empty());
        assert_eq!(state.last_restock_day, 0);
        assert!(state.magic_slots.is_empty());
        assert_eq!(state.last_magic_refresh_day, 0);
    }

    // ===== NpcRuntimeState::initialize_stock_from_template =====

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
            magic_item_pool: vec![],
            magic_slot_count: 0,
            magic_refresh_days: 7,
        };

        let state =
            NpcRuntimeState::initialize_stock_from_template("merchant_bob".to_string(), &template);

        assert_eq!(state.npc_id, "merchant_bob");
        assert!(state.services_consumed.is_empty());
        assert_eq!(state.last_restock_day, 0);
        assert!(state.magic_slots.is_empty());
        assert_eq!(state.last_magic_refresh_day, 0);

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
            magic_item_pool: vec![],
            magic_slot_count: 0,
            magic_refresh_days: 7,
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
        state.last_restock_day = 5;
        state.magic_slots = vec![101, 102];
        state.last_magic_refresh_day = 3;

        let serialized = ron::to_string(&state).expect("serialization must succeed");
        let deserialized: NpcRuntimeState =
            ron::from_str(&serialized).expect("deserialization must succeed");

        assert_eq!(deserialized.npc_id, state.npc_id);
        assert_eq!(deserialized.services_consumed, state.services_consumed);
        assert!(deserialized.stock.is_none());
        assert_eq!(deserialized.last_restock_day, 5);
        assert_eq!(deserialized.magic_slots, vec![101, 102]);
        assert_eq!(deserialized.last_magic_refresh_day, 3);
    }

    // ===== restock_daily =====

    #[test]
    fn test_restock_daily_restores_depleted_quantities() {
        let template = make_basic_template("shop", 1, 5);
        let mut state = make_depleted_state("merchant", &template, 1);

        assert_eq!(
            state.stock.as_ref().unwrap().get_entry(1).unwrap().quantity,
            0
        );
        state.restock_daily(&template);
        assert_eq!(
            state.stock.as_ref().unwrap().get_entry(1).unwrap().quantity,
            5
        );
    }

    #[test]
    fn test_restock_daily_preserves_non_template_items() {
        // An item sold *to* the merchant is not in the template; restock_daily must
        // not remove it.
        let template = make_basic_template("shop", 1, 3);
        let mut state =
            NpcRuntimeState::initialize_stock_from_template("vendor".to_string(), &template);

        // Inject a non-template item (player sold item_id 99 to the merchant).
        state.stock.as_mut().unwrap().entries.push(StockEntry {
            item_id: 99,
            quantity: 1,
            override_price: None,
        });

        state.restock_daily(&template);

        // Template item is restocked.
        assert_eq!(
            state.stock.as_ref().unwrap().get_entry(1).unwrap().quantity,
            3
        );
        // Non-template item is still present.
        assert!(state.stock.as_ref().unwrap().get_entry(99).is_some());
    }

    #[test]
    fn test_restock_daily_noop_on_no_stock() {
        let template = make_basic_template("shop", 1, 5);
        let mut state = NpcRuntimeState::new("priest".to_string());
        // stock is None — must not panic.
        state.restock_daily(&template);
        assert!(state.stock.is_none());
    }

    #[test]
    fn test_restock_daily_adds_missing_template_entry_to_stock() {
        // If a template entry's item_id is absent from the live stock (e.g. the
        // entry was never seeded), restock_daily should add it.
        let template = make_basic_template("shop", 7, 4);
        let mut state = NpcRuntimeState::new("vendor".to_string());
        state.stock = Some(MerchantStock::new()); // empty stock with no restock_template
                                                  // Manually set restock_template so tick_restock can look up the template.
        state.stock.as_mut().unwrap().restock_template = Some("shop".to_string());

        state.restock_daily(&template);

        assert_eq!(
            state.stock.as_ref().unwrap().get_entry(7).unwrap().quantity,
            4
        );
    }

    // ===== refresh_magic_slots =====

    #[test]
    fn test_refresh_magic_slots_populates_correct_count() {
        let template = make_magic_template("wizard", vec![100, 101, 102, 103, 104], 2, 7);
        let mut state = NpcRuntimeState::new("zara".to_string());
        state.stock = Some(MerchantStock::new());

        state.refresh_magic_slots(&template, 42);

        assert_eq!(state.magic_slots.len(), 2);
    }

    #[test]
    fn test_refresh_magic_slots_entries_added_to_stock() {
        let template = make_magic_template("wizard", vec![100, 101, 102, 103, 104], 2, 7);
        let mut state = NpcRuntimeState::new("zara".to_string());
        state.stock = Some(MerchantStock::new());

        state.refresh_magic_slots(&template, 42);

        let entries = &state.stock.as_ref().unwrap().entries;
        assert_eq!(entries.len(), 2, "one stock entry per magic slot");
        for entry in entries {
            assert_eq!(entry.quantity, 1);
            assert_eq!(entry.override_price, None);
        }
    }

    #[test]
    fn test_refresh_magic_slots_removes_old_slots() {
        let template = make_magic_template("wizard", vec![100, 101, 102, 103, 104], 2, 7);
        let mut state = NpcRuntimeState::new("zara".to_string());
        state.stock = Some(MerchantStock::new());

        state.refresh_magic_slots(&template, 1);
        let first_slots = state.magic_slots.clone();
        let first_entry_count = state.stock.as_ref().unwrap().entries.len();
        assert_eq!(first_entry_count, 2);

        // Second refresh — old entries must be removed before new ones are added.
        state.refresh_magic_slots(&template, 2);
        let second_entry_count = state.stock.as_ref().unwrap().entries.len();
        assert_eq!(
            second_entry_count, 2,
            "entry count must stay at slot_count after second refresh"
        );

        // Chosen items should differ between seeds (they may coincidentally be the
        // same — but the old entries must have been cleaned up regardless).
        let _ = first_slots; // suppress unused warning
    }

    #[test]
    fn test_refresh_magic_slots_noop_when_pool_empty() {
        let template = make_magic_template("wizard", vec![], 2, 7);
        let mut state = NpcRuntimeState::new("zara".to_string());
        state.stock = Some(MerchantStock::new());

        // Must not panic.
        state.refresh_magic_slots(&template, 42);

        assert!(state.magic_slots.is_empty());
        assert!(state.stock.as_ref().unwrap().entries.is_empty());
    }

    #[test]
    fn test_refresh_magic_slots_capped_by_pool_size() {
        // magic_slot_count (5) exceeds pool size (3) — only 3 items should be chosen.
        let template = make_magic_template("wizard", vec![100, 101, 102], 5, 7);
        let mut state = NpcRuntimeState::new("zara".to_string());
        state.stock = Some(MerchantStock::new());

        state.refresh_magic_slots(&template, 42);

        assert_eq!(state.magic_slots.len(), 3);
        assert_eq!(state.stock.as_ref().unwrap().entries.len(), 3);
    }

    #[test]
    fn test_refresh_magic_slots_reproducible_with_same_seed() {
        let template = make_magic_template("wizard", vec![100, 101, 102, 103, 104], 3, 7);

        let mut state_a = NpcRuntimeState::new("zara".to_string());
        state_a.stock = Some(MerchantStock::new());
        state_a.refresh_magic_slots(&template, 99);

        let mut state_b = NpcRuntimeState::new("zara".to_string());
        state_b.stock = Some(MerchantStock::new());
        state_b.refresh_magic_slots(&template, 99);

        assert_eq!(state_a.magic_slots, state_b.magic_slots);
    }

    #[test]
    fn test_refresh_magic_slots_different_seed_different_result() {
        // With a pool of 5 items and 3 slots, different seeds should produce
        // at least one different selection over a range of seed values.
        let template = make_magic_template("wizard", vec![100, 101, 102, 103, 104], 3, 7);

        let mut results: Vec<Vec<ItemId>> = Vec::new();
        for seed in 0u64..20 {
            let mut state = NpcRuntimeState::new("zara".to_string());
            state.stock = Some(MerchantStock::new());
            state.refresh_magic_slots(&template, seed);
            results.push(state.magic_slots);
        }

        // At least two distinct selections must exist across 20 different seeds.
        let first = &results[0];
        let has_different = results.iter().any(|r| r != first);
        assert!(
            has_different,
            "expected at least two distinct slot selections across 20 seeds"
        );
    }

    #[test]
    fn test_refresh_magic_slots_noop_when_stock_is_none() {
        let template = make_magic_template("wizard", vec![100, 101, 102], 2, 7);
        let mut state = NpcRuntimeState::new("zara".to_string());
        // stock is None — must not panic.
        state.refresh_magic_slots(&template, 42);
        assert!(state.magic_slots.is_empty());
    }

    // ===== tick_restock =====

    fn make_store_with_template(
        npc_id: &str,
        template: &MerchantStockTemplate,
        db: &MerchantStockTemplateDatabase,
    ) -> NpcRuntimeStore {
        let mut merchant = NpcDefinition::merchant(npc_id, "Merchant", "merchant.png");
        merchant.stock_template = Some(template.id.clone());
        let mut store = NpcRuntimeStore::new();
        store.initialize_merchant(&merchant, db);
        store
    }

    #[test]
    fn test_tick_restock_initial_zero_day_forces_restock() {
        // last_restock_day starts at 0; any day > 0 triggers restock.
        let template = make_basic_template("shop", 1, 5);
        let mut db = MerchantStockTemplateDatabase::new();
        db.add(template.clone());

        let mut store = make_store_with_template("merchant", &template, &db);
        // Deplete stock.
        store
            .get_mut(&"merchant".to_string())
            .unwrap()
            .stock
            .as_mut()
            .unwrap()
            .entries[0]
            .quantity = 0;

        let day1 = GameTime::new(1, 0, 0);
        store.tick_restock(&day1, &db);

        assert_eq!(
            store
                .get(&"merchant".to_string())
                .unwrap()
                .stock
                .as_ref()
                .unwrap()
                .get_entry(1)
                .unwrap()
                .quantity,
            5
        );
    }

    #[test]
    fn test_tick_restock_triggers_on_new_day() {
        let template = make_basic_template("shop", 1, 3);
        let mut db = MerchantStockTemplateDatabase::new();
        db.add(template.clone());

        let mut store = make_store_with_template("merchant", &template, &db);

        // Simulate: ticked on day 1 (sets last_restock_day = 1), deplete, tick on day 2.
        let day1 = GameTime::new(1, 0, 0);
        store.tick_restock(&day1, &db);
        store
            .get_mut(&"merchant".to_string())
            .unwrap()
            .stock
            .as_mut()
            .unwrap()
            .get_entry_mut(1)
            .unwrap()
            .quantity = 0;

        let day2 = GameTime::new(2, 6, 0);
        store.tick_restock(&day2, &db);

        assert_eq!(
            store
                .get(&"merchant".to_string())
                .unwrap()
                .stock
                .as_ref()
                .unwrap()
                .get_entry(1)
                .unwrap()
                .quantity,
            3
        );
    }

    #[test]
    fn test_tick_restock_no_restock_same_day() {
        let template = make_basic_template("shop", 1, 5);
        let mut db = MerchantStockTemplateDatabase::new();
        db.add(template.clone());

        let mut store = make_store_with_template("merchant", &template, &db);

        // Tick on day 1 — sets last_restock_day to 1 and restocks.
        let day1 = GameTime::new(1, 0, 0);
        store.tick_restock(&day1, &db);

        // Now deplete and tick again on the same day — should NOT restock.
        store
            .get_mut(&"merchant".to_string())
            .unwrap()
            .stock
            .as_mut()
            .unwrap()
            .get_entry_mut(1)
            .unwrap()
            .quantity = 0;

        store.tick_restock(&day1, &db);

        assert_eq!(
            store
                .get(&"merchant".to_string())
                .unwrap()
                .stock
                .as_ref()
                .unwrap()
                .get_entry(1)
                .unwrap()
                .quantity,
            0,
            "same-day tick must not restock"
        );
    }

    #[test]
    fn test_tick_restock_updates_last_restock_day() {
        let template = make_basic_template("shop", 1, 5);
        let mut db = MerchantStockTemplateDatabase::new();
        db.add(template.clone());

        let mut store = make_store_with_template("merchant", &template, &db);

        let day7 = GameTime::new(7, 12, 0);
        store.tick_restock(&day7, &db);

        assert_eq!(
            store.get(&"merchant".to_string()).unwrap().last_restock_day,
            7
        );
    }

    #[test]
    fn test_tick_restock_magic_refresh_on_interval() {
        // Template with magic_refresh_days = 7; tick at day 8 should trigger refresh.
        let template = make_magic_template("wizard_shop", vec![100, 101, 102, 103, 104], 2, 7);
        let mut db = MerchantStockTemplateDatabase::new();
        db.add(template.clone());

        let mut merchant = NpcDefinition::merchant("wizard", "Wizard", "wizard.png");
        merchant.stock_template = Some("wizard_shop".to_string());
        let mut store = NpcRuntimeStore::new();
        // Initialize manually so the stock.restock_template is set correctly.
        let mut state = NpcRuntimeState::new("wizard".to_string());
        state.stock = Some(MerchantStock {
            entries: vec![],
            restock_template: Some("wizard_shop".to_string()),
        });
        state.last_magic_refresh_day = 1; // pretend we refreshed on day 1
        store.insert(state);

        let day8 = GameTime::new(8, 0, 0);
        store.tick_restock(&day8, &db);

        let state = store.get(&"wizard".to_string()).unwrap();
        assert_eq!(state.magic_slots.len(), 2);
        assert_eq!(state.last_magic_refresh_day, 8);
    }

    #[test]
    fn test_tick_restock_magic_no_refresh_before_interval() {
        let template = make_magic_template("wizard_shop", vec![100, 101, 102, 103, 104], 2, 7);
        let mut db = MerchantStockTemplateDatabase::new();
        db.add(template.clone());

        // Set up a state where the last magic refresh was on day 5, interval is 7.
        let mut state = NpcRuntimeState::new("wizard".to_string());
        state.stock = Some(MerchantStock {
            entries: vec![],
            restock_template: Some("wizard_shop".to_string()),
        });
        state.last_magic_refresh_day = 5;
        // Manually pre-populate magic slots so we can verify they are unchanged.
        state.magic_slots = vec![100, 101];
        state
            .stock
            .as_mut()
            .unwrap()
            .entries
            .extend(state.magic_slots.iter().map(|&id| StockEntry {
                item_id: id,
                quantity: 1,
                override_price: None,
            }));

        let mut store = NpcRuntimeStore::new();
        store.insert(state);

        // Day 11 is only 6 days after day 5 — should NOT trigger a refresh.
        let day11 = GameTime::new(11, 0, 0);
        store.tick_restock(&day11, &db);

        let state = store.get(&"wizard".to_string()).unwrap();
        // Magic slots unchanged.
        assert_eq!(state.magic_slots, vec![100, 101]);
        assert_eq!(state.last_magic_refresh_day, 5);
    }

    #[test]
    fn test_tick_restock_skips_npc_without_template() {
        let db = MerchantStockTemplateDatabase::new(); // empty

        let mut store = NpcRuntimeStore::new();
        // NPC with stock but no restock_template.
        let mut state = NpcRuntimeState::new("ghost".to_string());
        state.stock = Some(MerchantStock::new()); // restock_template is None
        store.insert(state);

        // Must not panic and must not alter anything.
        let day3 = GameTime::new(3, 0, 0);
        store.tick_restock(&day3, &db);

        let state = store.get(&"ghost".to_string()).unwrap();
        assert_eq!(state.last_restock_day, 0);
        assert!(state.magic_slots.is_empty());
    }

    #[test]
    fn test_tick_restock_skips_npc_with_no_stock() {
        let template = make_basic_template("shop", 1, 5);
        let mut db = MerchantStockTemplateDatabase::new();
        db.add(template);

        let mut store = NpcRuntimeStore::new();
        // NPC with no stock at all (priest/non-merchant).
        store.insert(NpcRuntimeState::new("priest".to_string()));

        let day2 = GameTime::new(2, 0, 0);
        store.tick_restock(&day2, &db);

        let state = store.get(&"priest".to_string()).unwrap();
        assert_eq!(state.last_restock_day, 0);
    }

    // ===== NpcRuntimeStore =====

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
            magic_item_pool: vec![],
            magic_slot_count: 0,
            magic_refresh_days: 7,
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

    // ===== MerchantStockTemplateDatabase =====

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
            magic_item_pool: vec![],
            magic_slot_count: 0,
            magic_refresh_days: 7,
        });

        assert!(db.get("general_goods").is_some());
        assert!(db.get("other").is_none());
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn test_merchant_stock_template_database_load_from_string_success() {
        // The #[serde(default)] fields mean old-format RON (without magic fields)
        // must still parse successfully.
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
        // Phase-6 defaults applied.
        assert_eq!(shop_a.magic_slot_count, 0);
        assert!(shop_a.magic_item_pool.is_empty());
        assert_eq!(shop_a.magic_refresh_days, 7);

        let shop_b = db.get("shop_b").expect("shop_b must exist");
        assert!(shop_b.entries.is_empty());
    }

    #[test]
    fn test_merchant_stock_template_database_load_from_string_with_magic_fields() {
        let ron = r#"[
            (id: "magic_shop", entries: [],
             magic_item_pool: [100, 101, 102],
             magic_slot_count: 2,
             magic_refresh_days: 14),
        ]"#;

        let db =
            MerchantStockTemplateDatabase::load_from_string(ron).expect("must parse successfully");

        let tmpl = db.get("magic_shop").unwrap();
        assert_eq!(tmpl.magic_item_pool, vec![100, 101, 102]);
        assert_eq!(tmpl.magic_slot_count, 2);
        assert_eq!(tmpl.magic_refresh_days, 14);
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
            magic_item_pool: vec![],
            magic_slot_count: 0,
            magic_refresh_days: 7,
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
        state.last_restock_day = 3;
        state.magic_slots = vec![100, 101];
        state.last_magic_refresh_day = 2;
        store.insert(state);

        let serialized = ron::to_string(&store).expect("serialization must succeed");
        let deserialized: NpcRuntimeStore =
            ron::from_str(&serialized).expect("deserialization must succeed");

        let recovered = deserialized.get(&"merchant".to_string()).unwrap();
        assert_eq!(recovered.services_consumed, vec!["heal_all".to_string()]);
        assert_eq!(recovered.last_restock_day, 3);
        assert_eq!(recovered.magic_slots, vec![100, 101]);
        assert_eq!(recovered.last_magic_refresh_day, 2);
    }
}
