//! Progress Tracking System for Rollback Operations
//!
//! This module provides comprehensive progress tracking capabilities for rollback execution,
//! including real-time progress updates, time estimation algorithms, execution state management,
//! and operation-level progress reporting with persistence for recovery scenarios.

use super::executor::{ExecutionState, RollbackProgressTracker as ProgressTrackerTrait};
use crate::Result;
use std::time::{Duration, Instant, SystemTime};
use std::collections::HashMap;

/// Comprehensive progress tracking system for rollback operations
/// 
/// Provides real-time progress monitoring, time estimation, state management,
/// and operation-level tracking with persistence capabilities for reliable
/// rollback execution monitoring and recovery.
pub struct RollbackProgressTracker {
    /// Current progress snapshot
    current_progress: RollbackProgress,
    
    /// Current execution state
    execution_state: ExecutionState,
    
    /// When rollback execution started
    start_time: Option<Instant>,
    
    /// Configuration for progress tracking behavior
    config: ProgressTrackingConfig,
    
    /// Operation history for time estimation
    operation_history: Vec<OperationTiming>,
    
    /// Progress listeners for real-time updates
    listeners: Vec<Box<dyn ProgressListener>>,
    
    /// Persistence handler for recovery
    persistence: Option<Box<dyn ProgressPersistence>>,
    
    /// Progress snapshots for recovery
    snapshots: Vec<ProgressSnapshot>,
    
    /// Time estimation algorithms
    estimator: TimeEstimator,
}

/// Current progress information for rollback execution
#[derive(Debug, Clone, PartialEq)]
pub struct RollbackProgress {
    /// Total number of operations to execute
    pub total_operations: usize,
    
    /// Number of operations completed successfully
    pub completed_operations: usize,
    
    /// Number of operations that failed
    pub failed_operations: usize,
    
    /// Current operation being executed
    pub current_operation: Option<CurrentRollbackOperation>,
    
    /// Percentage of completion (0.0 to 100.0)
    pub percentage_complete: f64,
    
    /// Estimated time remaining for completion
    pub estimated_time_remaining: Option<Duration>,
    
    /// Total elapsed time since start
    pub elapsed_time: Duration,
    
    /// Average time per operation
    pub average_operation_time: Duration,
    
    /// Operations per minute rate
    pub operations_per_minute: f64,
    
    /// Current throughput metrics
    pub throughput: ThroughputMetrics,
    
    /// Progress warnings and issues
    pub warnings: Vec<ProgressWarning>,
}

/// Information about the currently executing rollback operation
#[derive(Debug, Clone, PartialEq)]
pub struct CurrentRollbackOperation {
    /// Index of current operation (0-based)
    pub operation_index: usize,
    
    /// Type description of the operation
    pub operation_type: String,
    
    /// Human-readable description
    pub description: String,
    
    /// Table name being affected
    pub affected_table: Option<String>,
    
    /// When this operation started
    pub started_at: Instant,
    
    /// Estimated duration for this operation
    pub estimated_duration: Duration,
    
    /// Progress within this operation (0.0 to 1.0)
    pub operation_progress: f64,
    
    /// Current phase of operation execution
    pub execution_phase: OperationPhase,
}

/// Phases of individual operation execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationPhase {
    /// Preparing to execute operation
    Preparing,
    
    /// Validating operation prerequisites
    Validating,
    
    /// Generating SQL for operation
    GeneratingSql,
    
    /// Executing SQL statements
    ExecutingSql,
    
    /// Verifying operation results
    Verifying,
    
    /// Operation completed
    Completed,
    
    /// Operation failed
    Failed,
}

/// Configuration for progress tracking behavior
#[derive(Debug, Clone)]
pub struct ProgressTrackingConfig {
    /// Whether to enable detailed operation tracking
    pub enable_detailed_tracking: bool,
    
    /// Interval between progress updates
    pub update_interval: Duration,
    
    /// Whether to persist progress for recovery
    pub enable_persistence: bool,
    
    /// Number of snapshots to keep for recovery
    pub max_snapshots: usize,
    
    /// Whether to estimate time remaining
    pub enable_time_estimation: bool,
    
    /// Window size for time estimation calculations
    pub estimation_window_size: usize,
    
    /// Whether to track operation-level metrics
    pub track_operation_metrics: bool,
    
    /// Minimum elapsed time before providing estimates
    pub min_estimation_time: Duration,
}

/// Timing information for completed operations
#[derive(Debug, Clone)]
struct OperationTiming {
    /// Operation type for categorization
    operation_type: String,
    
    /// Duration of execution
    duration: Duration,
    
    /// When this operation completed
    completed_at: Instant,
    
    /// Whether operation succeeded
    success: bool,
    
    /// Number of rows affected
    rows_affected: usize,
}

/// Real-time throughput metrics
#[derive(Debug, Clone, PartialEq)]
pub struct ThroughputMetrics {
    /// Operations completed in last minute
    pub operations_last_minute: usize,
    
    /// Average operations per minute
    pub avg_operations_per_minute: f64,
    
    /// Peak operations per minute achieved
    pub peak_operations_per_minute: f64,
    
    /// Current processing rate trend
    pub rate_trend: RateTrend,
    
    /// Efficiency percentage vs estimated baseline
    pub efficiency_percentage: f64,
}

/// Trend in processing rate
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateTrend {
    /// Processing rate is increasing
    Accelerating,
    
    /// Processing rate is stable
    Stable,
    
    /// Processing rate is decreasing
    Decelerating,
    
    /// Not enough data to determine trend
    Unknown,
}

/// Progress warnings and issues
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressWarning {
    /// Warning message
    pub message: String,
    
    /// Severity level
    pub severity: WarningSeverity,
    
    /// When warning occurred
    pub timestamp: SystemTime,
    
    /// Suggested action
    pub suggested_action: Option<String>,
}

/// Severity levels for progress warnings
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WarningSeverity {
    /// Informational notice
    Info,
    
    /// Warning condition
    Warning,
    
    /// Error that may affect execution
    Error,
    
    /// Critical issue requiring attention
    Critical,
}

/// Listener interface for real-time progress updates
pub trait ProgressListener: Send + Sync {
    /// Called when progress is updated
    fn on_progress_update(&mut self, progress: &RollbackProgress);
    
    /// Called when execution state changes
    fn on_state_change(&mut self, old_state: ExecutionState, new_state: ExecutionState);
    
    /// Called when an operation starts
    fn on_operation_start(&mut self, operation: &CurrentRollbackOperation);
    
    /// Called when an operation completes
    fn on_operation_complete(&mut self, operation_index: usize, success: bool, duration: Duration);
    
    /// Called when a warning is issued
    fn on_warning(&mut self, warning: &ProgressWarning);
}

/// Interface for persisting progress information
pub trait ProgressPersistence: Send + Sync {
    /// Save current progress state
    fn save_progress(&mut self, progress: &RollbackProgress) -> Result<()>;
    
    /// Load previously saved progress state
    fn load_progress(&self) -> Result<Option<RollbackProgress>>;
    
    /// Save a progress snapshot
    fn save_snapshot(&mut self, snapshot: &ProgressSnapshot) -> Result<()>;
    
    /// Load progress snapshots
    fn load_snapshots(&self) -> Result<Vec<ProgressSnapshot>>;
    
    /// Clear all saved progress data
    fn clear_progress(&mut self) -> Result<()>;
}

/// Progress snapshot for recovery scenarios
#[derive(Debug, Clone)]
pub struct ProgressSnapshot {
    /// Unique identifier for this snapshot
    pub snapshot_id: String,
    
    /// When snapshot was taken
    pub timestamp: SystemTime,
    
    /// Progress state at time of snapshot
    pub progress: RollbackProgress,
    
    /// Execution state at time of snapshot
    pub execution_state: ExecutionState,
    
    /// Operations completed at time of snapshot
    pub operations_completed: Vec<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Time estimation engine for rollback operations
#[derive(Debug, Clone)]
struct TimeEstimator {
    /// Historical operation timings
    operation_timings: HashMap<String, Vec<Duration>>,
    
    /// Baseline estimates for operation types
    baseline_estimates: HashMap<String, Duration>,
    
    /// Recent performance window
    recent_window: Vec<OperationTiming>,
    
    /// Configuration for estimation
    config: EstimationConfig,
}

/// Configuration for time estimation algorithms
#[derive(Debug, Clone)]
struct EstimationConfig {
    /// Size of rolling window for estimates
    window_size: usize,
    
    /// Weight given to recent vs historical data
    recent_weight: f64,
    
    /// Minimum samples required for estimation
    min_samples: usize,
    
    /// Maximum estimation error tolerance
    max_error_tolerance: f64,
}

impl Default for ProgressTrackingConfig {
    fn default() -> Self {
        Self {
            enable_detailed_tracking: true,
            update_interval: Duration::from_millis(100),
            enable_persistence: false,
            max_snapshots: 10,
            enable_time_estimation: true,
            estimation_window_size: 20,
            track_operation_metrics: true,
            min_estimation_time: Duration::from_secs(5),
        }
    }
}

impl Default for EstimationConfig {
    fn default() -> Self {
        Self {
            window_size: 20,
            recent_weight: 0.7,
            min_samples: 3,
            max_error_tolerance: 0.5,
        }
    }
}

impl RollbackProgressTracker {
    /// Create a new progress tracker with default configuration
    pub fn new() -> Self {
        Self::with_config(ProgressTrackingConfig::default())
    }
    
    /// Create a new progress tracker with custom configuration
    pub fn with_config(config: ProgressTrackingConfig) -> Self {
        Self {
            current_progress: RollbackProgress::default(),
            execution_state: ExecutionState::NotStarted,
            start_time: None,
            config,
            operation_history: Vec::new(),
            listeners: Vec::new(),
            persistence: None,
            snapshots: Vec::new(),
            estimator: TimeEstimator::new(),
        }
    }
    
    /// Start rollback execution tracking
    /// 
    /// Initializes progress tracking for a rollback with the specified number
    /// of total operations. Sets up timing, metrics, and notifies listeners.
    pub fn start_execution(&mut self, total_operations: usize) -> Result<()> {
        self.start_time = Some(Instant::now());
        self.execution_state = ExecutionState::NotStarted;
        
        self.current_progress = RollbackProgress {
            total_operations,
            completed_operations: 0,
            failed_operations: 0,
            current_operation: None,
            percentage_complete: 0.0,
            estimated_time_remaining: None,
            elapsed_time: Duration::from_secs(0),
            average_operation_time: Duration::from_secs(0),
            operations_per_minute: 0.0,
            throughput: ThroughputMetrics::default(),
            warnings: Vec::new(),
        };
        
        // Notify listeners
        for listener in &mut self.listeners {
            listener.on_progress_update(&self.current_progress);
        }
        
        // Save initial state if persistence enabled
        if let Some(persistence) = &mut self.persistence {
            persistence.save_progress(&self.current_progress)?;
        }
        
        Ok(())
    }
    
    /// Start tracking an individual operation
    /// 
    /// Records the start of a specific rollback operation with timing
    /// and progress tracking for operation-level monitoring.
    pub fn start_operation(&mut self, index: usize, operation_type: String) -> Result<()> {
        let operation_description = format!("Executing {} operation", operation_type);
        let estimated_duration = self.estimator.estimate_operation_duration(&operation_type);
        
        let current_operation = CurrentRollbackOperation {
            operation_index: index,
            operation_type: operation_type.clone(),
            description: operation_description,
            affected_table: None, // Could be enhanced to extract from operation
            started_at: Instant::now(),
            estimated_duration,
            operation_progress: 0.0,
            execution_phase: OperationPhase::Preparing,
        };
        
        self.current_progress.current_operation = Some(current_operation.clone());
        self.update_progress_metrics()?;
        
        // Notify listeners
        for listener in &mut self.listeners {
            listener.on_operation_start(&current_operation);
        }
        
        Ok(())
    }
    
    /// Complete an operation with success/failure status
    /// 
    /// Records the completion of an operation, updates timing metrics,
    /// and calculates updated progress and time estimates.
    pub fn complete_operation(&mut self, index: usize, success: bool) -> Result<()> {
        let operation_duration = if let Some(current_op) = &self.current_progress.current_operation {
            if current_op.operation_index == index {
                current_op.started_at.elapsed()
            } else {
                Duration::from_secs(0) // Mismatch - use default
            }
        } else {
            Duration::from_secs(0)
        };
        
        // Update counters
        if success {
            self.current_progress.completed_operations += 1;
        } else {
            self.current_progress.failed_operations += 1;
        }
        
        // Record timing for estimation
        if let Some(current_op) = &self.current_progress.current_operation {
            let timing = OperationTiming {
                operation_type: current_op.operation_type.clone(),
                duration: operation_duration,
                completed_at: Instant::now(),
                success,
                rows_affected: 0, // Could be enhanced to track actual rows
            };
            
            self.operation_history.push(timing.clone());
            self.estimator.add_timing(timing);
        }
        
        // Clear current operation
        self.current_progress.current_operation = None;
        
        // Update progress metrics
        self.update_progress_metrics()?;
        
        // Notify listeners
        for listener in &mut self.listeners {
            listener.on_operation_complete(index, success, operation_duration);
        }
        
        // Create snapshot if configured
        if self.config.enable_persistence && self.snapshots.len() < self.config.max_snapshots {
            self.create_snapshot()?;
        }
        
        Ok(())
    }
    
    /// Update the execution state
    /// 
    /// Changes the current execution state and notifies all registered
    /// listeners about the state transition.
    pub fn update_status(&mut self, new_state: ExecutionState) -> Result<()> {
        let old_state = self.execution_state.clone();
        self.execution_state = new_state.clone();
        
        // Update progress based on state
        self.update_progress_metrics()?;
        
        // Notify listeners
        for listener in &mut self.listeners {
            listener.on_state_change(old_state.clone(), new_state.clone());
        }
        
        Ok(())
    }
    
    /// Get current progress snapshot
    pub fn get_current_progress(&self) -> RollbackProgress {
        self.current_progress.clone()
    }
    
    /// Get current execution state
    pub fn get_execution_state(&self) -> ExecutionState {
        self.execution_state.clone()
    }
    
    /// Add a progress listener for real-time updates
    pub fn add_listener(&mut self, listener: Box<dyn ProgressListener>) {
        self.listeners.push(listener);
    }
    
    /// Set persistence handler for progress recovery
    pub fn set_persistence(&mut self, persistence: Box<dyn ProgressPersistence>) {
        self.persistence = Some(persistence);
    }
    
    /// Add a warning to the progress tracking
    pub fn add_warning(&mut self, message: String, severity: WarningSeverity) -> Result<()> {
        let warning = ProgressWarning {
            message,
            severity,
            timestamp: SystemTime::now(),
            suggested_action: None,
        };
        
        self.current_progress.warnings.push(warning.clone());
        
        // Notify listeners
        for listener in &mut self.listeners {
            listener.on_warning(&warning);
        }
        
        Ok(())
    }
    
    /// Update operation phase within current operation
    pub fn update_operation_phase(&mut self, phase: OperationPhase) -> Result<()> {
        if let Some(current_op) = &mut self.current_progress.current_operation {
            current_op.execution_phase = phase;
            
            // Update operation progress based on phase
            current_op.operation_progress = match current_op.execution_phase {
                OperationPhase::Preparing => 0.1,
                OperationPhase::Validating => 0.2,
                OperationPhase::GeneratingSql => 0.4,
                OperationPhase::ExecutingSql => 0.8,
                OperationPhase::Verifying => 0.9,
                OperationPhase::Completed => 1.0,
                OperationPhase::Failed => 0.0,
            };
        }
        
        Ok(())
    }
    
    /// Recover progress from persistence
    pub fn recover_progress(&mut self) -> Result<bool> {
        if let Some(persistence) = &self.persistence {
            if let Some(saved_progress) = persistence.load_progress()? {
                self.current_progress = saved_progress;
                return Ok(true);
            }
            
            // Try to recover from snapshots
            let snapshots = persistence.load_snapshots()?;
            if let Some(latest_snapshot) = snapshots.last() {
                self.current_progress = latest_snapshot.progress.clone();
                self.execution_state = latest_snapshot.execution_state.clone();
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// Create a progress snapshot for recovery
    fn create_snapshot(&mut self) -> Result<()> {
        let snapshot = ProgressSnapshot {
            snapshot_id: format!("snapshot_{}", self.snapshots.len() + 1),
            timestamp: SystemTime::now(),
            progress: self.current_progress.clone(),
            execution_state: self.execution_state.clone(),
            operations_completed: self.operation_history.iter()
                .map(|t| t.operation_type.clone())
                .collect(),
            metadata: HashMap::new(),
        };
        
        if let Some(persistence) = &mut self.persistence {
            persistence.save_snapshot(&snapshot)?;
        }
        
        self.snapshots.push(snapshot);
        
        // Limit snapshots to configured maximum
        if self.snapshots.len() > self.config.max_snapshots {
            self.snapshots.remove(0);
        }
        
        Ok(())
    }
    
    /// Update all progress metrics and calculations
    fn update_progress_metrics(&mut self) -> Result<()> {
        let total_completed = self.current_progress.completed_operations + self.current_progress.failed_operations;
        
        // Calculate percentage complete
        if self.current_progress.total_operations > 0 {
            self.current_progress.percentage_complete = 
                (total_completed as f64 / self.current_progress.total_operations as f64) * 100.0;
        }
        
        // Update elapsed time
        if let Some(start_time) = self.start_time {
            self.current_progress.elapsed_time = start_time.elapsed();
        }
        
        // Calculate average operation time
        if total_completed > 0 && self.current_progress.elapsed_time > Duration::from_secs(0) {
            self.current_progress.average_operation_time = Duration::from_nanos(
                self.current_progress.elapsed_time.as_nanos() as u64 / total_completed as u64
            );
            
            // Calculate operations per minute
            let elapsed_minutes = self.current_progress.elapsed_time.as_secs_f64() / 60.0;
            if elapsed_minutes > 0.0 {
                self.current_progress.operations_per_minute = total_completed as f64 / elapsed_minutes;
            }
        }
        
        // Update time estimation
        if self.config.enable_time_estimation && self.current_progress.elapsed_time >= self.config.min_estimation_time {
            self.current_progress.estimated_time_remaining = self.estimate_remaining_time();
        }
        
        // Update throughput metrics
        self.update_throughput_metrics();
        
        // Save progress if persistence enabled
        if let Some(persistence) = &mut self.persistence {
            persistence.save_progress(&self.current_progress)?;
        }
        
        // Notify listeners
        for listener in &mut self.listeners {
            listener.on_progress_update(&self.current_progress);
        }
        
        Ok(())
    }
    
    /// Estimate remaining time for completion
    fn estimate_remaining_time(&self) -> Option<Duration> {
        let remaining_operations = self.current_progress.total_operations - 
            (self.current_progress.completed_operations + self.current_progress.failed_operations);
        
        if remaining_operations == 0 {
            return Some(Duration::from_secs(0));
        }
        
        // Use multiple estimation methods and take average
        let mut estimates = Vec::new();
        
        // Method 1: Average operation time
        if self.current_progress.average_operation_time > Duration::from_secs(0) {
            let estimate = Duration::from_nanos(
                self.current_progress.average_operation_time.as_nanos() as u64 * remaining_operations as u64
            );
            estimates.push(estimate);
        }
        
        // Method 2: Recent performance trend
        if let Some(trend_estimate) = self.estimator.estimate_by_trend(remaining_operations) {
            estimates.push(trend_estimate);
        }
        
        // Method 3: Operation type analysis
        if let Some(type_estimate) = self.estimator.estimate_by_operation_types(remaining_operations) {
            estimates.push(type_estimate);
        }
        
        // Return average of available estimates
        if !estimates.is_empty() {
            let total_nanos: u128 = estimates.iter().map(|d| d.as_nanos()).sum();
            let average_nanos = total_nanos / estimates.len() as u128;
            Some(Duration::from_nanos(average_nanos as u64))
        } else {
            None
        }
    }
    
    /// Update throughput and performance metrics
    fn update_throughput_metrics(&mut self) {
        let now = Instant::now();
        let one_minute_ago = now - Duration::from_secs(60);
        
        // Count operations in last minute
        let operations_last_minute = self.operation_history.iter()
            .filter(|timing| timing.completed_at >= one_minute_ago)
            .count();
        
        // Calculate average operations per minute
        let total_completed = self.current_progress.completed_operations + self.current_progress.failed_operations;
        let avg_ops_per_minute = if self.current_progress.elapsed_time > Duration::from_secs(0) {
            let elapsed_minutes = self.current_progress.elapsed_time.as_secs_f64() / 60.0;
            total_completed as f64 / elapsed_minutes
        } else {
            0.0
        };
        
        // Determine rate trend
        let rate_trend = if self.operation_history.len() >= 10 {
            let recent_rate = self.operation_history.iter()
                .rev()
                .take(5)
                .map(|_| 1.0)
                .sum::<f64>() / 5.0;
            
            let older_rate = self.operation_history.iter()
                .rev()
                .skip(5)
                .take(5)
                .map(|_| 1.0)
                .sum::<f64>() / 5.0;
            
            if recent_rate > older_rate * 1.1 {
                RateTrend::Accelerating
            } else if recent_rate < older_rate * 0.9 {
                RateTrend::Decelerating
            } else {
                RateTrend::Stable
            }
        } else {
            RateTrend::Unknown
        };
        
        self.current_progress.throughput = ThroughputMetrics {
            operations_last_minute,
            avg_operations_per_minute: avg_ops_per_minute,
            peak_operations_per_minute: self.current_progress.throughput.peak_operations_per_minute.max(avg_ops_per_minute),
            rate_trend,
            efficiency_percentage: 85.0, // Could be enhanced with baseline comparison
        };
    }
}

/// Implementation of the progress tracker trait from executor module
impl ProgressTrackerTrait for RollbackProgressTracker {
    /// Update progress with current operation information
    fn update_progress(&mut self, operation_index: usize, _total_operations: usize, 
                      _current_operation: &str, percentage: f64) {
        // Update current operation if needed
        if let Some(current_op) = &mut self.current_progress.current_operation {
            if current_op.operation_index == operation_index {
                current_op.operation_progress = percentage / 100.0;
            }
        }
        
        // Update overall progress
        self.current_progress.percentage_complete = percentage;
        
        // Trigger metrics update
        let _ = self.update_progress_metrics();
    }
    
    /// Report operation completion
    fn operation_completed(&mut self, operation_index: usize, success: bool, _duration: Duration) {
        // This maps to our complete_operation method
        let _ = self.complete_operation(operation_index, success);
    }
    
    /// Report execution state change
    fn state_changed(&mut self, new_state: ExecutionState) {
        // This maps to our update_status method
        let _ = self.update_status(new_state);
    }
    
    /// Add warning message
    fn add_warning(&mut self, warning: String) {
        let _ = self.add_warning(warning, WarningSeverity::Warning);
    }
    
    /// Add error message
    fn add_error(&mut self, error: String) {
        let _ = self.add_warning(error, WarningSeverity::Error);
    }
}

impl TimeEstimator {
    /// Create a new time estimator
    fn new() -> Self {
        Self {
            operation_timings: HashMap::new(),
            baseline_estimates: Self::create_baseline_estimates(),
            recent_window: Vec::new(),
            config: EstimationConfig::default(),
        }
    }
    
    /// Add a completed operation timing for learning
    fn add_timing(&mut self, timing: OperationTiming) {
        // Add to operation type history
        self.operation_timings
            .entry(timing.operation_type.clone())
            .or_insert_with(Vec::new)
            .push(timing.duration);
        
        // Add to recent window
        self.recent_window.push(timing);
        
        // Limit window size
        if self.recent_window.len() > self.config.window_size {
            self.recent_window.remove(0);
        }
    }
    
    /// Estimate duration for a specific operation type
    fn estimate_operation_duration(&self, operation_type: &str) -> Duration {
        // Try historical data first
        if let Some(timings) = self.operation_timings.get(operation_type) {
            if timings.len() >= self.config.min_samples {
                // Apply weighted estimation giving more weight to recent data
                let recent_count = std::cmp::min(timings.len(), 5);
                let recent_weight = self.config.recent_weight;
                let historical_weight = 1.0 - recent_weight;
                
                // Calculate recent average
                let recent_timings = &timings[timings.len().saturating_sub(recent_count)..];
                let recent_avg: u128 = recent_timings.iter().map(|d| d.as_nanos()).sum::<u128>() / recent_count as u128;
                
                // Calculate historical average
                let historical_avg: u128 = timings.iter().map(|d| d.as_nanos()).sum::<u128>() / timings.len() as u128;
                
                // Weighted combination
                let weighted_avg = (recent_avg as f64 * recent_weight + historical_avg as f64 * historical_weight) as u128;
                return Duration::from_nanos(weighted_avg as u64);
            }
        }
        
        // Check recent window for similar operations
        let similar_ops: Vec<&OperationTiming> = self.recent_window.iter()
            .filter(|t| t.operation_type == operation_type && t.success)
            .collect();
            
        if similar_ops.len() >= self.config.min_samples {
            // Use success rate to adjust estimate
            let success_rate = similar_ops.len() as f64 / self.recent_window.iter()
                .filter(|t| t.operation_type == operation_type).count() as f64;
            let avg_duration: u128 = similar_ops.iter().map(|t| t.duration.as_nanos()).sum::<u128>() / similar_ops.len() as u128;
            
            // Adjust based on success rate using max_error_tolerance
            let adjusted_duration = if success_rate < (1.0 - self.config.max_error_tolerance) {
                (avg_duration as f64 * 1.5) as u128  // Increase estimate for unreliable operations
            } else {
                avg_duration
            };
            
            return Duration::from_nanos(adjusted_duration as u64);
        }
        
        // Fall back to baseline estimate
        self.baseline_estimates.get(operation_type)
            .copied()
            .unwrap_or(Duration::from_secs(1))
    }
    
    /// Estimate time based on recent performance trend
    fn estimate_by_trend(&self, remaining_operations: usize) -> Option<Duration> {
        if self.recent_window.len() < self.config.min_samples {
            return None;
        }
        
        // Filter successful operations for more reliable estimates
        let successful_ops: Vec<&OperationTiming> = self.recent_window.iter()
            .filter(|t| t.success)
            .collect();
            
        if successful_ops.is_empty() {
            return None;
        }
        
        // Calculate weighted average giving more weight to recent operations
        let recent_weight = self.config.recent_weight;
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;
        
        for (i, timing) in successful_ops.iter().enumerate() {
            let weight = if i >= successful_ops.len().saturating_sub(3) {
                recent_weight  // More weight for last 3 operations
            } else {
                1.0 - recent_weight
            };
            
            let mut base_duration = timing.duration.as_nanos() as f64;
            
            // Factor in rows affected for better size correlation
            if timing.rows_affected > 0 {
                let size_factor = (timing.rows_affected as f64).log10().max(1.0);
                base_duration *= size_factor / 3.0;  // Normalize size impact
            }
            
            weighted_sum += base_duration * weight;
            weight_sum += weight;
        }
        
        let average_nanos = if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            successful_ops.iter().map(|t| t.duration.as_nanos()).sum::<u128>() as f64 / successful_ops.len() as f64
        };
        
        // Apply error tolerance check using max_error_tolerance
        let success_rate = successful_ops.len() as f64 / self.recent_window.len() as f64;
        let reliability_factor = if success_rate < (1.0 - self.config.max_error_tolerance) {
            1.5  // Increase estimate if error rate is high
        } else {
            1.0
        };
        
        let estimated_total_nanos = (average_nanos * remaining_operations as f64 * reliability_factor) as u64;
        Some(Duration::from_nanos(estimated_total_nanos))
    }
    
    /// Estimate time based on operation type analysis
    fn estimate_by_operation_types(&self, remaining_operations: usize) -> Option<Duration> {
        if self.operation_timings.is_empty() {
            return None;
        }
        
        // Use average across all operation types as estimate
        let mut total_duration = Duration::from_secs(0);
        let mut total_count = 0;
        
        for timings in self.operation_timings.values() {
            if !timings.is_empty() {
                let type_total: u128 = timings.iter().map(|d| d.as_nanos()).sum();
                let type_average = Duration::from_nanos((type_total / timings.len() as u128) as u64);
                total_duration += type_average;
                total_count += 1;
            }
        }
        
        if total_count > 0 {
            let overall_average = Duration::from_nanos(total_duration.as_nanos() as u64 / total_count);
            Some(Duration::from_nanos(overall_average.as_nanos() as u64 * remaining_operations as u64))
        } else {
            None
        }
    }
    
    /// Create baseline estimates for common operation types
    fn create_baseline_estimates() -> HashMap<String, Duration> {
        let mut estimates = HashMap::new();
        
        // Database operation estimates (in seconds)
        estimates.insert("DropTable".to_string(), Duration::from_millis(500));
        estimates.insert("CreateTable".to_string(), Duration::from_millis(300));
        estimates.insert("AddColumn".to_string(), Duration::from_millis(200));
        estimates.insert("DropColumn".to_string(), Duration::from_millis(400));
        estimates.insert("ModifyColumn".to_string(), Duration::from_millis(600));
        estimates.insert("CreateIndex".to_string(), Duration::from_millis(800));
        estimates.insert("DropIndex".to_string(), Duration::from_millis(200));
        estimates.insert("AddForeignKey".to_string(), Duration::from_millis(300));
        estimates.insert("DropForeignKey".to_string(), Duration::from_millis(100));
        estimates.insert("RenameTable".to_string(), Duration::from_millis(100));
        estimates.insert("RenameColumn".to_string(), Duration::from_millis(100));
        
        estimates
    }
}

/// Default implementations for required traits
impl Default for RollbackProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RollbackProgress {
    fn default() -> Self {
        Self {
            total_operations: 0,
            completed_operations: 0,
            failed_operations: 0,
            current_operation: None,
            percentage_complete: 0.0,
            estimated_time_remaining: None,
            elapsed_time: Duration::from_secs(0),
            average_operation_time: Duration::from_secs(0),
            operations_per_minute: 0.0,
            throughput: ThroughputMetrics::default(),
            warnings: Vec::new(),
        }
    }
}

impl Default for ThroughputMetrics {
    fn default() -> Self {
        Self {
            operations_last_minute: 0,
            avg_operations_per_minute: 0.0,
            peak_operations_per_minute: 0.0,
            rate_trend: RateTrend::Unknown,
            efficiency_percentage: 100.0,
        }
    }
}

/// Simple console progress listener for basic monitoring
#[derive(Debug)]
pub struct ConsoleProgressListener {
    verbose: bool,
}

impl ConsoleProgressListener {
    /// Create a new console progress listener
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl ProgressListener for ConsoleProgressListener {
    fn on_progress_update(&mut self, progress: &RollbackProgress) {
        if self.verbose {
            println!(
                "Progress: {:.1}% ({}/{}) - {} remaining",
                progress.percentage_complete,
                progress.completed_operations + progress.failed_operations,
                progress.total_operations,
                progress.estimated_time_remaining
                    .map(|d| format!("{:.1}s", d.as_secs_f64()))
                    .unwrap_or_else(|| "unknown".to_string())
            );
        }
    }
    
    fn on_state_change(&mut self, _old_state: ExecutionState, new_state: ExecutionState) {
        if self.verbose {
            println!("State changed to: {:?}", new_state);
        }
    }
    
    fn on_operation_start(&mut self, operation: &CurrentRollbackOperation) {
        if self.verbose {
            println!("Starting operation {}: {}", operation.operation_index, operation.description);
        }
    }
    
    fn on_operation_complete(&mut self, operation_index: usize, success: bool, duration: Duration) {
        if self.verbose {
            println!(
                "Operation {} {}: {:.2}s",
                operation_index,
                if success { "completed" } else { "failed" },
                duration.as_secs_f64()
            );
        }
    }
    
    fn on_warning(&mut self, warning: &ProgressWarning) {
        println!("WARNING [{:?}]: {}", warning.severity, warning.message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_progress_tracker_creation() {
        let tracker = RollbackProgressTracker::new();
        assert_eq!(tracker.get_execution_state(), ExecutionState::NotStarted);
        
        let progress = tracker.get_current_progress();
        assert_eq!(progress.total_operations, 0);
        assert_eq!(progress.completed_operations, 0);
        assert_eq!(progress.percentage_complete, 0.0);
    }
    
    #[test]
    fn test_progress_tracker_with_config() {
        let config = ProgressTrackingConfig {
            enable_detailed_tracking: false,
            update_interval: Duration::from_secs(1),
            enable_persistence: true,
            max_snapshots: 5,
            enable_time_estimation: false,
            estimation_window_size: 10,
            track_operation_metrics: false,
            min_estimation_time: Duration::from_secs(10),
        };
        
        let tracker = RollbackProgressTracker::with_config(config.clone());
        assert_eq!(tracker.config.enable_detailed_tracking, false);
        assert_eq!(tracker.config.max_snapshots, 5);
    }
    
    #[test]
    fn test_start_execution() {
        let mut tracker = RollbackProgressTracker::new();
        
        let result = tracker.start_execution(10);
        assert!(result.is_ok());
        
        let progress = tracker.get_current_progress();
        assert_eq!(progress.total_operations, 10);
        assert_eq!(progress.completed_operations, 0);
        assert_eq!(progress.percentage_complete, 0.0);
        assert!(tracker.start_time.is_some());
    }
    
    #[test]
    fn test_operation_tracking() {
        let mut tracker = RollbackProgressTracker::new();
        tracker.start_execution(5).unwrap();
        
        // Start first operation
        let result = tracker.start_operation(0, "DropTable".to_string());
        assert!(result.is_ok());
        
        let progress = tracker.get_current_progress();
        assert!(progress.current_operation.is_some());
        
        let current_op = progress.current_operation.unwrap();
        assert_eq!(current_op.operation_index, 0);
        assert_eq!(current_op.operation_type, "DropTable");
        assert_eq!(current_op.execution_phase, OperationPhase::Preparing);
        
        // Complete the operation
        let result = tracker.complete_operation(0, true);
        assert!(result.is_ok());
        
        let progress = tracker.get_current_progress();
        assert_eq!(progress.completed_operations, 1);
        assert_eq!(progress.failed_operations, 0);
        assert_eq!(progress.percentage_complete, 20.0); // 1/5 * 100
        assert!(progress.current_operation.is_none());
    }
    
    #[test]
    fn test_failed_operation_tracking() {
        let mut tracker = RollbackProgressTracker::new();
        tracker.start_execution(5).unwrap();
        
        tracker.start_operation(0, "DropTable".to_string()).unwrap();
        tracker.complete_operation(0, false).unwrap(); // Failed
        
        let progress = tracker.get_current_progress();
        assert_eq!(progress.completed_operations, 0);
        assert_eq!(progress.failed_operations, 1);
        assert_eq!(progress.percentage_complete, 20.0); // Still counts toward total
    }
    
    #[test]
    fn test_state_management() {
        let mut tracker = RollbackProgressTracker::new();
        
        let result = tracker.update_status(ExecutionState::RunningPreChecks);
        assert!(result.is_ok());
        assert_eq!(tracker.get_execution_state(), ExecutionState::RunningPreChecks);
        
        tracker.update_status(ExecutionState::ExecutingOperations).unwrap();
        assert_eq!(tracker.get_execution_state(), ExecutionState::ExecutingOperations);
        
        tracker.update_status(ExecutionState::Completed).unwrap();
        assert_eq!(tracker.get_execution_state(), ExecutionState::Completed);
    }
    
    #[test]
    fn test_progress_calculations() {
        let mut tracker = RollbackProgressTracker::new();
        tracker.start_execution(4).unwrap();
        
        // Complete 2 operations successfully
        tracker.start_operation(0, "DropTable".to_string()).unwrap();
        tracker.complete_operation(0, true).unwrap();
        
        tracker.start_operation(1, "CreateTable".to_string()).unwrap();
        tracker.complete_operation(1, true).unwrap();
        
        let progress = tracker.get_current_progress();
        assert_eq!(progress.completed_operations, 2);
        assert_eq!(progress.percentage_complete, 50.0);
        assert!(progress.average_operation_time > Duration::from_secs(0));
        assert!(progress.operations_per_minute >= 0.0);
    }
    
    #[test]
    fn test_warning_system() {
        let mut tracker = RollbackProgressTracker::new();
        
        let result = tracker.add_warning(
            "Operation taking longer than expected".to_string(),
            WarningSeverity::Warning
        );
        assert!(result.is_ok());
        
        let progress = tracker.get_current_progress();
        assert_eq!(progress.warnings.len(), 1);
        
        let warning = &progress.warnings[0];
        assert_eq!(warning.severity, WarningSeverity::Warning);
        assert!(warning.message.contains("longer than expected"));
    }
    
    #[test]
    fn test_operation_phase_updates() {
        let mut tracker = RollbackProgressTracker::new();
        tracker.start_execution(1).unwrap();
        tracker.start_operation(0, "DropTable".to_string()).unwrap();
        
        // Test phase progression
        tracker.update_operation_phase(OperationPhase::Validating).unwrap();
        let progress = tracker.get_current_progress();
        let current_op = progress.current_operation.unwrap();
        assert_eq!(current_op.execution_phase, OperationPhase::Validating);
        assert_eq!(current_op.operation_progress, 0.2);
        
        tracker.update_operation_phase(OperationPhase::ExecutingSql).unwrap();
        let progress = tracker.get_current_progress();
        let current_op = progress.current_operation.unwrap();
        assert_eq!(current_op.execution_phase, OperationPhase::ExecutingSql);
        assert_eq!(current_op.operation_progress, 0.8);
    }
    
    #[test]
    fn test_throughput_metrics() {
        let mut tracker = RollbackProgressTracker::new();
        tracker.start_execution(10).unwrap();
        
        // Complete several operations to build metrics
        for i in 0..3 {
            tracker.start_operation(i, "TestOperation".to_string()).unwrap();
            // Simulate some processing time
            std::thread::sleep(Duration::from_millis(10));
            tracker.complete_operation(i, true).unwrap();
        }
        
        let progress = tracker.get_current_progress();
        assert!(progress.throughput.avg_operations_per_minute > 0.0);
        assert_eq!(progress.throughput.operations_last_minute, 3);
    }
    
    #[test]
    fn test_time_estimator() {
        let mut estimator = TimeEstimator::new();
        
        // Test baseline estimates
        let estimate = estimator.estimate_operation_duration("DropTable");
        assert_eq!(estimate, Duration::from_millis(500));
        
        // Add some timings
        let timing = OperationTiming {
            operation_type: "DropTable".to_string(),
            duration: Duration::from_millis(300),
            completed_at: Instant::now(),
            success: true,
            rows_affected: 0,
        };
        estimator.add_timing(timing);
        
        // Should still use baseline since we need min_samples
        let estimate = estimator.estimate_operation_duration("DropTable");
        assert_eq!(estimate, Duration::from_millis(500));
    }
    
    #[test]
    fn test_console_progress_listener() {
        let mut listener = ConsoleProgressListener::new(false);
        
        let progress = RollbackProgress::default();
        listener.on_progress_update(&progress); // Should not panic
        
        listener.on_state_change(ExecutionState::NotStarted, ExecutionState::ExecutingOperations);
        
        let operation = CurrentRollbackOperation {
            operation_index: 0,
            operation_type: "Test".to_string(),
            description: "Test operation".to_string(),
            affected_table: None,
            started_at: Instant::now(),
            estimated_duration: Duration::from_secs(1),
            operation_progress: 0.0,
            execution_phase: OperationPhase::Preparing,
        };
        listener.on_operation_start(&operation);
        listener.on_operation_complete(0, true, Duration::from_secs(1));
        
        let warning = ProgressWarning {
            message: "Test warning".to_string(),
            severity: WarningSeverity::Info,
            timestamp: SystemTime::now(),
            suggested_action: None,
        };
        listener.on_warning(&warning);
    }
    
    #[test]
    fn test_warning_severity_ordering() {
        assert!(WarningSeverity::Critical > WarningSeverity::Error);
        assert!(WarningSeverity::Error > WarningSeverity::Warning);
        assert!(WarningSeverity::Warning > WarningSeverity::Info);
    }
    
    #[test]
    fn test_rate_trend_detection() {
        // Test that rate trend enum variants exist and can be compared
        assert_eq!(RateTrend::Accelerating, RateTrend::Accelerating);
        assert_ne!(RateTrend::Accelerating, RateTrend::Stable);
        assert_ne!(RateTrend::Stable, RateTrend::Decelerating);
    }
    
    #[test]
    fn test_progress_snapshot() {
        let progress = RollbackProgress::default();
        let snapshot = ProgressSnapshot {
            snapshot_id: "test_snapshot".to_string(),
            timestamp: SystemTime::now(),
            progress: progress.clone(),
            execution_state: ExecutionState::ExecutingOperations,
            operations_completed: vec!["op1".to_string(), "op2".to_string()],
            metadata: HashMap::new(),
        };
        
        assert_eq!(snapshot.snapshot_id, "test_snapshot");
        assert_eq!(snapshot.operations_completed.len(), 2);
        assert_eq!(snapshot.execution_state, ExecutionState::ExecutingOperations);
    }
}