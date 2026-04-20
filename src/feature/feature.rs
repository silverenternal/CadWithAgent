//! Feature Tree Core Data Structures
//!
//! This module provides the foundation for parametric modeling in `CadAgent`.
//! The feature tree tracks modeling history, enables undo/redo, and supports
//! parametric modifications.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Feature Tree Architecture                │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
//! │  │   Sketch    │→ │   Extrude   │→ │   Fillet    │         │
//! │  │  (Base)     │  │  (Feature)  │  │  (Feature)  │         │
//! │  └─────────────┘  └─────────────┘  └─────────────┘         │
//! │         │                │                │                 │
//! │         ▼                ▼                ▼                 │
//! │  ┌─────────────────────────────────────────────────┐       │
//! │  │              Dependency Graph                    │       │
//! │  │  Tracks parent-child relationships between       │       │
//! │  │  features for efficient rebuild propagation      │       │
//! │  └─────────────────────────────────────────────────┘       │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Feature Types
//!
//! - **Sketch**: 2D profile on a plane
//! - **Extrude**: Linear extension of a sketch
//! - **Revolve**: Rotational sweep of a sketch
//! - **Fillet**: Edge rounding
//! - **Chamfer**: Edge beveling
//! - **Pattern**: Linear or circular pattern
//! - **Boolean**: Union, intersection, difference operations
//!
//! # Examples
//!
//! ```
//! use cadagent::feature::{Feature, FeatureTree, Sketch, ExtrudeDirection};
//! use cadagent::geometry::{Point, Line};
//!
//! // Create a simple sketch
//! let mut sketch = Sketch::new();
//! let pt1 = sketch.add_point(Point::new(0.0, 0.0));
//! let pt2 = sketch.add_point(Point::new(1.0, 0.0));
//! sketch.add_line(pt1, pt2);
//!
//! // Create feature tree and add features
//! let mut tree = FeatureTree::new();
//! let sketch_id = tree.add_feature(Feature::Sketch(sketch)).unwrap();
//! let extrude_id = tree.add_feature(Feature::Extrude {
//!     sketch_id,
//!     depth: 10.0,
//!     direction: ExtrudeDirection::Positive,
//!     taper_angle: None,
//! }).unwrap();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::geometry::{Point, Primitive};

/// Unique identifier for features
pub type FeatureId = Uuid;

/// Unique identifier for sketch points/curves
pub type SketchEntityId = Uuid;

/// Generate a new unique ID (public for `feature_tree` module)
pub fn generate_id() -> FeatureId {
    Uuid::new_v4()
}

/// Sketch entity (point, line, arc, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SketchEntity {
    /// Point in 2D space
    Point {
        id: SketchEntityId,
        point: Point,
        construction: bool,
    },
    /// Line segment between two points
    Line {
        id: SketchEntityId,
        start_id: SketchEntityId,
        end_id: SketchEntityId,
        construction: bool,
    },
    /// Arc defined by center, start, end points and direction
    Arc {
        id: SketchEntityId,
        center_id: SketchEntityId,
        start_id: SketchEntityId,
        end_id: SketchEntityId,
        clockwise: bool,
        construction: bool,
    },
    /// Circle defined by center and radius
    Circle {
        id: SketchEntityId,
        center_id: SketchEntityId,
        radius: f64,
        construction: bool,
    },
    /// Spline curve through control points
    Spline {
        id: SketchEntityId,
        control_point_ids: Vec<SketchEntityId>,
        degree: usize,
        construction: bool,
    },
}

impl SketchEntity {
    /// Get the entity ID
    #[allow(clippy::match_same_arms)]
    pub fn id(&self) -> SketchEntityId {
        match self {
            Self::Point { id, .. } => *id,
            Self::Line { id, .. } => *id,
            Self::Arc { id, .. } => *id,
            Self::Circle { id, .. } => *id,
            Self::Spline { id, .. } => *id,
        }
    }

    /// Check if this is a construction entity
    #[allow(clippy::match_same_arms)]
    pub fn is_construction(&self) -> bool {
        match self {
            Self::Point { construction, .. } => *construction,
            Self::Line { construction, .. } => *construction,
            Self::Arc { construction, .. } => *construction,
            Self::Circle { construction, .. } => *construction,
            Self::Spline { construction, .. } => *construction,
        }
    }

    /// Get referenced point IDs
    pub fn referenced_points(&self) -> Vec<SketchEntityId> {
        match self {
            Self::Point { .. } => vec![],
            Self::Line {
                start_id, end_id, ..
            } => vec![*start_id, *end_id],
            Self::Arc {
                center_id,
                start_id,
                end_id,
                ..
            } => {
                vec![*center_id, *start_id, *end_id]
            }
            Self::Circle { center_id, .. } => vec![*center_id],
            Self::Spline {
                control_point_ids, ..
            } => control_point_ids.clone(),
        }
    }
}

/// 2D Sketch for feature creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sketch {
    /// Unique identifier
    pub id: SketchEntityId,
    /// Sketch entities (points, lines, arcs, etc.)
    pub entities: HashMap<SketchEntityId, SketchEntity>,
    /// Sketch plane (default is XY plane)
    pub plane: SketchPlane,
    /// Name for UI display
    pub name: String,
}

impl Default for Sketch {
    fn default() -> Self {
        Self::new()
    }
}

impl Sketch {
    /// Create a new empty sketch
    pub fn new() -> Self {
        Self {
            id: generate_id(),
            entities: HashMap::new(),
            plane: SketchPlane::default(),
            name: String::new(),
        }
    }

    /// Create a named sketch
    pub fn with_name(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Self::new()
        }
    }

    /// Add a point to the sketch
    pub fn add_point(&mut self, point: Point) -> SketchEntityId {
        let id = generate_id();
        self.entities.insert(
            id,
            SketchEntity::Point {
                id,
                point,
                construction: false,
            },
        );
        id
    }

    /// Add a construction point
    pub fn add_construction_point(&mut self, point: Point) -> SketchEntityId {
        let id = generate_id();
        self.entities.insert(
            id,
            SketchEntity::Point {
                id,
                point,
                construction: true,
            },
        );
        id
    }

    /// Add a line between two existing points
    pub fn add_line(
        &mut self,
        start_id: SketchEntityId,
        end_id: SketchEntityId,
    ) -> Option<SketchEntityId> {
        // Verify points exist
        if !self.entities.contains_key(&start_id) || !self.entities.contains_key(&end_id) {
            return None;
        }

        let id = generate_id();
        self.entities.insert(
            id,
            SketchEntity::Line {
                id,
                start_id,
                end_id,
                construction: false,
            },
        );
        Some(id)
    }

    /// Add an arc
    pub fn add_arc(
        &mut self,
        center_id: SketchEntityId,
        start_id: SketchEntityId,
        end_id: SketchEntityId,
        clockwise: bool,
    ) -> Option<SketchEntityId> {
        // Verify points exist
        if !self.entities.contains_key(&center_id)
            || !self.entities.contains_key(&start_id)
            || !self.entities.contains_key(&end_id)
        {
            return None;
        }

        let id = generate_id();
        self.entities.insert(
            id,
            SketchEntity::Arc {
                id,
                center_id,
                start_id,
                end_id,
                clockwise,
                construction: false,
            },
        );
        Some(id)
    }

    /// Add a circle
    pub fn add_circle(&mut self, center_id: SketchEntityId, radius: f64) -> Option<SketchEntityId> {
        if !self.entities.contains_key(&center_id) {
            return None;
        }

        if radius <= 0.0 {
            return None;
        }

        let id = generate_id();
        self.entities.insert(
            id,
            SketchEntity::Circle {
                id,
                center_id,
                radius,
                construction: false,
            },
        );
        Some(id)
    }

    /// Get a sketch entity by ID
    pub fn get_entity(&self, id: SketchEntityId) -> Option<&SketchEntity> {
        self.entities.get(&id)
    }

    /// Get a point's coordinates
    pub fn get_point(&self, id: SketchEntityId) -> Option<Point> {
        match self.entities.get(&id) {
            Some(SketchEntity::Point { point, .. }) => Some(*point),
            _ => None,
        }
    }

    /// Get all profile entities (non-construction)
    pub fn get_profile_entities(&self) -> Vec<&SketchEntity> {
        self.entities
            .values()
            .filter(|e| !e.is_construction())
            .collect()
    }

    /// Get all construction entities
    pub fn get_construction_entities(&self) -> Vec<&SketchEntity> {
        self.entities
            .values()
            .filter(|e| e.is_construction())
            .collect()
    }

    /// Validate sketch integrity
    pub fn validate(&self) -> Result<(), SketchError> {
        for (id, entity) in &self.entities {
            if entity.id() != *id {
                return Err(SketchError::EntityIdMismatch(*id));
            }

            // Check referenced points exist
            for ref_id in entity.referenced_points() {
                if !self.entities.contains_key(&ref_id) {
                    return Err(SketchError::MissingReference(ref_id));
                }
            }
        }

        Ok(())
    }
}

/// Sketch plane definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SketchPlane {
    /// Origin point of the plane
    pub origin: Point,
    /// X axis direction (normalized)
    pub x_axis: Point,
    /// Y axis direction (normalized, perpendicular to `x_axis`)
    pub y_axis: Point,
}

impl Default for SketchPlane {
    fn default() -> Self {
        Self {
            origin: Point::origin(),
            x_axis: Point::new(1.0, 0.0),
            y_axis: Point::new(0.0, 1.0),
        }
    }
}

/// Sketch errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum SketchError {
    #[error("Entity ID mismatch for entity {0}")]
    EntityIdMismatch(SketchEntityId),

    #[error("Missing referenced entity {0}")]
    MissingReference(SketchEntityId),

    #[error("Invalid geometry: {0}")]
    InvalidGeometry(String),

    #[error("Duplicate entity {0}")]
    DuplicateEntity(SketchEntityId),
}

/// Extrude direction
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExtrudeDirection {
    /// Positive direction along the normal
    Positive,
    /// Negative direction along the normal
    Negative,
    /// Both directions (symmetric)
    Both,
    /// Custom direction vector
    Custom(f64, f64, f64),
}

/// Revolve axis
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RevolveAxis {
    /// Around X axis
    X,
    /// Around Y axis
    Y,
    /// Around Z axis
    Z,
    /// Custom axis defined by two points
    Custom {
        origin: [f64; 3],
        direction: [f64; 3],
    },
}

/// Pattern type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    /// Linear pattern along a direction
    Linear {
        direction: [f64; 3],
        spacing: f64,
        count: usize,
    },
    /// Circular pattern around an axis
    Circular {
        axis: RevolveAxis,
        angle: f64,
        count: usize,
    },
    /// Fill pattern within a boundary
    Fill {
        boundary_id: FeatureId,
        spacing: f64,
    },
}

/// Boolean operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BooleanOp {
    /// Union of two solids
    Union,
    /// Intersection of two solids
    Intersection,
    /// Difference (subtract tool from target)
    Difference,
}

/// Feature types supported in the feature tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Feature {
    /// Sketch feature - 2D profile on a plane
    Sketch(Sketch),

    /// Extrude feature - linear extension of a sketch
    Extrude {
        /// Parent sketch feature ID
        sketch_id: FeatureId,
        /// Extrusion depth
        depth: f64,
        /// Extrusion direction
        direction: ExtrudeDirection,
        /// Taper angle in radians (optional)
        taper_angle: Option<f64>,
    },

    /// Revolve feature - rotational sweep of a sketch
    Revolve {
        /// Parent sketch feature ID
        sketch_id: FeatureId,
        /// Revolution axis
        axis: RevolveAxis,
        /// Revolution angle in radians
        angle: f64,
        /// Direction of revolution
        clockwise: bool,
    },

    /// Fillet feature - edge rounding
    Fillet {
        /// Parent feature ID
        parent_id: FeatureId,
        /// Edge IDs to fillet
        edge_ids: Vec<usize>,
        /// Fillet radius
        radius: f64,
    },

    /// Chamfer feature - edge beveling
    Chamfer {
        /// Parent feature ID
        parent_id: FeatureId,
        /// Edge IDs to chamfer
        edge_ids: Vec<usize>,
        /// Chamfer distance
        distance: f64,
        /// Chamfer angle in radians
        angle: f64,
    },

    /// Pattern feature - linear or circular pattern
    Pattern {
        /// Parent feature ID to pattern
        parent_id: FeatureId,
        /// Pattern type
        pattern_type: PatternType,
    },

    /// Boolean feature - union, intersection, or difference
    Boolean {
        /// Target feature ID
        target_id: FeatureId,
        /// Tool feature ID
        tool_id: FeatureId,
        /// Boolean operation
        operation: BooleanOp,
    },

    /// Work plane - reference plane for sketches
    WorkPlane {
        /// Optional parent feature ID
        parent_id: Option<FeatureId>,
        /// Plane definition
        plane: SketchPlane,
    },

    /// Import feature - external geometry import
    Import {
        /// File path
        path: String,
        /// Imported geometry
        primitives: Vec<Primitive>,
    },
}

impl Feature {
    /// Get feature ID
    pub fn id(&self) -> FeatureId {
        match self {
            Self::Sketch(s) => s.id,
            Self::Extrude { .. }
            | Self::Revolve { .. }
            | Self::Fillet { .. }
            | Self::Chamfer { .. }
            | Self::Pattern { .. }
            | Self::Boolean { .. }
            | Self::WorkPlane { .. }
            | Self::Import { .. } => generate_id(),
        }
    }

    /// Get parent feature IDs (dependencies)
    #[allow(clippy::match_same_arms)]
    pub fn dependencies(&self) -> Vec<FeatureId> {
        match self {
            Self::Sketch(_) => vec![],
            Self::Extrude { sketch_id, .. } => vec![*sketch_id],
            Self::Revolve { sketch_id, .. } => vec![*sketch_id],
            Self::Fillet { parent_id, .. } => vec![*parent_id],
            Self::Chamfer { parent_id, .. } => vec![*parent_id],
            Self::Pattern { parent_id, .. } => vec![*parent_id],
            Self::Boolean {
                target_id, tool_id, ..
            } => vec![*target_id, *tool_id],
            Self::WorkPlane { parent_id, .. } => parent_id.iter().copied().collect(),
            Self::Import { .. } => vec![],
        }
    }

    /// Get the feature display name
    pub fn name(&self) -> String {
        match self {
            Self::Sketch(s) => s.name.clone(),
            Self::Extrude { .. } => "Extrude".to_string(),
            Self::Revolve { .. } => "Revolve".to_string(),
            Self::Fillet { .. } => "Fillet".to_string(),
            Self::Chamfer { .. } => "Chamfer".to_string(),
            Self::Pattern { .. } => "Pattern".to_string(),
            Self::Boolean { operation, .. } => format!("{operation:?}"),
            Self::WorkPlane { .. } => "WorkPlane".to_string(),
            Self::Import { path, .. } => format!("Import: {path}"),
        }
    }

    /// Check if this feature is a solid feature (creates 3D geometry)
    pub fn is_solid(&self) -> bool {
        matches!(
            self,
            Self::Extrude { .. }
                | Self::Revolve { .. }
                | Self::Fillet { .. }
                | Self::Chamfer { .. }
                | Self::Pattern { .. }
                | Self::Boolean { .. }
        )
    }

    /// Check if this feature is a sketch feature
    pub fn is_sketch(&self) -> bool {
        matches!(self, Self::Sketch(_))
    }

    /// Check if this feature is a reference feature
    pub fn is_reference(&self) -> bool {
        matches!(self, Self::WorkPlane { .. })
    }
}

/// Feature state for undo/redo and suppression
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureState {
    /// Feature is active and will be rebuilt
    Active,
    /// Feature is suppressed and will be skipped during rebuild
    Suppressed,
    /// Feature is deleted (kept in history for undo)
    Deleted,
}

/// Feature node in the tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureNode {
    /// Unique feature ID
    pub id: FeatureId,
    /// The feature data
    pub feature: Feature,
    /// Feature state
    pub state: FeatureState,
    /// Child feature IDs (features that depend on this one)
    pub children: Vec<FeatureId>,
    /// Cached result after evaluation (optional) - not serialized
    #[serde(skip)]
    pub cached_result: Option<Arc<Vec<Primitive>>>,
    /// User-defined parameters
    pub parameters: HashMap<String, f64>,
}

impl FeatureNode {
    /// Create a new feature node
    pub fn new(feature: Feature) -> Self {
        let id = feature.id();
        Self {
            id,
            feature,
            state: FeatureState::Active,
            children: Vec::new(),
            cached_result: None,
            parameters: HashMap::new(),
        }
    }

    /// Check if this node should be evaluated during rebuild
    pub fn should_evaluate(&self) -> bool {
        self.state == FeatureState::Active
    }

    /// Set a parameter value
    pub fn set_parameter(&mut self, name: &str, value: f64) {
        self.parameters.insert(name.to_string(), value);
    }

    /// Get a parameter value
    pub fn get_parameter(&self, name: &str) -> Option<f64> {
        self.parameters.get(name).copied()
    }
}

/// Feature tree errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum FeatureTreeError {
    #[error("Feature not found: {0}")]
    FeatureNotFound(FeatureId),

    #[error("Circular dependency detected: {0:?}")]
    CircularDependency(Vec<FeatureId>),

    #[error("Invalid dependency: {0} depends on non-existent {1}")]
    InvalidDependency(FeatureId, FeatureId),

    #[error("Rebuild failed: {0}")]
    RebuildFailed(String),

    #[error("Cannot rollback: {0}")]
    RollbackFailed(String),

    #[error("Parameter error: {0}")]
    ParameterError(String),

    #[error("Sketch error: {0}")]
    SketchError(#[from] SketchError),
}

/// Result type for feature tree operations
pub type FeatureTreeResult<T> = Result<T, FeatureTreeError>;
