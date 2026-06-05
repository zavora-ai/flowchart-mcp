//! In-memory flowchart handle store with bounded capacity.
//!
//! Each flowchart is keyed by a UUID handle. Access refreshes `last_used`;
//! capacity is enforced by LRU eviction and stale entries are swept after a
//! TTL of inactivity.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::engine::Document;

const CAPACITY: usize = 50;
const TTL: Duration = Duration::from_secs(30 * 60);

struct Entry {
    doc: Document,
    last_used: Instant,
}

pub struct FlowchartStore {
    map: HashMap<String, Entry>,
    capacity: usize,
    ttl: Duration,
}

impl FlowchartStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            capacity: CAPACITY,
            ttl: TTL,
        }
    }

    /// Insert a document, returning its new UUID handle.
    pub fn insert(&mut self, doc: Document) -> String {
        self.sweep();
        if self.map.len() >= self.capacity {
            self.evict_lru();
        }
        let handle = Uuid::new_v4().to_string();
        self.map.insert(
            handle.clone(),
            Entry {
                doc,
                last_used: Instant::now(),
            },
        );
        handle
    }

    /// Borrow a document mutably, refreshing its `last_used` timestamp.
    pub fn get_mut(&mut self, handle: &str) -> Option<&mut Document> {
        let entry = self.map.get_mut(handle)?;
        entry.last_used = Instant::now();
        Some(&mut entry.doc)
    }

    pub fn remove(&mut self, handle: &str) -> bool {
        self.map.remove(handle).is_some()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    fn sweep(&mut self) {
        let ttl = self.ttl;
        self.map.retain(|_, e| e.last_used.elapsed() < ttl);
    }

    fn evict_lru(&mut self) {
        if let Some(handle) = self
            .map
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(h, _)| h.clone())
        {
            self.map.remove(&handle);
        }
    }
}

impl Default for FlowchartStore {
    fn default() -> Self {
        Self::new()
    }
}

pub type Shared = Arc<RwLock<FlowchartStore>>;

pub fn new_store() -> Shared {
    Arc::new(RwLock::new(FlowchartStore::new()))
}

// ---------------------------------------------------------------------------
// Sequence-diagram store (parallel to FlowchartStore; same LRU + TTL policy).
// ---------------------------------------------------------------------------

use crate::sequence::Sequence;

struct SeqEntry {
    seq: Sequence,
    last_used: Instant,
}

pub struct SequenceStore {
    map: HashMap<String, SeqEntry>,
    capacity: usize,
    ttl: Duration,
}

impl SequenceStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            capacity: CAPACITY,
            ttl: TTL,
        }
    }

    pub fn insert(&mut self, seq: Sequence) -> String {
        self.map.retain(|_, e| e.last_used.elapsed() < self.ttl);
        if self.map.len() >= self.capacity {
            if let Some(h) = self.map.iter().min_by_key(|(_, e)| e.last_used).map(|(h, _)| h.clone()) {
                self.map.remove(&h);
            }
        }
        let handle = Uuid::new_v4().to_string();
        self.map.insert(handle.clone(), SeqEntry { seq, last_used: Instant::now() });
        handle
    }

    pub fn get_mut(&mut self, handle: &str) -> Option<&mut Sequence> {
        let e = self.map.get_mut(handle)?;
        e.last_used = Instant::now();
        Some(&mut e.seq)
    }

    pub fn remove(&mut self, handle: &str) -> bool {
        self.map.remove(handle).is_some()
    }
}

impl Default for SequenceStore {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedSeq = Arc<RwLock<SequenceStore>>;

pub fn new_seq_store() -> SharedSeq {
    Arc::new(RwLock::new(SequenceStore::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Direction;

    fn doc() -> Document {
        Document::new(Direction::TB)
    }

    #[test]
    fn insert_get_remove() {
        let mut s = FlowchartStore::new();
        let h = s.insert(doc());
        assert!(s.get_mut(&h).is_some());
        assert_eq!(s.len(), 1);
        assert!(s.remove(&h));
        assert!(!s.remove(&h));
        assert!(s.is_empty());
    }

    #[test]
    fn lru_eviction_at_capacity() {
        let mut s = FlowchartStore {
            map: HashMap::new(),
            capacity: 2,
            ttl: TTL,
        };
        let h1 = s.insert(doc());
        std::thread::sleep(Duration::from_millis(2));
        let h2 = s.insert(doc());
        std::thread::sleep(Duration::from_millis(2));
        assert!(s.get_mut(&h1).is_some()); // touch h1 → h2 is LRU
        std::thread::sleep(Duration::from_millis(2));
        let h3 = s.insert(doc());
        assert_eq!(s.len(), 2);
        assert!(s.get_mut(&h2).is_none());
        assert!(s.get_mut(&h1).is_some());
        assert!(s.get_mut(&h3).is_some());
    }
}
