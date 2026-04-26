use std::collections::{HashMap, VecDeque};

/// Default number of pages held in the LRU cache.
///
/// 256 pages × 4096 bytes/page = 1 MiB of RAM.  This is a reasonable default
/// for desktop and server workloads; callers that need a different trade-off
/// can construct a `PageCache` with an explicit capacity.
pub const DEFAULT_CACHE_CAPACITY: usize = 256;

/// A fixed-capacity Least-Recently-Used cache mapping keys of type `K` to
/// values of type `V`.
///
/// # Implementation
/// Internally the cache uses:
/// - A `HashMap<K, V>` for O(1) key → value lookup.
/// - A `VecDeque<K>` to track access order: the **front** holds the
///   most-recently-used key; the **back** holds the least-recently-used key.
///
/// On every `get` the accessed key is moved to the front in O(n) time
/// (linear scan to find its current position).  For the small capacities used
/// here (≤ 256 entries) this is entirely negligible compared with the disk
/// I/O it replaces.
///
/// On `put`, if the cache is full, the key at the back of the deque (the LRU
/// entry) is evicted before the new entry is inserted.
///
/// This implementation is dependency-free and intentionally simple.  If
/// O(1) LRU is ever required, a doubly-linked list threaded through the
/// HashMap entries can be substituted without changing the public API.
struct LruCache<K, V> {
    /// Maximum number of entries the cache may hold simultaneously.
    capacity: usize,
    /// The data store: key → value.
    map: HashMap<K, V>,
    /// Access-order ring: front = MRU, back = LRU.
    order: VecDeque<K>,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + std::hash::Hash + Clone,
{
    /// Create a new `LruCache` with the specified maximum number of entries.
    ///
    /// If `capacity` is 0 it is clamped to 1 so the cache is never completely
    /// useless.
    fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            capacity,
            map: HashMap::with_capacity(capacity + 1),
            order: VecDeque::with_capacity(capacity + 1),
        }
    }

    /// Look up `key` and, on a hit, promote it to the most-recently-used
    /// position.
    ///
    /// Returns a reference to the cached value, or `None` on a miss.
    fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // Move the key to the front of the order deque (O(n) scan).
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                if let Some(k) = self.order.remove(pos) {
                    self.order.push_front(k);
                }
            }
            self.map.get(key)
        } else {
            None
        }
    }

    /// Insert `value` under `key`, evicting the LRU entry if the cache is full.
    ///
    /// If `key` is already present its value is updated in place and it is
    /// promoted to MRU position (no eviction needed).
    fn put(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            // Update existing entry and move it to MRU.
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                if let Some(k) = self.order.remove(pos) {
                    self.order.push_front(k);
                }
            }
            self.map.insert(key, value);
        } else {
            // Evict the LRU entry if we are at capacity.
            if self.map.len() >= self.capacity {
                if let Some(lru_key) = self.order.pop_back() {
                    self.map.remove(&lru_key);
                }
            }
            // Insert the new entry at MRU position.
            self.order.push_front(key.clone());
            self.map.insert(key, value);
        }
    }

    /// Remove the entry for `key` and return its value, or `None` if absent.
    fn pop(&mut self, key: &K) -> Option<V> {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
        self.map.remove(key)
    }

    /// Evict all entries.
    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

// ─── PageCache ───────────────────────────────────────────────────────────────

/// An LRU cache that maps page file-offsets to their 4096-byte contents.
///
/// The cache sits between the `Pager` and the `Storage` backend.  When the
/// `Pager` reads a page it first checks the cache; only on a miss does it go
/// to disk.  On a cache hit the page is promoted to the most-recently-used
/// position automatically.
///
/// # Cache invalidation
/// When a page is written (dirtied), the `Pager` calls `put` to update the
/// cached copy so that subsequent reads within the same transaction see the
/// latest data.  When the `Pager` needs to discard in-flight state (e.g. on
/// rollback) it calls `clear`.
///
/// # Memory layout
/// Each page is heap-allocated as a `Box<[u8; 4096]>` so that the internal
/// `HashMap` stores only a thin pointer per entry rather than 4096 bytes
/// inline.
pub struct PageCache {
    /// The underlying LRU map: file offset → heap-allocated page data.
    cache: LruCache<u64, Box<[u8; 4096]>>,
}

impl PageCache {
    /// Create a new `PageCache` with the given maximum number of pages.
    ///
    /// If `capacity` is zero it is silently clamped to 1 so the cache always
    /// holds at least one page.
    ///
    /// # Parameters
    /// - `capacity`: Maximum number of 4096-byte pages to keep in memory
    ///   simultaneously.  Once the cache is full, the least-recently-used
    ///   page is evicted to make room.
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(capacity),
        }
    }

    /// Look up a page by its file offset.
    ///
    /// On a cache hit the page is promoted to the most-recently-used position.
    /// Returns `None` on a miss.
    ///
    /// # Parameters
    /// - `offset`: The byte offset of the page within the backing file.
    ///   Must be a multiple of 4096.
    pub fn get(&mut self, offset: u64) -> Option<&[u8; 4096]> {
        self.cache.get(&offset).map(|b| b.as_ref())
    }

    /// Insert or overwrite a page in the cache.
    ///
    /// If inserting this page causes the cache to exceed its capacity, the
    /// least-recently-used entry is silently dropped.
    ///
    /// # Parameters
    /// - `offset`: The byte offset of the page within the backing file.
    /// - `data`: The 4096-byte page contents to cache.
    pub fn put(&mut self, offset: u64, data: [u8; 4096]) {
        self.cache.put(offset, Box::new(data));
    }

    /// Remove a single page from the cache without reading or writing it.
    ///
    /// Used to force a fresh read from storage on the next access, e.g.
    /// after WAL recovery has overwritten data pages directly through the
    /// `Storage` trait.
    ///
    /// # Parameters
    /// - `offset`: The byte offset of the page to evict.
    pub fn invalidate(&mut self, offset: u64) {
        self.cache.pop(&offset);
    }

    /// Evict all pages from the cache.
    ///
    /// Called by `Pager::rollback` to discard any stale in-memory state and
    /// force all subsequent reads to go back to storage.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cache with capacity 2 should evict the LRU entry when a third page
    /// is inserted.
    #[test]
    fn test_eviction() {
        let mut c = PageCache::new(2);
        c.put(0, [1u8; 4096]);
        c.put(4096, [2u8; 4096]);
        // Access offset 0 so it becomes MRU; offset 4096 becomes LRU.
        c.get(0);
        // Insert a third page — offset 4096 (LRU) must be evicted.
        c.put(8192, [3u8; 4096]);
        assert!(c.get(0).is_some(), "MRU page should still be cached");
        assert!(c.get(4096).is_none(), "LRU page should have been evicted");
        assert!(
            c.get(8192).is_some(),
            "Newly inserted page should be cached"
        );
    }

    /// `invalidate` should remove exactly the specified page.
    #[test]
    fn test_invalidate() {
        let mut c = PageCache::new(4);
        c.put(0, [0u8; 4096]);
        c.put(4096, [1u8; 4096]);
        c.invalidate(0);
        assert!(c.get(0).is_none());
        assert!(c.get(4096).is_some());
    }

    /// `clear` should remove all pages.
    #[test]
    fn test_clear() {
        let mut c = PageCache::new(4);
        c.put(0, [0u8; 4096]);
        c.put(4096, [1u8; 4096]);
        c.clear();
        assert!(c.get(0).is_none());
        assert!(c.get(4096).is_none());
    }
}
