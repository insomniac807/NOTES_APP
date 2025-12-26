// Frontend shell: document list/edit, sync/discovery/trust controls, plugin loader,
// and lightweight markdown preview.
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type DocumentSummary = {
  id: string;
  doc_type: string;
  updated: string;
  title?: string;
  tags: string[];
};

type Document = {
  frontmatter: {
    id: string;
    type: string;
    title?: string;
    created: string;
    updated: string;
    tags?: string[];
    links?: string[];
  };
  body: string;
};

type Peer = {
  addr: string;
  device_id?: string;
  public_key?: string;
};

type DeviceIdentity = {
  device_id: string;
  public_key: string;
};

type TrustedDevice = {
  device_id: string;
  public_key: string;
  allow_auto_sync: boolean;
};

export function App() {
  const [status, setStatus] = useState("checking...");
  const [docs, setDocs] = useState<DocumentSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [docBody, setDocBody] = useState("");
  const [title, setTitle] = useState("");
  const [tagsInput, setTagsInput] = useState("");
  const [search, setSearch] = useState("");
  const [creating, setCreating] = useState(false);
  const [loading, setLoading] = useState(false);
  const [vault, setVault] = useState<string>("");
  const [vaultInput, setVaultInput] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const [peers, setPeers] = useState<Peer[]>([]);
  const [syncStatus, setSyncStatus] = useState<string>("idle");
  const [trusted, setTrusted] = useState<TrustedDevice[]>([]);
  const [trustId, setTrustId] = useState("");
  const [trustKey, setTrustKey] = useState("");
  const [pluginPath, setPluginPath] = useState("");
  const [peerMessage, setPeerMessage] = useState<string>("idle");
  const [device, setDevice] = useState<DeviceIdentity | null>(null);
  const [autoSyncEnabled, setAutoSyncEnabled] = useState<boolean>(false);
  const [discoveryPort, setDiscoveryPort] = useState<number>(53333);
  const [syncPort, setSyncPort] = useState<number>(53334);
  const [transportSecret, setTransportSecret] = useState<string>("");

  useEffect(() => {
    invoke<string>("health_check")
      .then((resp) => setStatus(resp))
      .catch((err) => setStatus(`error: ${err}`));
    refreshVault();
    refreshList();
    loadDevice();
    loadAutoSync();
    loadNetworkConfig();
  }, []);

  async function refreshList() {
    const list = await invoke<DocumentSummary[]>("list_documents");
    setDocs(list);
  }

  async function loadNetworkConfig() {
    try {
      const cfg = await invoke<{ discovery_port: number; sync_port: number; transport_secret?: string }>(
        "get_network_config",
      );
      setDiscoveryPort(cfg.discovery_port);
      setSyncPort(cfg.sync_port);
      setTransportSecret(cfg.transport_secret || "");
    } catch (err: any) {
      setError(String(err));
    }
  }

  async function saveNetworkConfig() {
    setError(null);
    try {
      await invoke("set_network_config", {
        discovery_port: Number(discoveryPort),
        sync_port: Number(syncPort),
        transport_secret: transportSecret || null,
      });
      setPeerMessage("network config saved (restart to fully apply)");
    } catch (err: any) {
      setError(String(err));
    }
  }

  async function runSearch(q: string) {
    if (!q.trim()) {
      return refreshList();
    }
    const list = await invoke<DocumentSummary[]>("search_documents", { query: q });
    setDocs(list);
  }

  async function refreshVault() {
    const path = await invoke<string>("get_vault_root");
    setVault(path);
    setVaultInput(path);
  }

  async function loadDoc(id: string) {
    setLoading(true);
    setError(null);
    try {
      const doc = await invoke<Document>("get_document", { id });
      setSelectedId(id);
      setDocBody(doc.body);
      setTitle(doc.frontmatter.title || "");
      setTagsInput((doc.frontmatter.tags || []).join(", "));
    } catch (err: any) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function discoverPeers() {
    setError(null);
    setPeerMessage("searching...");
    try {
      const found = await invoke<Peer[]>("discover_peers");
      setPeers(found);
      setPeerMessage(found.length === 0 ? "no peers found" : `found ${found.length} peer(s)`);
    } catch (err: any) {
      setPeerMessage("discovery failed");
      setError(String(err));
    }
  }

  async function addPeerToTrust(peer: Peer) {
    setError(null);
    setTrustId(peer.device_id || peer.addr.split(":")[0] || peer.addr);
    if (peer.public_key) {
      setTrustKey(peer.public_key);
    }
  }

  async function addTrustManual() {
    setError(null);
    if (!trustId || !trustKey) {
      setError("Device ID and public key required");
      return;
    }
    try {
      await invoke("add_trusted_device", { device_id: trustId, public_key: trustKey });
      await loadTrusted();
      setTrustId("");
      setTrustKey("");
    } catch (err: any) {
      setError(String(err));
    }
  }

  async function removeTrust(deviceId: string) {
    setError(null);
    try {
      await invoke("remove_trusted_device", { device_id: deviceId });
      await loadTrusted();
    } catch (err: any) {
      setError(String(err));
    }
  }

  async function setTrustAutoSync(deviceId: string, allow: boolean) {
    setError(null);
    try {
      await invoke("set_trusted_auto_sync", { device_id: deviceId, allow });
      await loadTrusted();
    } catch (err: any) {
      setError(String(err));
    }
  }

  async function loadPlugin() {
    setError(null);
    if (!pluginPath.trim()) return;
    try {
      await invoke("load_plugin_manifest", { path: pluginPath.trim() });
      setPluginPath("");
    } catch (err: any) {
      setError(String(err));
    }
  }

  function renderPreview(body: string) {
    return body.split("\n").map((line, idx) => {
      const trimmed = line.trim();
      if (!trimmed) {
        return <div key={idx} style={{ height: "0.6rem" }} />;
      }
      let style: React.CSSProperties = { marginBottom: "0.35rem" };
      let content: React.ReactNode = line;
      if (trimmed.startsWith("#")) {
        const level = trimmed.match(/^#+/)?.[0].length ?? 1;
        content = trimmed.replace(/^#+\s*/, "");
        style = {
          ...style,
          fontWeight: 700,
          fontSize: `${Math.max(1.2 - level * 0.1, 1)}rem`,
          marginTop: level === 1 ? "0.75rem" : "0.5rem",
        };
      }
      if (trimmed.startsWith("- ") || trimmed.startsWith("* ")) {
        style = { ...style, paddingLeft: "1rem", position: "relative" };
        return (
          <div key={idx} style={style}>
            <span style={{ position: "absolute", left: 0 }}>•</span> {trimmed.slice(2)}
          </div>
        );
      }
      return (
        <div key={idx} style={style}>
          {content}
        </div>
      );
    });
  }

  async function syncPeer(peer: string) {
    setError(null);
    setSyncStatus(`syncing ${peer}...`);
    try {
      await invoke("sync_now", { target_device: peer });
      setSyncStatus(`synced ${peer}`);
    } catch (err: any) {
      setSyncStatus(`error syncing ${peer}`);
      setError(String(err));
    }
  }

  async function createNote() {
    setCreating(true);
    setError(null);
    try {
      const frontmatter = {
        type: "note",
        title: "Untitled note",
        tags: [],
        links: [],
        created: "",
        updated: "",
      };
      const body = "# New Note\n";
      await invoke<Document>("create_document", { req: { frontmatter, body } });
      await refreshList();
      setError(null);
    } catch (err: any) {
      setError(String(err));
    } finally {
      setCreating(false);
    }
  }

  async function saveDoc() {
    if (!selectedId) return;
    setLoading(true);
    setError(null);
    try {
      const tags = tagsInput
        .split(",")
        .map((t) => t.trim())
        .filter((t) => t.length > 0);
      const frontmatter = {
        id: selectedId,
        type: "note",
        title: title || undefined,
        tags,
        links: [],
        created: "",
        updated: "",
      };
      await invoke<Document>("update_document", {
        req: { id: selectedId, frontmatter, body: docBody, before_hash: null },
      });
      await refreshList();
      setError(null);
    } catch (err: any) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function changeVault() {
    if (!vaultInput) return;
    setLoading(true);
    setError(null);
    try {
      const path = await invoke<string>("set_vault_root", { path: vaultInput });
      setVault(path);
      setSelectedId(null);
      setDocBody("");
      await refreshList();
      setError(null);
    } catch (err: any) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function loadTrusted() {
    try {
      const list = await invoke<{ device_id: string; public_key: string }[]>("list_trusted_devices");
      setTrusted(list);
    } catch (err: any) {
      setError(String(err));
    }
  }

  async function loadDevice() {
    try {
      const ident = await invoke<DeviceIdentity>("get_device_identity");
      setDevice(ident);
    } catch (err: any) {
      setError(String(err));
    }
  }

  async function loadAutoSync() {
    try {
      const enabled = await invoke<boolean>("get_auto_sync_enabled");
      setAutoSyncEnabled(enabled);
    } catch (err: any) {
      setError(String(err));
    }
  }

  async function toggleAutoSync(next: boolean) {
    setError(null);
    try {
      const enabled = await invoke<boolean>("set_auto_sync_enabled", { enabled: next });
      setAutoSyncEnabled(enabled);
    } catch (err: any) {
      setError(String(err));
    }
  }

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
    } catch (err) {
      console.error("clipboard", err);
    }
  }

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const isSave = (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s";
      const isNew = (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "n";
      if (isSave) {
        e.preventDefault();
        saveDoc().catch(() => {});
      }
      if (isNew) {
        e.preventDefault();
        createNote().catch(() => {});
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  return (
    <main style={{ display: "flex", height: "100vh", fontFamily: "Inter, system-ui, sans-serif" }}>
      <aside style={{ width: "260px", borderRight: "1px solid #ddd", padding: "1rem" }}>
        <h2>Documents</h2>
        <p style={{ color: "#666" }}>Health: {status}</p>
        {error && <p style={{ color: "red" }}>Error: {error}</p>}
        <div style={{ marginBottom: "0.75rem" }}>
          <div style={{ fontSize: "0.85rem", color: "#555", marginBottom: "0.25rem" }}>Vault</div>
          <input
            style={{ width: "100%", boxSizing: "border-box" }}
            value={vaultInput}
            onChange={(e) => setVaultInput(e.target.value)}
          />
          <button onClick={changeVault} disabled={loading} style={{ marginTop: "0.25rem" }}>
            Set Vault
          </button>
          <div style={{ fontSize: "0.75rem", color: "#888", marginTop: "0.25rem" }}>{vault}</div>
        </div>
        <button onClick={createNote} disabled={creating}>
          {creating ? "Creating..." : "New Note"}
        </button>
          <div style={{ marginTop: "0.75rem" }}>
            <input
              placeholder="Search title or tags"
              style={{ width: "100%", boxSizing: "border-box" }}
              value={search}
            onChange={(e) => {
              const val = e.target.value;
              setSearch(val);
              runSearch(val).catch((err) => setError(String(err)));
            }}
          />
        </div>
        <ul style={{ listStyle: "none", padding: 0, marginTop: "1rem" }}>
          {docs.length === 0 ? (
            <li style={{ color: "#666", fontSize: "0.9rem" }}>
              No documents yet. Click “New Note” or press Ctrl/Cmd+N to start.
            </li>
          ) : (
            docs.map((d) => (
              <li key={d.id} style={{ marginBottom: "0.5rem" }}>
                <button
                  onClick={() => loadDoc(d.id)}
                  style={{
                    background: selectedId === d.id ? "#eef" : "#fff",
                    width: "100%",
                    textAlign: "left",
                    padding: "0.5rem",
                    border: "1px solid #ccc",
                    borderRadius: "4px",
                    cursor: "pointer",
                  }}
                >
                  <div style={{ fontWeight: 600 }}>
                    {d.title && d.title.length > 0 ? d.title : d.id.slice(0, 8)}
                  </div>
                  <div style={{ color: "#666", fontSize: "0.8rem" }}>
                    {d.doc_type} • {new Date(d.updated).toLocaleString()}
                  </div>
                  {d.tags.length > 0 && (
                    <div style={{ color: "#555", fontSize: "0.8rem", marginTop: "0.25rem" }}>
                      {d.tags.map((t) => `#${t}`).join(" ")}
                    </div>
                  )}
                </button>
              </li>
            ))
          )}
        </ul>
        <div style={{ marginTop: "1rem" }}>
          <button onClick={discoverPeers} disabled={loading}>
            Discover Peers
          </button>
          <div style={{ fontSize: "0.8rem", color: "#666" }}>Sync status: {syncStatus}</div>
          <div style={{ fontSize: "0.8rem", color: peers.length === 0 ? "#999" : "#666" }}>
            Peers: {peerMessage}
          </div>
          <div style={{ marginTop: "0.5rem", fontSize: "0.85rem", color: "#444" }}>
            Auto-sync:{" "}
            <label style={{ cursor: "pointer" }}>
              <input
                type="checkbox"
                checked={autoSyncEnabled}
                onChange={(e) => toggleAutoSync(e.target.checked)}
                style={{ marginRight: "0.3rem" }}
              />
              {autoSyncEnabled ? "enabled" : "disabled"}
            </label>
          </div>
          {peers.length > 0 && (
            <ul style={{ listStyle: "none", padding: 0, marginTop: "0.5rem" }}>
              {peers.map((p) => (
                <li key={p.addr} style={{ marginBottom: "0.25rem" }}>
                  <div style={{ fontWeight: 600 }}>{p.device_id || p.addr}</div>
                  <div style={{ fontSize: "0.8rem", color: "#666" }}>{p.addr}</div>
                  {p.public_key && (
                    <div style={{ fontSize: "0.75rem", color: "#555" }}>pk: {p.public_key.slice(0, 10)}...</div>
                  )}
                  <button style={{ marginLeft: "0.5rem" }} onClick={() => addPeerToTrust(p)}>
                    Fill trust info
                  </button>
                  <button style={{ marginLeft: "0.5rem" }} onClick={() => syncPeer(p.addr)}>
                    Sync
                  </button>
                </li>
              ))}
            </ul>
          )}
          <div style={{ marginTop: "0.5rem" }}>
            <button onClick={loadTrusted}>Show Trusted Devices</button>
            {trusted.length > 0 && (
              <ul style={{ listStyle: "none", padding: 0, marginTop: "0.25rem" }}>
                {trusted.map((t) => (
                  <li key={t.device_id} style={{ fontSize: "0.8rem", marginBottom: "0.35rem" }}>
                    <div>{t.device_id} ({t.public_key.slice(0, 10)}...)</div>
                    <label style={{ fontSize: "0.75rem", color: "#444", cursor: "pointer" }}>
                      <input
                        type="checkbox"
                        checked={t.allow_auto_sync}
                        onChange={(e) => setTrustAutoSync(t.device_id, e.target.checked)}
                        style={{ marginRight: "0.35rem" }}
                      />
                      Allow auto-sync
                    </label>
                    <button style={{ marginLeft: "0.4rem" }} onClick={() => removeTrust(t.device_id)}>
                      Remove
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <div style={{ marginTop: "0.5rem" }}>
              <div style={{ fontSize: "0.8rem", color: "#555" }}>Add Trust</div>
              <input
                placeholder="Device ID"
                style={{ width: "100%", boxSizing: "border-box", marginBottom: "0.25rem" }}
                value={trustId}
                onChange={(e) => setTrustId(e.target.value)}
              />
              <input
                placeholder="Public Key (hex)"
                style={{ width: "100%", boxSizing: "border-box", marginBottom: "0.25rem" }}
                value={trustKey}
                onChange={(e) => setTrustKey(e.target.value)}
              />
              <button onClick={addTrustManual}>Add Trusted</button>
            </div>
            <div style={{ marginTop: "0.75rem" }}>
              <div style={{ fontSize: "0.8rem", color: "#555" }}>Plugin manifest path</div>
              <input
                placeholder="path/to/manifest.json"
                style={{ width: "100%", boxSizing: "border-box", marginBottom: "0.25rem" }}
                value={pluginPath}
                onChange={(e) => setPluginPath(e.target.value)}
              />
              <button onClick={loadPlugin}>Load Plugin</button>
            </div>
            {device && (
              <div style={{ marginTop: "0.75rem", fontSize: "0.85rem", color: "#444" }}>
                <div style={{ fontWeight: 600 }}>This device</div>
                <div>ID: {device.device_id}</div>
                <div style={{ wordBreak: "break-all" }}>Public key: {device.public_key}</div>
                <div style={{ marginTop: "0.3rem", display: "flex", gap: "0.35rem" }}>
                  <button onClick={() => copyToClipboard(device.device_id)}>Copy ID</button>
                  <button onClick={() => copyToClipboard(device.public_key)}>Copy key</button>
                </div>
              </div>
            )}
            <div style={{ marginTop: "0.75rem" }}>
              <div style={{ fontSize: "0.8rem", color: "#555", marginBottom: "0.25rem" }}>
                Transport security (optional)
              </div>
              <label style={{ fontSize: "0.8rem" }}>
                Discovery port:
                <input
                  type="number"
                  value={discoveryPort}
                  onChange={(e) => setDiscoveryPort(Number(e.target.value))}
                  style={{ width: "100%", boxSizing: "border-box", marginBottom: "0.25rem" }}
                />
              </label>
              <label style={{ fontSize: "0.8rem" }}>
                Sync port:
                <input
                  type="number"
                  value={syncPort}
                  onChange={(e) => setSyncPort(Number(e.target.value))}
                  style={{ width: "100%", boxSizing: "border-box", marginBottom: "0.25rem" }}
                />
              </label>
              <div style={{ fontSize: "0.8rem" }}>
                Shared secret (32-byte hex, enables encrypted transport)
              </div>
              <input
                placeholder="e.g. 64 hex chars"
                style={{ width: "100%", boxSizing: "border-box", marginBottom: "0.25rem" }}
                value={transportSecret}
                onChange={(e) => setTransportSecret(e.target.value)}
              />
              <button onClick={saveNetworkConfig}>Save network config</button>
              <div style={{ fontSize: "0.75rem", color: "#777", marginTop: "0.25rem" }}>
                Note: changes apply after restart; peers must share the same secret.
              </div>
            </div>
          </div>
        </div>
      </aside>
      <section style={{ flex: 1, padding: "1rem" }}>
        {selectedId ? (
          <div style={{ display: "grid", gridTemplateColumns: "1.2fr 0.8fr", gap: "1rem" }}>
            <div>
              <div style={{ marginBottom: "0.5rem", display: "flex", gap: "0.5rem" }}>
                <div>ID: {selectedId}</div>
                {loading && <span style={{ color: "#888" }}>Saving...</span>}
              </div>
              <div style={{ marginBottom: "0.5rem" }}>
                <label style={{ display: "block", marginBottom: "0.25rem" }}>Title</label>
                <input
                  style={{ width: "100%", boxSizing: "border-box" }}
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                />
              </div>
              <div style={{ marginBottom: "0.5rem" }}>
                <label style={{ display: "block", marginBottom: "0.25rem" }}>
                  Tags (comma separated)
                </label>
                <input
                  style={{ width: "100%", boxSizing: "border-box" }}
                  value={tagsInput}
                  onChange={(e) => setTagsInput(e.target.value)}
                />
              </div>
              <textarea
                style={{ width: "100%", height: "70vh", fontFamily: "monospace", fontSize: "14px" }}
                value={docBody}
                onChange={(e) => setDocBody(e.target.value)}
              />
              <div style={{ marginTop: "0.5rem" }}>
                <button onClick={saveDoc} disabled={loading}>
                  Save
                </button>
              </div>
            </div>
            <div
              style={{
                border: "1px solid #ddd",
                borderRadius: "6px",
                padding: "0.75rem",
                background: "#fafafa",
                height: "calc(70vh + 120px)",
                overflow: "auto",
              }}
            >
              <div style={{ fontSize: "0.85rem", color: "#666", marginBottom: "0.5rem" }}>
                Preview
              </div>
              <div style={{ whiteSpace: "pre-wrap", fontFamily: "Inter, system-ui, sans-serif" }}>
                {renderPreview(docBody)}
              </div>
            </div>
          </div>
        ) : (
          <p>Select a document to view/edit.</p>
        )}
      </section>
    </main>
  );
}
