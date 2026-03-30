#![allow(dead_code, unused_imports, unused_variables)]
//! Config file watcher using the `notify` crate.
//! Sends a ConfigReloaded event on the app bus when any config file changes.
use std::path::PathBuf;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Config as NotifyConfig};
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
}

impl ConfigWatcher {
    /// Spawn a watcher on the given config paths. Returns a receiver of change events.
    pub fn spawn(paths: Vec<PathBuf>) -> (Self, mpsc::UnboundedReceiver<PathBuf>) {
        let (tx, rx) = mpsc::unbounded_channel();

        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            match res {
                Ok(event) => {
                    for path in event.paths {
                        let _ = tx.send(path);
                    }
                }
                Err(e) => warn!("Config watch error: {e}"),
            }
        });

        let mut watcher = match watcher {
            Ok(w) => w,
            Err(e) => {
                warn!("Failed to create config watcher: {e}");
                // Return a dummy watcher that never fires
                let dummy = notify::recommended_watcher(move |_| {})
                    .unwrap_or_else(|_| panic!("failed to create dummy watcher"));
                return (Self { _watcher: dummy }, rx);
            }
        };

        for path in &paths {
            if path.exists() {
                if let Err(e) = watcher.watch(path, RecursiveMode::NonRecursive) {
                    warn!("Failed to watch {}: {e}", path.display());
                }
            }
        }

        info!("Config watcher started on {} paths", paths.len());
        (Self { _watcher: watcher }, rx)
    }
}
