/// Rollback System - Phase 2.1 Implementation
/// 
/// This module provides comprehensive rollback capabilities for database migrations.
/// The rollback system follows a multi-phase implementation approach with enterprise-grade
/// safety validation, risk assessment, execution tracking, and automated plan generation.

pub mod types;
pub mod plan;
pub mod generator;

// Re-export all public types for convenient access
pub use types::*;
pub use plan::*;
pub use generator::*;