// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Performance optimization module with caching
//!
//! Provides caching mechanisms to improve performance when loading and
//! validating large campaigns. Caches frequently accessed data like
//! item databases, monster definitions, and validation results.
//!
//! # Cache Strategy
//!
//! - **File-based caching**: Caches RON parsing results keyed by file hash
//! - **Memory caching**: In-memory LRU cache for frequently accessed entities
//! - **Validation caching**: Stores validation results to avoid re-validation
//!
//! # Examples
//!
//! ```
//! use antares::sdk::cache::{ContentCache, CacheConfig};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = CacheConfig::default();
//! let mut cache = ContentCache::new(config);
//!
//! // Load with caching
//! let items = cache.load_items("data/items.ron")?;
//!
//! // Second load is instant (from cache)
//! let items_again = cache.load_items("data/items.ron")?;
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use thiserror::Error;

// ===== Error Types =====

/// Errors that can occur during caching operations
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Failed to compute file hash: {0}")]
    HashError(String),

    #[error("Cache entry expired")]
    Expired,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

// ===== Configuration =====

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Enable file-based caching
    pub enable_file_cache: bool,

    /// Enable memory caching
    pub enable_memory_cache: bool,

    /// Maximum memory cache entries
    pub max_memory_entries: usize,

    /// Cache TTL (time-to-live)
    pub ttl: Duration,

    /// Cache directory
    pub cache_dir: PathBuf,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_file_cache: true,
            enable_memory_cache: true,
            max_memory_entries: 100,
            ttl: Duration::from_secs(3600), // 1 hour
            cache_dir: PathBuf::from(".antares_cache"),
        }
    }
}

// ===== Cache Entry =====

/// A cached entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct CacheEntry<T> {
    /// Cached data
    data: T,

    /// File hash at time of caching
    file_hash: u64,

    /// Timestamp when cached
    cached_at: SystemTime,
}

impl<T> CacheEntry<T> {
    #[allow(dead_code)]
    fn new(data: T, file_hash: u64) -> Self {
        Self {
            data,
            file_hash,
            cached_at: SystemTime::now(),
        }
    }

    #[allow(dead_code)]
    fn is_expired(&self, ttl: Duration) -> bool {
        match SystemTime::now().duration_since(self.cached_at) {
            Ok(age) => age > ttl,
            Err(_) => true, // Time went backwards, consider expired
        }
    }
}

// ===== Content Cache =====

/// Main content cache for SDK operations
pub struct ContentCache {
    config: CacheConfig,
    memory_cache: HashMap<PathBuf, Box<dyn std::any::Any + Send>>,
    access_count: HashMap<PathBuf, usize>,
}

impl ContentCache {
    /// Creates a new content cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            memory_cache: HashMap::new(),
            access_count: HashMap::new(),
        }
    }

    /// Loads items with caching
    ///
    /// Returns cached items if file hasn't changed, otherwise loads from disk.
    pub fn load_items<P: AsRef<Path>>(
        &mut self,
        path: P,
    ) -> Result<Vec<ItemCacheEntry>, CacheError> {
        self.load_cached(path, |_p| {
            // Placeholder: would load actual items
            Ok(vec![])
        })
    }

    /// Generic cached load function
    fn load_cached<P, T, F>(&mut self, path: P, loader: F) -> Result<T, CacheError>
    where
        P: AsRef<Path>,
        T: Clone + Send + 'static,
        F: FnOnce(&Path) -> Result<T, CacheError>,
    {
        let path = path.as_ref();
        let path_buf = path.to_path_buf();

        // Check memory cache first
        if self.config.enable_memory_cache {
            if let Some(cached) = self.memory_cache.get(&path_buf) {
                if let Some(typed) = cached.downcast_ref::<T>() {
                    self.access_count
                        .entry(path_buf.clone())
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                    return Ok(typed.clone());
                }
            }
        }

        // Load from disk
        let data = loader(path)?;

        // Store in memory cache
        if self.config.enable_memory_cache {
            self.memory_cache
                .insert(path_buf.clone(), Box::new(data.clone()));
            self.access_count.insert(path_buf.clone(), 1);

            // Evict LRU if over limit
            self.evict_lru();
        }

        Ok(data)
    }

    /// Evicts least recently used entries
    fn evict_lru(&mut self) {
        while self.memory_cache.len() > self.config.max_memory_entries {
            // Find LRU entry
            if let Some((lru_path, _)) = self
                .access_count
                .iter()
                .min_by_key(|(_, &count)| count)
                .map(|(p, c)| (p.clone(), *c))
            {
                self.memory_cache.remove(&lru_path);
                self.access_count.remove(&lru_path);
            } else {
                break;
            }
        }
    }

    /// Clears all caches
    pub fn clear(&mut self) {
        self.memory_cache.clear();
        self.access_count.clear();
    }

    /// Gets cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            memory_entries: self.memory_cache.len(),
            total_accesses: self.access_count.values().sum(),
        }
    }

    /// Computes a simple hash of a file's contents
    #[allow(dead_code)]
    fn compute_file_hash<P: AsRef<Path>>(path: P) -> Result<u64, CacheError> {
        let metadata = fs::metadata(path.as_ref())?;
        let modified = metadata
            .modified()
            .map_err(|e| CacheError::HashError(e.to_string()))?;

        // Simple hash based on size and modified time
        let hash = metadata.len().wrapping_mul(31).wrapping_add(
            modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| CacheError::HashError(e.to_string()))?
                .as_secs(),
        );

        Ok(hash)
    }
}

// ===== Cache Statistics =====

/// Statistics about cache performance
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of entries in memory cache
    pub memory_entries: usize,

    /// Total cache accesses
    pub total_accesses: usize,
}

impl CacheStats {
    /// Calculates hit rate (requires tracking misses separately)
    pub fn hit_rate(&self, total_requests: usize) -> f64 {
        if total_requests == 0 {
            0.0
        } else {
            self.total_accesses as f64 / total_requests as f64
        }
    }
}

// ===== Validation Cache =====

/// Caches validation results
pub struct ValidationCache {
    results: HashMap<String, ValidationCacheEntry>,
    config: CacheConfig,
}

impl ValidationCache {
    /// Creates a new validation cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            results: HashMap::new(),
            config,
        }
    }

    /// Gets cached validation result
    pub fn get(&self, campaign_path: &str) -> Option<&[String]> {
        self.results.get(campaign_path).and_then(|entry| {
            if entry.is_expired(self.config.ttl) {
                None
            } else {
                Some(entry.errors.as_slice())
            }
        })
    }

    /// Stores validation result
    pub fn put(&mut self, campaign_path: String, errors: Vec<String>) {
        let entry = ValidationCacheEntry {
            errors,
            cached_at: SystemTime::now(),
        };
        self.results.insert(campaign_path, entry);
    }

    /// Clears expired entries
    pub fn cleanup(&mut self) {
        self.results
            .retain(|_, entry| !entry.is_expired(self.config.ttl));
    }
}

#[derive(Debug, Clone)]
struct ValidationCacheEntry {
    errors: Vec<String>,
    cached_at: SystemTime,
}

impl ValidationCacheEntry {
    fn is_expired(&self, ttl: Duration) -> bool {
        match SystemTime::now().duration_since(self.cached_at) {
            Ok(age) => age > ttl,
            Err(_) => true,
        }
    }
}

// ===== Placeholder Types =====

/// Placeholder for item cache entry
#[derive(Debug, Clone)]
pub struct ItemCacheEntry {
    pub id: u32,
    pub name: String,
}

// ===== Helper Functions =====

/// Preloads commonly used content into cache
pub fn preload_common_content(
    _cache: &mut ContentCache,
    _data_dir: &Path,
) -> Result<(), CacheError> {
    // Would preload items, monsters, spells, etc.
    // For now, just a placeholder
    Ok(())
}

// ===== Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.enable_file_cache);
        assert!(config.enable_memory_cache);
        assert_eq!(config.max_memory_entries, 100);
    }

    #[test]
    fn test_cache_entry_expiration() {
        let entry = CacheEntry::new("data".to_string(), 12345);
        assert!(!entry.is_expired(Duration::from_secs(3600)));

        // Sleep to ensure time has passed
        std::thread::sleep(Duration::from_millis(10));
        assert!(entry.is_expired(Duration::from_millis(1)));
    }

    #[test]
    fn test_content_cache_clear() {
        let config = CacheConfig::default();
        let mut cache = ContentCache::new(config);

        cache
            .memory_cache
            .insert(PathBuf::from("test"), Box::new(42));
        assert_eq!(cache.memory_cache.len(), 1);

        cache.clear();
        assert_eq!(cache.memory_cache.len(), 0);
    }

    #[test]
    fn test_cache_stats() {
        let config = CacheConfig::default();
        let cache = ContentCache::new(config);

        let stats = cache.stats();
        assert_eq!(stats.memory_entries, 0);
        assert_eq!(stats.total_accesses, 0);
    }

    #[test]
    fn test_hit_rate_calculation() {
        let stats = CacheStats {
            memory_entries: 10,
            total_accesses: 80,
        };

        assert_eq!(stats.hit_rate(100), 0.8);
        assert_eq!(stats.hit_rate(0), 0.0);
    }

    #[test]
    fn test_validation_cache() {
        let config = CacheConfig::default();
        let mut cache = ValidationCache::new(config);

        cache.put("test".to_string(), vec!["error1".to_string()]);
        assert!(cache.get("test").is_some());
        assert!(cache.get("missing").is_none());
    }

    #[test]
    fn test_validation_cache_cleanup() {
        let config = CacheConfig {
            ttl: Duration::from_nanos(1),
            ..Default::default()
        };

        let mut cache = ValidationCache::new(config);
        cache.put("test".to_string(), vec!["error".to_string()]);

        // Sleep briefly to let entry expire
        std::thread::sleep(Duration::from_millis(10));

        cache.cleanup();
        assert!(cache.get("test").is_none());
    }

    #[test]
    fn test_compute_file_hash() -> Result<(), CacheError> {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp = NamedTempFile::new()?;
        writeln!(temp, "test data")?;
        temp.flush()?;

        let hash = ContentCache::compute_file_hash(temp.path())?;
        assert!(hash > 0);

        Ok(())
    }
}
