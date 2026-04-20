//! Common parser infrastructure for STEP, IGES, and other CAD formats
//!
//! This module provides shared configuration and utilities for CAD file parsers,
//! ensuring consistent behavior across different file format implementations.
//!
//! # Overview
//!
//! The parser common module defines:
//!
//! - [`ParserConfig`]: Configuration for parsing tolerance and debug options
//! - [`CadMetadata`]: Metadata extracted from CAD files (author, units, etc.)
//! - [`AssemblyStructure`]: Hierarchical representation of CAD assemblies
//! - [`AssemblyComponent`]: Individual components within an assembly
//!
//! # Example
//!
//! ```rust
//! use cadagent::parser::parser_common::{ParserConfig, CadMetadata, AssemblyStructure, AssemblyComponent};
//!
//! // Configure parser with custom tolerance
//! let config = ParserConfig::new()
//!     .with_tolerance(1e-6)
//!     .with_debug(true);
//!
//! // Create metadata
//! let metadata = CadMetadata {
//!     name: Some("My Assembly".to_string()),
//!     units: Some("mm".to_string()),
//!     ..Default::default()
//! };
//!
//! // Build assembly structure
//! let assembly = AssemblyStructure::new("MainAssembly")
//!     .add_child(AssemblyComponent::new("Part1")
//!         .with_entity_refs(vec![1, 2, 3]));
//! ```

use serde::{Deserialize, Serialize};

/// Common parser configuration
///
/// Used by STEP, IGES, and other CAD format parsers to control parsing behavior.
///
/// # Fields
///
/// * `tolerance` - Geometric tolerance for parsing operations (default: 1e-6)
/// * `debug` - Enable debug logging for troubleshooting
///
/// # Example
///
/// ```rust
/// use cadagent::parser::parser_common::ParserConfig;
///
/// let config = ParserConfig::default()
///     .with_tolerance(1e-8)
///     .with_debug(true);
///
/// assert_eq!(config.tolerance, 1e-8);
/// assert!(config.debug);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    /// Parsing tolerance for geometric operations
    pub tolerance: f64,
    /// Enable debug logging
    pub debug: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            tolerance: 1e-6,
            debug: false,
        }
    }
}

impl ParserConfig {
    /// Create a new parser config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the parsing tolerance
    ///
    /// # Arguments
    /// * `tolerance` - The tolerance for geometric operations
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Enable or disable debug mode
    ///
    /// # Arguments
    /// * `debug` - Whether to enable debug logging
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
}

/// Common metadata for CAD models
///
/// Extracted from CAD file headers and properties. Provides information about
/// the model's origin, creation, and target environment.
///
/// # Fields
///
/// * `name` - Model name or title
/// * `author` - Author or creator name
/// * `organization` - Organization that created the model
/// * `creation_date` - Creation timestamp
/// * `modification_date` - Last modification timestamp
/// * `source_software` - CAD software that created the file
/// * `target_software` - Target CAD software or version for export
/// * `units` - Units of measurement (e.g., "mm", "inch", "m")
///
/// # Example
///
/// ```rust
/// use cadagent::parser::parser_common::CadMetadata;
///
/// let metadata = CadMetadata {
///     name: Some("Engine Block".to_string()),
///     author: Some("Design Team".to_string()),
///     units: Some("mm".to_string()),
///     ..Default::default()
/// };
///
/// assert_eq!(metadata.name, Some("Engine Block".to_string()));
/// assert_eq!(metadata.units, Some("mm".to_string()));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CadMetadata {
    /// Model name or title
    pub name: Option<String>,
    /// Author or creator
    pub author: Option<String>,
    /// Organization
    pub organization: Option<String>,
    /// Creation timestamp
    pub creation_date: Option<String>,
    /// Last modification timestamp
    pub modification_date: Option<String>,
    /// CAD software that created the file
    pub source_software: Option<String>,
    /// Target CAD software or version
    pub target_software: Option<String>,
    /// Units (e.g., "mm", "inch", "m")
    pub units: Option<String>,
}

/// Assembly structure for CAD models
///
/// Represents a hierarchical assembly of components, where each component
/// can reference geometry entities and have transformation matrices applied.
///
/// # Example
///
/// ```rust
/// use cadagent::parser::parser_common::{AssemblyStructure, AssemblyComponent};
///
/// let assembly = AssemblyStructure::new("Engine")
///     .add_child(AssemblyComponent::new("Cylinder")
///         .with_entity_refs(vec![1, 2, 3]))
///     .add_child(AssemblyComponent::new("Piston")
///         .with_entity_refs(vec![4, 5, 6]));
///
/// assert_eq!(assembly.name, "Engine");
/// assert_eq!(assembly.children.len(), 2);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyStructure {
    /// Assembly name
    pub name: String,
    /// Child components
    pub children: Vec<AssemblyComponent>,
}

/// Component in an assembly
///
/// Represents an individual part or sub-assembly within a larger assembly.
/// Components can be nested to create complex hierarchical structures.
///
/// # Fields
///
/// * `name` - Component name
/// * `entity_refs` - References to geometry entities (indices into geometry list)
/// * `transformation` - Optional 4x4 transformation matrix (row-major order)
/// * `children` - Nested child components for sub-assemblies
///
/// # Transformation Matrix Format
///
/// The transformation matrix is stored in row-major order as `[f64; 16]`:
/// ```text
/// [ m00, m01, m02, m03,
///   m10, m11, m12, m13,
///   m20, m21, m22, m23,
///   m30, m31, m32, m33 ]
/// ```
///
/// For a translation of (10, 20, 30), the matrix would be:
/// ```text
/// [ 1, 0, 0, 10,
///   0, 1, 0, 20,
///   0, 0, 1, 30,
///   0, 0, 0, 1 ]
/// ```
///
/// # Example
///
/// ```rust
/// use cadagent::parser::parser_common::AssemblyComponent;
///
/// let component = AssemblyComponent::new("TranslatedPart")
///     .with_entity_refs(vec![1, 2, 3])
///     .with_transformation([
///         1.0, 0.0, 0.0, 10.0,
///         0.0, 1.0, 0.0, 20.0,
///         0.0, 0.0, 1.0, 30.0,
///         0.0, 0.0, 0.0, 1.0,
///     ]);
///
/// assert!(component.transformation.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyComponent {
    /// Component name
    pub name: String,
    /// Reference to geometry entities
    pub entity_refs: Vec<usize>,
    /// Transformation matrix (4x4, row-major)
    pub transformation: Option<[f64; 16]>,
    /// Child components (nested assemblies)
    pub children: Vec<AssemblyComponent>,
}

impl AssemblyStructure {
    /// Create a new assembly structure
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            children: Vec::new(),
        }
    }

    /// Add a child component
    pub fn add_child(mut self, child: AssemblyComponent) -> Self {
        self.children.push(child);
        self
    }
}

impl AssemblyComponent {
    /// Create a new assembly component
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entity_refs: Vec::new(),
            transformation: None,
            children: Vec::new(),
        }
    }

    /// Set entity references
    pub fn with_entity_refs(mut self, refs: Vec<usize>) -> Self {
        self.entity_refs = refs;
        self
    }

    /// Set transformation matrix
    pub fn with_transformation(mut self, transform: [f64; 16]) -> Self {
        self.transformation = Some(transform);
        self
    }

    /// Add a child component
    pub fn add_child(mut self, child: AssemblyComponent) -> Self {
        self.children.push(child);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_config_default() {
        let config = ParserConfig::default();
        assert_eq!(config.tolerance, 1e-6);
        assert!(!config.debug);
    }

    #[test]
    fn test_parser_config_builder() {
        let config = ParserConfig::new().with_tolerance(1e-8).with_debug(true);

        assert_eq!(config.tolerance, 1e-8);
        assert!(config.debug);
    }

    #[test]
    fn test_parser_config_new() {
        let config = ParserConfig::new();
        assert_eq!(config.tolerance, 1e-6);
        assert!(!config.debug);
    }

    #[test]
    fn test_parser_config_with_tolerance_only() {
        let config = ParserConfig::default().with_tolerance(1e-3);
        assert_eq!(config.tolerance, 1e-3);
        assert!(!config.debug);
    }

    #[test]
    fn test_parser_config_with_debug_only() {
        let config = ParserConfig::default().with_debug(true);
        assert_eq!(config.tolerance, 1e-6);
        assert!(config.debug);
    }

    #[test]
    fn test_parser_config_clone() {
        let config1 = ParserConfig::new().with_tolerance(1e-5).with_debug(true);
        let config2 = config1.clone();

        assert_eq!(config1.tolerance, config2.tolerance);
        assert_eq!(config1.debug, config2.debug);
    }

    #[test]
    fn test_parser_config_debug_display() {
        let config = ParserConfig::default().with_debug(true);
        assert!(format!("{:?}", config).contains("debug"));
    }

    #[test]
    fn test_metadata_default() {
        let metadata = CadMetadata::default();
        assert!(metadata.name.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.organization.is_none());
        assert!(metadata.creation_date.is_none());
        assert!(metadata.modification_date.is_none());
        assert!(metadata.source_software.is_none());
        assert!(metadata.target_software.is_none());
        assert!(metadata.units.is_none());
    }

    #[test]
    fn test_metadata_clone() {
        let metadata1 = CadMetadata {
            name: Some("Test Model".to_string()),
            author: Some("John Doe".to_string()),
            units: Some("mm".to_string()),
            ..Default::default()
        };
        let metadata2 = metadata1.clone();

        assert_eq!(metadata1.name, metadata2.name);
        assert_eq!(metadata1.author, metadata2.author);
        assert_eq!(metadata1.units, metadata2.units);
    }

    #[test]
    fn test_metadata_debug_display() {
        let metadata = CadMetadata {
            name: Some("Test".to_string()),
            ..Default::default()
        };
        assert!(format!("{:?}", metadata).contains("Test"));
    }

    #[test]
    fn test_assembly_structure() {
        let assembly =
            AssemblyStructure::new("MainAssembly").add_child(AssemblyComponent::new("Part1"));

        assert_eq!(assembly.name, "MainAssembly");
        assert_eq!(assembly.children.len(), 1);
        assert_eq!(assembly.children[0].name, "Part1");
    }

    #[test]
    fn test_assembly_structure_multiple_children() {
        let assembly = AssemblyStructure::new("MainAssembly")
            .add_child(AssemblyComponent::new("Part1"))
            .add_child(AssemblyComponent::new("Part2"))
            .add_child(AssemblyComponent::new("Part3"));

        assert_eq!(assembly.name, "MainAssembly");
        assert_eq!(assembly.children.len(), 3);
        assert_eq!(assembly.children[0].name, "Part1");
        assert_eq!(assembly.children[1].name, "Part2");
        assert_eq!(assembly.children[2].name, "Part3");
    }

    #[test]
    fn test_assembly_component_builder() {
        let component = AssemblyComponent::new("Component")
            .with_entity_refs(vec![1, 2, 3])
            .with_transformation([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ])
            .add_child(AssemblyComponent::new("SubComponent"));

        assert_eq!(component.name, "Component");
        assert_eq!(component.entity_refs, vec![1, 2, 3]);
        assert!(component.transformation.is_some());
        assert_eq!(component.children.len(), 1);
        assert_eq!(component.children[0].name, "SubComponent");
    }

    #[test]
    fn test_assembly_component_clone() {
        let component1 = AssemblyComponent::new("Component").with_entity_refs(vec![1, 2, 3]);
        let component2 = component1.clone();

        assert_eq!(component1.name, component2.name);
        assert_eq!(component1.entity_refs, component2.entity_refs);
        assert_eq!(component1.transformation, component2.transformation);
    }

    #[test]
    fn test_assembly_component_debug_display() {
        let component = AssemblyComponent::new("TestComponent");
        assert!(format!("{:?}", component).contains("TestComponent"));
    }

    #[test]
    fn test_nested_assembly_structure() {
        let _sub_assembly =
            AssemblyStructure::new("SubAssembly").add_child(AssemblyComponent::new("SubPart"));

        let main_assembly = AssemblyStructure::new("MainAssembly")
            .add_child(AssemblyComponent::new("Part1"))
            .add_child(
                AssemblyComponent::new("Part2").add_child(AssemblyComponent::new("NestedPart")),
            );

        assert_eq!(main_assembly.name, "MainAssembly");
        assert_eq!(main_assembly.children.len(), 2);
        assert_eq!(main_assembly.children[1].children.len(), 1);
        assert_eq!(main_assembly.children[1].children[0].name, "NestedPart");
    }

    #[test]
    fn test_assembly_component_with_transformation() {
        let transform = [
            1.0, 0.0, 0.0, 10.0, 0.0, 1.0, 0.0, 20.0, 0.0, 0.0, 1.0, 30.0, 0.0, 0.0, 0.0, 1.0,
        ];

        let component = AssemblyComponent::new("TransformedPart").with_transformation(transform);

        assert_eq!(component.name, "TransformedPart");
        assert_eq!(component.transformation, Some(transform));
    }

    #[test]
    fn test_metadata_with_all_fields() {
        let metadata = CadMetadata {
            name: Some("Test Model".to_string()),
            author: Some("John Doe".to_string()),
            organization: Some("Test Org".to_string()),
            creation_date: Some("2024-01-01".to_string()),
            modification_date: Some("2024-01-02".to_string()),
            source_software: Some("CAD Software v1.0".to_string()),
            target_software: Some("CAD Software v2.0".to_string()),
            units: Some("mm".to_string()),
        };

        assert_eq!(metadata.name, Some("Test Model".to_string()));
        assert_eq!(metadata.author, Some("John Doe".to_string()));
        assert_eq!(metadata.organization, Some("Test Org".to_string()));
        assert_eq!(metadata.creation_date, Some("2024-01-01".to_string()));
        assert_eq!(metadata.modification_date, Some("2024-01-02".to_string()));
        assert_eq!(
            metadata.source_software,
            Some("CAD Software v1.0".to_string())
        );
        assert_eq!(
            metadata.target_software,
            Some("CAD Software v2.0".to_string())
        );
        assert_eq!(metadata.units, Some("mm".to_string()));
    }
}
