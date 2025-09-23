/// Rollback System - Phase 2.1 & 2.2 Implementation
/// 
/// This module provides comprehensive rollback capabilities for database migrations.
/// The rollback system follows a multi-phase implementation approach with enterprise-grade
/// safety validation, risk assessment, execution tracking, automated plan generation,
/// and comprehensive SQL generation with escaping and injection prevention.

pub mod types;
pub mod plan;
pub mod generator;
pub mod sql_generator;
pub mod safety;

// Re-export all public types for convenient access
pub use types::*;
pub use plan::*;
pub use generator::*;
pub use sql_generator::*;
pub use safety::*;