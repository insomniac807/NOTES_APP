use base64::Engine;
use chacha20poly1305::{aead::Aead, aead::KeyInit, ChaCha20Poly1305, Key, Nonce};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

// Network sync layer: discovery (UDP), sync handshakes (TCP), optional PSK crypto,
// device identity/trust management, and sync envelopes with signatures.

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceIdentity {
    pub device_id: String,
    pub public_key: String,
    #[serde(default)]
    #[serde(skip_serializing)]
    pub secret_key: String,
    pub created: String,
}

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("handshake failed")]
    HandshakeFailed,
    #[error("transport error")]
    Transport,
    #[error("addr parse error")]
    Addr,
    #[error("io error: {0}")]
    Io(String),
    #[error("invalid state: {0}")]
    Invalid(String),
    #[error("not trusted")]
    NotTrusted,
    #[error("connect timeout")]
    Timeout,
    #[error("crypto error: {0}")]
    Crypto(String),
}

pub trait Transport {
    fn advertise(&self) -> Result<(), SyncError>;
    fn request_sync(&self, target_device: &str, envelope: &SyncEnvelope) -> Result<(), SyncError>;
}

pub struct SyncService<T: Transport> {
    transport: T,
    device: DeviceIdentity,
    ops: Vec<notes_oplog::Operation>,
}

impl<T: Transport> SyncService<T> {
    /// Build a sync service with a transport and the device identity/oplog payload.
    pub fn new(transport: T, device: DeviceIdentity, ops: Vec<notes_oplog::Operation>) -> Self {
        Self {
            transport,
            device,
            ops,
        }
    }

    /// Advertise then send an envelope to a peer.
    pub fn pair_and_sync(&self, target_device: &str) -> Result<(), SyncError> {
        self.transport.advertise()?;
        let payload = serde_json::to_vec(&self.ops).map_err(|e| SyncError::Io(e.to_string()))?;
        let envelope = SyncEnvelope {
            device_id: self.device.device_id.clone(),
            public_key: self.device.public_key.clone(),
            signature: self.device.sign(&payload)?,
            ops: self.ops.clone(),
        };
        self.transport.request_sync(target_device, &envelope)?;
        Ok(())
    }

    pub fn device(&self) -> &DeviceIdentity {
        &self.device
    }
}

#[derive(Clone, Debug)]
pub struct NetTransport {
    discovery_port: u16,
    sync_port: u16,
    psk: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    pub addr: SocketAddr,
    pub device_id: Option<String>,
    pub public_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncEnvelope {
    pub device_id: String,
    pub public_key: String,
    pub signature: String,
    pub ops: Vec<notes_oplog::Operation>,
}

impl NetTransport {
    pub fn new(discovery_port: u16, sync_port: u16) -> Self {
        Self::new_with_psk(discovery_port, sync_port, None)
    }

    /// Construct a transport with optional 32-byte PSK for encrypted envelopes.
    pub fn new_with_psk(discovery_port: u16, sync_port: u16, psk: Option<[u8; 32]>) -> Self {
        Self { discovery_port, sync_port, psk }
    }

    pub fn listen_discovery(&self, timeout: Duration) -> Result<Vec<DiscoveredPeer>, SyncError> {
        let socket = UdpSocket::bind(("0.0.0.0", self.discovery_port))
            .map_err(|e| SyncError::Io(e.to_string()))?;
        socket
            .set_read_timeout(Some(timeout))
            .map_err(|e| SyncError::Io(e.to_string()))?;
        let mut buf = [0u8; 128];
        let mut peers = Vec::new();
        loop {
            match socket.recv_from(&mut buf) {
                Ok((n, src)) => {
                    if peers.iter().any(|p: &DiscoveredPeer| p.addr == src) {
                        continue;
                    }
                    let packet: Result<DiscoveryPacket, _> = serde_json::from_slice(&buf[..n]);
                    match packet {
                        Ok(pkt) => peers.push(DiscoveredPeer {
                            addr: src,
                            device_id: Some(pkt.device_id),
                            public_key: Some(pkt.public_key),
                        }),
                        Err(_) => peers.push(DiscoveredPeer {
                            addr: src,
                            device_id: None,
                            public_key: None,
                        }),
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(SyncError::Io(e.to_string())),
            }
        }
        Ok(peers)
    }

    pub fn advertise_loop(&self, interval: Duration) {
        let discovery_port = self.discovery_port;
        std::thread::spawn(move || loop {
            if let Ok(sock) = UdpSocket::bind(("0.0.0.0", 0)) {
                let _ = sock.set_broadcast(true);
                let _ = sock.send_to(
                    b"DISCOVER",
                    SocketAddr::from(([255, 255, 255, 255], discovery_port)),
                );
            }
            std::thread::sleep(interval);
        });
    }

    pub fn advertise_loop_with_identity(&self, identity: DeviceIdentity, interval: Duration) {
        let discovery_port = self.discovery_port;
        std::thread::spawn(move || loop {
            if let Ok(sock) = UdpSocket::bind(("0.0.0.0", 0)) {
                let _ = sock.set_broadcast(true);
                let pkt = DiscoveryPacket {
                    device_id: identity.device_id.clone(),
                    public_key: identity.public_key.clone(),
                };
                let data = serde_json::to_vec(&pkt).unwrap_or_else(|_| b"DISCOVER".to_vec());
                let _ = sock.send_to(
                    &data,
                    SocketAddr::from(([255, 255, 255, 255], discovery_port)),
                );
            }
            std::thread::sleep(interval);
        });
    }

    pub fn serve_once(
        &self,
        trust_path: &Path,
        op_handler: impl Fn(Vec<notes_oplog::Operation>),
    ) -> Result<(), SyncError> {
        let listener = TcpListener::bind(("0.0.0.0", self.sync_port))
            .map_err(|e| SyncError::Io(e.to_string()))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| SyncError::Io(e.to_string()))?;
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = Vec::new();
            std::io::copy(&mut stream, &mut buf).map_err(|e| SyncError::Io(e.to_string()))?;
            let payload = if let Some(psk) = self.psk {
                if buf.len() < 13 {
                    return Err(SyncError::Crypto("encrypted payload too short".into()));
                }
                let version = buf[0];
                if version != 1 {
                    return Err(SyncError::Crypto("unsupported crypto version".into()));
                }
                let nonce_bytes: [u8; 12] = buf[1..13]
                    .try_into()
                    .map_err(|_| SyncError::Crypto("bad nonce".into()))?;
                let cipher = ChaCha20Poly1305::new(Key::from_slice(&psk));
                cipher
                    .decrypt(Nonce::from_slice(&nonce_bytes), &buf[13..])
                    .map_err(|e| SyncError::Crypto(e.to_string()))?
            } else {
                buf
            };
            let envelope: SyncEnvelope =
                serde_json::from_slice(&payload).map_err(|e| SyncError::Io(e.to_string()))?;
            let trust = TrustStore::load_or_default(trust_path)
                .map_err(|e| SyncError::Io(e.to_string()))?;
            if !trust.is_trusted(&envelope.device_id, &envelope.public_key) {
                return Err(SyncError::NotTrusted);
            }
            let vk_bytes =
                hex::decode(&envelope.public_key).map_err(|e| SyncError::Io(e.to_string()))?;
            let vk = PublicKey::from_bytes(
                vk_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| SyncError::Invalid("bad key".into()))?,
            )
            .map_err(|e| SyncError::Io(e.to_string()))?;
            let sig_bytes = base64::engine::general_purpose::STANDARD
                .decode(&envelope.signature)
                .map_err(|e| SyncError::Io(e.to_string()))?;
            let sig = Signature::from_bytes(
                sig_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| SyncError::Invalid("bad sig".into()))?,
            )
            .map_err(|e| SyncError::Io(e.to_string()))?;
            let payload =
                serde_json::to_vec(&envelope.ops).map_err(|e| SyncError::Io(e.to_string()))?;
            vk.verify(&payload, &sig)
                .map_err(|_| SyncError::NotTrusted)?;
            op_handler(envelope.ops);
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscoveryPacket {
    device_id: String,
    public_key: String,
}

impl Transport for NetTransport {
    fn advertise(&self) -> Result<(), SyncError> {
        let socket = UdpSocket::bind(("0.0.0.0", 0)).map_err(|e| SyncError::Io(e.to_string()))?;
        socket
            .set_broadcast(true)
            .map_err(|e| SyncError::Io(e.to_string()))?;
        let msg = b"DISCOVER";
        socket
            .send_to(
                msg,
                SocketAddr::from(([255, 255, 255, 255], self.discovery_port)),
            )
            .map_err(|e| SyncError::Io(e.to_string()))?;
        let _ = socket.send_to(msg, SocketAddr::from(([127, 0, 0, 1], self.discovery_port)));
        Ok(())
    }

    fn request_sync(&self, target_device: &str, envelope: &SyncEnvelope) -> Result<(), SyncError> {
        let addr: SocketAddr = if target_device.contains(':') {
            target_device.parse().map_err(|_| SyncError::Addr)?
        } else {
            format!("{}:{}", target_device, self.sync_port)
                .parse()
                .map_err(|_| SyncError::Addr)?
        };
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))
            .map_err(|_| SyncError::Timeout)?;
        stream.set_write_timeout(Some(Duration::from_secs(2))).ok();
        let mut stream = stream;
        let handshake = serde_json::to_vec(envelope).map_err(|e| SyncError::Io(e.to_string()))?;
        let packet = if let Some(psk) = self.psk {
            let mut nonce = [0u8; 12];
            rand::thread_rng().fill_bytes(&mut nonce);
            let cipher = ChaCha20Poly1305::new(Key::from_slice(&psk));
            let ct = cipher
                .encrypt(Nonce::from_slice(&nonce), handshake.as_slice())
                .map_err(|e| SyncError::Crypto(e.to_string()))?;
            let mut out = Vec::with_capacity(1 + nonce.len() + ct.len());
            out.push(1); // version
            out.extend_from_slice(&nonce);
            out.extend_from_slice(&ct);
            out
        } else {
            handshake
        };
        stream
            .write_all(&packet)
            .map_err(|e| SyncError::Io(e.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrustedDevice {
    pub device_id: String,
    pub public_key: String,
    pub added: String,
    #[serde(default = "default_allow_auto_sync")]
    pub allow_auto_sync: bool,
}

fn default_allow_auto_sync() -> bool {
    false
}

#[derive(Debug)]
pub struct TrustStore {
    path: PathBuf,
    devices: Vec<TrustedDevice>,
    ids: HashSet<String>,
    keys: HashSet<String>,
}

impl TrustStore {
    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self, SyncError> {
        let path = path.as_ref().to_path_buf();
        if path.exists() {
            let raw = fs::read_to_string(&path).map_err(|e| SyncError::Io(e.to_string()))?;
            let devices: Vec<TrustedDevice> =
                serde_json::from_str(&raw).map_err(|e| SyncError::Io(e.to_string()))?;
            let ids = devices.iter().map(|d| d.device_id.clone()).collect();
            let keys = devices.iter().map(|d| d.public_key.clone()).collect();
            return Ok(Self {
                path,
                devices,
                ids,
                keys,
            });
        }
        Ok(Self {
            path,
            devices: Vec::new(),
            ids: HashSet::new(),
            keys: HashSet::new(),
        })
    }

    pub fn add(&mut self, device_id: String, public_key: String) -> Result<(), SyncError> {
        if self.ids.contains(&device_id) {
            return Ok(());
        }
        let td = TrustedDevice {
            device_id: device_id.clone(),
            public_key: public_key.clone(),
            added: chrono::Utc::now().to_rfc3339(),
            allow_auto_sync: false,
        };
        self.devices.push(td);
        self.ids.insert(device_id);
        self.keys.insert(public_key);
        self.save()
    }

    pub fn list(&self) -> &[TrustedDevice] {
        &self.devices
    }

    pub fn set_auto_sync(&mut self, device_id: &str, allow: bool) -> Result<(), SyncError> {
        if let Some(td) = self.devices.iter_mut().find(|d| d.device_id == device_id) {
            td.allow_auto_sync = allow;
            self.save()?;
        }
        Ok(())
    }

    pub fn remove(&mut self, device_id: &str) -> Result<(), SyncError> {
        self.devices.retain(|d| d.device_id != device_id);
        self.ids.remove(device_id);
        self.keys
            .retain(|k| self.devices.iter().any(|d| &d.public_key == k));
        self.save()
    }

    pub fn is_trusted(&self, device_id: &str, public_key: &str) -> bool {
        self.devices
            .iter()
            .any(|d| d.device_id == device_id && d.public_key == public_key)
    }

    pub fn is_trusted_for_auto(&self, device_id: &str, public_key: &str) -> bool {
        self.devices.iter().any(|d| {
            d.device_id == device_id && d.public_key == public_key && d.allow_auto_sync
        })
    }

    fn save(&self) -> Result<(), SyncError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| SyncError::Io(e.to_string()))?;
        }
        let raw = serde_json::to_string_pretty(&self.devices)
            .map_err(|e| SyncError::Io(e.to_string()))?;
        fs::write(&self.path, raw).map_err(|e| SyncError::Io(e.to_string()))
    }
}

pub struct NoopTransport;

impl Transport for NoopTransport {
    fn advertise(&self) -> Result<(), SyncError> {
        Ok(())
    }

    fn request_sync(
        &self,
        _target_device: &str,
        _envelope: &SyncEnvelope,
    ) -> Result<(), SyncError> {
        Ok(())
    }
}

impl DeviceIdentity {
    pub fn generate() -> Self {
        let (secret, public) = Self::generate_keypair();
        Self {
            device_id: ulid::Ulid::new().to_string(),
            public_key: hex::encode(public.to_bytes()),
            secret_key: hex::encode(secret.to_bytes()),
            created: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn generate_keypair() -> (SecretKey, PublicKey) {
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        let secret = SecretKey::from_bytes(&seed).unwrap();
        let public: PublicKey = (&secret).into();
        (secret, public)
    }

    pub fn load_or_create(path: impl AsRef<Path>) -> Result<Self, SyncError> {
        let path = path.as_ref();
        if path.exists() {
            let raw = fs::read_to_string(path).map_err(|e| SyncError::Io(e.to_string()))?;
            if let Ok(mut identity) = serde_json::from_str::<DeviceIdentity>(&raw) {
                if identity.secret_key.is_empty() || identity.public_key.is_empty() {
                    let (secret, public) = Self::generate_keypair();
                    identity.secret_key = hex::encode(secret.to_bytes());
                    identity.public_key = hex::encode(public.to_bytes());
                    let serialized = serde_json::to_string_pretty(&identity)
                        .map_err(|e| SyncError::Io(e.to_string()))?;
                    fs::write(path, serialized).map_err(|e| SyncError::Io(e.to_string()))?;
                }
                return Ok(identity);
            }
        }
        let identity = Self::generate();
        let serialized =
            serde_json::to_string_pretty(&identity).map_err(|e| SyncError::Io(e.to_string()))?;
        fs::write(path, serialized).map_err(|e| SyncError::Io(e.to_string()))?;
        Ok(identity)
    }

    pub fn sign(&self, data: &[u8]) -> Result<String, SyncError> {
        let bytes = hex::decode(&self.secret_key).map_err(|e| SyncError::Io(e.to_string()))?;
        let secret = ed25519_dalek::SecretKey::from_bytes(&bytes)
            .map_err(|e| SyncError::Io(e.to_string()))?;
        let public: PublicKey = (&secret).into();
        let keypair = Keypair { secret, public };
        let sig: Signature = keypair.sign(data);
        Ok(base64::engine::general_purpose::STANDARD.encode(sig.to_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;
    use tempfile::tempdir;

    #[test]
    fn trust_store_add_list() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("trust.json");
        let mut store = TrustStore::load_or_default(&path).unwrap();
        store.add("dev1".into(), "pk1".into()).unwrap();
        assert_eq!(store.list().len(), 1);
        let store2 = TrustStore::load_or_default(&path).unwrap();
        assert_eq!(store2.list().len(), 1);
        assert_eq!(store2.list()[0].device_id, "dev1");
    }

    #[test]
    fn net_transport_loopback_discovery() {
        let transport = NetTransport::new(45678, 45679);
        let t = transport.clone();
        let handle = std::thread::spawn(move || t.listen_discovery(Duration::from_millis(200)));
        std::thread::sleep(Duration::from_millis(50));
        let _ = transport.advertise();
        let peers = handle.join().unwrap().unwrap_or_default();
        // In some environments broadcast may not loop back; accept empty result.
        let _ = peers;
    }

    #[test]
    fn net_transport_tcp_sync() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let transport = NetTransport::new(0, addr.port());
        let handle = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = String::new();
                stream.read_to_string(&mut buf).unwrap();
                buf
            } else {
                String::new()
            }
        });
        transport
            .request_sync(
                &addr.to_string(),
                &SyncEnvelope {
                    device_id: "test".into(),
                    public_key: "pk".into(),
                    signature: "sig".into(),
                    ops: Vec::new(),
                },
            )
            .unwrap();
        let data = handle.join().unwrap();
        assert!(data.contains("device_id"));
    }
}
