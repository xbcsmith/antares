//! Test Play Integration Module
//!
//! This module provides functionality for launching the Antares game engine
//! with the current campaign for testing purposes.

use crate::{CampaignBuilderApp, CampaignError};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

/// Test play session state
#[derive(Debug)]
pub struct TestPlaySession {
    /// Process handle for the game
    process: Option<Child>,
    /// Campaign being tested
    campaign_id: String,
    /// Output log buffer
    output_log: Vec<String>,
    /// Error log buffer
    error_log: Vec<String>,
    /// Test play start time
    start_time: std::time::Instant,
    /// Is session active
    is_active: bool,
}

impl TestPlaySession {
    /// Creates a new test play session
    ///
    /// # Arguments
    ///
    /// * `campaign_id` - ID of the campaign to test
    pub fn new(campaign_id: String) -> Self {
        Self {
            process: None,
            campaign_id,
            output_log: Vec::new(),
            error_log: Vec::new(),
            start_time: std::time::Instant::now(),
            is_active: false,
        }
    }

    /// Launches the game with the campaign
    ///
    /// # Arguments
    ///
    /// * `game_executable` - Path to the Antares game executable
    /// * `debug_mode` - Enable debug logging if true
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if game launched successfully
    ///
    /// # Errors
    ///
    /// Returns error if game executable not found or fails to start
    pub fn launch(
        &mut self,
        game_executable: &PathBuf,
        debug_mode: bool,
    ) -> Result<(), std::io::Error> {
        // Build command
        let mut cmd = Command::new(game_executable);
        cmd.arg("--campaign").arg(&self.campaign_id);

        if debug_mode {
            cmd.arg("--debug");
        }

        // Capture output
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Spawn process
        let child = cmd.spawn()?;
        self.process = Some(child);
        self.is_active = true;
        self.start_time = std::time::Instant::now();

        Ok(())
    }

    /// Checks if the game process is still running
    ///
    /// # Returns
    ///
    /// Returns `true` if game is running, `false` otherwise
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut process) = self.process {
            match process.try_wait() {
                Ok(Some(_status)) => {
                    self.is_active = false;
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    self.is_active = false;
                    false
                }
            }
        } else {
            false
        }
    }

    /// Terminates the game process
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if terminated successfully
    pub fn terminate(&mut self) -> Result<(), std::io::Error> {
        if let Some(ref mut process) = self.process {
            process.kill()?;
            self.is_active = false;
        }
        Ok(())
    }

    /// Gets the output log
    ///
    /// # Returns
    ///
    /// Returns reference to output log lines
    pub fn output_log(&self) -> &[String] {
        &self.output_log
    }

    /// Gets the error log
    ///
    /// # Returns
    ///
    /// Returns reference to error log lines
    pub fn error_log(&self) -> &[String] {
        &self.error_log
    }

    /// Gets elapsed time since test play started
    ///
    /// # Returns
    ///
    /// Returns duration since start
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Adds a line to the output log
    ///
    /// # Arguments
    ///
    /// * `line` - Log line to add
    pub fn add_output(&mut self, line: String) {
        self.output_log.push(line);
    }

    /// Adds a line to the error log
    ///
    /// # Arguments
    ///
    /// * `line` - Error line to add
    pub fn add_error(&mut self, line: String) {
        self.error_log.push(line);
    }

    /// Clears all logs
    pub fn clear_logs(&mut self) {
        self.output_log.clear();
        self.error_log.clear();
    }

    /// Gets the campaign ID being tested
    pub fn campaign_id(&self) -> &str {
        &self.campaign_id
    }

    /// Checks if session is active
    pub fn is_active(&self) -> bool {
        self.is_active
    }
}

impl Drop for TestPlaySession {
    fn drop(&mut self) {
        // Ensure process is terminated when session is dropped
        let _ = self.terminate();
    }
}

/// Test play configuration
#[derive(Debug, Clone)]
pub struct TestPlayConfig {
    /// Path to game executable
    pub game_executable: PathBuf,
    /// Enable debug mode
    pub debug_mode: bool,
    /// Auto-save campaign before launching
    pub auto_save: bool,
    /// Validate campaign before launching
    pub validate_first: bool,
    /// Maximum log lines to keep
    pub max_log_lines: usize,
}

impl Default for TestPlayConfig {
    fn default() -> Self {
        Self {
            game_executable: PathBuf::from("antares"),
            debug_mode: true,
            auto_save: true,
            validate_first: true,
            max_log_lines: 1000,
        }
    }
}

impl CampaignBuilderApp {
    /// Launches test play session
    ///
    /// # Arguments
    ///
    /// * `config` - Test play configuration
    ///
    /// # Returns
    ///
    /// Returns `Ok(TestPlaySession)` if launched successfully
    ///
    /// # Errors
    ///
    /// Returns error if validation fails, save fails, or game fails to launch
    pub fn launch_test_play(
        &mut self,
        config: &TestPlayConfig,
    ) -> Result<TestPlaySession, CampaignError> {
        // Auto-save if enabled
        if config.auto_save {
            self.save_campaign()?;
        }

        // Validate if enabled
        if config.validate_first {
            self.validate_campaign();
            let has_errors = self
                .validation_errors
                .iter()
                .any(|e| matches!(e.severity, crate::Severity::Error));

            if has_errors {
                return Err(CampaignError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Campaign has validation errors",
                )));
            }
        }

        // Check game executable exists
        if !config.game_executable.exists() {
            return Err(CampaignError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "Game executable not found: {}",
                    config.game_executable.display()
                ),
            )));
        }

        // Create and launch session
        let mut session = TestPlaySession::new(self.campaign.id.clone());
        session
            .launch(&config.game_executable, config.debug_mode)
            .map_err(CampaignError::Io)?;

        self.status_message = format!("Test play launched: {}", self.campaign.name);
        Ok(session)
    }

    /// Checks if test play is available (game executable exists)
    ///
    /// # Arguments
    ///
    /// * `config` - Test play configuration to check
    ///
    /// # Returns
    ///
    /// Returns `true` if test play can be launched
    pub fn can_launch_test_play(&self, config: &TestPlayConfig) -> bool {
        config.game_executable.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_play_session_creation() {
        let session = TestPlaySession::new("test_campaign".to_string());
        assert_eq!(session.campaign_id(), "test_campaign");
        assert!(!session.is_active());
        assert!(session.output_log().is_empty());
        assert!(session.error_log().is_empty());
    }

    #[test]
    fn test_test_play_session_logging() {
        let mut session = TestPlaySession::new("test".to_string());

        session.add_output("Output line 1".to_string());
        session.add_output("Output line 2".to_string());
        session.add_error("Error line 1".to_string());

        assert_eq!(session.output_log().len(), 2);
        assert_eq!(session.error_log().len(), 1);
        assert_eq!(session.output_log()[0], "Output line 1");
        assert_eq!(session.error_log()[0], "Error line 1");
    }

    #[test]
    fn test_test_play_session_clear_logs() {
        let mut session = TestPlaySession::new("test".to_string());

        session.add_output("Test".to_string());
        session.add_error("Error".to_string());
        assert!(!session.output_log().is_empty());
        assert!(!session.error_log().is_empty());

        session.clear_logs();
        assert!(session.output_log().is_empty());
        assert!(session.error_log().is_empty());
    }

    #[test]
    fn test_test_play_session_elapsed() {
        let session = TestPlaySession::new("test".to_string());
        let elapsed = session.elapsed();
        assert!(elapsed.as_secs() < 1); // Should be very recent
    }

    #[test]
    fn test_test_play_config_default() {
        let config = TestPlayConfig::default();
        assert!(config.auto_save);
        assert!(config.validate_first);
        assert!(config.debug_mode);
        assert_eq!(config.max_log_lines, 1000);
    }

    #[test]
    fn test_test_play_config_custom() {
        let config = TestPlayConfig {
            game_executable: PathBuf::from("/usr/bin/antares"),
            debug_mode: false,
            auto_save: false,
            validate_first: false,
            max_log_lines: 500,
        };

        assert!(!config.auto_save);
        assert!(!config.validate_first);
        assert!(!config.debug_mode);
        assert_eq!(config.max_log_lines, 500);
    }

    #[test]
    fn test_test_play_session_is_running_without_process() {
        let mut session = TestPlaySession::new("test".to_string());
        assert!(!session.is_running());
    }

    #[test]
    fn test_test_play_session_terminate_without_process() {
        let mut session = TestPlaySession::new("test".to_string());
        // Should not panic
        assert!(session.terminate().is_ok());
    }

    #[test]
    fn test_test_play_session_campaign_id_immutable() {
        let session = TestPlaySession::new("original_id".to_string());
        assert_eq!(session.campaign_id(), "original_id");
        // Campaign ID should not change
        assert_eq!(session.campaign_id(), "original_id");
    }

    #[test]
    fn test_test_play_session_log_ordering() {
        let mut session = TestPlaySession::new("test".to_string());

        session.add_output("Line 1".to_string());
        session.add_output("Line 2".to_string());
        session.add_output("Line 3".to_string());

        let log = session.output_log();
        assert_eq!(log[0], "Line 1");
        assert_eq!(log[1], "Line 2");
        assert_eq!(log[2], "Line 3");
    }

    #[test]
    fn test_test_play_config_executable_path() {
        let mut config = TestPlayConfig::default();
        assert_eq!(config.game_executable, PathBuf::from("antares"));

        config.game_executable = PathBuf::from("/custom/path/antares");
        assert_eq!(
            config.game_executable,
            PathBuf::from("/custom/path/antares")
        );
    }
}
