//! Context management module for CadAgent
//!
//! This module provides context storage and management capabilities based on
//! `tokitai-context`, enabling:
//!
//! - **Multi-turn dialog state tracking**: Git-style branch management for conversation history
//! - **Error case library**: Persistent storage of error patterns and solutions
//! - **Task planning**: DAG-based task dependency and execution tracking
//! - **Knowledge persistence**: Long-term memory for design patterns and user preferences
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    DialogStateManager                       │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │  DialogMemoryManager                                  │  │
//! │  │  ├─ Transient Layer  (temporary data)                 │  │
//! │  │  ├─ ShortTerm Layer  (recent N turns)                 │  │
//! │  │  └─ LongTerm Layer   (permanent knowledge)            │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! │                                                              │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │  BranchManager                                        │  │
//! │  │  ├─ Branch: main (default conversation)               │  │
//! │  │  ├─ Branch: design-option-a (exploration)             │  │
//! │  │  └─ Branch: design-option-b (exploration)             │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! │                                                              │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │  MergeHandler                                         │  │
//! │  │  ├─ FastForward                                       │  │
//! │  │  ├─ SelectiveMerge                                    │  │
//! │  │  ├─ AIAssisted (requires AI feature)                  │  │
//! │  │  └─ ThreeWayMerge                                     │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! │                                                              │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │  AIIntegration (requires AI feature)                  │  │
//! │  │  ├─ Branch Purpose Inference                          │  │
//! │  │  ├─ Merge Recommendations                             │  │
//! │  │  ├─ Branch Summarization                              │  │
//! │  │  └─ Conflict Resolution                               │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Structure
//!
//! ```text
//! context/
//! ├── mod.rs              # This module
//! ├── utils.rs            # Common utilities
//! ├── dialog_memory/      # Layered memory management
//! ├── branch/             # Branch operations
//! ├── merge/              # Merge operations
//! ├── ai/                 # AI integration
//! ├── error_library/      # Error case library (modular)
//! ├── task/               # Task planning (modular)
//! ├── dialog_state.rs     # High-level dialog state manager
//! ├── error_library.rs    # Error case library (backward compat)
//! └── task_planner.rs     # Task planning (backward compat)
//! ```
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use cadagent::context::{DialogStateManager, DialogStateConfig};
//!
//! // Create dialog state manager
//! let config = DialogStateConfig::default();
//! let mut manager = DialogStateManager::new("session-123", config).unwrap();
//!
//! // Add user message
//! manager.add_user_message("帮我分析这个 CAD 图纸").unwrap();
//!
//! // Add assistant response
//! manager.add_assistant_response("好的，我正在分析...", Some("tool_chain")).unwrap();
//!
//! // Search context semantically
//! let hits = manager.search_context("CAD 分析").unwrap();
//! for hit in hits {
//!     println!("Found: {} (score: {})", hit.hash, hit.score);
//! }
//! ```
//!
//! # Integration with Other Modules
//!
//! - **llm_reasoning**: Uses `DialogStateManager` for conversation history
//! - **cad_verifier**: Uses `ErrorCaseLibrary` for error pattern learning
//! - **analysis**: Uses `TaskPlanner` for multi-step task execution
//! - **feature**: Uses branch management for design exploration

// New modular structure
pub mod ai;
pub mod backend;
pub mod branch;
pub mod dialog_memory;
pub mod error_library;
pub mod merge;
pub mod mock_llm;
pub mod task;
pub mod utils;

// Legacy high-level managers (for backward compatibility)
pub mod dialog_state;
pub mod error_library_legacy;
pub mod task_planner;

// Re-export from new modules
pub use ai::AIIntegration;
pub use backend::{BackendStats, ContextBackend, MemoryBackend, SearchResult};
pub use branch::{BranchManager, BranchManagerConfig, BranchMetadata, CrossBranchSearchHit};
pub use dialog_memory::{DialogMemoryConfig, DialogMemoryManager, DialogMessage};
pub use merge::{DesignComparison, MergeHandler, MergeHandlerConfig, MergeResult};
pub use mock_llm::MockLLMClient;
pub use task::{
    PlanStatus, TaskNode, TaskPlan, TaskPlanStats, TaskPlanner, TaskPlannerConfig, TaskStatus,
};

// Re-export legacy high-level managers
pub use dialog_state::DialogStateConfig;
pub use dialog_state::DialogStateManager;
pub use error_library_legacy::*;

// Re-export key types from tokitai-context for advanced usage
pub use tokitai_context::facade::{
    Context, ContextConfig, ContextItem, ContextStats, Layer, RecoveryReport, SearchHit,
};

// AIContext is available when tokitai-context has the "ai" feature enabled
pub use tokitai_context::facade::AIContext;

pub use tokitai_context::parallel::ParallelContextManager;
pub use tokitai_context::parallel::ParallelContextManagerConfig;

// Re-export AI types
pub use ai::{BranchPurpose, BranchSummary, ConflictResolution, MergeRecommendation};
// Re-export task types
pub use task::{TaskExecutor, TaskExecutorConfig, TaskNode as TaskNodeType};
