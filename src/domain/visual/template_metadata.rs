// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Template metadata system for creature templates
//!
//! Provides categorization, tagging, and difficulty ratings for creature templates
//! to help users browse and discover templates in the template gallery.

use serde::{Deserialize, Serialize};

/// Metadata for a creature template
///
/// Each template has companion metadata that describes its category,
/// difficulty, and other descriptive information for browsing and discovery.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::template_metadata::{TemplateMetadata, TemplateCategory, Difficulty};
///
/// let metadata = TemplateMetadata {
///     category: TemplateCategory::Dragon,
///     tags: vec!["flying".to_string(), "fire".to_string()],
///     difficulty: Difficulty::Advanced,
///     author: "Game Designer".to_string(),
///     description: "A fearsome dragon with wings and tail".to_string(),
///     thumbnail_path: Some("assets/thumbnails/dragon.png".to_string()),
/// };
///
/// assert_eq!(metadata.category, TemplateCategory::Dragon);
/// assert_eq!(metadata.difficulty, Difficulty::Advanced);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Category of the template (humanoid, quadruped, etc.)
    pub category: TemplateCategory,

    /// Searchable tags for the template
    pub tags: Vec<String>,

    /// Difficulty level for customizing this template
    pub difficulty: Difficulty,

    /// Author or creator of the template
    pub author: String,

    /// Human-readable description
    pub description: String,

    /// Optional path to thumbnail image
    pub thumbnail_path: Option<String>,
}

/// Category classification for creature templates
///
/// Templates are organized by general body type and theme
/// to help users find appropriate starting points for their creatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TemplateCategory {
    /// Two-legged humanoid creatures (bipeds)
    Humanoid,

    /// Four-legged creatures (animals, beasts)
    Quadruped,

    /// Dragon-like creatures with wings and tails
    Dragon,

    /// Mechanical or robotic creatures
    Robot,

    /// Skeletal or ghostly undead creatures
    Undead,

    /// Feral beasts with claws and fangs
    Beast,

    /// User-created custom templates
    Custom,
}

/// Difficulty rating for template customization
///
/// Indicates how easy or complex it is to customize a template.
/// Simpler templates have fewer meshes and simpler structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Difficulty {
    /// Simple templates suitable for beginners (1-3 meshes)
    Beginner,

    /// Moderate complexity (4-8 meshes)
    Intermediate,

    /// Complex templates requiring experience (9+ meshes)
    Advanced,
}

impl TemplateMetadata {
    /// Creates new template metadata
    ///
    /// # Arguments
    ///
    /// * `category` - Template category
    /// * `difficulty` - Difficulty level
    /// * `author` - Author name
    /// * `description` - Template description
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::template_metadata::{TemplateMetadata, TemplateCategory, Difficulty};
    ///
    /// let metadata = TemplateMetadata::new(
    ///     TemplateCategory::Humanoid,
    ///     Difficulty::Beginner,
    ///     "Antares Team".to_string(),
    ///     "Basic humanoid template".to_string(),
    /// );
    ///
    /// assert_eq!(metadata.tags.len(), 0);
    /// assert_eq!(metadata.thumbnail_path, None);
    /// ```
    pub fn new(
        category: TemplateCategory,
        difficulty: Difficulty,
        author: String,
        description: String,
    ) -> Self {
        Self {
            category,
            tags: Vec::new(),
            difficulty,
            author,
            description,
            thumbnail_path: None,
        }
    }

    /// Adds a tag to the template
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::template_metadata::{TemplateMetadata, TemplateCategory, Difficulty};
    ///
    /// let mut metadata = TemplateMetadata::new(
    ///     TemplateCategory::Dragon,
    ///     Difficulty::Advanced,
    ///     "Designer".to_string(),
    ///     "Dragon template".to_string(),
    /// );
    ///
    /// metadata.add_tag("flying");
    /// metadata.add_tag("fire");
    ///
    /// assert_eq!(metadata.tags.len(), 2);
    /// assert!(metadata.tags.contains(&"flying".to_string()));
    /// ```
    pub fn add_tag(&mut self, tag: &str) {
        self.tags.push(tag.to_string());
    }

    /// Sets the thumbnail path
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::template_metadata::{TemplateMetadata, TemplateCategory, Difficulty};
    ///
    /// let mut metadata = TemplateMetadata::new(
    ///     TemplateCategory::Robot,
    ///     Difficulty::Intermediate,
    ///     "Builder".to_string(),
    ///     "Robot template".to_string(),
    /// );
    ///
    /// metadata.set_thumbnail("assets/robot.png");
    ///
    /// assert_eq!(metadata.thumbnail_path, Some("assets/robot.png".to_string()));
    /// ```
    pub fn set_thumbnail(&mut self, path: &str) {
        self.thumbnail_path = Some(path.to_string());
    }

    /// Checks if the template has a specific tag
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::template_metadata::{TemplateMetadata, TemplateCategory, Difficulty};
    ///
    /// let mut metadata = TemplateMetadata::new(
    ///     TemplateCategory::Beast,
    ///     Difficulty::Intermediate,
    ///     "Creator".to_string(),
    ///     "Beast template".to_string(),
    /// );
    ///
    /// metadata.add_tag("claws");
    ///
    /// assert!(metadata.has_tag("claws"));
    /// assert!(!metadata.has_tag("wings"));
    /// ```
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

impl TemplateCategory {
    /// Returns all available categories
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::template_metadata::TemplateCategory;
    ///
    /// let categories = TemplateCategory::all();
    /// assert_eq!(categories.len(), 7);
    /// assert!(categories.contains(&TemplateCategory::Humanoid));
    /// ```
    pub fn all() -> Vec<TemplateCategory> {
        vec![
            TemplateCategory::Humanoid,
            TemplateCategory::Quadruped,
            TemplateCategory::Dragon,
            TemplateCategory::Robot,
            TemplateCategory::Undead,
            TemplateCategory::Beast,
            TemplateCategory::Custom,
        ]
    }

    /// Returns a human-readable name for the category
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::template_metadata::TemplateCategory;
    ///
    /// assert_eq!(TemplateCategory::Humanoid.display_name(), "Humanoid");
    /// assert_eq!(TemplateCategory::Quadruped.display_name(), "Quadruped");
    /// ```
    pub fn display_name(&self) -> &'static str {
        match self {
            TemplateCategory::Humanoid => "Humanoid",
            TemplateCategory::Quadruped => "Quadruped",
            TemplateCategory::Dragon => "Dragon",
            TemplateCategory::Robot => "Robot",
            TemplateCategory::Undead => "Undead",
            TemplateCategory::Beast => "Beast",
            TemplateCategory::Custom => "Custom",
        }
    }
}

impl Difficulty {
    /// Returns all difficulty levels
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::template_metadata::Difficulty;
    ///
    /// let levels = Difficulty::all();
    /// assert_eq!(levels.len(), 3);
    /// assert!(levels.contains(&Difficulty::Beginner));
    /// ```
    pub fn all() -> Vec<Difficulty> {
        vec![
            Difficulty::Beginner,
            Difficulty::Intermediate,
            Difficulty::Advanced,
        ]
    }

    /// Returns a human-readable name for the difficulty
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::template_metadata::Difficulty;
    ///
    /// assert_eq!(Difficulty::Beginner.display_name(), "Beginner");
    /// assert_eq!(Difficulty::Advanced.display_name(), "Advanced");
    /// ```
    pub fn display_name(&self) -> &'static str {
        match self {
            Difficulty::Beginner => "Beginner",
            Difficulty::Intermediate => "Intermediate",
            Difficulty::Advanced => "Advanced",
        }
    }

    /// Returns a description of the difficulty level
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::template_metadata::Difficulty;
    ///
    /// let desc = Difficulty::Beginner.description();
    /// assert!(desc.contains("1-3 meshes"));
    /// ```
    pub fn description(&self) -> &'static str {
        match self {
            Difficulty::Beginner => "Simple templates suitable for beginners (1-3 meshes)",
            Difficulty::Intermediate => "Moderate complexity (4-8 meshes)",
            Difficulty::Advanced => "Complex templates requiring experience (9+ meshes)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_metadata_creation() {
        let metadata = TemplateMetadata::new(
            TemplateCategory::Humanoid,
            Difficulty::Beginner,
            "Test Author".to_string(),
            "Test description".to_string(),
        );

        assert_eq!(metadata.category, TemplateCategory::Humanoid);
        assert_eq!(metadata.difficulty, Difficulty::Beginner);
        assert_eq!(metadata.author, "Test Author");
        assert_eq!(metadata.description, "Test description");
        assert!(metadata.tags.is_empty());
        assert_eq!(metadata.thumbnail_path, None);
    }

    #[test]
    fn test_add_tag() {
        let mut metadata = TemplateMetadata::new(
            TemplateCategory::Dragon,
            Difficulty::Advanced,
            "Author".to_string(),
            "Dragon".to_string(),
        );

        metadata.add_tag("flying");
        metadata.add_tag("fire");
        metadata.add_tag("wings");

        assert_eq!(metadata.tags.len(), 3);
        assert!(metadata.tags.contains(&"flying".to_string()));
        assert!(metadata.tags.contains(&"fire".to_string()));
        assert!(metadata.tags.contains(&"wings".to_string()));
    }

    #[test]
    fn test_set_thumbnail() {
        let mut metadata = TemplateMetadata::new(
            TemplateCategory::Robot,
            Difficulty::Intermediate,
            "Author".to_string(),
            "Robot".to_string(),
        );

        assert_eq!(metadata.thumbnail_path, None);

        metadata.set_thumbnail("assets/robot.png");

        assert_eq!(
            metadata.thumbnail_path,
            Some("assets/robot.png".to_string())
        );
    }

    #[test]
    fn test_has_tag() {
        let mut metadata = TemplateMetadata::new(
            TemplateCategory::Beast,
            Difficulty::Intermediate,
            "Author".to_string(),
            "Beast".to_string(),
        );

        metadata.add_tag("claws");
        metadata.add_tag("fangs");

        assert!(metadata.has_tag("claws"));
        assert!(metadata.has_tag("fangs"));
        assert!(!metadata.has_tag("wings"));
        assert!(!metadata.has_tag("fire"));
    }

    #[test]
    fn test_template_category_all() {
        let categories = TemplateCategory::all();

        assert_eq!(categories.len(), 7);
        assert!(categories.contains(&TemplateCategory::Humanoid));
        assert!(categories.contains(&TemplateCategory::Quadruped));
        assert!(categories.contains(&TemplateCategory::Dragon));
        assert!(categories.contains(&TemplateCategory::Robot));
        assert!(categories.contains(&TemplateCategory::Undead));
        assert!(categories.contains(&TemplateCategory::Beast));
        assert!(categories.contains(&TemplateCategory::Custom));
    }

    #[test]
    fn test_template_category_display_name() {
        assert_eq!(TemplateCategory::Humanoid.display_name(), "Humanoid");
        assert_eq!(TemplateCategory::Quadruped.display_name(), "Quadruped");
        assert_eq!(TemplateCategory::Dragon.display_name(), "Dragon");
        assert_eq!(TemplateCategory::Robot.display_name(), "Robot");
        assert_eq!(TemplateCategory::Undead.display_name(), "Undead");
        assert_eq!(TemplateCategory::Beast.display_name(), "Beast");
        assert_eq!(TemplateCategory::Custom.display_name(), "Custom");
    }

    #[test]
    fn test_difficulty_all() {
        let difficulties = Difficulty::all();

        assert_eq!(difficulties.len(), 3);
        assert!(difficulties.contains(&Difficulty::Beginner));
        assert!(difficulties.contains(&Difficulty::Intermediate));
        assert!(difficulties.contains(&Difficulty::Advanced));
    }

    #[test]
    fn test_difficulty_display_name() {
        assert_eq!(Difficulty::Beginner.display_name(), "Beginner");
        assert_eq!(Difficulty::Intermediate.display_name(), "Intermediate");
        assert_eq!(Difficulty::Advanced.display_name(), "Advanced");
    }

    #[test]
    fn test_difficulty_description() {
        let beginner_desc = Difficulty::Beginner.description();
        assert!(beginner_desc.contains("1-3 meshes"));

        let intermediate_desc = Difficulty::Intermediate.description();
        assert!(intermediate_desc.contains("4-8 meshes"));

        let advanced_desc = Difficulty::Advanced.description();
        assert!(advanced_desc.contains("9+ meshes"));
    }

    #[test]
    fn test_difficulty_ordering() {
        assert!(Difficulty::Beginner < Difficulty::Intermediate);
        assert!(Difficulty::Intermediate < Difficulty::Advanced);
        assert!(Difficulty::Beginner < Difficulty::Advanced);
    }

    #[test]
    fn test_template_metadata_serialization() {
        let metadata = TemplateMetadata {
            category: TemplateCategory::Dragon,
            tags: vec!["flying".to_string(), "fire".to_string()],
            difficulty: Difficulty::Advanced,
            author: "Test Author".to_string(),
            description: "Test dragon".to_string(),
            thumbnail_path: Some("assets/dragon.png".to_string()),
        };

        let ron = ron::to_string(&metadata).expect("Failed to serialize");
        let deserialized: TemplateMetadata = ron::from_str(&ron).expect("Failed to deserialize");

        assert_eq!(metadata, deserialized);
    }

    #[test]
    fn test_template_category_equality() {
        assert_eq!(TemplateCategory::Humanoid, TemplateCategory::Humanoid);
        assert_ne!(TemplateCategory::Humanoid, TemplateCategory::Dragon);
    }

    #[test]
    fn test_difficulty_equality() {
        assert_eq!(Difficulty::Beginner, Difficulty::Beginner);
        assert_ne!(Difficulty::Beginner, Difficulty::Advanced);
    }
}
