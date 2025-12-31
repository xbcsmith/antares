// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Logging module for the Campaign Builder
//!
//! Provides configurable logging levels and a log message buffer for the debug panel.
//!
//! # Log Levels
//!
//! - `Error`: Critical errors that prevent operations
//! - `Warn`: Warnings about potential issues
//! - `Info`: General informational messages
//! - `Debug`: Debug information for development
//! - `Verbose`: Detailed trace-level information
//!
//! # Examples
//!
//! ```ignore
//! use crate::logging::{Logger, LogLevel};
//!
//! let mut logger = Logger::new(LogLevel::Debug);
//! logger.info("Application started");
//! logger.debug("Loading campaign...");
//! logger.verbose("Parsed 42 items from RON file");
//! ```

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Maximum number of log messages to retain in the buffer
const MAX_LOG_MESSAGES: usize = 500;

/// Log level for filtering messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Critical errors
    Error = 0,
    /// Warnings about potential issues
    Warn = 1,
    /// General informational messages
    Info = 2,
    /// Debug information
    Debug = 3,
    /// Verbose trace-level information
    Verbose = 4,
}

impl LogLevel {
    /// Returns the display name of the log level
    pub fn name(&self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Verbose => "VERBOSE",
        }
    }

    /// Returns a short single-character prefix for the log level
    pub fn prefix(&self) -> &'static str {
        match self {
            LogLevel::Error => "E",
            LogLevel::Warn => "W",
            LogLevel::Info => "I",
            LogLevel::Debug => "D",
            LogLevel::Verbose => "V",
        }
    }

    /// Returns the color for the log level (egui Color32 RGB values)
    pub fn color(&self) -> [u8; 3] {
        match self {
            LogLevel::Error => [255, 100, 100],   // Red
            LogLevel::Warn => [255, 200, 100],    // Orange
            LogLevel::Info => [200, 200, 200],    // Light gray
            LogLevel::Debug => [150, 200, 255],   // Light blue
            LogLevel::Verbose => [150, 150, 150], // Gray
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A single log message with metadata
#[derive(Debug, Clone)]
pub struct LogMessage {
    /// The log level of this message
    pub level: LogLevel,
    /// The message content
    pub message: String,
    /// The category/source of the message
    pub category: String,
    /// Timestamp when the message was created (seconds since app start)
    pub timestamp: f64,
}

impl LogMessage {
    /// Creates a new log message
    pub fn new(level: LogLevel, category: impl Into<String>, message: impl Into<String>) -> Self {
        // Use a simple timestamp based on system time
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);

        Self {
            level,
            message: message.into(),
            category: category.into(),
            timestamp,
        }
    }

    /// Formats the log message for display
    pub fn format(&self) -> String {
        format!(
            "[{}] [{}] {}: {}",
            self.format_timestamp(),
            self.level.prefix(),
            self.category,
            self.message
        )
    }

    /// Formats the timestamp as HH:MM:SS
    fn format_timestamp(&self) -> String {
        let total_secs = self.timestamp as u64;
        let hours = (total_secs / 3600) % 24;
        let minutes = (total_secs / 60) % 60;
        let seconds = total_secs % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }
}

/// Logger that stores messages and filters by level
#[derive(Debug, Clone)]
pub struct Logger {
    /// Current log level threshold
    level: LogLevel,
    /// Ring buffer of recent log messages
    messages: VecDeque<LogMessage>,
    /// Start time of the logger for relative timestamps
    start_time: Instant,
    /// Whether to also print to stderr
    print_to_stderr: bool,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(LogLevel::Info)
    }
}

impl Logger {
    /// Creates a new logger with the specified minimum log level
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            messages: VecDeque::with_capacity(MAX_LOG_MESSAGES),
            start_time: Instant::now(),
            print_to_stderr: true,
        }
    }

    /// Creates a new logger from command-line arguments
    ///
    /// Checks for `--verbose` or `-v` flags to enable verbose logging.
    pub fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let level = if args.iter().any(|a| a == "--verbose" || a == "-v") {
            LogLevel::Verbose
        } else if args.iter().any(|a| a == "--debug" || a == "-d") {
            LogLevel::Debug
        } else if args.iter().any(|a| a == "--quiet" || a == "-q") {
            LogLevel::Warn
        } else {
            LogLevel::Info
        };

        Self::new(level)
    }

    /// Indicates whether debug logging has been enabled by either an environment
    /// variable or CLI flag check. This is intended for code paths where a
    /// Logger instance may not be available/constructed yet and a quick check
    /// (bool) is desired.
    ///
    /// Environment variable:
    /// - ANTARES_DEBUG=1 | true | on | yes (case-insensitive) => enables debug
    /// - ANTARES_DEBUG=0 | false | off | no => disables debug
    ///
    /// CLI flags:
    /// - `--debug` / `-d` or `--verbose` / `-v` are accepted as well.
    pub fn debug_enabled() -> bool {
        // First, check explicit environment variable
        if let Ok(val) = std::env::var("ANTARES_DEBUG") {
            let v = val.trim().to_ascii_lowercase();
            match v.as_str() {
                "1" | "true" | "on" | "yes" => return true,
                "0" | "false" | "off" | "no" => return false,
                _ => {}
            }
        }

        // Next fallback: check CLI args for debug/verbose flags
        // Note: In tests or other contexts std::env::args() may include the test harness args.
        let mut args_iter = std::env::args();
        args_iter.any(|a| a == "--debug" || a == "-d" || a == "--verbose" || a == "-v")
    }

    /// Sets the log level
    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    /// Gets the current log level
    pub fn level(&self) -> LogLevel {
        self.level
    }

    /// Sets whether to print to stderr
    pub fn set_print_to_stderr(&mut self, print: bool) {
        self.print_to_stderr = print;
    }

    /// Logs a message at the specified level
    pub fn log(&mut self, level: LogLevel, category: &str, message: &str) {
        // Always store the message, but only print if at or above threshold
        let msg = LogMessage::new(level, category, message);

        // Print to stderr if enabled and level is at or above threshold
        if self.print_to_stderr && level <= self.level {
            eprintln!("{}", msg.format());
        }

        // Store in buffer
        if self.messages.len() >= MAX_LOG_MESSAGES {
            self.messages.pop_front();
        }
        self.messages.push_back(msg);
    }

    /// Logs an error message
    pub fn error(&mut self, category: &str, message: &str) {
        self.log(LogLevel::Error, category, message);
    }

    /// Logs a warning message
    pub fn warn(&mut self, category: &str, message: &str) {
        self.log(LogLevel::Warn, category, message);
    }

    /// Logs an info message
    pub fn info(&mut self, category: &str, message: &str) {
        self.log(LogLevel::Info, category, message);
    }

    /// Logs a debug message
    pub fn debug(&mut self, category: &str, message: &str) {
        self.log(LogLevel::Debug, category, message);
    }

    /// Logs a verbose message
    pub fn verbose(&mut self, category: &str, message: &str) {
        self.log(LogLevel::Verbose, category, message);
    }

    /// Returns all stored messages
    pub fn messages(&self) -> &VecDeque<LogMessage> {
        &self.messages
    }

    /// Returns messages filtered by the current log level
    pub fn filtered_messages(&self) -> impl Iterator<Item = &LogMessage> {
        self.messages.iter().filter(|m| m.level <= self.level)
    }

    /// Returns messages filtered by a specific level
    pub fn messages_at_level(&self, level: LogLevel) -> impl Iterator<Item = &LogMessage> {
        self.messages.iter().filter(move |m| m.level <= level)
    }

    /// Clears all stored messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Returns the number of messages at each level
    pub fn message_counts(&self) -> LogCounts {
        let mut counts = LogCounts::default();
        for msg in &self.messages {
            match msg.level {
                LogLevel::Error => counts.error += 1,
                LogLevel::Warn => counts.warn += 1,
                LogLevel::Info => counts.info += 1,
                LogLevel::Debug => counts.debug += 1,
                LogLevel::Verbose => counts.verbose += 1,
            }
        }
        counts
    }

    /// Returns how long the logger has been running
    pub fn uptime(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

/// Counts of messages at each log level
#[derive(Debug, Default, Clone, Copy)]
pub struct LogCounts {
    pub error: usize,
    pub warn: usize,
    pub info: usize,
    pub debug: usize,
    pub verbose: usize,
}

impl LogCounts {
    /// Returns the total number of messages
    pub fn total(&self) -> usize {
        self.error + self.warn + self.info + self.debug + self.verbose
    }
}

/// Log categories used throughout the application
pub mod category {
    pub const APP: &str = "App";
    pub const FILE_IO: &str = "FileIO";
    pub const EDITOR: &str = "Editor";
    pub const VALIDATION: &str = "Validation";
    pub const UI: &str = "UI";
    pub const DATA: &str = "Data";
    pub const CAMPAIGN: &str = "Campaign";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Verbose);
    }

    #[test]
    fn test_logger_stores_messages() {
        let mut logger = Logger::new(LogLevel::Verbose);
        logger.set_print_to_stderr(false);

        logger.info("Test", "Hello");
        logger.debug("Test", "World");

        assert_eq!(logger.messages().len(), 2);
    }

    #[test]
    fn test_logger_filters_by_level() {
        let mut logger = Logger::new(LogLevel::Info);
        logger.set_print_to_stderr(false);

        logger.error("Test", "Error message");
        logger.info("Test", "Info message");
        logger.debug("Test", "Debug message");
        logger.verbose("Test", "Verbose message");

        // All messages are stored
        assert_eq!(logger.messages().len(), 4);

        // But only error and info are visible at Info level
        let filtered: Vec<_> = logger.filtered_messages().collect();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_logger_max_capacity() {
        let mut logger = Logger::new(LogLevel::Verbose);
        logger.set_print_to_stderr(false);

        for i in 0..MAX_LOG_MESSAGES + 100 {
            logger.info("Test", &format!("Message {}", i));
        }

        assert_eq!(logger.messages().len(), MAX_LOG_MESSAGES);

        // First message should be 100 (the first 100 were discarded)
        let first_msg = logger.messages().front().unwrap();
        assert!(first_msg.message.contains("100"));
    }

    #[test]
    fn test_message_counts() {
        let mut logger = Logger::new(LogLevel::Verbose);
        logger.set_print_to_stderr(false);

        logger.error("Test", "e1");
        logger.error("Test", "e2");
        logger.warn("Test", "w1");
        logger.info("Test", "i1");
        logger.info("Test", "i2");
        logger.info("Test", "i3");
        logger.debug("Test", "d1");

        let counts = logger.message_counts();
        assert_eq!(counts.error, 2);
        assert_eq!(counts.warn, 1);
        assert_eq!(counts.info, 3);
        assert_eq!(counts.debug, 1);
        assert_eq!(counts.verbose, 0);
        assert_eq!(counts.total(), 7);
    }

    #[test]
    fn test_log_message_format() {
        let msg = LogMessage::new(LogLevel::Info, "Test", "Hello world");
        let formatted = msg.format();

        assert!(formatted.contains("[I]"));
        assert!(formatted.contains("Test"));
        assert!(formatted.contains("Hello world"));
    }

    #[test]
    fn test_log_level_colors() {
        // Just verify colors are defined and different
        assert_ne!(LogLevel::Error.color(), LogLevel::Info.color());
        assert_ne!(LogLevel::Debug.color(), LogLevel::Verbose.color());
    }
}
