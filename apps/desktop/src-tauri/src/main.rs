#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! Tauri desktop entrypoint: wires store + sync + trust + plugin host, exposes Tauri commands,
//! manages config/oplog/trust/device identity, and launches background sync/discovery loops.

use chrono::Utc;
use notes_core::Document;
use notes_oplog::{Operation, OperationType};
use notes_plugin_host::PluginHost;
use notes_store::{DocumentSummary, Store};
use notes_sync::{DeviceIdentity, SyncService, TrustStore, TrustedDevice};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppConfig {
    vault_root: String,
    #[serde(default = "default_discovery_port")]
    discovery_port: u16,
    #[serde(default = "default_sync_port")]
    sync_port: u16,
    #[serde(default = "default_auto_sync")]
    auto_sync_enabled: bool,
    #[serde(default)]
    transport_secret: Option<String>, // hex-encoded 32-byte PSK
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            vault_root: resolve_default_vault_root(),
            discovery_port: default_discovery_port(),
            sync_port: default_sync_port(),
            auto_sync_enabled: default_auto_sync(),
            transport_secret: None,
        }
    }
}

#[derive(Clone)]
struct OpLogStore {
    path: PathBuf,
    entries: Vec<Operation>,
    seen: HashSet<String>,
}

impl OpLogStore {
    /// Load op-log from disk and prime the dedup set; falls back to an empty log.
    fn load(path: PathBuf) -> Self {
        if path.exists() {
            if let Ok(mut file) = File::open(&path) {
                let mut buf = String::new();
                if file.read_to_string(&mut buf).is_ok() {
                    if let Ok(entries) = serde_json::from_str::<Vec<Operation>>(&buf) {
                        let seen = entries.iter().map(|o| o.key()).collect();
                        return Self {
                            path,
                            entries,
                            seen,
                        };
                    }
                }
            }
        }
        Self {
            path,
            entries: Vec::new(),
            seen: HashSet::new(),
        }
    }

    /// Persist op-log to disk.
    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_string_pretty(&self.entries).map_err(|e| e.to_string())?;
        let mut file = File::create(&self.path).map_err(|e| e.to_string())?;
        file.write_all(data.as_bytes()).map_err(|e| e.to_string())
    }

    /// Add a single op if not already seen; returns whether it was inserted.
    fn add(&mut self, op: Operation) -> Result<bool, String> {
        let key = op.key();
        if self.seen.contains(&key) {
            return Ok(false);
        }
        self.seen.insert(key);
        self.entries.push(op);
        self.save()?;
        Ok(true)
    }

    fn merge(&mut self, incoming: &[Operation]) -> Result<Vec<Operation>, String> {
        let mut accepted = Vec::new();
        for op in incoming {
            let key = op.key();
            if self.seen.contains(&key) {
                continue;
            }
            self.seen.insert(key);
            self.entries.push(op.clone());
            accepted.push(op.clone());
        }
        if !accepted.is_empty() {
            self.save()?;
        }
        Ok(accepted)
    }

    /// Return all known ops (for sync payloads).
    fn entries(&self) -> Vec<Operation> {
        self.entries.clone()
    }

    fn contains(&self, op: &Operation) -> bool {
        self.seen.contains(&op.key())
    }
}

struct AppState {
    store: Arc<Mutex<Store>>,
    plugins: Mutex<PluginHost>,
    config: Mutex<AppConfig>,
    config_path: PathBuf,
    device_identity: DeviceIdentity,
    trust_store: Arc<Mutex<TrustStore>>,
    op_log: Arc<Mutex<OpLogStore>>,
    auto_sync_enabled: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Debug, Serialize)]
struct PeerView {
    addr: String,
    device_id: Option<String>,
    public_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NetworkSettings {
    discovery_port: u16,
    sync_port: u16,
    transport_secret: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDocRequest {
    pub frontmatter: serde_yaml::Value,
    pub body: String,
}

#[tauri::command]
fn health_check() -> &'static str {
    "ok"
}

#[tauri::command]
fn create_document(
    state: tauri::State<AppState>,
    req: CreateDocRequest,
) -> Result<Document, String> {
    let mut fm = req.frontmatter;
    let document_id = match fm {
        serde_yaml::Value::Mapping(ref map) => map
            .get(&serde_yaml::Value::String("id".into()))
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| ulid::Ulid::new().to_string()),
        _ => ulid::Ulid::new().to_string(),
    };

    // ensure id is present
    if let serde_yaml::Value::Mapping(ref mut map) = fm {
        map.insert(
            serde_yaml::Value::String("id".into()),
            serde_yaml::Value::String(document_id.clone()),
        );
    }

    let payload = serde_json::json!({
        "frontmatter": fm,
        "body": req.body,
    });

    let mut op = Operation {
        op_id: ulid::Ulid::new().to_string(),
        device_id: state.device_identity.device_id.clone(),
        timestamp: Utc::now().to_rfc3339(),
        op_type: OperationType::CreateDocument,
        document_id: document_id.clone(),
        payload,
        before_hash: None,
        after_hash: None,
    };

    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    let res = store.apply(op.clone()).map_err(|e| e.to_string())?;
    if let Some(doc) = res.clone() {
        op.after_hash = Some(doc.hash_content());
        if let Ok(mut log) = state.op_log.lock() {
            let _ = log.add(op);
        }
        return Ok(doc);
    }
    Err("failed to create document".into())
}

#[tauri::command]
fn load_plugin_manifest(state: tauri::State<AppState>, path: String) -> Result<(), String> {
    let mut host = state.plugins.lock().map_err(|e| e.to_string())?;
    let manifest = host
        .load_manifest_from_path(&path)
        .map_err(|e| e.to_string())?;
    host.register_plugin(manifest).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_documents(state: tauri::State<AppState>) -> Result<Vec<DocumentSummary>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store.list_documents().map_err(|e| e.to_string())
}

#[tauri::command]
fn search_documents(
    state: tauri::State<AppState>,
    query: String,
) -> Result<Vec<DocumentSummary>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store.search_documents(&query).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_document(state: tauri::State<AppState>, id: String) -> Result<Document, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store
        .load_document(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "not found".into())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateDocRequest {
    pub id: String,
    pub frontmatter: serde_yaml::Value,
    pub body: String,
    pub before_hash: Option<String>,
}

#[tauri::command]
fn update_document(
    state: tauri::State<AppState>,
    req: UpdateDocRequest,
) -> Result<Document, String> {
    let payload = serde_json::json!({
        "frontmatter": req.frontmatter,
        "body": req.body,
    });
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    let before_hash = store
        .load_document(&req.id)
        .map_err(|e| e.to_string())?
        .map(|d| d.hash_content())
        .or(req.before_hash.clone());
    let mut op = Operation {
        op_id: ulid::Ulid::new().to_string(),
        device_id: state.device_identity.device_id.clone(),
        timestamp: Utc::now().to_rfc3339(),
        op_type: OperationType::UpdateDocument,
        document_id: req.id.clone(),
        payload,
        before_hash,
        after_hash: None,
    };

    let doc = store
        .update_document(op.clone())
        .map_err(|e| e.to_string())?;
    op.after_hash = Some(doc.hash_content());
    if let Ok(mut log) = state.op_log.lock() {
        let _ = log.add(op);
    }
    Ok(doc)
}

#[tauri::command]
fn delete_document(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    let before_hash = store
        .load_document(&id)
        .map_err(|e| e.to_string())?
        .map(|d| d.hash_content());
    store.delete_document(&id).map_err(|e| e.to_string())?;
    if let Ok(mut log) = state.op_log.lock() {
        let _ = log.add(Operation {
            op_id: ulid::Ulid::new().to_string(),
            device_id: state.device_identity.device_id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            op_type: OperationType::DeleteDocument,
            document_id: id,
            payload: serde_json::Value::Null,
            before_hash,
            after_hash: None,
        });
    }
    Ok(())
}

#[tauri::command]
fn get_vault_root(state: tauri::State<AppState>) -> Result<String, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    Ok(store.root_path().to_string_lossy().to_string())
}

#[tauri::command]
fn set_vault_root(state: tauri::State<AppState>, path: String) -> Result<String, String> {
    let mut store = state.store.lock().map_err(|e| e.to_string())?;
    *store = Store::with_root(&path).map_err(|e| e.to_string())?;
    {
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        cfg.vault_root = path.clone();
        save_config(&state.config_path, &cfg).map_err(|e| e.to_string())?;
    }
    Ok(store.root_path().to_string_lossy().to_string())
}

#[tauri::command]
fn get_device_identity(state: tauri::State<AppState>) -> Result<DeviceIdentity, String> {
    Ok(state.device_identity.clone())
}

#[tauri::command]
fn list_trusted_devices(state: tauri::State<AppState>) -> Result<Vec<TrustedDevice>, String> {
    let trust = state.trust_store.lock().map_err(|e| e.to_string())?;
    Ok(trust.list().to_vec())
}

#[tauri::command]
fn discover_peers(state: tauri::State<AppState>) -> Result<Vec<PeerView>, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?.clone();
    let transport =
        notes_sync::NetTransport::new_with_psk(cfg.discovery_port, cfg.sync_port, psk_from_config(&cfg));
    let peers = transport.listen_discovery(std::time::Duration::from_millis(300));
    let peers = match peers {
        Ok(peers) => peers,
        Err(_) => Vec::new(), // treat discovery failures as "no peers" to avoid UI errors
    };
    Ok(peers
        .into_iter()
        .map(|p| PeerView {
            addr: p.addr.to_string(),
            device_id: p.device_id,
            public_key: p.public_key,
        })
        .collect())
}

fn start_sync_listener(
    store: Arc<Mutex<Store>>,
    op_log: Arc<Mutex<OpLogStore>>,
    trust_path: PathBuf,
    sync_port: u16,
    psk: Option<[u8; 32]>,
) {
    // Runs a tiny TCP listener loop that decrypts/verifies incoming envelopes, checks trust,
    // applies only new ops to the store, and persists them to the op-log.
    std::thread::spawn(move || loop {
        let transport = notes_sync::NetTransport::new_with_psk(0, sync_port, psk);
        let _ = transport.serve_once(&trust_path, |incoming_ops| {
            if let (Ok(mut log), Ok(mut store_guard)) = (op_log.lock(), store.lock()) {
                let fresh: Vec<_> = incoming_ops
                    .into_iter()
                    .filter(|op| !log.contains(op))
                    .collect();
                let mut applied = Vec::new();
                for op in fresh {
                    if store_guard.apply(op.clone()).is_ok() {
                        applied.push(op);
                    }
                }
                let _ = log.merge(&applied);
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(200));
    });
}

fn start_auto_sync(
    op_log: Arc<Mutex<OpLogStore>>,
    trust_store: Arc<Mutex<TrustStore>>,
    device_identity: DeviceIdentity,
    enabled: Arc<std::sync::atomic::AtomicBool>,
    psk: Option<[u8; 32]>,
    discovery_port: u16,
    sync_port: u16,
) {
    // Periodically discovers peers and pushes ops to trusted+auto-approved devices.
    std::thread::spawn(move || loop {
        if !enabled.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_secs(2));
            continue;
        }
        let transport = notes_sync::NetTransport::new_with_psk(discovery_port, sync_port, psk);
    let peers = transport
        .listen_discovery(std::time::Duration::from_millis(250))
        .unwrap_or_default();
        let ops = op_log.lock().map(|l| l.entries()).unwrap_or_default();
        for peer in peers {
            let trusted = peer
                .device_id
                .as_ref()
                .and_then(|id| peer.public_key.as_ref().map(|pk| (id, pk)))
                .map(|(id, pk)| {
                    trust_store
                        .lock()
                        .map(|t| t.is_trusted_for_auto(id, pk))
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            if !trusted {
                continue;
            }
            let svc = SyncService::new(transport.clone(), device_identity.clone(), ops.clone());
            let _ = svc.pair_and_sync(&peer.addr.to_string());
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    });
}

#[tauri::command]
fn add_trusted_device(
    state: tauri::State<AppState>,
    device_id: String,
    public_key: String,
) -> Result<(), String> {
    let mut trust = state.trust_store.lock().map_err(|e| e.to_string())?;
    trust.add(device_id, public_key).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_trusted_device(state: tauri::State<AppState>, device_id: String) -> Result<(), String> {
    let mut trust = state.trust_store.lock().map_err(|e| e.to_string())?;
    trust.remove(&device_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_auto_sync_enabled(state: tauri::State<AppState>) -> Result<bool, String> {
    Ok(state
        .auto_sync_enabled
        .load(std::sync::atomic::Ordering::Relaxed))
}

#[tauri::command]
fn set_auto_sync_enabled(state: tauri::State<AppState>, enabled: bool) -> Result<bool, String> {
    state
        .auto_sync_enabled
        .store(enabled, std::sync::atomic::Ordering::Relaxed);
    {
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        cfg.auto_sync_enabled = enabled;
        save_config(&state.config_path, &cfg).map_err(|e| e.to_string())?;
    }
    Ok(enabled)
}

#[tauri::command]
fn set_trusted_auto_sync(
    state: tauri::State<AppState>,
    device_id: String,
    allow: bool,
) -> Result<(), String> {
    let mut trust = state.trust_store.lock().map_err(|e| e.to_string())?;
    trust
        .set_auto_sync(&device_id, allow)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_network_config(state: tauri::State<AppState>) -> Result<NetworkSettings, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?.clone();
    Ok(NetworkSettings {
        discovery_port: cfg.discovery_port,
        sync_port: cfg.sync_port,
        transport_secret: cfg.transport_secret,
    })
}

#[tauri::command]
fn set_network_config(
    state: tauri::State<AppState>,
    discovery_port: u16,
    sync_port: u16,
    transport_secret: Option<String>,
) -> Result<NetworkSettings, String> {
    {
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        cfg.discovery_port = discovery_port;
        cfg.sync_port = sync_port;
        cfg.transport_secret = transport_secret.clone();
        save_config(&state.config_path, &cfg).map_err(|e| e.to_string())?;
    }
    Ok(NetworkSettings {
        discovery_port,
        sync_port,
        transport_secret,
    })
}

#[tauri::command]
fn sync_now(state: tauri::State<AppState>, target_device: String) -> Result<(), String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?.clone();
    let transport =
        notes_sync::NetTransport::new_with_psk(cfg.discovery_port, cfg.sync_port, psk_from_config(&cfg));
    let ops = state
        .op_log
        .lock()
        .map_err(|e| e.to_string())?
        .entries();
    let service = SyncService::new(transport, state.device_identity.clone(), ops);
    service
        .pair_and_sync(&target_device)
        .map_err(|e| e.to_string())
}

fn load_config(path: &PathBuf) -> Result<AppConfig, String> {
    if path.exists() {
        let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).map_err(|e| e.to_string())
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        Ok(AppConfig::default())
    }
}

fn save_config(path: &PathBuf, cfg: &AppConfig) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, raw).map_err(|e| e.to_string())
}

fn resolve_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("notes-desktop")
        .join("config.json")
}

fn resolve_default_vault_root() -> String {
    let base = dirs::data_dir()
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")));
    base.join("notes-desktop")
        .join("vault")
        .to_string_lossy()
        .to_string()
}

fn resolve_device_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("notes-desktop")
        .join("device.json")
}

fn resolve_trust_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("notes-desktop")
        .join("trust.json")
}

fn resolve_oplog_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")))
        .join("notes-desktop")
        .join("oplog.json")
}

fn default_discovery_port() -> u16 {
    53333
}

fn default_sync_port() -> u16 {
    53334
}

fn default_auto_sync() -> bool {
    false
}

fn psk_from_config(cfg: &AppConfig) -> Option<[u8; 32]> {
    cfg.transport_secret.as_ref().and_then(|s| {
        hex::decode(s)
            .ok()
            .and_then(|bytes| {
                if bytes.len() == 32 {
                    let mut key = [0u8; 32];
                    key.copy_from_slice(&bytes);
                    Some(key)
                } else {
                    None
                }
            })
    })
}

fn main() {
    let config_path = resolve_config_path();
    let config = load_config(&config_path).unwrap_or_else(|_| AppConfig::default());
    let store = Store::with_root(&config.vault_root)
        .or_else(|_| Store::with_root(resolve_default_vault_root()))
        .expect("failed to initialize store");
    let device_identity =
        DeviceIdentity::load_or_create(resolve_device_path()).expect("failed to load device id");
    let trust_store =
        TrustStore::load_or_default(resolve_trust_path()).expect("failed to load trust store");
    let psk = psk_from_config(&config);
    notes_sync::NetTransport::new_with_psk(config.discovery_port, config.sync_port, psk)
        .advertise_loop_with_identity(device_identity.clone(), std::time::Duration::from_secs(5));
    let store = Arc::new(Mutex::new(store));
    let trust_store = Arc::new(Mutex::new(trust_store));
    let op_log: Arc<Mutex<OpLogStore>> =
        Arc::new(Mutex::new(OpLogStore::load(resolve_oplog_path())));
    let auto_sync_enabled =
        Arc::new(std::sync::atomic::AtomicBool::new(config.auto_sync_enabled));
    start_sync_listener(
        store.clone(),
        op_log.clone(),
        resolve_trust_path(),
        config.sync_port,
        psk,
    );
    start_auto_sync(
        op_log.clone(),
        trust_store.clone(),
        device_identity.clone(),
        auto_sync_enabled.clone(),
        psk,
        config.discovery_port,
        config.sync_port,
    );

    tauri::Builder::default()
        .manage(AppState {
            store,
            plugins: Mutex::new(PluginHost::new()),
            config: Mutex::new(config),
            config_path,
            device_identity,
            trust_store,
            op_log,
            auto_sync_enabled,
        })
        .invoke_handler(tauri::generate_handler![
            health_check,
            create_document,
            load_plugin_manifest,
            list_documents,
            search_documents,
            get_document,
            update_document,
            delete_document,
            get_vault_root,
            set_vault_root,
            get_device_identity,
            list_trusted_devices,
            add_trusted_device,
            remove_trusted_device,
            get_auto_sync_enabled,
            set_auto_sync_enabled,
            set_trusted_auto_sync,
            get_network_config,
            set_network_config,
            sync_now,
            discover_peers
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
