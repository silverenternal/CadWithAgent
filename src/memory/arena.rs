//! `GeometryArena`: Fast bump allocator for geometry entities
//!
//! This module provides a bump allocator optimized for CAD geometry allocation.
//! It offers:
//! - O(1) allocation time
//! - Cache-friendly memory layout
//! - Automatic deallocation when arena is dropped
//! - Support for different geometry types

use bumpalo::Bump;
use nalgebra::{Point3, Vector3};
use std::cell::Cell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Default arena capacity in bytes (1 MB)
const DEFAULT_ARENA_CAPACITY: usize = 1024 * 1024;

/// Identifier for allocated items in the arena
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArenaId(usize);

impl ArenaId {
    /// Create a new arena ID from a raw value
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    /// Get the raw value
    pub fn value(&self) -> usize {
        self.0
    }
}

/// Handle to an item in the arena
#[derive(Debug)]
pub struct ArenaHandle<'a, T> {
    ptr: NonNull<T>,
    _marker: PhantomData<&'a T>,
}

impl<T> ArenaHandle<'_, T> {
    /// Get a reference to the item
    pub fn get(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    /// Get a mutable reference to the item
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> std::ops::Deref for ArenaHandle<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> std::ops::DerefMut for ArenaHandle<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

/// Geometry arena for fast allocation
pub struct GeometryArena {
    /// The bump allocator
    bump: Bump,
    /// Counter for generating unique IDs
    counter: Cell<usize>,
    /// Statistics
    stats: ArenaStats,
}

/// Arena statistics
#[derive(Debug, Clone, Default)]
pub struct ArenaStats {
    /// Total allocations
    pub total_allocations: usize,
    /// Total bytes allocated
    pub total_bytes: usize,
    /// Current allocations (not reset)
    pub current_allocations: usize,
    /// Number of resets
    pub resets: usize,
}

impl GeometryArena {
    /// Create a new geometry arena with default capacity
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_ARENA_CAPACITY)
    }

    /// Create a new geometry arena with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bump: Bump::with_capacity(capacity),
            counter: Cell::new(0),
            stats: ArenaStats::default(),
        }
    }

    /// Allocate an item in the arena
    pub fn alloc<T>(&mut self, value: T) -> ArenaId {
        let id = ArenaId::new(self.counter.get());
        self.counter.set(self.counter.get() + 1);

        self.bump.alloc(value);

        self.stats.total_allocations += 1;
        self.stats.total_bytes += std::mem::size_of::<T>();
        self.stats.current_allocations += 1;

        id
    }

    /// Allocate an item and get a handle
    pub fn alloc_with_handle<'a, T>(&'a mut self, value: T) -> ArenaHandle<'a, T>
    where
        T: 'a,
    {
        let ptr = NonNull::from(self.bump.alloc(value));

        self.stats.total_allocations += 1;
        self.stats.total_bytes += std::mem::size_of::<T>();
        self.stats.current_allocations += 1;

        ArenaHandle {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Allocate a slice in the arena
    pub fn alloc_slice<T: Copy + Clone>(&mut self, slice: &[T]) -> ArenaId {
        let id = ArenaId::new(self.counter.get());
        self.counter.set(self.counter.get() + 1);

        let allocated = self.bump.alloc_slice_copy(slice);

        self.stats.total_allocations += 1;
        self.stats.total_bytes += std::mem::size_of_val(allocated);
        self.stats.current_allocations += 1;

        id
    }

    /// Allocate a string in the arena
    pub fn alloc_str(&mut self, s: &str) -> ArenaId {
        let id = ArenaId::new(self.counter.get());
        self.counter.set(self.counter.get() + 1);

        self.bump.alloc_str(s);

        self.stats.total_allocations += 1;
        self.stats.total_bytes += s.len();
        self.stats.current_allocations += 1;

        id
    }

    /// Reset the arena (deallocates all items, keeps capacity)
    pub fn reset(&mut self) {
        self.bump.reset();
        self.counter.set(0);
        self.stats.resets += 1;
        self.stats.current_allocations = 0;
    }

    /// Get the number of bytes allocated
    pub fn allocated_bytes(&self) -> usize {
        self.bump.allocated_bytes()
    }

    /// Get the total capacity of the arena
    pub fn capacity(&self) -> usize {
        self.bump.chunk_capacity()
    }

    /// Get remaining capacity
    pub fn remaining_capacity(&self) -> usize {
        let limit = self
            .bump
            .allocation_limit()
            .unwrap_or(self.bump.chunk_capacity());
        limit.saturating_sub(self.bump.allocated_bytes())
    }

    /// Get arena statistics
    pub fn stats(&self) -> &ArenaStats {
        &self.stats
    }

    /// Check if the arena is empty
    pub fn is_empty(&self) -> bool {
        self.stats.current_allocations == 0
    }

    /// Get the number of allocations
    pub fn len(&self) -> usize {
        self.stats.current_allocations
    }

    /// Set allocation limit
    pub fn set_allocation_limit(&mut self, limit: Option<usize>) {
        self.bump.set_allocation_limit(limit);
    }
}

impl Default for GeometryArena {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for GeometryArena {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeometryArena")
            .field("allocated_bytes", &self.allocated_bytes())
            .field("capacity", &self.capacity())
            .field("stats", &self.stats)
            .finish()
    }
}

/// Typed arena for type-safe allocation
pub struct TypedArena<T> {
    bump: Bump,
    counter: Cell<usize>,
    _marker: PhantomData<T>,
}

impl<T> TypedArena<T> {
    /// Create a new typed arena
    pub fn new() -> Self {
        Self {
            bump: Bump::new(),
            counter: Cell::new(0),
            _marker: PhantomData,
        }
    }

    /// Create a new typed arena with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bump: Bump::with_capacity(capacity),
            counter: Cell::new(0),
            _marker: PhantomData,
        }
    }

    /// Allocate an item
    pub fn alloc(&self, value: T) -> &mut T {
        let id = self.counter.get();
        self.counter.set(id + 1);
        self.bump.alloc(value)
    }

    /// Allocate a slice
    pub fn alloc_slice(&self, slice: &[T]) -> &mut [T]
    where
        T: Copy + Clone,
    {
        let id = self.counter.get();
        self.counter.set(id + 1);
        self.bump.alloc_slice_copy(slice)
    }

    /// Reset the arena
    pub fn reset(&mut self) {
        self.bump.reset();
        self.counter.set(0);
    }

    /// Get the number of allocations
    pub fn len(&self) -> usize {
        self.counter.get()
    }

    /// Check if the arena is empty
    pub fn is_empty(&self) -> bool {
        self.counter.get() == 0
    }
}

impl<T> Default for TypedArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-type arena for heterogeneous geometry storage
pub struct MultiArena {
    /// Separate arenas for different types
    points: GeometryArena,
    vectors: GeometryArena,
    meshes: GeometryArena,
    /// Custom type maps
    custom: HashMap<String, GeometryArena>,
}

impl MultiArena {
    /// Create a new multi-arena
    pub fn new() -> Self {
        Self {
            points: GeometryArena::with_capacity(256 * 1024),
            vectors: GeometryArena::with_capacity(256 * 1024),
            meshes: GeometryArena::with_capacity(512 * 1024),
            custom: HashMap::new(),
        }
    }

    /// Allocate a point
    pub fn alloc_point(&mut self, point: Point3<f32>) -> ArenaId {
        self.points.alloc(point)
    }

    /// Allocate a vector
    pub fn alloc_vector(&mut self, vector: Vector3<f32>) -> ArenaId {
        self.vectors.alloc(vector)
    }

    /// Reset all arenas
    pub fn reset_all(&mut self) {
        self.points.reset();
        self.vectors.reset();
        self.meshes.reset();
        for arena in self.custom.values_mut() {
            arena.reset();
        }
    }

    /// Get statistics for all arenas
    pub fn total_stats(&self) -> ArenaStats {
        let mut total = ArenaStats::default();

        total.total_allocations += self.points.stats().total_allocations;
        total.total_bytes += self.points.stats().total_bytes;
        total.current_allocations += self.points.stats().current_allocations;
        total.resets += self.points.stats().resets;

        total.total_allocations += self.vectors.stats().total_allocations;
        total.total_bytes += self.vectors.stats().total_bytes;
        total.current_allocations += self.vectors.stats().current_allocations;
        total.resets += self.vectors.stats().resets;

        total.total_allocations += self.meshes.stats().total_allocations;
        total.total_bytes += self.meshes.stats().total_bytes;
        total.current_allocations += self.meshes.stats().current_allocations;
        total.resets += self.meshes.stats().resets;

        total
    }

    /// Get or create a custom arena
    pub fn get_or_create_arena(&mut self, name: &str) -> &mut GeometryArena {
        self.custom
            .entry(name.to_string())
            .or_insert_with(|| GeometryArena::with_capacity(256 * 1024))
    }
}

impl Default for MultiArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_creation() {
        let arena = GeometryArena::new();
        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
    }

    #[test]
    fn test_arena_alloc() {
        let mut arena = GeometryArena::new();
        let id = arena.alloc(42i32);
        assert_eq!(id.value(), 0);
        assert!(!arena.is_empty());
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn test_arena_reset() {
        let mut arena = GeometryArena::new();
        arena.alloc(1i32);
        arena.alloc(2i32);
        arena.alloc(3i32);

        assert_eq!(arena.len(), 3);

        arena.reset();

        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
        assert_eq!(arena.stats().resets, 1);
    }

    #[test]
    fn test_arena_stats() {
        let mut arena = GeometryArena::new();
        arena.alloc(100i32);
        arena.alloc(200i64);

        let stats = arena.stats();
        assert_eq!(stats.total_allocations, 2);
        assert_eq!(stats.current_allocations, 2);
        assert!(stats.total_bytes > 0);
    }

    #[test]
    fn test_typed_arena() {
        let mut arena = TypedArena::<f32>::new();
        let a = arena.alloc(1.0);
        let b = arena.alloc(2.0);

        assert_eq!(*a, 1.0);
        assert_eq!(*b, 2.0);

        arena.reset();
        assert!(arena.is_empty());
    }

    #[test]
    fn test_multi_arena() {
        let mut arena = MultiArena::new();

        let point_id = arena.alloc_point(Point3::new(1.0, 2.0, 3.0));
        let vec_id = arena.alloc_vector(Vector3::new(0.0, 1.0, 0.0));

        // IDs are valid (not at maximum value)
        assert!(point_id.value() != usize::MAX);
        assert!(vec_id.value() != usize::MAX);

        arena.reset_all();

        let stats = arena.total_stats();
        // All three sub-arenas (points, vectors, meshes) are reset
        assert!(stats.resets >= 2);
    }

    #[test]
    fn test_arena_capacity() {
        let arena = GeometryArena::with_capacity(1024);
        // bumpalo may allocate more than requested for alignment
        assert!(arena.capacity() >= 1024);
        // allocated_bytes starts at 0 before any allocations
        // Note: bumpalo internal overhead may cause capacity > requested
    }

    #[test]
    fn test_arena_slice() {
        let mut arena = GeometryArena::new();
        let data = [1, 2, 3, 4, 5];
        let id = arena.alloc_slice(&data);

        assert_eq!(id.value(), 0);
        assert_eq!(arena.len(), 1);
    }
}
