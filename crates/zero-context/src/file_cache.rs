use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// LRU: `order[0]` is oldest (evicted first), `order[last]` is most recently used.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub content: String,
    pub modified_at: std::time::SystemTime,
    pub size: usize,
}

pub struct FileCache {
    entries: HashMap<PathBuf, CacheEntry>,
    /// LRU order: front = LRU, back = MRU
    order: Vec<PathBuf>,
    max_entries: usize,
    max_bytes: usize,
    current_bytes: usize,
}

impl FileCache {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: Vec::new(),
            max_entries,
            max_bytes,
            current_bytes: 0,
        }
    }

    pub fn default_limits() -> Self {
        Self::new(100, 25 * 1024 * 1024)
    }

    /// Move `path` to MRU (back of `order`).
    pub fn get(&mut self, path: &Path) -> Option<&CacheEntry> {
        let key = path.to_path_buf();
        if !self.entries.contains_key(&key) {
            return None;
        }
        if let Some(pos) = self.order.iter().position(|p| p == &key) {
            self.order.remove(pos);
        }
        self.order.push(key.clone());
        self.entries.get(&key)
    }

    pub fn insert(&mut self, path: PathBuf, content: String) {
        let size = content.len();
        let modified_at = std::time::SystemTime::now();

        if let Some(old) = self.entries.remove(&path) {
            self.current_bytes = self.current_bytes.saturating_sub(old.size);
            if let Some(pos) = self.order.iter().position(|p| p == &path) {
                self.order.remove(pos);
            }
        }

        self.entries.insert(
            path.clone(),
            CacheEntry {
                content,
                modified_at,
                size,
            },
        );
        self.order.push(path);
        self.current_bytes += size;

        while self.len() > self.max_entries || self.current_bytes > self.max_bytes {
            self.evict_lru();
        }
    }

    fn evict_lru(&mut self) {
        let Some(oldest) = self.order.first().cloned() else {
            return;
        };
        self.order.remove(0);
        if let Some(entry) = self.entries.remove(&oldest) {
            self.current_bytes = self.current_bytes.saturating_sub(entry.size);
        }
    }

    pub fn invalidate(&mut self, path: &Path) {
        let key = path.to_path_buf();
        if let Some(entry) = self.entries.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(entry.size);
            if let Some(pos) = self.order.iter().position(|p| p == &key) {
                self.order.remove(pos);
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.current_bytes = 0;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn bytes_used(&self) -> usize {
        self.current_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lru_evicts_oldest_on_entry_limit() {
        let mut c = FileCache::new(2, 1024);
        c.insert(PathBuf::from("/a"), "a".into());
        c.insert(PathBuf::from("/b"), "b".into());
        c.insert(PathBuf::from("/c"), "c".into());
        assert_eq!(c.len(), 2);
        assert!(!c.entries.contains_key(&PathBuf::from("/a")));
        assert!(c.entries.contains_key(&PathBuf::from("/c")));
    }

    #[test]
    fn evicts_when_byte_limit_exceeded() {
        let mut c = FileCache::new(10, 5);
        c.insert(PathBuf::from("/a"), "aaaa".into()); // 4 bytes
        c.insert(PathBuf::from("/b"), "bbbb".into()); // would exceed 5 if both kept
        assert!(c.bytes_used() <= 5);
    }

    #[test]
    fn get_promotes_to_mru() {
        let mut c = FileCache::new(2, 1024);
        c.insert(PathBuf::from("/a"), "a".into());
        c.insert(PathBuf::from("/b"), "b".into());
        let _ = c.get(Path::new("/a"));
        c.insert(PathBuf::from("/c"), "c".into());
        assert!(c.entries.contains_key(&PathBuf::from("/a")));
        assert!(!c.entries.contains_key(&PathBuf::from("/b")));
    }
}
