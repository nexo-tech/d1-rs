/// Rollback System - Phase 2.1, 2.2, 4.1, 4.2, 5.1 & 5.2 Implementation
/// 
/// This module provides comprehensive rollback capabilities for database migrations.
/// The rollback system follows a multi-phase implementation approach with enterprise-grade
/// safety validation, risk assessment, execution tracking, automated plan generation,
/// comprehensive SQL generation with escaping and injection prevention, and complete
/// orchestration management.

pub mod types;
pub mod plan;
pub mod generator;
pub mod sql_generator;
pub mod safety;
pub mod executor;
pub mod progress;
pub mod manager;
pub mod results;

// Re-export all public types for convenient access
pub use types::*;
pub use plan::*;
pub use generator::*;
pub use sql_generator::*;
pub use safety::{RollbackRiskAssessor, SafetyIssueDetector};
pub use executor::{RollbackExecutionEngine, CheckResult, ValidationResult, ExecutionState, 
                   SingleOperationResult, ExecutionMetrics};
pub use progress::{RollbackProgressTracker, RollbackProgress, CurrentRollbackOperation, OperationPhase};
pub use manager::{RollbackManager, RollbackFeasibility, RollbackScript, ScriptMetadata, ManualStep};
// Re-export all result types from the new comprehensive results module
pub use results::*;