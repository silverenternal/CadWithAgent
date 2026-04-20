//! Dependency Graph - Track entity dependencies for impact analysis
//!
//! This module provides a directed graph for tracking dependencies between
//! entities, enabling efficient impact analysis when changes occur.
//!
//! # Performance Optimizations
//!
//! - **Topological sort caching**: Cached sort results avoid recomputation
//! - **SmallVec for edges**: Reduces allocations for nodes with few dependencies
//! - **Reference-based traversal**: BFS/DFS use references to reduce allocations
//! - **Incremental updates**: Roots/leaves updated incrementally on edge changes
//! - **Compact edge representation**: Uses NodeId directly instead of full edge structs

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};

/// Unique identifier for a node in the dependency graph
pub type NodeId = String;

/// Compact dependency edge using SmallVec optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Target node (the dependency)
    pub to: NodeId,
    /// Edge weight (for priority calculations)
    pub weight: f64,
    /// Optional edge label
    pub label: Option<String>,
}

impl DependencyEdge {
    /// Create a new dependency edge
    pub fn new(to: impl Into<NodeId>) -> Self {
        Self {
            to: to.into(),
            weight: 1.0,
            label: None,
        }
    }

    /// Set the edge weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Set the edge label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Error types for dependency graph operations
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DependencyGraphError {
    /// Attempted to create a circular dependency
    #[error("Circular dependency detected: {0:?}")]
    CircularDependency(Vec<NodeId>),
    /// Node not found in the graph
    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),
    /// Edge already exists
    #[error("Edge already exists: {0} -> {1}")]
    EdgeExists(NodeId, NodeId),
}

/// Result type for dependency graph operations
pub type DependencyGraphResult<T> = Result<T, DependencyGraphError>;

/// Dependency Graph - Tracks dependencies between entities
///
/// The dependency graph is a directed acyclic graph (DAG) where:
/// - Nodes represent entities (features, geometry, etc.)
/// - Edges represent "depends on" relationships
///
/// If A -> B, then A depends on B, and changes to B affect A.
///
/// # Performance Optimizations
///
/// - Uses `SmallVec<[DependencyEdge; 4]>` to reduce allocations for nodes with few dependencies
/// - Compact edge representation (removed redundant `from` field)
/// - Cached topological ordering
///
/// # Examples
///
/// ```
/// use cadagent::incremental::DependencyGraph;
///
/// let mut graph = DependencyGraph::new();
///
/// // Add dependencies
/// graph.add_dependency("extrude_1", "sketch_1");
/// graph.add_dependency("fillet_1", "extrude_1");
///
/// // Get all entities that depend on sketch_1
/// let dependents = graph.get_all_dependents("sketch_1");
/// assert!(dependents.contains(&"extrude_1".to_string()));
/// assert!(dependents.contains(&"fillet_1".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// Adjacency list: node -> list of edges (dependencies) using SmallVec optimization
    dependencies: HashMap<NodeId, SmallVec<[DependencyEdge; 4]>>,
    /// Reverse adjacency list: node -> list of nodes that depend on it
    dependents: HashMap<NodeId, SmallVec<[NodeId; 4]>>,
    /// Set of all nodes
    nodes: HashSet<NodeId>,
    /// Nodes with no dependencies (roots)
    roots: HashSet<NodeId>,
    /// Nodes with no dependents (leaves)
    leaves: HashSet<NodeId>,
    /// Cached topological ordering (invalidated on graph modification)
    #[serde(skip)]
    cached_topo_order: RefCell<Option<Vec<NodeId>>>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
            nodes: HashSet::new(),
            roots: HashSet::new(),
            leaves: HashSet::new(),
            cached_topo_order: RefCell::new(None),
        }
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.dependencies.values().map(|edges| edges.len()).sum()
    }

    /// Check if a node exists in the graph
    pub fn has_node(&self, node_id: &str) -> bool {
        self.nodes.contains(node_id)
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node_id: impl Into<NodeId>) {
        let id = node_id.into();
        if self.nodes.insert(id.clone()) {
            self.dependencies.entry(id.clone()).or_default();
            self.dependents.entry(id.clone()).or_default();
            // New nodes are both roots and leaves initially
            self.roots.insert(id.clone());
            self.leaves.insert(id);
        }
    }

    /// Remove a node from the graph
    ///
    /// Also removes all edges connected to this node.
    pub fn remove_node(&mut self, node_id: &str) -> Vec<NodeId> {
        let mut removed_edges = Vec::new();

        // Remove outgoing edges
        if let Some(edges) = self.dependencies.remove(node_id) {
            for edge in &edges {
                if let Some(dependents) = self.dependents.get_mut(&edge.to) {
                    dependents.retain(|d| d != node_id);
                }
            }
            removed_edges.extend(edges.into_iter().map(|e| e.to));
        }

        // Remove incoming edges
        if let Some(deps) = self.dependents.remove(node_id) {
            for dep in &deps {
                if let Some(edges) = self.dependencies.get_mut(dep) {
                    edges.retain(|e| e.to != node_id);
                }
            }
            removed_edges.extend(deps);
        }

        self.nodes.remove(node_id);
        self.roots.remove(node_id);
        self.leaves.remove(node_id);

        // Invalidate cached topological order
        *self.cached_topo_order.borrow_mut() = None;

        removed_edges
    }

    /// Add a dependency edge
    ///
    /// Creates an edge from `from` to `to`, meaning `from` depends on `to`.
    ///
    /// # Arguments
    /// * `from` - The dependent node
    /// * `to` - The dependency node
    ///
    /// # Errors
    /// Returns an error if adding this edge would create a cycle
    pub fn add_dependency(
        &mut self,
        from: impl Into<NodeId>,
        to: impl Into<NodeId>,
    ) -> DependencyGraphResult<()> {
        let from = from.into();
        let to = to.into();

        // Ensure nodes exist
        self.add_node(from.clone());
        self.add_node(to.clone());

        // Check for cycles before adding
        if self.would_create_cycle(&from, &to) {
            return Err(DependencyGraphError::CircularDependency(
                self.find_cycle_path(&from, &to),
            ));
        }

        // Add edge
        let edge = DependencyEdge::new(to.clone());
        self.dependencies
            .entry(from.clone())
            .or_default()
            .push(edge);
        self.dependents
            .entry(to.clone())
            .or_default()
            .push(from.clone());

        // Update roots and leaves
        self.roots.remove(&from);
        self.leaves.remove(&to);

        // Invalidate cached topological order
        *self.cached_topo_order.borrow_mut() = None;

        Ok(())
    }

    /// Check if adding an edge would create a cycle
    fn would_create_cycle(&self, from: &NodeId, to: &NodeId) -> bool {
        // If 'to' can reach 'from', then adding from->to creates a cycle
        self.can_reach(to, from)
    }

    /// Check if there's a path from source to target using iterative DFS
    fn can_reach(&self, source: &NodeId, target: &NodeId) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![source];

        while let Some(current) = stack.pop() {
            if current == target {
                return true;
            }

            if visited.insert(current) {
                if let Some(edges) = self.dependencies.get(current) {
                    for edge in edges {
                        if !visited.contains(&edge.to) {
                            stack.push(&edge.to);
                        }
                    }
                }
            }
        }

        false
    }

    /// Find the path that would create a cycle
    fn find_cycle_path(&self, from: &NodeId, to: &NodeId) -> Vec<NodeId> {
        let mut path = vec![from.clone()];
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((to, vec![to.clone()]));

        while let Some((current, current_path)) = queue.pop_front() {
            if current == from {
                path.extend(current_path);
                return path;
            }

            if visited.insert(current) {
                if let Some(deps) = self.dependents.get(current) {
                    for dep in deps {
                        if !visited.contains(dep) {
                            let mut new_path = current_path.clone();
                            new_path.push(dep.clone());
                            queue.push_back((dep, new_path));
                        }
                    }
                }
            }
        }

        path
    }

    /// Check if an edge exists
    pub fn has_edge(&self, from: &str, to: &str) -> bool {
        self.dependencies
            .get(from)
            .is_some_and(|edges| edges.iter().any(|e| e.to == to))
    }

    /// Get direct dependencies of a node
    pub fn get_dependencies(&self, node_id: &str) -> Vec<&NodeId> {
        self.dependencies
            .get(node_id)
            .map(|edges| edges.iter().map(|e| &e.to).collect())
            .unwrap_or_default()
    }

    /// Get direct dependents of a node
    pub fn get_dependents(&self, node_id: &str) -> Vec<&NodeId> {
        self.dependents
            .get(node_id)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Get all transitive dependencies of a node using iterative DFS
    pub fn get_all_dependencies(&self, node_id: &str) -> HashSet<NodeId> {
        let mut result = HashSet::new();
        let mut stack = Vec::new();

        if let Some(edges) = self.dependencies.get(node_id) {
            for edge in edges {
                stack.push(&edge.to);
            }
        }

        while let Some(current) = stack.pop() {
            if result.insert(current.clone()) {
                if let Some(edges) = self.dependencies.get(current) {
                    for edge in edges {
                        if !result.contains(&edge.to) {
                            stack.push(&edge.to);
                        }
                    }
                }
            }
        }

        result
    }

    /// Get all transitive dependents of a node using iterative DFS
    ///
    /// This returns all nodes that would be affected by a change to the given node.
    pub fn get_all_dependents(&self, node_id: &str) -> HashSet<NodeId> {
        let mut result = HashSet::new();
        let mut stack = Vec::new();

        if let Some(deps) = self.dependents.get(node_id) {
            for dep in deps {
                stack.push(dep);
            }
        }

        while let Some(current) = stack.pop() {
            if result.insert(current.clone()) {
                if let Some(deps) = self.dependents.get(current) {
                    for dep in deps {
                        if !result.contains(dep) {
                            stack.push(dep);
                        }
                    }
                }
            }
        }

        result
    }

    /// Get topological ordering of nodes using iterative DFS
    ///
    /// Returns nodes in an order such that all dependencies come before dependents.
    /// This is useful for determining rebuild order.
    ///
    /// # Performance
    ///
    /// Results are cached. Subsequent calls without graph modifications return
    /// the cached order in O(1) time.
    pub fn topological_sort(&self) -> DependencyGraphResult<Vec<NodeId>> {
        // Return cached result if available
        if let Some(cached) = self.cached_topo_order.borrow().as_ref() {
            return Ok(cached.clone());
        }

        let mut in_degree: HashMap<&NodeId, usize> = HashMap::new();

        // Calculate in-degrees
        for node in &self.nodes {
            let degree = self.dependencies.get(node).map_or(0, |edges| edges.len());
            in_degree.insert(node, degree);
        }

        // Start with nodes that have no dependencies (using Vec as stack)
        let mut stack: Vec<&NodeId> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut result = Vec::new();

        while let Some(node) = stack.pop() {
            result.push(node.clone());

            // Reduce in-degree of dependents
            if let Some(deps) = self.dependents.get(node) {
                for dep in deps {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 {
                            stack.push(dep);
                        }
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            return Err(DependencyGraphError::CircularDependency(vec![]));
        }

        // Cache the result
        *self.cached_topo_order.borrow_mut() = Some(result.clone());

        Ok(result)
    }

    /// Get root nodes (nodes with no dependencies)
    pub fn roots(&self) -> &HashSet<NodeId> {
        &self.roots
    }

    /// Get leaf nodes (nodes with no dependents)
    pub fn leaves(&self) -> &HashSet<NodeId> {
        &self.leaves
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Clear the graph
    pub fn clear(&mut self) {
        self.dependencies.clear();
        self.dependents.clear();
        self.nodes.clear();
        self.roots.clear();
        self.leaves.clear();
        *self.cached_topo_order.borrow_mut() = None;
    }

    /// Get all nodes in the graph
    pub fn nodes(&self) -> &HashSet<NodeId> {
        &self.nodes
    }

    /// Export graph to DOT format (for visualization)
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph DependencyGraph {\n");

        for (from, edges) in &self.dependencies {
            for edge in edges {
                dot.push_str(&format!("  \"{}\" -> \"{}\"", from, edge.to));
                if let Some(label) = &edge.label {
                    dot.push_str(&format!(" [label=\"{label}\"]"));
                }
                dot.push_str(";\n");
            }
        }

        dot.push('}');
        dot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let graph = DependencyGraph::new();
        assert!(graph.is_empty());
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = DependencyGraph::new();
        graph.add_node("node1");

        assert!(!graph.is_empty());
        assert_eq!(graph.node_count(), 1);
        assert!(graph.has_node("node1"));
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("B", "A").unwrap(); // B depends on A

        assert!(graph.has_edge("B", "A"));
        assert!(graph.has_node("A"));
        assert!(graph.has_node("B"));
    }

    #[test]
    fn test_get_dependencies() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("B", "A").unwrap();
        graph.add_dependency("B", "C").unwrap();

        let deps = graph.get_dependencies("B");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&&"A".to_string()));
        assert!(deps.contains(&&"C".to_string()));
    }

    #[test]
    fn test_get_dependents() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("B", "A").unwrap();
        graph.add_dependency("C", "A").unwrap();

        let dependents = graph.get_dependents("A");
        assert_eq!(dependents.len(), 2);
        assert!(dependents.contains(&&"B".to_string()));
        assert!(dependents.contains(&&"C".to_string()));
    }

    #[test]
    fn test_get_all_dependents() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("B", "A").unwrap();
        graph.add_dependency("C", "B").unwrap();

        let all_dependents = graph.get_all_dependents("A");
        assert!(all_dependents.contains("B"));
        assert!(all_dependents.contains("C"));
    }

    #[test]
    fn test_get_all_dependencies() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("C", "B").unwrap();
        graph.add_dependency("B", "A").unwrap();

        let all_deps = graph.get_all_dependencies("C");
        assert!(all_deps.contains("B"));
        assert!(all_deps.contains("A"));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("B", "A").unwrap();
        graph.add_dependency("C", "B").unwrap();

        // This would create a cycle: A -> C
        let result = graph.add_dependency("A", "C");
        assert!(matches!(
            result,
            Err(DependencyGraphError::CircularDependency(_))
        ));
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("B", "A").unwrap();
        graph.add_dependency("C", "B").unwrap();
        graph.add_dependency("D", "A").unwrap();

        let sorted = graph.topological_sort().unwrap();

        // A must come before B, C, and D
        let a_idx = sorted.iter().position(|x| x == "A").unwrap();
        let b_idx = sorted.iter().position(|x| x == "B").unwrap();
        let c_idx = sorted.iter().position(|x| x == "C").unwrap();
        let d_idx = sorted.iter().position(|x| x == "D").unwrap();

        assert!(a_idx < b_idx);
        assert!(b_idx < c_idx);
        assert!(a_idx < d_idx);
    }

    #[test]
    fn test_remove_node() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("B", "A").unwrap();
        graph.add_dependency("C", "B").unwrap();

        let removed = graph.remove_node("B");

        assert!(!graph.has_node("B"));
        assert!(!graph.has_edge("B", "A"));
        assert!(!graph.has_edge("C", "B"));
        assert_eq!(removed.len(), 2);
    }

    #[test]
    fn test_roots_and_leaves() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("B", "A").unwrap();
        graph.add_dependency("C", "B").unwrap();

        // A is a root (no dependencies)
        assert!(graph.roots().contains("A"));
        // C is a leaf (no dependents)
        assert!(graph.leaves().contains("C"));
    }

    #[test]
    fn test_to_dot() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("B", "A").unwrap();

        let dot = graph.to_dot();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("\"B\" -> \"A\""));
    }

    #[test]
    fn test_clear() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("B", "A").unwrap();

        graph.clear();

        assert!(graph.is_empty());
        assert_eq!(graph.node_count(), 0);
    }
}
