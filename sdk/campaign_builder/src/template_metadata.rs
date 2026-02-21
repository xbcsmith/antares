// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Template metadata system for creature templates
//!
//! This module provides metadata structures for categorizing, searching, and
//! filtering creature templates. It enables the template browser to present
//! rich information about templates before instantiation.
//!
//! # Architecture
//!
//! The metadata system consists of:
//! - `TemplateMetadata`: Rich information about a template
//! - `TemplateCategory`: Classification of template types
//! - `Complexity`: Difficulty level for editing
//! - `TemplateRegistry`: Central registry with search/filter capabilities
//!
//! # Examples
//!
//! ```
//! use campaign_builder::template_metadata::{
//!     TemplateMetadata, TemplateCategory, Complexity, TemplateRegistry
//! };
//! use antares::domain::visual::CreatureDefinition;
//!
//! let metadata = TemplateMetadata {
//!     id: "humanoid_knight".to_string(),
//!     name: "Knight".to_string(),
//!     category: TemplateCategory::Humanoid,
//!     complexity: Complexity::Beginner,
//!     mesh_count: 6,
//!     description: "Basic humanoid warrior".to_string(),
//!     tags: vec!["warrior".to_string(), "armor".to_string()],
//! };
//!
//! let mut registry = TemplateRegistry::new();
//! // registry.register(metadata, creature_def, generator_fn);
//! ```

use antares::domain::visual::CreatureDefinition;
use std::collections::HashMap;

/// Type alias for template ID
pub type TemplateId = String;

/// Metadata describing a creature template
///
/// Contains rich information for browsing and selecting templates,
/// including categorization, complexity, and searchable tags.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateMetadata {
    /// Unique identifier for this template
    pub id: TemplateId,

    /// Display name of the template
    pub name: String,

    /// Category classification
    pub category: TemplateCategory,

    /// Complexity/difficulty level
    pub complexity: Complexity,

    /// Number of meshes in the template
    pub mesh_count: usize,

    /// Human-readable description
    pub description: String,

    /// Searchable tags for filtering
    pub tags: Vec<String>,
}

/// Categories for creature templates
///
/// Used for filtering and organizing templates in the browser UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateCategory {
    /// Humanoid bipeds (knights, mages, etc.)
    Humanoid,

    /// Natural creatures (wolves, bears, etc.)
    Creature,

    /// Undead monsters (skeletons, zombies, etc.)
    Undead,

    /// Mechanical/robotic entities
    Robot,

    /// Basic geometric primitives
    Primitive,
}

impl TemplateCategory {
    /// Returns all available categories
    pub fn all() -> Vec<Self> {
        vec![
            Self::Humanoid,
            Self::Creature,
            Self::Undead,
            Self::Robot,
            Self::Primitive,
        ]
    }

    /// Returns the display name of the category
    pub fn name(&self) -> &str {
        match self {
            Self::Humanoid => "Humanoid",
            Self::Creature => "Creature",
            Self::Undead => "Undead",
            Self::Robot => "Robot",
            Self::Primitive => "Primitive",
        }
    }

    /// Returns a description of the category
    pub fn description(&self) -> &str {
        match self {
            Self::Humanoid => "Bipedal humanoid creatures like knights, mages, and warriors",
            Self::Creature => "Natural creatures like wolves, bears, and birds",
            Self::Undead => "Undead monsters like skeletons, zombies, and ghosts",
            Self::Robot => "Mechanical or robotic entities",
            Self::Primitive => "Basic geometric shapes and primitives",
        }
    }
}

/// Complexity/difficulty level for templates
///
/// Indicates how complex the template is to edit and customize.
/// Used to help users find appropriate starting points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Complexity {
    /// Simple templates with few meshes, ideal for learning
    Beginner,

    /// Moderate complexity with multiple parts
    Intermediate,

    /// Complex templates with many parts and details
    Advanced,

    /// Highly complex templates requiring expertise
    Expert,
}

impl Complexity {
    /// Returns all complexity levels
    pub fn all() -> Vec<Self> {
        vec![
            Self::Beginner,
            Self::Intermediate,
            Self::Advanced,
            Self::Expert,
        ]
    }

    /// Returns the display name of the complexity level
    pub fn name(&self) -> &str {
        match self {
            Self::Beginner => "Beginner",
            Self::Intermediate => "Intermediate",
            Self::Advanced => "Advanced",
            Self::Expert => "Expert",
        }
    }

    /// Returns a description of the complexity level
    pub fn description(&self) -> &str {
        match self {
            Self::Beginner => "Simple templates with 1-5 meshes, great for learning",
            Self::Intermediate => "Moderate templates with 6-10 meshes",
            Self::Advanced => "Complex templates with 11-20 meshes",
            Self::Expert => "Highly complex templates with 20+ meshes",
        }
    }

    /// Returns the recommended complexity based on mesh count
    pub fn from_mesh_count(count: usize) -> Self {
        match count {
            0..=5 => Self::Beginner,
            6..=10 => Self::Intermediate,
            11..=20 => Self::Advanced,
            _ => Self::Expert,
        }
    }
}

/// Generator function type for templates
pub type TemplateGenerator = fn(&str, u32) -> CreatureDefinition;

/// Entry in the template registry
///
/// Combines metadata with the actual creature definition and generator function.
#[derive(Clone)]
pub struct TemplateEntry {
    /// Template metadata
    pub metadata: TemplateMetadata,

    /// Example creature instance
    pub example_creature: CreatureDefinition,

    /// Generator function for creating instances
    pub generator: TemplateGenerator,
}

/// Central registry for creature templates
///
/// Manages all available templates with search, filter, and generation capabilities.
pub struct TemplateRegistry {
    /// Registered templates by ID
    templates: HashMap<TemplateId, TemplateEntry>,
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRegistry {
    /// Creates a new empty template registry
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Registers a new template
    ///
    /// # Arguments
    ///
    /// * `metadata` - Template metadata
    /// * `example_creature` - Example creature instance
    /// * `generator` - Generator function
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::template_metadata::{
    ///     TemplateMetadata, TemplateCategory, Complexity, TemplateRegistry
    /// };
    /// use antares::domain::visual::CreatureDefinition;
    ///
    /// fn my_generator(name: &str, id: u32) -> CreatureDefinition {
    ///     CreatureDefinition {
    ///         id,
    ///         name: name.to_string(),
    ///         meshes: vec![],
    ///         mesh_transforms: vec![],
    ///         scale: 1.0,
    ///         color_tint: None,
    ///     }
    /// }
    ///
    /// let metadata = TemplateMetadata {
    ///     id: "test".to_string(),
    ///     name: "Test".to_string(),
    ///     category: TemplateCategory::Primitive,
    ///     complexity: Complexity::Beginner,
    ///     mesh_count: 1,
    ///     description: "Test template".to_string(),
    ///     tags: vec![],
    /// };
    ///
    /// let example = my_generator("Example", 0);
    /// let mut registry = TemplateRegistry::new();
    /// registry.register(metadata, example, my_generator);
    /// ```
    pub fn register(
        &mut self,
        metadata: TemplateMetadata,
        example_creature: CreatureDefinition,
        generator: TemplateGenerator,
    ) {
        let id = metadata.id.clone();
        self.templates.insert(
            id,
            TemplateEntry {
                metadata,
                example_creature,
                generator,
            },
        );
    }

    /// Returns all registered templates
    ///
    /// # Returns
    ///
    /// Vector of all template entries
    pub fn all_templates(&self) -> Vec<&TemplateEntry> {
        self.templates.values().collect()
    }

    /// Returns templates filtered by category
    ///
    /// # Arguments
    ///
    /// * `category` - The category to filter by
    ///
    /// # Returns
    ///
    /// Vector of templates in the specified category
    pub fn by_category(&self, category: TemplateCategory) -> Vec<&TemplateEntry> {
        self.templates
            .values()
            .filter(|entry| entry.metadata.category == category)
            .collect()
    }

    /// Returns templates filtered by complexity
    ///
    /// # Arguments
    ///
    /// * `complexity` - The complexity level to filter by
    ///
    /// # Returns
    ///
    /// Vector of templates at the specified complexity level
    pub fn by_complexity(&self, complexity: Complexity) -> Vec<&TemplateEntry> {
        self.templates
            .values()
            .filter(|entry| entry.metadata.complexity == complexity)
            .collect()
    }

    /// Searches templates by name or tags
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string (case-insensitive)
    ///
    /// # Returns
    ///
    /// Vector of templates matching the search query
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::template_metadata::TemplateRegistry;
    ///
    /// let registry = TemplateRegistry::new();
    /// let results = registry.search("knight");
    /// // Returns templates with "knight" in name or tags
    /// ```
    pub fn search(&self, query: &str) -> Vec<&TemplateEntry> {
        let query_lower = query.to_lowercase();

        self.templates
            .values()
            .filter(|entry| {
                // Search in name
                entry
                    .metadata
                    .name
                    .to_lowercase()
                    .contains(&query_lower)
                    // Search in description
                    || entry
                        .metadata
                        .description
                        .to_lowercase()
                        .contains(&query_lower)
                    // Search in tags
                    || entry
                        .metadata
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Gets a specific template by ID
    ///
    /// # Arguments
    ///
    /// * `id` - Template ID
    ///
    /// # Returns
    ///
    /// Optional reference to the template entry
    pub fn get(&self, id: &str) -> Option<&TemplateEntry> {
        self.templates.get(id)
    }

    /// Generates a new creature from a template
    ///
    /// # Arguments
    ///
    /// * `template_id` - ID of the template to use
    /// * `name` - Name for the new creature
    /// * `id` - ID for the new creature
    ///
    /// # Returns
    ///
    /// Result containing the generated creature or an error message
    ///
    /// # Examples
    ///
    /// ```
    /// use campaign_builder::template_metadata::TemplateRegistry;
    ///
    /// let registry = TemplateRegistry::new();
    /// // let creature = registry.generate("humanoid_knight", "Sir Lancelot", 42);
    /// ```
    pub fn generate(
        &self,
        template_id: &str,
        name: &str,
        id: u32,
    ) -> Result<CreatureDefinition, String> {
        let entry = self
            .templates
            .get(template_id)
            .ok_or_else(|| format!("Template '{}' not found", template_id))?;

        Ok((entry.generator)(name, id))
    }

    /// Returns the number of registered templates
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Returns true if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Returns all unique categories present in registered templates
    pub fn available_categories(&self) -> Vec<TemplateCategory> {
        let mut categories: Vec<_> = self
            .templates
            .values()
            .map(|entry| entry.metadata.category)
            .collect();
        categories.sort_by_key(|c| format!("{:?}", c));
        categories.dedup();
        categories
    }

    /// Returns all unique tags present in registered templates
    pub fn available_tags(&self) -> Vec<String> {
        let mut tags: Vec<_> = self
            .templates
            .values()
            .flat_map(|entry| entry.metadata.tags.iter().cloned())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_creature(name: &str, id: u32, mesh_count: usize) -> CreatureDefinition {
        use antares::domain::visual::{MeshDefinition, MeshTransform};

        let meshes = (0..mesh_count)
            .map(|_| MeshDefinition {
                name: None,
                vertices: vec![[0.0, 0.0, 0.0]],
                indices: vec![0],
                normals: None,
                uvs: None,
                color: [1.0, 1.0, 1.0, 1.0],
                lod_levels: None,
                lod_distances: None,
                material: None,
                texture_path: None,
            })
            .collect();

        let transforms = (0..mesh_count).map(|_| MeshTransform::default()).collect();

        CreatureDefinition {
            id,
            name: name.to_string(),
            meshes,
            mesh_transforms: transforms,
            scale: 1.0,
            color_tint: None,
        }
    }

    fn test_generator(name: &str, id: u32) -> CreatureDefinition {
        create_test_creature(name, id, 3)
    }

    #[test]
    fn test_template_metadata_creation() {
        let metadata = TemplateMetadata {
            id: "test_template".to_string(),
            name: "Test Template".to_string(),
            category: TemplateCategory::Humanoid,
            complexity: Complexity::Beginner,
            mesh_count: 5,
            description: "A test template".to_string(),
            tags: vec!["test".to_string(), "example".to_string()],
        };

        assert_eq!(metadata.id, "test_template");
        assert_eq!(metadata.name, "Test Template");
        assert_eq!(metadata.category, TemplateCategory::Humanoid);
        assert_eq!(metadata.complexity, Complexity::Beginner);
        assert_eq!(metadata.mesh_count, 5);
    }

    #[test]
    fn test_template_category_all() {
        let categories = TemplateCategory::all();
        assert_eq!(categories.len(), 5);
        assert!(categories.contains(&TemplateCategory::Humanoid));
        assert!(categories.contains(&TemplateCategory::Creature));
        assert!(categories.contains(&TemplateCategory::Undead));
        assert!(categories.contains(&TemplateCategory::Robot));
        assert!(categories.contains(&TemplateCategory::Primitive));
    }

    #[test]
    fn test_template_category_names() {
        assert_eq!(TemplateCategory::Humanoid.name(), "Humanoid");
        assert_eq!(TemplateCategory::Creature.name(), "Creature");
        assert_eq!(TemplateCategory::Undead.name(), "Undead");
        assert_eq!(TemplateCategory::Robot.name(), "Robot");
        assert_eq!(TemplateCategory::Primitive.name(), "Primitive");
    }

    #[test]
    fn test_complexity_all() {
        let levels = Complexity::all();
        assert_eq!(levels.len(), 4);
        assert!(levels.contains(&Complexity::Beginner));
        assert!(levels.contains(&Complexity::Intermediate));
        assert!(levels.contains(&Complexity::Advanced));
        assert!(levels.contains(&Complexity::Expert));
    }

    #[test]
    fn test_complexity_names() {
        assert_eq!(Complexity::Beginner.name(), "Beginner");
        assert_eq!(Complexity::Intermediate.name(), "Intermediate");
        assert_eq!(Complexity::Advanced.name(), "Advanced");
        assert_eq!(Complexity::Expert.name(), "Expert");
    }

    #[test]
    fn test_complexity_from_mesh_count() {
        assert_eq!(Complexity::from_mesh_count(3), Complexity::Beginner);
        assert_eq!(Complexity::from_mesh_count(7), Complexity::Intermediate);
        assert_eq!(Complexity::from_mesh_count(15), Complexity::Advanced);
        assert_eq!(Complexity::from_mesh_count(25), Complexity::Expert);
    }

    #[test]
    fn test_complexity_ordering() {
        assert!(Complexity::Beginner < Complexity::Intermediate);
        assert!(Complexity::Intermediate < Complexity::Advanced);
        assert!(Complexity::Advanced < Complexity::Expert);
    }

    #[test]
    fn test_template_registry_new() {
        let registry = TemplateRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_template_registry_register() {
        let mut registry = TemplateRegistry::new();

        let metadata = TemplateMetadata {
            id: "test".to_string(),
            name: "Test".to_string(),
            category: TemplateCategory::Primitive,
            complexity: Complexity::Beginner,
            mesh_count: 1,
            description: "Test".to_string(),
            tags: vec![],
        };

        let creature = test_generator("Example", 0);
        registry.register(metadata, creature, test_generator);

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_template_registry_all_templates() {
        let mut registry = TemplateRegistry::new();

        for i in 0..3 {
            let metadata = TemplateMetadata {
                id: format!("test_{}", i),
                name: format!("Test {}", i),
                category: TemplateCategory::Primitive,
                complexity: Complexity::Beginner,
                mesh_count: 1,
                description: "Test".to_string(),
                tags: vec![],
            };
            let creature = test_generator("Example", 0);
            registry.register(metadata, creature, test_generator);
        }

        let all = registry.all_templates();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_template_registry_by_category() {
        let mut registry = TemplateRegistry::new();

        let categories = [
            TemplateCategory::Humanoid,
            TemplateCategory::Creature,
            TemplateCategory::Humanoid,
        ];

        for (i, category) in categories.iter().enumerate() {
            let metadata = TemplateMetadata {
                id: format!("test_{}", i),
                name: format!("Test {}", i),
                category: *category,
                complexity: Complexity::Beginner,
                mesh_count: 1,
                description: "Test".to_string(),
                tags: vec![],
            };
            let creature = test_generator("Example", 0);
            registry.register(metadata, creature, test_generator);
        }

        let humanoids = registry.by_category(TemplateCategory::Humanoid);
        assert_eq!(humanoids.len(), 2);

        let creatures = registry.by_category(TemplateCategory::Creature);
        assert_eq!(creatures.len(), 1);
    }

    #[test]
    fn test_template_registry_by_complexity() {
        let mut registry = TemplateRegistry::new();

        let complexities = [
            Complexity::Beginner,
            Complexity::Advanced,
            Complexity::Beginner,
        ];

        for (i, complexity) in complexities.iter().enumerate() {
            let metadata = TemplateMetadata {
                id: format!("test_{}", i),
                name: format!("Test {}", i),
                category: TemplateCategory::Primitive,
                complexity: *complexity,
                mesh_count: 1,
                description: "Test".to_string(),
                tags: vec![],
            };
            let creature = test_generator("Example", 0);
            registry.register(metadata, creature, test_generator);
        }

        let beginners = registry.by_complexity(Complexity::Beginner);
        assert_eq!(beginners.len(), 2);

        let advanced = registry.by_complexity(Complexity::Advanced);
        assert_eq!(advanced.len(), 1);
    }

    #[test]
    fn test_template_registry_search() {
        let mut registry = TemplateRegistry::new();

        let templates = [
            ("knight", "A brave knight", vec!["warrior", "armor"]),
            ("mage", "A powerful mage", vec!["magic", "staff"]),
            ("warrior", "A strong warrior", vec!["fighter", "sword"]),
        ];

        for (i, (name, desc, tags)) in templates.iter().enumerate() {
            let metadata = TemplateMetadata {
                id: format!("test_{}", i),
                name: name.to_string(),
                category: TemplateCategory::Humanoid,
                complexity: Complexity::Beginner,
                mesh_count: 1,
                description: desc.to_string(),
                tags: tags.iter().map(|s| s.to_string()).collect(),
            };
            let creature = test_generator("Example", 0);
            registry.register(metadata, creature, test_generator);
        }

        // Search by name
        let results = registry.search("knight");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.name, "knight");

        // Search by description
        let results = registry.search("powerful");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.name, "mage");

        // Search by tag
        let results = registry.search("armor");
        assert_eq!(results.len(), 1);

        // Search case-insensitive
        let results = registry.search("WARRIOR");
        assert_eq!(results.len(), 2); // "warrior" name and "warrior" tag in knight
    }

    #[test]
    fn test_template_registry_get() {
        let mut registry = TemplateRegistry::new();

        let metadata = TemplateMetadata {
            id: "my_template".to_string(),
            name: "My Template".to_string(),
            category: TemplateCategory::Primitive,
            complexity: Complexity::Beginner,
            mesh_count: 1,
            description: "Test".to_string(),
            tags: vec![],
        };
        let creature = test_generator("Example", 0);
        registry.register(metadata, creature, test_generator);

        assert!(registry.get("my_template").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_template_registry_generate() {
        let mut registry = TemplateRegistry::new();

        let metadata = TemplateMetadata {
            id: "test_gen".to_string(),
            name: "Test".to_string(),
            category: TemplateCategory::Primitive,
            complexity: Complexity::Beginner,
            mesh_count: 3,
            description: "Test".to_string(),
            tags: vec![],
        };
        let creature = test_generator("Example", 0);
        registry.register(metadata, creature, test_generator);

        let result = registry.generate("test_gen", "My Creature", 42);
        assert!(result.is_ok());

        let creature = result.unwrap();
        assert_eq!(creature.name, "My Creature");
        assert_eq!(creature.id, 42);
        assert_eq!(creature.meshes.len(), 3);

        // Test nonexistent template
        let result = registry.generate("nonexistent", "Test", 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_template_registry_available_categories() {
        let mut registry = TemplateRegistry::new();

        let categories = [
            TemplateCategory::Humanoid,
            TemplateCategory::Creature,
            TemplateCategory::Humanoid, // Duplicate
        ];

        for (i, category) in categories.iter().enumerate() {
            let metadata = TemplateMetadata {
                id: format!("test_{}", i),
                name: format!("Test {}", i),
                category: *category,
                complexity: Complexity::Beginner,
                mesh_count: 1,
                description: "Test".to_string(),
                tags: vec![],
            };
            let creature = test_generator("Example", 0);
            registry.register(metadata, creature, test_generator);
        }

        let available = registry.available_categories();
        assert_eq!(available.len(), 2); // Duplicates removed
        assert!(available.contains(&TemplateCategory::Humanoid));
        assert!(available.contains(&TemplateCategory::Creature));
    }

    #[test]
    fn test_template_registry_available_tags() {
        let mut registry = TemplateRegistry::new();

        let tag_sets = [
            vec!["warrior", "armor"],
            vec!["magic", "staff"],
            vec!["warrior", "sword"], // "warrior" duplicate
        ];

        for (i, tags) in tag_sets.iter().enumerate() {
            let metadata = TemplateMetadata {
                id: format!("test_{}", i),
                name: format!("Test {}", i),
                category: TemplateCategory::Humanoid,
                complexity: Complexity::Beginner,
                mesh_count: 1,
                description: "Test".to_string(),
                tags: tags.iter().map(|s| s.to_string()).collect(),
            };
            let creature = test_generator("Example", 0);
            registry.register(metadata, creature, test_generator);
        }

        let available = registry.available_tags();
        assert_eq!(available.len(), 5); // Unique tags only
        assert!(available.contains(&"warrior".to_string()));
        assert!(available.contains(&"armor".to_string()));
        assert!(available.contains(&"magic".to_string()));
        assert!(available.contains(&"staff".to_string()));
        assert!(available.contains(&"sword".to_string()));
    }

    #[test]
    fn test_template_category_description() {
        assert!(!TemplateCategory::Humanoid.description().is_empty());
        assert!(!TemplateCategory::Creature.description().is_empty());
    }

    #[test]
    fn test_complexity_description() {
        assert!(!Complexity::Beginner.description().is_empty());
        assert!(!Complexity::Expert.description().is_empty());
    }
}
