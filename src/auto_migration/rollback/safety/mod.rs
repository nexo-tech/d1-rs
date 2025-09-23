/// Safety and Risk Assessment Module for Rollback Operations
/// 
/// This module provides comprehensive safety validation, risk assessment, and issue detection
/// capabilities for rollback operations. It includes multi-dimensional risk analysis with
/// data loss assessment, dependency validation, schema compatibility checks, and performance
/// impact estimation.

pub mod risk_assessor;
pub mod issue_detector;

// Re-export public types from safety modules
pub use risk_assessor::*;
pub use issue_detector::*;