//! Minimal plugin host: validates manifests, registers plugins/commands, and stores WASM bytes.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub permissions: Vec<String>,
    pub entrypoint: String,
}

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("invalid manifest")]
    InvalidManifest,
    #[error("io error: {0}")]
    Io(String),
}

pub struct PluginHost {
    allowed_permissions: HashSet<String>,
    loaded: HashMap<String, Manifest>,
    commands: HashSet<String>,
    plugins: HashMap<String, Vec<u8>>,
}

impl PluginHost {
    /// Initialize host with a fixed allowlist of permissions.
    pub fn new() -> Self {
        let allowed = [
            "documents:read",
            "documents:write",
            "documents:create",
            "documents:delete",
            "ui:panel",
            "ui:toolbar",
            "ui:context-menu",
            "commands:register",
            "events:subscribe",
            "media:read",
            "media:annotate",
        ];
        Self {
            allowed_permissions: allowed.iter().map(|s| s.to_string()).collect(),
            loaded: HashMap::new(),
            commands: HashSet::new(),
            plugins: HashMap::new(),
        }
    }

    /// Validate manifest format, API version, and permissions against allowlist.
    pub fn validate_manifest(&self, manifest: &Manifest) -> Result<(), PluginError> {
        if manifest.id.is_empty() || manifest.api_version != "0.1" {
            return Err(PluginError::InvalidManifest);
        }
        for perm in &manifest.permissions {
            if !self.allowed_permissions.contains(perm) {
                return Err(PluginError::PermissionDenied(perm.clone()));
            }
        }
        Ok(())
    }

    /// Load and validate a manifest from disk.
    pub fn load_manifest_from_path(&self, path: impl AsRef<Path>) -> Result<Manifest, PluginError> {
        let raw = fs::read_to_string(path).map_err(|e| PluginError::Io(e.to_string()))?;
        let manifest: Manifest =
            serde_json::from_str(&raw).map_err(|_| PluginError::InvalidManifest)?;
        self.validate_manifest(&manifest)?;
        Ok(manifest)
    }

    /// Register a manifest in-memory after validation.
    pub fn register_plugin(&mut self, manifest: Manifest) -> Result<(), PluginError> {
        self.validate_manifest(&manifest)?;
        self.loaded.insert(manifest.id.clone(), manifest);
        Ok(())
    }

    /// Store plugin bytes (e.g., WASM) keyed by plugin id.
    pub fn load_plugin_bytes(&mut self, id: &str, bytes: Vec<u8>) {
        self.plugins.insert(id.to_string(), bytes);
    }

    /// Return list of loaded plugin ids.
    pub fn loaded_plugins(&self) -> Vec<String> {
        self.loaded.keys().cloned().collect()
    }

    /// Register a new command id for a plugin; rejects duplicates.
    pub fn register_command(&mut self, command_id: &str) -> Result<(), PluginError> {
        if self.commands.contains(command_id) {
            return Err(PluginError::InvalidManifest);
        }
        self.commands.insert(command_id.to_string());
        Ok(())
    }
}
