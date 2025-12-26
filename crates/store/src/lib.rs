use chrono::Utc;
use notes_core::{Document, DocumentError, DocumentType, Frontmatter};
use notes_oplog::{Operation, OperationType};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("conflict detected for document {0}")]
    Conflict(String),
    #[error("hash mismatch for document {0}")]
    HashMismatch(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("db error: {0}")]
    Db(String),
    #[error("document error: {0}")]
    Document(String),
    #[error("unsupported operation")]
    Unsupported,
    #[error("not found")]
    NotFound,
}

#[derive(Debug, Deserialize)]
struct DocPayload {
    frontmatter: serde_yaml::Value,
    body: String,
}

pub struct Store {
    root: PathBuf,
    conn: Connection,
    seen_ops: std::collections::HashSet<String>,
}

impl Default for Store {
    fn default() -> Self {
        Self::with_root("vault").unwrap_or_else(|_| Self {
            root: PathBuf::from("vault"),
            conn: Connection::open_in_memory().unwrap(),
            seen_ops: std::collections::HashSet::new(),
        })
    }
}
impl Store {
    pub fn new() -> Result<Self, StoreError> {
        Self::with_root("vault")
    }

    pub fn with_root(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|e| StoreError::Io(e.to_string()))?;
        let conn =
            Connection::open(root.join("index.db")).map_err(|e| StoreError::Db(e.to_string()))?;
        Self::init_db(&conn)?;
        Ok(Self {
            root,
            conn,
            seen_ops: std::collections::HashSet::new(),
        })
    }

    pub fn root_path(&self) -> PathBuf {
        self.root.clone()
    }

    /// Apply an op to the vault (create/update/delete), enforcing before/after hashes and
    /// writing conflicts when needed. Returns the written document for create/update.
    pub fn apply(&mut self, op: Operation) -> Result<Option<Document>, StoreError> {
        let op_key = op.key();
        if self.seen_ops.contains(&op_key) {
            return Ok(None);
        }
        match op.op_type {
            OperationType::CreateDocument | OperationType::UpdateDocument => {
                // parse payload → document and ensure updated timestamps
                let payload: DocPayload = serde_json::from_value(op.payload.clone())
                    .map_err(|e| StoreError::Document(e.to_string()))?;

                let mut doc = self.payload_to_document(&payload)?;
                doc.frontmatter.updated = Utc::now().to_rfc3339();
                let computed_hash = doc.hash_content();

                // conflict detection
                if let Some(current) = self.load_document(&op.document_id)? {
                    if let Some(before_hash) = op.before_hash.as_ref() {
                        if &current.hash_content() != before_hash {
                            self.write_conflict(&op.document_id, &doc)?;
                            return Err(StoreError::Conflict(op.document_id));
                        }
                    }
                }

                // ensure caller-supplied after_hash matches what we will write
                if let Some(expected_after) = op.after_hash.as_ref() {
                    if expected_after != &computed_hash {
                        return Err(StoreError::HashMismatch(op.document_id));
                    }
                }

                self.write_document(&op.document_id, &doc)?;
                self.upsert_index(&doc)?;
                self.seen_ops.insert(op_key);
                Ok(Some(doc))
            }
            OperationType::DeleteDocument => {
                let path = self.doc_path(&op.document_id);
                let _ = fs::remove_file(path);
                self.delete_index(&op.document_id)?;
                self.seen_ops.insert(op_key);
                Ok(None)
            }
            _ => Err(StoreError::Unsupported),
        }
    }

    pub fn load_document(&self, id: &str) -> Result<Option<Document>, StoreError> {
        let path = self.doc_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path).map_err(|e| StoreError::Io(e.to_string()))?;
        let doc = Document::from_markdown(&raw).map_err(|e| StoreError::Document(e.to_string()))?;
        Ok(Some(doc))
    }

    fn write_document(&self, id: &str, doc: &Document) -> Result<(), StoreError> {
        let path = self.doc_path(id);
        let content = doc
            .to_markdown()
            .map_err(|e| StoreError::Document(e.to_string()))?;
        fs::write(path, content).map_err(|e| StoreError::Io(e.to_string()))
    }

    fn write_conflict(&self, id: &str, doc: &Document) -> Result<(), StoreError> {
        let ts = Utc::now().format("%Y%m%d%H%M%S");
        let filename = format!("{id}.conflict.{ts}.md");
        let path = self.root.join(filename);
        let content = doc
            .to_markdown()
            .map_err(|e| StoreError::Document(e.to_string()))?;
        fs::write(path, content).map_err(|e| StoreError::Io(e.to_string()))
    }

    fn doc_path(&self, id: &str) -> PathBuf {
        self.root.join(format!("{id}.md"))
    }

    fn payload_to_document(&self, payload: &DocPayload) -> Result<Document, StoreError> {
        #[derive(Deserialize)]
        struct PartialFrontmatter {
            id: Option<String>,
            #[serde(rename = "type")]
            doc_type: Option<String>,
            title: Option<String>,
            created: Option<String>,
            updated: Option<String>,
            #[serde(default)]
            tags: Vec<String>,
            #[serde(default)]
            links: Vec<String>,
        }

        let partial: PartialFrontmatter = serde_yaml::from_value(payload.frontmatter.clone())
            .map_err(|_| StoreError::Document(DocumentError::Frontmatter.to_string()))?;

        let now = Utc::now().to_rfc3339();
        let doc_type = match partial.doc_type.as_deref() {
            Some("note") | None => DocumentType::Note,
            Some("source") => DocumentType::Source,
            Some("highlight") => DocumentType::Highlight,
            Some("annotation") => DocumentType::Annotation,
            Some("reference") => DocumentType::Reference,
            Some("system") => DocumentType::System,
            Some(_) => return Err(StoreError::Document("unknown document type".into())),
        };

        let mut frontmatter = Frontmatter {
            id: partial.id.unwrap_or_else(notes_core::generate_id),
            doc_type,
            title: partial.title,
            created: partial.created.unwrap_or_else(|| now.clone()),
            updated: partial.updated.unwrap_or_else(|| now.clone()),
            tags: partial.tags,
            links: partial.links,
        };

        if frontmatter
            .title
            .as_ref()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .is_none()
        {
            if let Some(derived) = derive_title(&payload.body) {
                frontmatter.title = Some(derived);
            }
        }

        Ok(Document {
            frontmatter,
            body: payload.body.clone(),
        })
    }

    fn init_db(conn: &Connection) -> Result<(), StoreError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS documents(
                id TEXT PRIMARY KEY,
                doc_type TEXT NOT NULL,
                updated TEXT NOT NULL,
                title TEXT,
                tags TEXT
            );",
        )
        .map_err(|e| StoreError::Db(e.to_string()))?;

        // ensure columns exist for older dbs
        let mut cols = Vec::new();
        let mut stmt = conn
            .prepare("PRAGMA table_info(documents);")
            .map_err(|e| StoreError::Db(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| Ok(row.get::<_, String>(1)?))
            .map_err(|e| StoreError::Db(e.to_string()))?;
        for r in rows {
            cols.push(r.map_err(|e| StoreError::Db(e.to_string()))?);
        }
        if !cols.contains(&"title".to_string()) {
            conn.execute("ALTER TABLE documents ADD COLUMN title TEXT;", [])
                .ok();
        }
        if !cols.contains(&"tags".to_string()) {
            conn.execute("ALTER TABLE documents ADD COLUMN tags TEXT;", [])
                .ok();
        }
        Ok(())
    }

    fn upsert_index(&self, doc: &Document) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO documents(id, doc_type, updated, title, tags) VALUES(?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET doc_type=excluded.doc_type, updated=excluded.updated, title=excluded.title, tags=excluded.tags;",
                params![
                    doc.frontmatter.id,
                    format!("{:?}", doc.frontmatter.doc_type).to_lowercase(),
                    doc.frontmatter.updated,
                    doc.frontmatter.title.clone().unwrap_or_default(),
                    serde_json::to_string(&doc.frontmatter.tags).unwrap_or_else(|_| "[]".into())
                ],
            )
            .map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(())
    }

    fn delete_index(&self, id: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM documents WHERE id=?1", params![id])
            .map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(())
    }

    pub fn list_documents(&self) -> Result<Vec<DocumentSummary>, StoreError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, doc_type, updated, title, tags FROM documents ORDER BY updated DESC",
            )
            .map_err(|e| StoreError::Db(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let tags_json: String = row.get(4)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                let summary = DocumentSummary {
                    id: row.get(0)?,
                    doc_type: row.get(1)?,
                    updated: row.get(2)?,
                    title: row.get(3).ok(),
                    tags,
                };
                Ok(summary)
            })
            .map_err(|e| StoreError::Db(e.to_string()))?;
        let mut docs = Vec::new();
        for r in rows {
            docs.push(r.map_err(|e| StoreError::Db(e.to_string()))?);
        }
        Ok(docs)
    }

    pub fn search_documents(&self, query: &str) -> Result<Vec<DocumentSummary>, StoreError> {
        let like = format!("%{}%", query);
        let mut stmt = self
            .conn
            .prepare("SELECT id, doc_type, updated, title, tags FROM documents WHERE title LIKE ?1 OR tags LIKE ?1 OR id LIKE ?1 ORDER BY updated DESC")
            .map_err(|e| StoreError::Db(e.to_string()))?;
        let rows = stmt
            .query_map(params![like], |row| {
                let tags_json: String = row.get(4)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                Ok(DocumentSummary {
                    id: row.get(0)?,
                    doc_type: row.get(1)?,
                    updated: row.get(2)?,
                    title: row.get(3).ok(),
                    tags,
                })
            })
            .map_err(|e| StoreError::Db(e.to_string()))?;
        let mut docs = Vec::new();
        for r in rows {
            docs.push(r.map_err(|e| StoreError::Db(e.to_string()))?);
        }
        Ok(docs)
    }

    pub fn delete_document(&self, id: &str) -> Result<(), StoreError> {
        let path = self.doc_path(id);
        let _ = fs::remove_file(path);
        self.delete_index(id)
    }

    /// Update a document with conflict/hash checks.
    pub fn update_document(&mut self, op: Operation) -> Result<Document, StoreError> {
        match op.op_type {
            OperationType::UpdateDocument => {
                let payload: DocPayload = serde_json::from_value(op.payload.clone())
                    .map_err(|e| StoreError::Document(e.to_string()))?;
                let mut doc = self.payload_to_document(&payload)?;
                doc.frontmatter.updated = Utc::now().to_rfc3339();
                let computed_hash = doc.hash_content();

                if let Some(current) = self.load_document(&op.document_id)? {
                    if let Some(before_hash) = op.before_hash.as_ref() {
                        if &current.hash_content() != before_hash {
                            self.write_conflict(&op.document_id, &doc)?;
                            return Err(StoreError::Conflict(op.document_id));
                        }
                    }
                } else {
                    return Err(StoreError::NotFound);
                }

                if let Some(expected_after) = op.after_hash.as_ref() {
                    if expected_after != &computed_hash {
                        return Err(StoreError::HashMismatch(op.document_id));
                    }
                }

                self.write_document(&op.document_id, &doc)?;
                self.upsert_index(&doc)?;
                self.seen_ops.insert(op.key());
                Ok(doc)
            }
            _ => Err(StoreError::Unsupported),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentSummary {
    pub id: String,
    pub doc_type: String,
    pub updated: String,
    pub title: Option<String>,
    pub tags: Vec<String>,
}

impl DocumentSummary {
    pub fn display_title(&self) -> String {
        self.title
            .as_ref()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .unwrap_or_else(|| self.id.clone())
    }
}

fn derive_title(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim_start_matches('#').trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_payload(id: Option<String>, body: &str) -> serde_json::Value {
        let mut fm = serde_yaml::Mapping::new();
        if let Some(id) = id {
            fm.insert(
                serde_yaml::Value::String("id".into()),
                serde_yaml::Value::String(id),
            );
        }
        fm.insert(
            serde_yaml::Value::String("type".into()),
            serde_yaml::Value::String("note".into()),
        );
        fm.insert(
            serde_yaml::Value::String("title".into()),
            serde_yaml::Value::String("Test Title".into()),
        );
        serde_json::json!({
            "frontmatter": serde_yaml::Value::Mapping(fm),
            "body": body
        })
    }

    #[test]
    fn create_and_list() {
        let dir = tempdir().unwrap();
        let mut store = Store::with_root(dir.path()).unwrap();
        let op = Operation {
            op_id: "op1".into(),
            device_id: "dev".into(),
            timestamp: Utc::now().to_rfc3339(),
            op_type: OperationType::CreateDocument,
            document_id: "doc1".into(),
            payload: make_payload(Some("doc1".into()), "hello"),
            before_hash: None,
            after_hash: None,
        };
        let doc = store.apply(op).unwrap().unwrap();
        assert_eq!(doc.frontmatter.id, "doc1");
        let list = store.list_documents().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "doc1");
        assert_eq!(list[0].title.as_deref(), Some("Test Title"));
    }

    #[test]
    fn conflict_detection_on_update() {
        let dir = tempdir().unwrap();
        let mut store = Store::with_root(dir.path()).unwrap();
        let create_op = Operation {
            op_id: "op1".into(),
            device_id: "dev".into(),
            timestamp: Utc::now().to_rfc3339(),
            op_type: OperationType::CreateDocument,
            document_id: "doc1".into(),
            payload: make_payload(Some("doc1".into()), "first"),
            before_hash: None,
            after_hash: None,
        };
        store.apply(create_op).unwrap();

        let update_op = Operation {
            op_id: "op2".into(),
            device_id: "dev".into(),
            timestamp: Utc::now().to_rfc3339(),
            op_type: OperationType::UpdateDocument,
            document_id: "doc1".into(),
            payload: make_payload(Some("doc1".into()), "second"),
            before_hash: Some("mismatch".into()),
            after_hash: None,
        };
        let err = store.update_document(update_op).unwrap_err();
        assert!(matches!(err, StoreError::Conflict(_)));
    }
}
