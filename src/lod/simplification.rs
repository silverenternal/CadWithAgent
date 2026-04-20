//! Mesh Simplification Algorithms
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
//!
//! This module provides algorithms for reducing mesh complexity while preserving
//! visual quality. Supported algorithms:
//!
//! - **Vertex Clustering**: Fast simplification by merging nearby vertices
//! - **Edge Collapse**: Quality-focused simplification using edge collapse operations
//!
//! # Architecture
//!
//! ```text
//! Mesh Simplification Flow:
//!
//!     ┌──────────────────┐
//!     │  Original Mesh   │ (High precision, 10K+ faces)
//!     └────────┬─────────┘
//!              │
//!              ▼
//!     ┌──────────────────┐
//!     │  MeshSimplifier  │ (Selects algorithm based on config)
//!     └────────┬─────────┘
//!              │
//!       ┌───────┴───────┐
//!       ▼               ▼
//! ┌──────────────┐ ┌──────────────┐
//! │ Vertex       │ │ Edge         │
//! │ Clustering   │ │ Collapse     │
//! │ (Fast)       │ │ (Quality)    │
//! └──────────────┘ └──────────────┘
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use cadagent::lod::{MeshSimplifier, LodLevel};
//! use cadagent::geometry::nurbs::Mesh;
//!
//! let simplifier = MeshSimplifier::new();
//! let original_mesh = Mesh::new(); // Your mesh here
//!
//! // Simplify based on LOD level
//! let simplified = simplifier.simplify(&original_mesh, LodLevel::Medium);
//!
//! // Or use specific algorithms
//! let clustered = simplifier.vertex_clustering(&original_mesh, 0.5);
//! let collapsed = simplifier.edge_collapse(&original_mesh, 1000);
//! ```

use crate::geometry::nurbs::Mesh;
use crate::geometry::Point3D;
use nalgebra::Vector3;
use std::collections::{HashMap, HashSet};

/// Mesh simplification strategies
///
/// Each strategy offers different trade-offs between speed and quality:
/// - `VertexClustering`: Very fast, suitable for real-time LOD, but lower quality
/// - `EdgeCollapse`: Slower but preserves mesh features better
/// - `Auto`: Automatically selects based on mesh size and target reduction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimplificationStrategy {
    /// Fast vertex clustering - good for real-time LOD
    VertexClustering,
    /// Quality-focused edge collapse - slower but better mesh quality
    EdgeCollapse,
    /// Automatic selection based on mesh size and target reduction
    Auto,
}

/// Configuration for mesh simplification
///
/// Controls the behavior and quality settings for mesh simplification algorithms.
#[derive(Debug, Clone)]
pub struct SimplificationConfig {
    /// Minimum number of vertices to keep (absolute)
    pub min_vertices: usize,
    /// Maximum number of vertices to keep (absolute)
    pub max_vertices: Option<usize>,
    /// Target reduction ratio (0.0 = no reduction, 1.0 = maximum reduction)
    pub target_ratio: f64,
    /// Preserve boundary edges (prevents gaps in watertight meshes)
    pub preserve_boundaries: bool,
    /// Snap vertices to grid (for vertex clustering)
    pub snap_to_grid: bool,
    /// Grid size for vertex clustering
    pub grid_size: f64,
}

impl Default for SimplificationConfig {
    fn default() -> Self {
        Self {
            min_vertices: 4,
            max_vertices: None,
            target_ratio: 0.5,
            preserve_boundaries: true,
            snap_to_grid: true,
            grid_size: 0.1,
        }
    }
}

impl SimplificationConfig {
    /// Create configuration for a specific LOD level
    ///
    /// # Arguments
    /// * `level` - Target LOD level
    ///
    /// # Returns
    /// Configuration optimized for the specified LOD level
    pub fn for_lod_level(level: LodLevel) -> Self {
        match level {
            LodLevel::High => Self {
                target_ratio: 0.0,
                ..Default::default()
            },
            LodLevel::Medium => Self {
                target_ratio: 0.5,
                ..Default::default()
            },
            LodLevel::Low => Self {
                target_ratio: 0.9,
                min_vertices: 10,
                ..Default::default()
            },
        }
    }
}

/// Mesh Simplifier - Reduces mesh complexity while preserving visual quality
///
/// The simplifier supports two main algorithms:
/// - **Vertex Clustering**: Fast, grid-based vertex merging
/// - **Edge Collapse**: Quality-focused, iteratively collapses shortest edges
///
/// # Examples
///
/// ```rust,ignore
/// use cadagent::lod::{MeshSimplifier, LodLevel};
/// use cadagent::geometry::nurbs::Mesh;
///
/// let simplifier = MeshSimplifier::new();
/// let mesh = Mesh::new(); // Your mesh here
/// let simplified = simplifier.simplify(&mesh, LodLevel::Medium);
/// ```
#[derive(Debug, Clone)]
pub struct MeshSimplifier {
    config: SimplificationConfig,
    strategy: SimplificationStrategy,
}

impl Default for MeshSimplifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshSimplifier {
    /// Create a new mesh simplifier with default configuration
    ///
    /// Uses default settings with Auto strategy for adaptive simplification.
    pub fn new() -> Self {
        Self {
            config: SimplificationConfig::default(),
            strategy: SimplificationStrategy::Auto,
        }
    }

    /// Create simplifier with custom configuration
    ///
    /// # Arguments
    /// * `config` - Custom simplification configuration
    pub fn with_config(config: SimplificationConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Set the simplification strategy
    ///
    /// # Arguments
    /// * `strategy` - The algorithm to use for simplification
    pub fn set_strategy(&mut self, strategy: SimplificationStrategy) {
        self.strategy = strategy;
    }

    /// Get current configuration
    ///
    /// # Returns
    /// Reference to the current simplification configuration
    pub fn config(&self) -> &SimplificationConfig {
        &self.config
    }

    /// Simplify mesh according to LOD level
    ///
    /// # Arguments
    /// * `mesh` - Input mesh to simplify
    /// * `level` - Target LOD level
    ///
    /// # Returns
    /// Simplified mesh with reduced vertex/face count
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cadagent::lod::{MeshSimplifier, LodLevel};
    /// use cadagent::geometry::nurbs::Mesh;
    ///
    /// let simplifier = MeshSimplifier::new();
    /// let mesh = Mesh::new(); // Your mesh here
    /// let simplified = simplifier.simplify(&mesh, LodLevel::Medium);
    /// ```
    pub fn simplify(&self, mesh: &Mesh, level: LodLevel) -> Mesh {
        if level == LodLevel::High {
            mesh.clone()
        } else {
            let config = SimplificationConfig::for_lod_level(level);
            self.simplify_with_config(mesh, &config)
        }
    }

    /// Simplify mesh with custom configuration
    ///
    /// # Arguments
    /// * `mesh` - Input mesh to simplify
    /// * `config` - Simplification configuration
    ///
    /// # Returns
    /// Simplified mesh with applied configuration
    pub fn simplify_with_config(&self, mesh: &Mesh, config: &SimplificationConfig) -> Mesh {
        if config.target_ratio <= 0.0 {
            return mesh.clone();
        }

        let target_vertices = self.calculate_target_vertices(mesh, config);

        if target_vertices >= mesh.num_vertices() {
            return mesh.clone();
        }

        // Choose strategy based on configuration and mesh size
        let strategy = self.choose_strategy(mesh, target_vertices);

        match strategy {
            SimplificationStrategy::VertexClustering => {
                self.vertex_clustering_with_config(mesh, config)
            }
            SimplificationStrategy::EdgeCollapse => {
                self.edge_collapse_with_config(mesh, target_vertices)
            }
            SimplificationStrategy::Auto => {
                // Auto: use vertex clustering for large reductions, edge collapse for quality
                if config.target_ratio > 0.7 {
                    self.vertex_clustering_with_config(mesh, config)
                } else {
                    self.edge_collapse_with_config(mesh, target_vertices)
                }
            }
        }
    }

    /// Calculate target vertex count based on configuration
    fn calculate_target_vertices(&self, mesh: &Mesh, config: &SimplificationConfig) -> usize {
        let current = mesh.num_vertices();
        let target = (current as f64 * (1.0 - config.target_ratio)) as usize;

        target
            .max(config.min_vertices)
            .min(config.max_vertices.unwrap_or(usize::MAX))
    }

    /// Choose the best simplification strategy
    fn choose_strategy(&self, mesh: &Mesh, target_vertices: usize) -> SimplificationStrategy {
        match self.strategy {
            SimplificationStrategy::Auto => {
                // For very large reductions, vertex clustering is faster
                // For moderate reductions, edge collapse gives better quality
                let reduction = 1.0 - (target_vertices as f64 / mesh.num_vertices() as f64);
                if reduction > 0.8 {
                    SimplificationStrategy::VertexClustering
                } else {
                    SimplificationStrategy::EdgeCollapse
                }
            }
            other => other,
        }
    }

    /// Vertex clustering simplification
    ///
    /// This algorithm divides 3D space into a grid and merges all vertices
    /// within each grid cell into a single vertex. This is very fast but
    /// may produce lower quality results compared to edge collapse.
    ///
    /// # Arguments
    /// * `mesh` - Input mesh
    /// * `cell_size` - Size of grid cells (larger = more simplification)
    ///
    /// # Returns
    /// Simplified mesh with merged vertices
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cadagent::lod::MeshSimplifier;
    /// use cadagent::geometry::nurbs::Mesh;
    ///
    /// let simplifier = MeshSimplifier::new();
    /// let mesh = Mesh::new(); // Your mesh here
    /// let simplified = simplifier.vertex_clustering(&mesh, 0.5);
    /// ```
    pub fn vertex_clustering(&self, mesh: &Mesh, cell_size: f64) -> Mesh {
        let mut config = self.config.clone();
        config.grid_size = cell_size;
        config.snap_to_grid = true;
        self.vertex_clustering_with_config(mesh, &config)
    }

    /// Vertex clustering with configuration
    fn vertex_clustering_with_config(&self, mesh: &Mesh, config: &SimplificationConfig) -> Mesh {
        if mesh.num_vertices() == 0 {
            return Mesh::new();
        }

        let cell_size = config.grid_size.max(f64::EPSILON);
        let inv_cell_size = 1.0 / cell_size;

        // Map grid cell coordinates to merged vertex index
        let mut cell_map: HashMap<(i32, i32, i32), usize> = HashMap::new();
        // Accumulated vertex positions for averaging
        let mut vertex_accumulators: Vec<(Vector3<f64>, usize)> = Vec::new();

        // Helper function to get grid cell coordinates
        let get_cell = |pos: &Vector3<f64>| -> (i32, i32, i32) {
            (
                (pos.x * inv_cell_size).floor() as i32,
                (pos.y * inv_cell_size).floor() as i32,
                (pos.z * inv_cell_size).floor() as i32,
            )
        };

        // First pass: cluster vertices
        let mut vertex_to_new: Vec<usize> = Vec::with_capacity(mesh.vertices.len());

        for vertex in &mesh.vertices {
            let pos = Vector3::new(vertex.x, vertex.y, vertex.z);
            let cell = get_cell(&pos);

            let new_idx = if let Some(&idx) = cell_map.get(&cell) {
                idx
            } else {
                let new_idx = vertex_accumulators.len();
                vertex_accumulators.push((Vector3::zeros(), 0));
                cell_map.insert(cell, new_idx);
                new_idx
            };

            // Accumulate position for averaging
            let (acc_pos, acc_count) = &mut vertex_accumulators[new_idx];
            *acc_pos += pos;
            *acc_count += 1;

            vertex_to_new.push(new_idx);
        }

        // Compute averaged vertex positions
        let new_vertices: Vec<Point3D> = vertex_accumulators
            .iter()
            .map(|(pos, count)| {
                let avg = pos / (*count as f64);
                Point3D::new(avg.x, avg.y, avg.z)
            })
            .collect();

        // Second pass: rebuild indices, removing degenerate triangles
        let mut new_indices = Vec::with_capacity(mesh.indices.len());

        for &[i0, i1, i2] in &mesh.indices {
            let ni0 = vertex_to_new[i0 as usize];
            let ni1 = vertex_to_new[i1 as usize];
            let ni2 = vertex_to_new[i2 as usize];

            // Skip degenerate triangles (all vertices the same)
            if ni0 != ni1 && ni1 != ni2 && ni0 != ni2 {
                new_indices.push([ni0 as u32, ni1 as u32, ni2 as u32]);
            }
        }

        // Renumber indices to be contiguous
        let mut index_map: Vec<usize> = vec![usize::MAX; new_vertices.len()];
        let mut new_index = 0;
        let mut final_indices = Vec::with_capacity(new_indices.len());

        for &[i0, i1, i2] in &new_indices {
            let mi0 = index_map[i0 as usize];
            let mi1 = index_map[i1 as usize];
            let mi2 = index_map[i2 as usize];

            let fi0 = if mi0 == usize::MAX {
                index_map[i0 as usize] = new_index;
                new_index += 1;
                i0
            } else {
                mi0 as u32
            };

            let fi1 = if mi1 == usize::MAX {
                index_map[i1 as usize] = new_index;
                new_index += 1;
                i1
            } else {
                mi1 as u32
            };

            let fi2 = if mi2 == usize::MAX {
                index_map[i2 as usize] = new_index;
                new_index += 1;
                i2
            } else {
                mi2 as u32
            };

            final_indices.push([fi0, fi1, fi2]);
        }

        // Build final vertex list (only used vertices)
        let final_vertices: Vec<Point3D> = (0..new_index)
            .map(|i| {
                let old_idx = index_map.iter().position(|&x| x == i).unwrap();
                new_vertices[old_idx]
            })
            .collect();

        Mesh {
            vertices: final_vertices,
            indices: final_indices,
        }
    }

    /// Edge collapse simplification
    ///
    /// This algorithm iteratively collapses edges to reduce vertex count.
    /// It uses edge length as a heuristic (collapses shortest edges first),
    /// preserving mesh quality better than vertex clustering.
    ///
    /// # Arguments
    /// * `mesh` - Input mesh
    /// * `target_faces` - Target number of triangles
    ///
    /// # Returns
    /// Simplified mesh with reduced face count
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cadagent::lod::MeshSimplifier;
    /// use cadagent::geometry::nurbs::Mesh;
    ///
    /// let simplifier = MeshSimplifier::new();
    /// let mesh = Mesh::new(); // Your mesh here
    /// let simplified = simplifier.edge_collapse(&mesh, 1000);
    /// ```
    pub fn edge_collapse(&self, mesh: &Mesh, target_faces: usize) -> Mesh {
        if mesh.num_triangles() <= target_faces || mesh.num_vertices() == 0 {
            return mesh.clone();
        }

        self.edge_collapse_with_config(mesh, target_faces)
    }

    /// Edge collapse with configuration
    fn edge_collapse_with_config(&self, mesh: &Mesh, target_faces: usize) -> Mesh {
        // Simplified edge collapse implementation
        // A full implementation would use quadric error metrics

        // Build vertex adjacency information
        let mut vertex_edges: HashMap<usize, Vec<usize>> = HashMap::new();

        for (edge_idx, &[v0, v1, _]) in mesh.indices.iter().enumerate() {
            vertex_edges.entry(v0 as usize).or_default().push(edge_idx);
            vertex_edges.entry(v1 as usize).or_default().push(edge_idx);
        }

        // Calculate edge lengths and sort by length (collapse shortest first)
        let mut edge_lengths: Vec<(f64, usize, usize, usize)> = Vec::new();

        for (tri_idx, &[v0, v1, v2]) in mesh.indices.iter().enumerate() {
            let edges = [
                (v0 as usize, v1 as usize),
                (v1 as usize, v2 as usize),
                (v2 as usize, v0 as usize),
            ];

            for &(a, b) in &edges {
                let len = self.edge_length(&mesh.vertices[a], &mesh.vertices[b]);
                edge_lengths.push((len, a, b, tri_idx));
            }
        }

        edge_lengths.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Track collapsed vertices
        let mut collapsed: HashSet<usize> = HashSet::new();
        let mut vertex_remap: HashMap<usize, usize> = HashMap::new();

        // Collapse edges until we reach target
        let target_edges = target_faces * 3;
        let mut current_edges = mesh.indices.len() * 3;

        for (_len, v0, v1, _tri) in edge_lengths {
            if current_edges <= target_edges {
                break;
            }

            // Skip if either vertex is already collapsed
            if collapsed.contains(&v0) || collapsed.contains(&v1) {
                continue;
            }

            // Collapse edge: merge v1 into v0
            collapsed.insert(v1);
            vertex_remap.insert(v1, v0);

            // Update edge count (approximate)
            current_edges = current_edges.saturating_sub(3);
        }

        // Rebuild mesh with collapsed vertices
        let mut new_vertices = Vec::new();
        let mut old_to_new: Vec<Option<u32>> = vec![None; mesh.vertices.len()];

        for (i, vertex) in mesh.vertices.iter().enumerate() {
            if !collapsed.contains(&i) {
                let new_idx = new_vertices.len() as u32;
                new_vertices.push(*vertex);
                old_to_new[i] = Some(new_idx);
            }
        }

        // Rebuild indices
        let mut new_indices = Vec::new();
        for &[v0, v1, v2] in &mesh.indices {
            let mut nv0 = old_to_new[v0 as usize];
            let mut nv1 = old_to_new[v1 as usize];
            let mut nv2 = old_to_new[v2 as usize];

            // Follow remap chain
            if nv0.is_none() {
                let remapped = vertex_remap
                    .get(&(v0 as usize))
                    .copied()
                    .unwrap_or(v0 as usize);
                nv0 = old_to_new[remapped];
            }
            if nv1.is_none() {
                let remapped = vertex_remap
                    .get(&(v1 as usize))
                    .copied()
                    .unwrap_or(v1 as usize);
                nv1 = old_to_new[remapped];
            }
            if nv2.is_none() {
                let remapped = vertex_remap
                    .get(&(v2 as usize))
                    .copied()
                    .unwrap_or(v2 as usize);
                nv2 = old_to_new[remapped];
            }

            if let (Some(n0), Some(n1), Some(n2)) = (nv0, nv1, nv2) {
                // Skip degenerate triangles
                if n0 != n1 && n1 != n2 && n0 != n2 {
                    new_indices.push([n0, n1, n2]);
                }
            }
        }

        Mesh {
            vertices: new_vertices,
            indices: new_indices,
        }
    }

    /// Calculate edge length between two vertices
    ///
    /// # Arguments
    /// * `a` - First vertex position
    /// * `b` - Second vertex position
    ///
    /// # Returns
    /// Euclidean distance between the two vertices
    fn edge_length(&self, a: &Point3D, b: &Point3D) -> f64 {
        let da = b.x - a.x;
        let db = b.y - a.y;
        let dc = b.z - a.z;
        (da * da + db * db + dc * dc).sqrt()
    }

    /// Get mesh statistics
    ///
    /// # Arguments
    /// * `mesh` - The mesh to analyze
    ///
    /// # Returns
    /// A tuple of (`vertex_count`, `face_count`)
    pub fn mesh_stats(mesh: &Mesh) -> (usize, usize) {
        (mesh.num_vertices(), mesh.num_triangles())
    }

    /// Calculate simplification ratio achieved
    ///
    /// # Arguments
    /// * `original` - Original mesh before simplification
    /// * `simplified` - Simplified mesh after processing
    ///
    /// # Returns
    /// A value between 0.0 (no simplification) and 1.0 (maximum simplification)
    pub fn simplification_ratio(original: &Mesh, simplified: &Mesh) -> f64 {
        if original.num_vertices() == 0 {
            return 0.0;
        }
        1.0 - (simplified.num_vertices() as f64 / original.num_vertices() as f64)
    }
}

// Re-export LodLevel for convenience
pub use super::lod_manager::LodLevel;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mesh() -> Mesh {
        // Create a simple cube mesh
        let vertices = vec![
            // Front face
            Point3D::new(0.0, 0.0, 0.0),
            Point3D::new(1.0, 0.0, 0.0),
            Point3D::new(1.0, 1.0, 0.0),
            Point3D::new(0.0, 1.0, 0.0),
            // Back face
            Point3D::new(0.0, 0.0, 1.0),
            Point3D::new(1.0, 0.0, 1.0),
            Point3D::new(1.0, 1.0, 1.0),
            Point3D::new(0.0, 1.0, 1.0),
        ];

        let indices = vec![
            // Front
            [0, 1, 2],
            [0, 2, 3],
            // Back
            [5, 4, 7],
            [5, 7, 6],
            // Top
            [3, 2, 6],
            [3, 6, 7],
            // Bottom
            [4, 5, 1],
            [4, 1, 0],
            // Right
            [1, 5, 6],
            [1, 6, 2],
            // Left
            [4, 0, 3],
            [4, 3, 7],
        ];

        Mesh { vertices, indices }
    }

    #[test]
    fn test_simplification_config_default() {
        let config = SimplificationConfig::default();
        assert_eq!(config.target_ratio, 0.5);
        assert!(config.preserve_boundaries);
    }

    #[test]
    fn test_simplification_config_for_lod() {
        let high_config = SimplificationConfig::for_lod_level(LodLevel::High);
        assert_eq!(high_config.target_ratio, 0.0);

        let med_config = SimplificationConfig::for_lod_level(LodLevel::Medium);
        assert_eq!(med_config.target_ratio, 0.5);

        let low_config = SimplificationConfig::for_lod_level(LodLevel::Low);
        assert_eq!(low_config.target_ratio, 0.9);
    }

    #[test]
    fn test_mesh_simplifier_new() {
        let simplifier = MeshSimplifier::new();
        assert_eq!(simplifier.config().target_ratio, 0.5);
    }

    #[test]
    fn test_mesh_simplifier_simplify_high() {
        let simplifier = MeshSimplifier::new();
        let mesh = create_test_mesh();
        let original_vertices = mesh.vertices.len();

        let simplified = simplifier.simplify(&mesh, LodLevel::High);

        assert_eq!(simplified.vertices.len(), original_vertices);
    }

    #[test]
    fn test_mesh_simplifier_simplify_medium() {
        let simplifier = MeshSimplifier::new();
        let mesh = create_test_mesh();

        let simplified = simplifier.simplify(&mesh, LodLevel::Medium);

        // Medium LOD should not increase vertices
        assert!(simplified.vertices.len() <= mesh.vertices.len());
        // Should have some geometry (either vertices or at least not crash)
        assert!(!simplified.vertices.is_empty());
    }

    #[test]
    fn test_mesh_simplifier_simplify_low() {
        let simplifier = MeshSimplifier::new();
        let mesh = create_test_mesh();

        let simplified = simplifier.simplify(&mesh, LodLevel::Low);

        // Low LOD should not increase vertices
        assert!(simplified.vertices.len() <= mesh.vertices.len());
        // Should keep at least min_vertices
        assert!(simplified.vertices.len() >= simplifier.config().min_vertices);
    }

    #[test]
    fn test_vertex_clustering() {
        let simplifier = MeshSimplifier::new();
        let mesh = create_test_mesh();

        // Large cell size should produce significant simplification
        let simplified = simplifier.vertex_clustering(&mesh, 10.0);
        assert!(simplified.vertices.len() <= mesh.vertices.len());

        // Small cell size should preserve more detail
        let simplified_fine = simplifier.vertex_clustering(&mesh, 0.01);
        assert!(simplified_fine.vertices.len() >= simplified.vertices.len());
    }

    #[test]
    fn test_edge_collapse() {
        let simplifier = MeshSimplifier::new();
        let mesh = create_test_mesh();
        let original_faces = mesh.indices.len();

        // Request fewer faces
        let simplified = simplifier.edge_collapse(&mesh, original_faces / 2);

        assert!(simplified.indices.len() <= original_faces);
        assert!(!simplified.vertices.is_empty());
    }

    #[test]
    fn test_simplification_ratio() {
        let mesh = create_test_mesh();
        let simplified = Mesh {
            vertices: mesh.vertices[..4].to_vec(),
            indices: mesh.indices[..2].to_vec(),
        };

        let ratio = MeshSimplifier::simplification_ratio(&mesh, &simplified);
        assert!(ratio > 0.0 && ratio < 1.0);
    }

    #[test]
    fn test_mesh_stats() {
        let mesh = create_test_mesh();
        let (verts, faces) = MeshSimplifier::mesh_stats(&mesh);
        assert_eq!(verts, 8);
        assert_eq!(faces, 12);
    }

    #[test]
    fn test_empty_mesh() {
        let simplifier = MeshSimplifier::new();
        let empty_mesh = Mesh::new();

        let simplified = simplifier.simplify(&empty_mesh, LodLevel::Medium);
        assert_eq!(simplified.vertices.len(), 0);
        assert_eq!(simplified.indices.len(), 0);
    }

    #[test]
    fn test_simplifier_config() {
        let mut simplifier = MeshSimplifier::new();

        let config = SimplificationConfig {
            target_ratio: 0.75,
            ..Default::default()
        };
        simplifier.config = config.clone();

        assert_eq!(simplifier.config().target_ratio, 0.75);
    }
}
