#![allow(dead_code, unused_imports, unused_variables)]
//! BYOK (Bring Your Own Key) secure key vault.
//! Keys stored in OS keychain (macOS Keychain, Windows Credential Manager, Linux libsecret).
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "rmtide";

/// Metadata stored alongside each key (in-memory only, not in keychain).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMeta {
    pub provider: String,
    pub label: String,           // "work", "personal", etc.
    pub last_used: Option<u64>,  // Unix timestamp
    pub created_at: u64,         // Unix timestamp
    pub rotation_days: u32,      // 0 = no reminder
}

impl KeyMeta {
    pub fn new(provider: &str, label: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            provider: provider.to_string(),
            label: label.to_string(),
            last_used: None,
            created_at: now,
            rotation_days: 90,
        }
    }

    /// Age in days.
    pub fn age_days(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        (now - self.created_at) / 86400
    }

    /// True if the key is overdue for rotation.
    pub fn needs_rotation(&self) -> bool {
        self.rotation_days > 0 && self.age_days() >= self.rotation_days as u64
    }

    /// Masked display — shows last 4 chars only.
    pub fn mask(key: &str) -> String {
        if key.len() <= 4 {
            return "****".to_string();
        }
        format!("{}...{}", &"*".repeat(8), &key[key.len()-4..])
    }
}

/// Named keychain entry: (provider, label) -> key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyId {
    pub provider: String,
    pub label: String,
}

impl KeyId {
    pub fn new(provider: &str, label: &str) -> Self {
        Self { provider: provider.to_string(), label: label.to_string() }
    }
    /// The keychain username (account field).
    fn account(&self) -> String {
        format!("{}:{}", self.provider, self.label)
    }
}

/// The in-process key vault. Keys are fetched from the OS keychain on demand.
pub struct KeyVault {
    /// In-memory metadata (does NOT contain raw keys).
    meta: RwLock<HashMap<String, KeyMeta>>,
}

impl KeyVault {
    pub fn new() -> Self {
        Self { meta: RwLock::new(HashMap::new()) }
    }

    /// Store a key in the OS keychain and record metadata.
    pub fn set_key(&self, id: &KeyId, key: &str, label: &str) -> anyhow::Result<()> {
        let entry = keyring::Entry::new(SERVICE, &id.account())?;
        entry.set_password(key)?;
        let meta = KeyMeta::new(&id.provider, label);
        self.meta.write().insert(id.account(), meta);
        Ok(())
    }

    /// Retrieve a key from the OS keychain.
    pub fn get_key(&self, id: &KeyId) -> Option<String> {
        let entry = keyring::Entry::new(SERVICE, &id.account()).ok()?;
        let key = entry.get_password().ok()?;
        // Update last_used timestamp
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if let Some(m) = self.meta.write().get_mut(&id.account()) {
            m.last_used = Some(now);
        }
        Some(key)
    }

    /// Convenience: get key for provider using "default" label.
    pub fn get_provider_key(&self, provider: &str) -> Option<String> {
        self.get_key(&KeyId::new(provider, "default"))
    }

    /// Delete a key from the OS keychain.
    pub fn delete_key(&self, id: &KeyId) -> anyhow::Result<()> {
        let entry = keyring::Entry::new(SERVICE, &id.account())?;
        entry.delete_password()?;
        self.meta.write().remove(&id.account());
        Ok(())
    }

    /// List all registered key IDs (from in-memory metadata).
    pub fn list_keys(&self) -> Vec<(KeyId, KeyMeta)> {
        self.meta.read().iter().map(|(account, meta)| {
            let parts: Vec<&str> = account.splitn(2, ':').collect();
            let id = KeyId {
                provider: parts.get(0).unwrap_or(&"").to_string(),
                label: parts.get(1).unwrap_or(&"default").to_string(),
            };
            (id, meta.clone())
        }).collect()
    }

    /// Seed from existing config keys (backwards-compatible migration).
    pub fn seed_from_config(&self, provider: &str, key: Option<&str>) {
        if let Some(k) = key {
            if !k.is_empty() {
                let id = KeyId::new(provider, "default");
                let _ = self.set_key(&id, k, "default");
            }
        }
    }

    /// Keys that need rotation.
    pub fn stale_keys(&self) -> Vec<(KeyId, u64)> {
        self.meta.read().iter()
            .filter(|(_, m)| m.needs_rotation())
            .map(|(account, meta)| {
                let parts: Vec<&str> = account.splitn(2, ':').collect();
                let id = KeyId {
                    provider: parts.get(0).unwrap_or(&"").to_string(),
                    label: parts.get(1).unwrap_or(&"default").to_string(),
                };
                (id, meta.age_days())
            })
            .collect()
    }
}

impl Default for KeyVault {
    fn default() -> Self { Self::new() }
}
