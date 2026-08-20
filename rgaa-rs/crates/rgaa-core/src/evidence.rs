use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceRef {
    pub kind: String,
    pub hash: String,
    pub location: Option<String>,
}

impl EvidenceRef {
    pub fn new(kind: impl Into<String>, hash: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            hash: hash.into(),
            location: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_reference_preserves_kind_and_hash() {
        let evidence = EvidenceRef::new("dom_snapshot", "sha256:abc");
        let json = serde_json::to_string(&evidence).unwrap();
        let decoded: EvidenceRef = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, evidence);
    }
}
