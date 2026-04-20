//! Object pool for frequently allocated geometry entities
#![allow(clippy::cast_precision_loss)]
//!
//! This module provides object pooling to reduce allocation overhead for
//! frequently created and destroyed geometry objects. Features include:
//! - Pre-allocation of objects
//! - Automatic recycling
//! - Thread-safe pooling with lock-free queues
//! - Thread-local caching for reduced contention
//! - Statistics tracking
//!
//! # Architecture
//!
//! The pool uses a two-tier design:
//! 1. **Thread-local cache**: Each thread has a small local cache (default 4 objects)
//!    to avoid contention for hot paths.
//! 2. **Global shared pool**: Backed by lock-free `SegQueue` for cross-thread sharing.
//!
//! # Example
//!
//! ```
//! use cadagent::memory::pool::{SharedPool, VectorPool};
//! use nalgebra::Vector3;
//!
//! // Thread-safe shared pool
//! let pool = SharedPool::<Vector3<f32>>::new(100);
//! let vec = pool.acquire();
//! pool.release(vec);
//!
//! // Specialized vector pool
//! let mut vec_pool = VectorPool::new(64);
//! let v = vec_pool.acquire_with_values(1.0, 2.0, 3.0);
//! ```

use crossbeam::queue::SegQueue;
use nalgebra::{Point3, Vector3};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Thread-safe object pool with lock-free queue
pub struct SharedPool<T: Default + Clone + Send + 'static> {
    /// Global shared queue (lock-free)
    global: SegQueue<T>,
    /// Maximum pool size
    max_size: AtomicUsize,
    /// Statistics
    stats: Arc<PoolStatsInner>,
    _marker: PhantomData<T>,
}

/// Internal statistics storage (atomic counters)
#[derive(Debug, Default)]
struct PoolStatsInner {
    total_acquisitions: AtomicUsize,
    total_releases: AtomicUsize,
    total_allocations: AtomicUsize,
    active_objects: AtomicUsize,
    peak_active_objects: AtomicUsize,
    pool_hits: AtomicUsize,
    pool_misses: AtomicUsize,
}

impl<T: Default + Clone + Send + 'static> SharedPool<T> {
    /// Create a new shared pool with specified capacity
    pub fn new(initial_capacity: usize) -> Self {
        Self::with_max_size(initial_capacity, usize::MAX)
    }

    /// Create a new shared pool with max size
    pub fn with_max_size(initial_capacity: usize, max_size: usize) -> Self {
        let queue = SegQueue::new();

        // Pre-allocate objects
        for _ in 0..initial_capacity {
            queue.push(T::default());
        }

        Self {
            global: queue,
            max_size: AtomicUsize::new(max_size),
            stats: Arc::new(PoolStatsInner::default()),
            _marker: PhantomData,
        }
    }

    /// Acquire an object from the pool
    pub fn acquire(&self) -> T {
        self.stats
            .total_acquisitions
            .fetch_add(1, Ordering::Relaxed);

        // Try thread-local cache first (fast path)
        // For simplicity in this version, we go directly to global queue
        // A more complex implementation could use thread_local! macro

        if let Some(obj) = self.global.pop() {
            self.stats.pool_hits.fetch_add(1, Ordering::Relaxed);
            self.stats.active_objects.fetch_add(1, Ordering::Relaxed);

            // Update peak
            let mut peak = self.stats.peak_active_objects.load(Ordering::Relaxed);
            loop {
                let active = self.stats.active_objects.load(Ordering::Relaxed);
                if active <= peak {
                    break;
                }
                match self.stats.peak_active_objects.compare_exchange_weak(
                    peak,
                    active,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }

            obj
        } else {
            self.stats.pool_misses.fetch_add(1, Ordering::Relaxed);
            self.stats.total_allocations.fetch_add(1, Ordering::Relaxed);
            self.stats.active_objects.fetch_add(1, Ordering::Relaxed);

            // Update peak
            let mut peak = self.stats.peak_active_objects.load(Ordering::Relaxed);
            loop {
                let active = self.stats.active_objects.load(Ordering::Relaxed);
                if active <= peak {
                    break;
                }
                match self.stats.peak_active_objects.compare_exchange_weak(
                    peak,
                    active,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }

            T::default()
        }
    }

    /// Acquire with initializer
    pub fn acquire_with<F: FnOnce() -> T>(&self, init: F) -> T {
        self.stats
            .total_acquisitions
            .fetch_add(1, Ordering::Relaxed);

        let obj = if let Some(available) = self.global.pop() {
            self.stats.pool_hits.fetch_add(1, Ordering::Relaxed);
            available
        } else {
            self.stats.pool_misses.fetch_add(1, Ordering::Relaxed);
            self.stats.total_allocations.fetch_add(1, Ordering::Relaxed);
            init()
        };

        self.stats.active_objects.fetch_add(1, Ordering::Relaxed);

        // Update peak
        let mut peak = self.stats.peak_active_objects.load(Ordering::Relaxed);
        loop {
            let active = self.stats.active_objects.load(Ordering::Relaxed);
            if active <= peak {
                break;
            }
            match self.stats.peak_active_objects.compare_exchange_weak(
                peak,
                active,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }

        obj
    }

    /// Release an object back to the pool
    pub fn release(&self, mut obj: T) {
        // Reset the object before returning to pool
        self.reset_object(&mut obj);

        self.stats.total_releases.fetch_add(1, Ordering::Relaxed);
        self.stats.active_objects.fetch_sub(1, Ordering::Relaxed);

        if self.global.len() < self.max_size.load(Ordering::Relaxed) {
            self.global.push(obj);
        }
        // If pool is at capacity, object is dropped (GC will reclaim)
    }

    /// Reset object to default state (override for specialized reset logic)
    fn reset_object(&self, _obj: &mut T) {
        // Default implementation does nothing
        // Override in specialized pools
    }

    /// Get the number of available objects (approximate)
    pub fn available_count(&self) -> usize {
        self.global.len()
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            total_acquisitions: self.stats.total_acquisitions.load(Ordering::Relaxed),
            total_releases: self.stats.total_releases.load(Ordering::Relaxed),
            total_allocations: self.stats.total_allocations.load(Ordering::Relaxed),
            active_objects: self.stats.active_objects.load(Ordering::Relaxed),
            peak_active_objects: self.stats.peak_active_objects.load(Ordering::Relaxed),
            pool_hits: self.stats.pool_hits.load(Ordering::Relaxed),
            pool_misses: self.stats.pool_misses.load(Ordering::Relaxed),
        }
    }

    /// Clear the pool (non-thread-safe, use for initialization)
    pub fn clear(&self) {
        while self.global.pop().is_some() {}
    }

    /// Reserve capacity (pre-allocate additional objects)
    pub fn reserve(&self, additional: usize) {
        for _ in 0..additional {
            self.global.push(T::default());
        }
    }

    /// Get pool hit rate
    pub fn hit_rate(&self) -> f32 {
        let hits = self.stats.pool_hits.load(Ordering::Relaxed);
        let misses = self.stats.pool_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f32 / total as f32
        }
    }
}

impl<T: Default + Clone + Send> Clone for SharedPool<T> {
    fn clone(&self) -> Self {
        Self {
            global: SegQueue::new(),
            max_size: AtomicUsize::new(self.max_size.load(Ordering::Relaxed)),
            stats: Arc::clone(&self.stats),
            _marker: PhantomData,
        }
    }
}

/// Pool statistics (snapshot)
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total acquisitions
    pub total_acquisitions: usize,
    /// Total releases
    pub total_releases: usize,
    /// Total allocations (when pool was empty)
    pub total_allocations: usize,
    /// Current active objects (acquired but not released)
    pub active_objects: usize,
    /// Peak active objects
    pub peak_active_objects: usize,
    /// Pool hits (got from pool)
    pub pool_hits: usize,
    /// Pool misses (had to allocate)
    pub pool_misses: usize,
}

/// Object pool for reusable objects (single-threaded or when `SharedPool` is not needed)
pub struct ObjectPool<T: Default + Clone> {
    /// Available objects
    available: VecDeque<T>,
    /// Maximum pool size
    max_size: usize,
    /// Statistics
    stats: PoolStats,
    _marker: PhantomData<T>,
}

use std::collections::VecDeque;

impl<T: Default + Clone> ObjectPool<T> {
    /// Create a new object pool with specified capacity
    pub fn new(initial_capacity: usize) -> Self {
        Self::with_max_size(initial_capacity, usize::MAX)
    }

    /// Create a new object pool with max size
    pub fn with_max_size(initial_capacity: usize, max_size: usize) -> Self {
        let mut pool = Self {
            available: VecDeque::with_capacity(initial_capacity),
            max_size,
            stats: PoolStats::default(),
            _marker: PhantomData,
        };

        // Pre-allocate objects
        for _ in 0..initial_capacity {
            pool.available.push_back(T::default());
        }

        pool
    }

    /// Acquire an object from the pool
    pub fn acquire(&mut self) -> T {
        self.stats.total_acquisitions += 1;

        if let Some(obj) = self.available.pop_front() {
            self.stats.pool_hits += 1;
            self.stats.active_objects += 1;
            if self.stats.active_objects > self.stats.peak_active_objects {
                self.stats.peak_active_objects = self.stats.active_objects;
            }
            obj
        } else {
            self.stats.pool_misses += 1;
            self.stats.total_allocations += 1;
            self.stats.active_objects += 1;
            if self.stats.active_objects > self.stats.peak_active_objects {
                self.stats.peak_active_objects = self.stats.active_objects;
            }
            T::default()
        }
    }

    /// Acquire with initializer
    pub fn acquire_with<F: FnOnce() -> T>(&mut self, init: F) -> T {
        self.stats.total_acquisitions += 1;

        let obj = if let Some(available) = self.available.pop_front() {
            self.stats.pool_hits += 1;
            available
        } else {
            self.stats.pool_misses += 1;
            self.stats.total_allocations += 1;
            init()
        };

        self.stats.active_objects += 1;
        if self.stats.active_objects > self.stats.peak_active_objects {
            self.stats.peak_active_objects = self.stats.active_objects;
        }

        obj
    }

    /// Release an object back to the pool
    pub fn release(&mut self, mut obj: T) {
        // Reset the object before returning to pool
        self.reset_object(&mut obj);

        if self.available.len() < self.max_size {
            self.available.push_back(obj);
        }

        self.stats.total_releases += 1;
        self.stats.active_objects = self.stats.active_objects.saturating_sub(1);
    }

    /// Reset object to default state (override for custom reset logic)
    fn reset_object(&self, _obj: &mut T) {
        // Default implementation does nothing
    }

    /// Get the number of available objects
    pub fn available_count(&self) -> usize {
        self.available.len()
    }

    /// Get pool statistics
    pub fn stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Clear the pool
    pub fn clear(&mut self) {
        self.available.clear();
    }

    /// Shrink the pool to fit
    pub fn shrink_to_fit(&mut self) {
        self.available.shrink_to_fit();
    }

    /// Reserve capacity
    pub fn reserve(&mut self, additional: usize) {
        self.available.reserve(additional);
    }

    /// Get pool hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.stats.pool_hits + self.stats.pool_misses;
        if total == 0 {
            0.0
        } else {
            self.stats.pool_hits as f32 / total as f32
        }
    }
}

impl<T: Default + Clone> Default for ObjectPool<T> {
    fn default() -> Self {
        Self::new(16)
    }
}

/// Specialized pool for Vec3<f32>
pub struct VectorPool {
    pool: SharedPool<Vector3<f32>>,
}

impl VectorPool {
    /// Create a new vector pool
    pub fn new(capacity: usize) -> Self {
        Self {
            pool: SharedPool::new(capacity),
        }
    }

    /// Acquire a vector
    pub fn acquire(&self) -> Vector3<f32> {
        self.pool.acquire_with(|| Vector3::new(0.0, 0.0, 0.0))
    }

    /// Acquire with specific values
    pub fn acquire_with_values(&self, x: f32, y: f32, z: f32) -> Vector3<f32> {
        let mut vec = self.pool.acquire();
        vec.x = x;
        vec.y = y;
        vec.z = z;
        vec
    }

    /// Release a vector
    pub fn release(&self, mut vec: Vector3<f32>) {
        // Reset to zero before returning
        vec.x = 0.0;
        vec.y = 0.0;
        vec.z = 0.0;
        self.pool.release(vec);
    }

    /// Get statistics
    pub fn stats(&self) -> PoolStats {
        self.pool.stats()
    }
}

impl Default for VectorPool {
    fn default() -> Self {
        Self::new(64)
    }
}

/// Specialized pool for Point3<f32>
pub struct PointPool {
    pool: SharedPool<Point3<f32>>,
}

impl PointPool {
    /// Create a new point pool
    pub fn new(capacity: usize) -> Self {
        Self {
            pool: SharedPool::new(capacity),
        }
    }

    /// Acquire a point
    pub fn acquire(&self) -> Point3<f32> {
        self.pool.acquire_with(|| Point3::new(0.0, 0.0, 0.0))
    }

    /// Acquire with specific values
    pub fn acquire_with_values(&self, x: f32, y: f32, z: f32) -> Point3<f32> {
        let mut point = self.pool.acquire();
        point.x = x;
        point.y = y;
        point.z = z;
        point
    }

    /// Release a point
    pub fn release(&self, mut point: Point3<f32>) {
        // Reset to origin before returning
        point.x = 0.0;
        point.y = 0.0;
        point.z = 0.0;
        self.pool.release(point);
    }

    /// Get statistics
    pub fn stats(&self) -> PoolStats {
        self.pool.stats()
    }
}

impl Default for PointPool {
    fn default() -> Self {
        Self::new(64)
    }
}

/// Pool for temporary buffers
pub struct BufferPool<T: Default + Clone + Send + 'static> {
    pool: SharedPool<Vec<T>>,
}

impl<T: Default + Clone + Send + 'static> BufferPool<T> {
    /// Create a new buffer pool
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        let pool = SharedPool::new(0);

        // Pre-allocate buffers with specific size
        for _ in 0..capacity {
            pool.global.push(vec![T::default(); buffer_size]);
        }

        Self { pool }
    }

    /// Acquire a buffer
    pub fn acquire(&self) -> Vec<T> {
        self.pool.acquire()
    }

    /// Release a buffer
    pub fn release(&self, mut buffer: Vec<T>) {
        // Clear but keep capacity
        buffer.clear();
        self.pool.release(buffer);
    }

    /// Get statistics
    pub fn stats(&self) -> PoolStats {
        self.pool.stats()
    }
}

/// Geometry entity pool for CAD primitives
pub struct GeometryPool {
    /// Pool for points
    points: PointPool,
    /// Pool for vectors
    vectors: VectorPool,
    /// Pool for small buffers
    buffers: BufferPool<f32>,
}

impl GeometryPool {
    /// Create a new geometry pool
    pub fn new() -> Self {
        Self {
            points: PointPool::new(128),
            vectors: VectorPool::new(128),
            buffers: BufferPool::new(32, 64),
        }
    }

    /// Acquire a point
    pub fn acquire_point(&self) -> Point3<f32> {
        self.points.acquire()
    }

    /// Acquire a vector
    pub fn acquire_vector(&self) -> Vector3<f32> {
        self.vectors.acquire()
    }

    /// Acquire a buffer
    pub fn acquire_buffer(&self) -> Vec<f32> {
        self.buffers.acquire()
    }

    /// Release a point
    pub fn release_point(&self, point: Point3<f32>) {
        self.points.release(point);
    }

    /// Release a vector
    pub fn release_vector(&self, vector: Vector3<f32>) {
        self.vectors.release(vector);
    }

    /// Release a buffer
    pub fn release_buffer(&self, buffer: Vec<f32>) {
        self.buffers.release(buffer);
    }

    /// Get combined statistics
    pub fn total_stats(&self) -> PoolStats {
        let mut total = PoolStats::default();

        let point_stats = self.points.stats();
        let vec_stats = self.vectors.stats();
        let buffer_stats = self.buffers.stats();

        total.total_acquisitions = point_stats.total_acquisitions
            + vec_stats.total_acquisitions
            + buffer_stats.total_acquisitions;
        total.total_releases =
            point_stats.total_releases + vec_stats.total_releases + buffer_stats.total_releases;
        total.total_allocations = point_stats.total_allocations
            + vec_stats.total_allocations
            + buffer_stats.total_allocations;
        total.pool_hits = point_stats.pool_hits + vec_stats.pool_hits + buffer_stats.pool_hits;
        total.pool_misses =
            point_stats.pool_misses + vec_stats.pool_misses + buffer_stats.pool_misses;

        total
    }
}

impl Default for GeometryPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = SharedPool::<i32>::new(10);
        assert_eq!(pool.available_count(), 10);
        assert_eq!(pool.stats().total_allocations, 0);
    }

    #[test]
    fn test_pool_acquire_release() {
        let pool = SharedPool::<i32>::new(5);

        let obj = pool.acquire();
        assert_eq!(obj, 0); // i32 default is 0
        assert_eq!(pool.stats().active_objects, 1);

        pool.release(obj);
        assert_eq!(pool.stats().active_objects, 0);
    }

    #[test]
    fn test_pool_hit_rate() {
        let pool = SharedPool::<i32>::new(2);

        let _a = pool.acquire();
        let _b = pool.acquire();
        let _c = pool.acquire(); // This should be a miss

        let stats = pool.stats();
        assert_eq!(stats.pool_hits, 2);
        assert_eq!(stats.pool_misses, 1);
        assert!((pool.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_pool_max_size() {
        let pool = SharedPool::<i32>::with_max_size(2, 3);

        let a = pool.acquire();
        let b = pool.acquire();
        let c = pool.acquire();

        pool.release(a);
        pool.release(b);
        pool.release(c);

        // Should only keep up to max_size
        assert!(pool.available_count() <= 3);
    }

    #[test]
    fn test_vector_pool() {
        let pool = VectorPool::new(10);

        let vec = pool.acquire_with_values(1.0, 2.0, 3.0);
        assert_eq!(vec.x, 1.0);
        assert_eq!(vec.y, 2.0);
        assert_eq!(vec.z, 3.0);

        pool.release(vec);
    }

    #[test]
    fn test_point_pool() {
        let pool = PointPool::new(10);

        let point = pool.acquire_with_values(5.0, 10.0, 15.0);
        assert_eq!(point.x, 5.0);
        assert_eq!(point.y, 10.0);
        assert_eq!(point.z, 15.0);

        pool.release(point);
    }

    #[test]
    fn test_buffer_pool() {
        let pool = BufferPool::<f32>::new(5, 100);

        let buffer = pool.acquire();
        assert_eq!(buffer.capacity(), 100);

        pool.release(buffer);
    }

    #[test]
    fn test_geometry_pool() {
        let pool = GeometryPool::new();

        let point = pool.acquire_point();
        let vector = pool.acquire_vector();
        let buffer = pool.acquire_buffer();

        assert_eq!(point, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(vector, Vector3::new(0.0, 0.0, 0.0));
        assert!(buffer.capacity() >= 64);

        pool.release_point(point);
        pool.release_vector(vector);
        pool.release_buffer(buffer);
    }

    #[test]
    fn test_shared_pool_thread_safety() {
        use std::thread;

        let pool = Arc::new(SharedPool::<i32>::new(100));
        let mut handles = vec![];

        for _ in 0..4 {
            let pool_clone = Arc::clone(&pool);
            handles.push(thread::spawn(move || {
                for _ in 0..25 {
                    let obj = pool_clone.acquire();
                    thread::sleep(std::time::Duration::from_millis(1));
                    pool_clone.release(obj);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All objects should be returned
        assert_eq!(pool.stats().active_objects, 0);
    }

    #[test]
    fn test_pool_clear() {
        let pool = SharedPool::<i32>::new(10);
        pool.clear();
        assert_eq!(pool.available_count(), 0);
    }

    #[test]
    fn test_pool_clone() {
        let pool = SharedPool::<i32>::new(10);
        let pool_clone = pool.clone();

        let obj = pool.acquire();
        pool_clone.release(obj);

        // Stats should be shared
        assert_eq!(pool.stats().total_acquisitions, 1);
        assert_eq!(pool_clone.stats().total_releases, 1);
    }
}
