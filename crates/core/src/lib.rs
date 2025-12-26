use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DocumentType {
    Note,
    Source,
    Highlight,
    Annotation,
    Reference,
    System,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Frontmatter {
    pub id: String,
    #[serde(rename = "type")]
    pub doc_type: DocumentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub title: Option<String>,
    pub created: String,
    pub updated: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub links: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Document {
    pub frontmatter: Frontmatter,
    pub body: String,
}

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("failed to parse frontmatter")]
    Frontmatter,
    #[error("failed to serialize document")]
    Serialize,
    #[error("invalid document")]
    Invalid,
}

impl Document {
    pub fn hash_content(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        let content = self.to_markdown().unwrap_or_default();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn to_markdown(&self) -> Result<String, DocumentError> {
        let fm = serde_yaml::to_string(&self.frontmatter).map_err(|_| DocumentError::Serialize)?;
        Ok(format!("---\n{}---\n\n{}", fm, self.body))
    }

    pub fn from_markdown(raw: &str) -> Result<Self, DocumentError> {
        let mut sections = raw.splitn(3, "---");
        let _before = sections.next();
        let fm_section = sections.next().ok_or(DocumentError::Frontmatter)?;
        let body_section = sections.next().unwrap_or_default();
        let frontmatter: Frontmatter =
            serde_yaml::from_str(fm_section).map_err(|_| DocumentError::Frontmatter)?;
        Ok(Self {
            frontmatter,
            body: body_section.trim_start_matches('\n').to_string(),
        })
    }
}

pub fn generate_id() -> String {
    ulid::Ulid::new().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_markdown() {
        let doc = Document {
            frontmatter: Frontmatter {
                id: generate_id(),
                doc_type: DocumentType::Note,
                title: None,
                created: "2025-01-01T00:00:00Z".into(),
                updated: "2025-01-01T00:00:00Z".into(),
                tags: vec!["tag".into()],
                links: vec![],
            },
            body: "Hello".into(),
        };
        let md = doc.to_markdown().unwrap();
        let parsed = Document::from_markdown(&md).unwrap();
        assert_eq!(parsed.frontmatter.id, doc.frontmatter.id);
        assert_eq!(parsed.body, doc.body);
    }
}
