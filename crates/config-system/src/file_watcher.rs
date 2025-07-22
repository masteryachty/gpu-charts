//! File system watcher for configuration hot-reload

use crate::{ConfigError, Result};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Configuration file change event
#[derive(Debug, Clone)]
pub struct ConfigFileEvent {
    pub path: PathBuf,
    pub event_type: ConfigFileEventType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFileEventType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// File watcher for configuration files
pub struct ConfigFileWatcher {
    /// Watched paths
    watched_paths: Arc<RwLock<Vec<PathBuf>>>,

    /// Event sender
    event_tx: mpsc::UnboundedSender<ConfigFileEvent>,

    /// File system watcher
    watcher: Option<RecommendedWatcher>,

    /// Debounce duration
    debounce_duration: Duration,
}

impl ConfigFileWatcher {
    /// Create a new file watcher
    pub fn new(debounce_ms: u64) -> Result<(Self, mpsc::UnboundedReceiver<ConfigFileEvent>)> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let watcher = Self {
            watched_paths: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            watcher: None,
            debounce_duration: Duration::from_millis(debounce_ms),
        };

        Ok((watcher, event_rx))
    }

    /// Start watching a configuration file or directory
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        // Check if path exists
        if !path.exists() {
            return Err(ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path does not exist: {:?}", path),
            )));
        }

        // Initialize watcher if needed
        if self.watcher.is_none() {
            self.init_watcher()?;
        }

        // Add to watched paths
        {
            let mut paths = self.watched_paths.write();
            if !paths.contains(&path) {
                paths.push(path.clone());
            }
        }

        // Start watching
        if let Some(watcher) = &mut self.watcher {
            let mode = if path.is_dir() {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };

            watcher.watch(&path, mode).map_err(|e| {
                ConfigError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to watch path: {}", e),
                ))
            })?;
        }

        log::info!("Started watching: {:?}", path);
        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref().to_path_buf();

        // Remove from watched paths
        {
            let mut paths = self.watched_paths.write();
            paths.retain(|p| p != &path);
        }

        // Stop watching
        if let Some(watcher) = &mut self.watcher {
            watcher.unwatch(&path).map_err(|e| {
                ConfigError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to unwatch path: {}", e),
                ))
            })?;
        }

        log::info!("Stopped watching: {:?}", path);
        Ok(())
    }

    /// Initialize the file system watcher
    fn init_watcher(&mut self) -> Result<()> {
        let tx = self.event_tx.clone();
        let watched_paths = self.watched_paths.clone();
        let debounce = self.debounce_duration;

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    if let Some(config_event) = Self::process_event(&event, &watched_paths) {
                        let _ = tx.send(config_event);
                    }
                }
                Err(e) => {
                    log::error!("Watch error: {:?}", e);
                }
            },
            Config::default()
                .with_poll_interval(debounce)
                .with_compare_contents(true),
        )
        .map_err(|e| {
            ConfigError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create watcher: {}", e),
            ))
        })?;

        self.watcher = Some(watcher);
        Ok(())
    }

    /// Process a notify event into a config event
    fn process_event(
        event: &Event,
        watched_paths: &Arc<RwLock<Vec<PathBuf>>>,
    ) -> Option<ConfigFileEvent> {
        // Filter for relevant event types
        let event_type = match event.kind {
            EventKind::Create(_) => ConfigFileEventType::Created,
            EventKind::Modify(_) => ConfigFileEventType::Modified,
            EventKind::Remove(_) => ConfigFileEventType::Deleted,
            EventKind::Rename(_) => ConfigFileEventType::Renamed,
            _ => return None,
        };

        // Check if any path is relevant
        for path in &event.paths {
            // Check if it's a config file
            if Self::is_config_file(path) {
                // Check if it's under a watched path
                let paths = watched_paths.read();
                for watched in paths.iter() {
                    if path.starts_with(watched) || path == watched {
                        return Some(ConfigFileEvent {
                            path: path.clone(),
                            event_type,
                        });
                    }
                }
            }
        }

        None
    }

    /// Check if a file is a configuration file
    fn is_config_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            matches!(
                ext.to_str(),
                Some("yaml") | Some("yml") | Some("json") | Some("toml")
            )
        } else {
            false
        }
    }

    /// Get list of watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.read().clone()
    }
}

/// Debounced file watcher that aggregates rapid changes
pub struct DebouncedWatcher {
    /// Inner watcher
    watcher: ConfigFileWatcher,

    /// Pending events
    pending_events: Arc<RwLock<Vec<(std::time::Instant, ConfigFileEvent)>>>,

    /// Debounce duration
    debounce_duration: Duration,
}

impl DebouncedWatcher {
    /// Create a new debounced watcher
    pub fn new(debounce_ms: u64) -> Result<(Self, mpsc::UnboundedReceiver<Vec<ConfigFileEvent>>)> {
        let (watcher, mut event_rx) = ConfigFileWatcher::new(debounce_ms)?;
        let (debounced_tx, debounced_rx) = mpsc::unbounded_channel();

        let pending_events = Arc::new(RwLock::new(Vec::new()));
        let pending_clone = pending_events.clone();
        let debounce_duration = Duration::from_millis(debounce_ms);

        // Spawn debouncing task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));

            loop {
                tokio::select! {
                    Some(event) = event_rx.recv() => {
                        // Add to pending
                        let mut pending = pending_clone.write();
                        pending.push((std::time::Instant::now(), event));
                    }
                    _ = interval.tick() => {
                        // Check for ready events
                        let now = std::time::Instant::now();
                        let mut pending = pending_clone.write();

                        // Collect events that have been pending long enough
                        let ready: Vec<_> = pending
                            .drain(..)
                            .filter(|(time, _)| now.duration_since(*time) >= debounce_duration)
                            .map(|(_, event)| event)
                            .collect();

                        if !ready.is_empty() {
                            // Deduplicate by path
                            let mut deduped = Vec::new();
                            let mut seen_paths = std::collections::HashSet::new();

                            for event in ready.into_iter().rev() {
                                if seen_paths.insert(event.path.clone()) {
                                    deduped.push(event);
                                }
                            }

                            deduped.reverse();

                            if !deduped.is_empty() {
                                let _ = debounced_tx.send(deduped);
                            }
                        }
                    }
                }
            }
        });

        Ok((
            Self {
                watcher,
                pending_events,
                debounce_duration,
            },
            debounced_rx,
        ))
    }

    /// Watch a path
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.watcher.watch(path)
    }

    /// Unwatch a path
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.watcher.unwatch(path)
    }

    /// Get watched paths
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watcher.watched_paths()
    }
}
