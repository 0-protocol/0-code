use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMemory {
    pub entries: Vec<MemoryEntry>,
    pub project_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub created_at: String,
    pub source: MemorySource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemorySource {
    Global,  // ~/.zero/ZERO.md
    User,    // ~/.zero/memory/
    Project, // .zero/ZERO.md in project
    Local,   // .zero/memory/ in project
}

pub struct MemoryManager {
    base_dir: PathBuf,
    project_dir: Option<PathBuf>,
}

impl MemoryManager {
    pub fn new(project_path: &Path) -> Self {
        let hash = Self::project_hash(project_path);
        let base_dir = home_zero_dir()
            .unwrap_or_else(|| PathBuf::from(".zero"))
            .join("projects")
            .join(&hash)
            .join("memory");
        Self {
            base_dir,
            project_dir: Some(project_path.to_path_buf()),
        }
    }

    /// Load ZERO.md hierarchy (global → user files → project → local).
    pub fn load_zero_md(&self) -> Vec<String> {
        let mut out = Vec::new();

        if let Some(home) = home_zero_dir() {
            let global = home.join("ZERO.md");
            if let Ok(s) = fs::read_to_string(&global) {
                out.push(s);
            }
            let user_mem = home.join("memory");
            if user_mem.is_dir() {
                append_sorted_files(&user_mem, &mut out);
            }
        }

        if let Some(proj) = &self.project_dir {
            let project_zero = proj.join(".zero").join("ZERO.md");
            if let Ok(s) = fs::read_to_string(&project_zero) {
                out.push(s);
            }
            let local_mem = proj.join(".zero").join("memory");
            if local_mem.is_dir() {
                append_sorted_files(&local_mem, &mut out);
            }
        }

        out
    }

    pub fn save_entry(&self, entry: &MemoryEntry) -> std::io::Result<()> {
        fs::create_dir_all(&self.base_dir)?;
        let path = self.base_dir.join("entries.jsonl");
        let line = serde_json::to_string(entry).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
        })?;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        writeln!(f, "{line}")?;
        Ok(())
    }

    pub fn load_entries(&self) -> Vec<MemoryEntry> {
        let path = self.base_dir.join("entries.jsonl");
        let Ok(data) = fs::read_to_string(&path) else {
            return Vec::new();
        };
        data.lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    /// Stable hash from canonical project path (sha256 hex).
    pub fn project_hash(path: &Path) -> String {
        let normalized = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let s = normalized.to_string_lossy();
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn project_dir(&self) -> Option<&Path> {
        self.project_dir.as_deref()
    }
}

fn home_zero_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|h| h.join(".zero"))
}

fn append_sorted_files(dir: &Path, out: &mut Vec<String>) {
    let mut paths: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path()).collect(),
        Err(_) => return,
    };
    paths.sort();
    for p in paths {
        if p.is_file() {
            if let Ok(s) = fs::read_to_string(&p) {
                out.push(s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn project_hash_is_stable_hex() {
        let tmp = std::env::temp_dir().join("zero-ctx-test-proj");
        let _ = fs::create_dir_all(&tmp);
        let h1 = MemoryManager::project_hash(&tmp);
        let h2 = MemoryManager::project_hash(&tmp);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        for ch in h1.chars() {
            assert!(ch.is_ascii_hexdigit());
        }
    }
}
