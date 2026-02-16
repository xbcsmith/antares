// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Campaign loading system for loading game data on startup
//!
//! This module provides Bevy systems for loading campaign data including
//! creature databases, items, spells, and other game content.
//!
//! # Examples
//!
//! ```no_run
//! use antares::game::systems::campaign_loading::load_campaign_data;
//! use bevy::prelude::*;
//!
//! fn setup_app(app: &mut App) {
//!     app.add_systems(Startup, load_campaign_data);
//! }
//! ```

use std::path::PathBuf;

use bevy::prelude::*;

use crate::domain::campaign_loader::{CampaignLoader, GameData};
use crate::game::resources::GameDataResource;

/// Loads campaign data on startup
///
/// This system loads the campaign's game data including creatures, items,
/// spells, and other content. It should be run during the Startup stage.
///
/// # Errors
///
/// Logs errors if campaign data fails to load, but continues with empty data.
///
/// # Examples
///
/// ```no_run
/// use antares::game::systems::campaign_loading::load_campaign_data;
/// use bevy::prelude::*;
///
/// App::new()
///     .add_systems(Startup, load_campaign_data)
///     .run();
/// ```
pub fn load_campaign_data(mut commands: Commands) {
    info!("Loading campaign data...");

    // Default campaign path - in the future this could be configurable
    let base_data_path = PathBuf::from("data");
    let campaign_path = PathBuf::from("campaigns/tutorial");

    let mut loader = CampaignLoader::new(base_data_path, campaign_path);

    match loader.load_game_data() {
        Ok(game_data) => {
            let creature_count = game_data.creatures.count();
            info!(
                "Campaign data loaded successfully. Creatures: {}",
                creature_count
            );

            // Insert the game data as a resource
            commands.insert_resource(GameDataResource::new(game_data));
        }
        Err(e) => {
            error!("Failed to load campaign data: {}", e);
            warn!("Continuing with empty game data");

            // Insert empty game data to prevent crashes
            commands.insert_resource(GameDataResource::new(GameData::new()));
        }
    }
}

/// Loads campaign data from a specific path
///
/// This system loads campaign data from the specified paths.
///
/// # Arguments
///
/// * `base_data_path` - Path to base game data
/// * `campaign_path` - Path to campaign directory
///
/// # Returns
///
/// Returns a system that can be added to a Bevy app
///
/// # Examples
///
/// ```no_run
/// use antares::game::systems::campaign_loading::load_campaign_data_from_path;
/// use bevy::prelude::*;
/// use std::path::PathBuf;
///
/// let system = load_campaign_data_from_path(
///     PathBuf::from("data"),
///     PathBuf::from("campaigns/my_campaign")
/// );
///
/// App::new()
///     .add_systems(Startup, system)
///     .run();
/// ```
pub fn load_campaign_data_from_path(
    base_data_path: PathBuf,
    campaign_path: PathBuf,
) -> impl Fn(Commands) {
    move |mut commands: Commands| {
        info!("Loading campaign data from {:?}...", campaign_path);

        let mut loader = CampaignLoader::new(base_data_path.clone(), campaign_path.clone());

        match loader.load_game_data() {
            Ok(game_data) => {
                let creature_count = game_data.creatures.count();
                info!(
                    "Campaign data loaded successfully. Creatures: {}",
                    creature_count
                );

                commands.insert_resource(GameDataResource::new(game_data));
            }
            Err(e) => {
                error!("Failed to load campaign data: {}", e);
                warn!("Continuing with empty game data");

                commands.insert_resource(GameDataResource::new(GameData::new()));
            }
        }
    }
}

/// Validates loaded campaign data
///
/// This system runs after campaign data is loaded to validate that all
/// cross-references are correct and no data is missing.
///
/// # Examples
///
/// ```no_run
/// use antares::game::systems::campaign_loading::validate_campaign_data;
/// use bevy::prelude::*;
///
/// App::new()
///     .add_systems(Startup, validate_campaign_data)
///     .run();
/// ```
pub fn validate_campaign_data(game_data: Res<GameDataResource>) {
    info!("Validating campaign data...");

    match game_data.data().validate() {
        Ok(()) => {
            info!("Campaign data validation successful");
        }
        Err(e) => {
            error!("Campaign data validation failed: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_campaign_data_creates_resource() {
        let mut app = App::new();
        app.add_systems(Startup, load_campaign_data);
        app.update();

        // Resource should exist even if loading failed
        assert!(app.world().get_resource::<GameDataResource>().is_some());
    }

    #[test]
    fn test_validate_campaign_data_with_empty_data() {
        let mut app = App::new();
        app.insert_resource(GameDataResource::new(GameData::new()));
        app.add_systems(Update, validate_campaign_data);
        app.update();

        // Should not panic with empty data
    }

    #[test]
    fn test_load_campaign_from_nonexistent_path() {
        let system = load_campaign_data_from_path(
            PathBuf::from("nonexistent_data"),
            PathBuf::from("nonexistent_campaign"),
        );

        let mut app = App::new();
        app.add_systems(Startup, system);
        app.update();

        // Should create resource with empty data
        let resource = app.world().get_resource::<GameDataResource>();
        assert!(resource.is_some());
    }
}
