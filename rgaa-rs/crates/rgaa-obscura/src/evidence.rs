use crate::ObscuraError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceArtifact {
    pub kind: String,
    pub bytes: Vec<u8>,
}

impl EvidenceArtifact {
    pub fn new(kind: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            kind: kind.into(),
            bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceRef {
    pub kind: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct EvidenceStore {
    root: PathBuf,
}

impl EvidenceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn write(&self, evidence: EvidenceArtifact) -> Result<EvidenceRef, ObscuraError> {
        fs::create_dir_all(&self.root).map_err(|error| {
            ObscuraError::Evidence(format!("failed to create evidence directory: {error}"))
        })?;
        let digest = Sha256::digest(&evidence.bytes);
        let hash = format!("sha256:{digest:x}");
        let extension = match evidence.kind.as_str() {
            "screenshot" => "png",
            "tree" | "state" => "json",
            _ => "bin",
        };
        let destination = self.root.join(format!("{digest:x}.{extension}"));
        if !destination.exists() {
            let temporary = self.root.join(format!(".{digest:x}.tmp"));
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|error| {
                    ObscuraError::Evidence(format!("failed to create evidence file: {error}"))
                })?;
            if let Err(error) = write_and_sync(&mut file, &evidence.bytes) {
                let _ = fs::remove_file(&temporary);
                return Err(ObscuraError::Evidence(format!(
                    "failed to write evidence: {error}"
                )));
            }
            fs::rename(&temporary, &destination).map_err(|error| {
                ObscuraError::Evidence(format!("failed to commit evidence: {error}"))
            })?;
        }
        Ok(EvidenceRef {
            kind: evidence.kind,
            path: destination.to_string_lossy().into_owned(),
            sha256: hash,
        })
    }
}

fn write_and_sync(file: &mut File, bytes: &[u8]) -> std::io::Result<()> {
    file.write_all(bytes)?;
    file.sync_all()
}
