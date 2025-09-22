//! Comprehensive migration validation modules
//! 
//! This module provides type-safe, compile-time migration validation with zero placeholders.
//! Each validator focuses on a specific aspect of migration safety and correctness.

pub mod schema_compatibility;
pub mod data_integrity;
pub mod performance_impact;
pub mod safety_analyzer;
pub mod breaking_change_detector;

pub use schema_compatibility::*;
pub use data_integrity::*;
pub use performance_impact::*;
pub use safety_analyzer::*;
pub use breaking_change_detector::*;

use super::SafetyWarning;

/// Comprehensive validation result with detailed analysis
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the migration is safe to execute
    pub is_safe: bool,
    
    /// All safety warnings discovered during validation
    pub safety_warnings: Vec<SafetyWarning>,
    
    /// Schema compatibility analysis results
    pub schema_compatibility: SchemaCompatibilityResult,
    
    /// Data integrity analysis results  
    pub data_integrity: DataIntegrityResult,
    
    /// Performance impact analysis results
    pub performance_impact: PerformanceImpactResult,
    
    /// Breaking change analysis results
    pub breaking_changes: BreakingChangeResult,
    
    /// Overall risk assessment
    pub risk_level: RiskLevel,
    
    /// Recommended actions before execution
    pub recommendations: Vec<ValidationRecommendation>,
}

impl ValidationResult {
    /// Check if validation passed with no critical issues
    pub fn has_critical_issues(&self) -> bool {
        !self.is_safe || 
        self.risk_level == RiskLevel::Critical ||
        self.breaking_changes.has_blocking_changes ||
        self.data_integrity.has_data_loss_risk()
    }
    
    /// Get all warnings grouped by severity
    pub fn warnings_by_severity(&self) -> ValidationWarnings {
        ValidationWarnings::from_results(self)
    }
}

/// Risk assessment levels for migrations
#[derive(Debug, Clone)]
pub enum RiskLevel {
    /// No risks detected - safe to execute
    Low,
    /// Minor risks that should be reviewed
    Medium,
    /// Significant risks requiring careful consideration
    High,
    /// Critical risks that may prevent execution
    Critical,
}

/// Recommended actions based on validation results
#[derive(Debug, Clone)]
pub enum ValidationRecommendation {
    /// Create backup before proceeding
    CreateBackup,
    /// Test on staging environment first
    TestOnStaging,
    /// Review breaking changes with team
    ReviewBreakingChanges,
    /// Consider zero-downtime migration approach
    UseZeroDowntime,
    /// Split migration into smaller steps
    SplitMigration,
    /// Optimize queries before migration
    OptimizeQueries,
    /// Schedule during low-traffic period
    ScheduleOffPeak,
    /// Custom recommendation with specific message
    Custom(String),
}

/// Validation warnings grouped by severity
#[derive(Debug, Clone)]
pub struct ValidationWarnings {
    pub critical: Vec<SafetyWarning>,
    pub high: Vec<SafetyWarning>,
    pub medium: Vec<SafetyWarning>,
    pub low: Vec<SafetyWarning>,
}

impl ValidationWarnings {
    fn from_results(result: &ValidationResult) -> Self {
        let mut warnings = Self {
            critical: Vec::new(),
            high: Vec::new(),
            medium: Vec::new(),
            low: Vec::new(),
        };
        
        for warning in &result.safety_warnings {
            match warning.warning_type {
                super::SafetyWarningType::DataLoss => warnings.critical.push(warning.clone()),
                super::SafetyWarningType::BreakingChange => warnings.high.push(warning.clone()),
                super::SafetyWarningType::PerformanceImpact => warnings.medium.push(warning.clone()),
                super::SafetyWarningType::ComplexOperation => warnings.low.push(warning.clone()),
            }
        }
        
        warnings
    }
}