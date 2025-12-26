use serde::{Deserialize, Serialize};

// Operation log types shared across crates: defines operation kinds and hashing helpers
// used for sync/signing/deduplication.

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    CreateDocument,
    UpdateDocument,
    DeleteDocument,
    AttachFile,
    DetachFile,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Operation {
    pub op_id: String,
    pub device_id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub op_type: OperationType,
    pub document_id: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
}

impl Operation {
    pub fn op_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.op_id.as_bytes());
        hasher.update(self.device_id.as_bytes());
        hasher.update(self.timestamp.as_bytes());
        hasher.update(format!("{:?}", self.op_type));
        hasher.update(self.document_id.as_bytes());
        hasher.update(self.payload.to_string().as_bytes());
        if let Some(before) = &self.before_hash {
            hasher.update(before.as_bytes());
        }
        if let Some(after) = &self.after_hash {
            hasher.update(after.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.device_id, self.op_id)
    }
}

impl Operation {
    pub fn digest(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.op_id.as_bytes());
        hasher.update(self.device_id.as_bytes());
        hasher.update(self.timestamp.as_bytes());
        hasher.update(format!("{:?}", self.op_type));
        hasher.update(self.document_id.as_bytes());
        hasher.update(self.payload.to_string().as_bytes());
        if let Some(before) = &self.before_hash {
            hasher.update(before.as_bytes());
        }
        if let Some(after) = &self.after_hash {
            hasher.update(after.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }
}
