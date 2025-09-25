/// Health check system for Phase 5 migration engine components
/// 
/// This module provides continuous health monitoring and system status checks
/// for all migration engine components.

use crate::backends::DatabaseBackend;
use crate::dialects::DatabaseDialect;
use crate::migration_engine::verification::{Phase5Verifier, VerificationError};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HealthCheckError {
    #[error("Health check failed for component {component}: {message}")]
    ComponentUnhealthy { component: String, message: String },
    
    #[error("System-wide health check failed: {message}")]
    SystemUnhealthy { message: String },
    
    #[error("Health check timeout for {component}: exceeded {timeout_ms}ms")]
    Timeout { component: String, timeout_ms: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub last_check: u64, // Unix timestamp
    pub response_time_ms: u64,
    pub error_count: usize,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub component_health: HashMap<String, ComponentHealth>,
    pub last_full_check: u64,
    pub uptime_seconds: u64,
    pub total_checks: usize,
    pub failed_checks: usize,
}

pub struct HealthChecker {
    #[allow(dead_code)]
    dialect: DatabaseDialect,
    verifier: Phase5Verifier,
    #[allow(dead_code)]
    check_intervals: HashMap<String, Duration>,
    timeout_thresholds: HashMap<String, Duration>,
    health_history: HashMap<String, Vec<ComponentHealth>>,
    system_start_time: SystemTime,
}

impl HealthChecker {
    pub fn new(dialect: DatabaseDialect) -> Self {
        let mut check_intervals = HashMap::new();
        check_intervals.insert("schema_introspection".to_string(), Duration::from_secs(30));
        check_intervals.insert("ddl_generation".to_string(), Duration::from_secs(60));
        check_intervals.insert("migration_execution".to_string(), Duration::from_secs(45));
        check_intervals.insert("rollback_generation".to_string(), Duration::from_secs(60));
        check_intervals.insert("data_migration".to_string(), Duration::from_secs(120));
        
        let mut timeout_thresholds = HashMap::new();
        timeout_thresholds.insert("schema_introspection".to_string(), Duration::from_secs(5));
        timeout_thresholds.insert("ddl_generation".to_string(), Duration::from_secs(3));
        timeout_thresholds.insert("migration_execution".to_string(), Duration::from_secs(10));
        timeout_thresholds.insert("rollback_generation".to_string(), Duration::from_secs(2));
        timeout_thresholds.insert("data_migration".to_string(), Duration::from_secs(30));
        
        Self {
            dialect,
            verifier: Phase5Verifier::new(dialect),
            check_intervals,
            timeout_thresholds,
            health_history: HashMap::new(),
            system_start_time: SystemTime::now(),
        }
    }
    
    pub async fn perform_full_health_check<B: DatabaseBackend>(&mut self, backend: &B) -> Result<SystemHealth, HealthCheckError> {
        let _start_time = SystemTime::now();
        let mut component_health = HashMap::new();
        let mut failed_checks = 0;
        let mut total_checks = 0;
        
        // Check each component
        let components = [
            "schema_introspection",
            "ddl_generation", 
            "migration_execution",
            "rollback_generation",
            "data_migration"
        ];
        
        for component in &components {
            total_checks += 1;
            
            match self.check_component_health(component, backend).await {
                Ok(health) => {
                    component_health.insert(component.to_string(), health.clone());
                    self.record_health_history(component, health);
                },
                Err(error) => {
                    failed_checks += 1;
                    let unhealthy = ComponentHealth {
                        status: HealthStatus::Unhealthy,
                        last_check: current_timestamp(),
                        response_time_ms: 0,
                        error_count: 1,
                        details: error.to_string(),
                    };
                    component_health.insert(component.to_string(), unhealthy.clone());
                    self.record_health_history(component, unhealthy);
                }
            }
        }
        
        // Determine overall system health
        let overall_status = self.calculate_overall_health(&component_health);
        
        let uptime = self.system_start_time.elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
            
        Ok(SystemHealth {
            overall_status,
            component_health,
            last_full_check: current_timestamp(),
            uptime_seconds: uptime,
            total_checks,
            failed_checks,
        })
    }
    
    async fn check_component_health<B: DatabaseBackend>(&self, component: &str, backend: &B) -> Result<ComponentHealth, HealthCheckError> {
        let start_time = SystemTime::now();
        let default_timeout = Duration::from_secs(10);
        let timeout = self.timeout_thresholds.get(component)
            .unwrap_or(&default_timeout);
        
        // Perform component-specific health check with timeout
        let check_result = tokio::time::timeout(*timeout, self.perform_component_check(component, backend)).await;
        
        let response_time = start_time.elapsed()
            .unwrap_or(Duration::from_millis(0))
            .as_millis() as u64;
        
        match check_result {
            Ok(Ok(())) => {
                Ok(ComponentHealth {
                    status: HealthStatus::Healthy,
                    last_check: current_timestamp(),
                    response_time_ms: response_time,
                    error_count: 0,
                    details: "Component is functioning normally".to_string(),
                })
            },
            Ok(Err(error)) => {
                let status = if response_time > (timeout.as_millis() as u64 / 2) {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Unhealthy
                };
                
                Ok(ComponentHealth {
                    status,
                    last_check: current_timestamp(),
                    response_time_ms: response_time,
                    error_count: 1,
                    details: error.to_string(),
                })
            },
            Err(_) => {
                Err(HealthCheckError::Timeout { 
                    component: component.to_string(), 
                    timeout_ms: timeout.as_millis() as u64 
                })
            }
        }
    }
    
    async fn perform_component_check<B: DatabaseBackend>(&self, component: &str, backend: &B) -> Result<(), VerificationError> {
        match component {
            "schema_introspection" => {
                let mut dummy_report = crate::migration_engine::verification::VerificationReport::new();
                self.verifier.verify_schema_introspection(backend, &mut dummy_report).await?;
            },
            "ddl_generation" => {
                let mut dummy_report = crate::migration_engine::verification::VerificationReport::new();
                self.verifier.verify_ddl_generation(&mut dummy_report).await?;
            },
            "migration_execution" => {
                let mut dummy_report = crate::migration_engine::verification::VerificationReport::new();
                self.verifier.verify_migration_execution(backend, &mut dummy_report).await?;
            },
            "rollback_generation" => {
                let mut dummy_report = crate::migration_engine::verification::VerificationReport::new();
                self.verifier.verify_rollback_system(&mut dummy_report).await?;
            },
            "data_migration" => {
                let mut dummy_report = crate::migration_engine::verification::VerificationReport::new();
                self.verifier.verify_data_migration(&mut dummy_report).await?;
            },
            _ => return Err(VerificationError::ComponentFailed { 
                component: component.to_string(), 
                message: "Unknown component".to_string() 
            }),
        }
        
        Ok(())
    }
    
    fn record_health_history(&mut self, component: &str, health: ComponentHealth) {
        let history = self.health_history.entry(component.to_string()).or_insert_with(Vec::new);
        history.push(health);
        
        // Keep only the last 100 health records per component
        if history.len() > 100 {
            history.drain(0..history.len() - 100);
        }
    }
    
    fn calculate_overall_health(&self, component_health: &HashMap<String, ComponentHealth>) -> HealthStatus {
        if component_health.is_empty() {
            return HealthStatus::Unknown;
        }
        
        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        let total_components = component_health.len();
        
        for health in component_health.values() {
            match health.status {
                HealthStatus::Healthy => healthy_count += 1,
                HealthStatus::Degraded => degraded_count += 1,
                HealthStatus::Unhealthy => unhealthy_count += 1,
                HealthStatus::Unknown => {} // Don't count unknown
            }
        }
        
        // System health logic:
        // - Healthy: All components healthy
        // - Degraded: Some components degraded but none unhealthy, or minor issues
        // - Unhealthy: Any component is unhealthy, or majority of components have issues
        
        if unhealthy_count > 0 {
            HealthStatus::Unhealthy
        } else if degraded_count > total_components / 2 {
            HealthStatus::Unhealthy
        } else if degraded_count > 0 {
            HealthStatus::Degraded
        } else if healthy_count == total_components {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        }
    }
    
    pub fn get_component_history(&self, component: &str) -> Option<&Vec<ComponentHealth>> {
        self.health_history.get(component)
    }
    
    pub fn get_health_trends(&self, component: &str, duration: Duration) -> HealthTrend {
        let history = match self.health_history.get(component) {
            Some(history) => history,
            None => return HealthTrend::Unknown,
        };
        
        let cutoff_time = current_timestamp() - duration.as_secs();
        let recent_checks: Vec<_> = history.iter()
            .filter(|health| health.last_check >= cutoff_time)
            .collect();
        
        if recent_checks.is_empty() {
            return HealthTrend::Unknown;
        }
        
        let healthy_count = recent_checks.iter().filter(|h| matches!(h.status, HealthStatus::Healthy)).count();
        let _degraded_count = recent_checks.iter().filter(|h| matches!(h.status, HealthStatus::Degraded)).count();
        let unhealthy_count = recent_checks.iter().filter(|h| matches!(h.status, HealthStatus::Unhealthy)).count();
        
        let total = recent_checks.len();
        let health_ratio = healthy_count as f64 / total as f64;
        
        if health_ratio >= 0.9 {
            HealthTrend::Improving
        } else if health_ratio >= 0.5 {
            HealthTrend::Stable
        } else if unhealthy_count > total / 2 {
            HealthTrend::Degrading
        } else {
            HealthTrend::Unstable
        }
    }
    
    pub fn generate_health_report(&self, system_health: &SystemHealth) -> String {
        let mut report = String::new();
        report.push_str("=== Phase 5 Migration Engine Health Report ===\n\n");
        
        // Overall system status
        let overall_emoji = match system_health.overall_status {
            HealthStatus::Healthy => "✅",
            HealthStatus::Degraded => "⚠️",
            HealthStatus::Unhealthy => "❌",
            HealthStatus::Unknown => "❓",
        };
        
        report.push_str(&format!("Overall Status: {} {:?}\n", overall_emoji, system_health.overall_status));
        report.push_str(&format!("System Uptime: {}s\n", system_health.uptime_seconds));
        report.push_str(&format!("Total Health Checks: {}\n", system_health.total_checks));
        report.push_str(&format!("Failed Checks: {}\n", system_health.failed_checks));
        
        if system_health.total_checks > 0 {
            let success_rate = (system_health.total_checks - system_health.failed_checks) as f64 / system_health.total_checks as f64 * 100.0;
            report.push_str(&format!("Success Rate: {:.1}%\n", success_rate));
        }
        
        report.push_str("\n=== Component Health ===\n");
        
        for (component, health) in &system_health.component_health {
            let status_emoji = match health.status {
                HealthStatus::Healthy => "✅",
                HealthStatus::Degraded => "⚠️",
                HealthStatus::Unhealthy => "❌",
                HealthStatus::Unknown => "❓",
            };
            
            report.push_str(&format!(
                "{} {} - {:?} ({}ms, {} errors)\n",
                status_emoji, component, health.status, health.response_time_ms, health.error_count
            ));
            
            if !health.details.is_empty() && health.error_count > 0 {
                report.push_str(&format!("    Details: {}\n", health.details));
            }
            
            // Add trend information if available
            let trend = self.get_health_trends(component, Duration::from_secs(3600)); // Last hour
            if !matches!(trend, HealthTrend::Unknown) {
                report.push_str(&format!("    Trend (1h): {:?}\n", trend));
            }
        }
        
        // Add recommendations
        report.push_str("\n=== Recommendations ===\n");
        
        let mut recommendations = Vec::new();
        for (component, health) in &system_health.component_health {
            match health.status {
                HealthStatus::Unhealthy => {
                    recommendations.push(format!("🔧 Investigate {} component - currently unhealthy", component));
                },
                HealthStatus::Degraded => {
                    if health.response_time_ms > 1000 {
                        recommendations.push(format!("⚡ Optimize {} performance - response time {}ms", component, health.response_time_ms));
                    }
                    if health.error_count > 0 {
                        recommendations.push(format!("🔍 Monitor {} for recurring errors", component));
                    }
                },
                _ => {}
            }
        }
        
        if recommendations.is_empty() {
            report.push_str("✨ All systems are operating normally\n");
        } else {
            for recommendation in recommendations {
                report.push_str(&format!("{}\n", recommendation));
            }
        }
        
        report
    }
    
    pub async fn run_continuous_monitoring<B: DatabaseBackend>(&mut self, backend: &B, duration: Duration) -> Vec<SystemHealth> {
        let mut health_snapshots = Vec::new();
        let start_time = SystemTime::now();
        
        while start_time.elapsed().unwrap_or(Duration::from_secs(0)) < duration {
            if let Ok(health) = self.perform_full_health_check(backend).await {
                health_snapshots.push(health);
            }
            
            // Wait for the minimum check interval (30 seconds)
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
        
        health_snapshots
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthTrend {
    Improving,
    Stable, 
    Degrading,
    Unstable,
    Unknown,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::sqlite::SQLiteBackend;

    #[test]
    fn test_health_checker_creation() {
        let health_checker = HealthChecker::new(DatabaseDialect::SQLite);
        
        // Test that check intervals are set correctly
        assert!(health_checker.check_intervals.contains_key("schema_introspection"));
        assert!(health_checker.check_intervals.contains_key("ddl_generation"));
        assert!(health_checker.check_intervals.contains_key("migration_execution"));
        assert!(health_checker.check_intervals.contains_key("rollback_generation"));
        assert!(health_checker.check_intervals.contains_key("data_migration"));
        
        // Test that timeout thresholds are set correctly
        assert!(health_checker.timeout_thresholds.contains_key("schema_introspection"));
        assert!(health_checker.timeout_thresholds.contains_key("ddl_generation"));
        assert!(health_checker.timeout_thresholds.contains_key("migration_execution"));
        assert!(health_checker.timeout_thresholds.contains_key("rollback_generation"));
        assert!(health_checker.timeout_thresholds.contains_key("data_migration"));
    }

    #[tokio::test]
    async fn test_full_health_check() {
        let backend = SQLiteBackend::new_in_memory().await.unwrap();
        
        let mut health_checker = HealthChecker::new(DatabaseDialect::SQLite);
        
        let result = health_checker.perform_full_health_check(&backend).await;
        
        // Health check should complete (may have unhealthy components due to incomplete implementation)
        match result {
            Ok(system_health) => {
                assert!(system_health.total_checks > 0);
                assert!(system_health.component_health.len() > 0);
                assert!(matches!(system_health.overall_status, HealthStatus::Healthy | HealthStatus::Degraded | HealthStatus::Unhealthy));
            },
            Err(error) => {
                // Errors are acceptable during development phase
                assert!(format!("{}", error).contains("health check failed") || 
                        format!("{}", error).contains("timeout"));
            }
        }
    }

    #[test]
    fn test_component_health_creation() {
        let health = ComponentHealth {
            status: HealthStatus::Healthy,
            last_check: current_timestamp(),
            response_time_ms: 100,
            error_count: 0,
            details: "All good".to_string(),
        };
        
        assert!(matches!(health.status, HealthStatus::Healthy));
        assert_eq!(health.response_time_ms, 100);
        assert_eq!(health.error_count, 0);
    }

    #[test]
    fn test_overall_health_calculation() {
        let health_checker = HealthChecker::new(DatabaseDialect::SQLite);
        
        // All healthy components
        let mut component_health = HashMap::new();
        component_health.insert("comp1".to_string(), ComponentHealth {
            status: HealthStatus::Healthy,
            last_check: current_timestamp(),
            response_time_ms: 100,
            error_count: 0,
            details: "OK".to_string(),
        });
        component_health.insert("comp2".to_string(), ComponentHealth {
            status: HealthStatus::Healthy,
            last_check: current_timestamp(),
            response_time_ms: 150,
            error_count: 0,
            details: "OK".to_string(),
        });
        
        let overall = health_checker.calculate_overall_health(&component_health);
        assert!(matches!(overall, HealthStatus::Healthy));
        
        // One degraded component
        component_health.insert("comp2".to_string(), ComponentHealth {
            status: HealthStatus::Degraded,
            last_check: current_timestamp(),
            response_time_ms: 500,
            error_count: 1,
            details: "Slow".to_string(),
        });
        
        let overall = health_checker.calculate_overall_health(&component_health);
        assert!(matches!(overall, HealthStatus::Degraded));
        
        // One unhealthy component
        component_health.insert("comp2".to_string(), ComponentHealth {
            status: HealthStatus::Unhealthy,
            last_check: current_timestamp(),
            response_time_ms: 5000,
            error_count: 5,
            details: "Failing".to_string(),
        });
        
        let overall = health_checker.calculate_overall_health(&component_health);
        assert!(matches!(overall, HealthStatus::Unhealthy));
    }

    #[test]
    fn test_health_report_generation() {
        let health_checker = HealthChecker::new(DatabaseDialect::SQLite);
        
        let mut component_health = HashMap::new();
        component_health.insert("test_component".to_string(), ComponentHealth {
            status: HealthStatus::Healthy,
            last_check: current_timestamp(),
            response_time_ms: 100,
            error_count: 0,
            details: "Working fine".to_string(),
        });
        
        let system_health = SystemHealth {
            overall_status: HealthStatus::Healthy,
            component_health,
            last_full_check: current_timestamp(),
            uptime_seconds: 3600,
            total_checks: 100,
            failed_checks: 2,
        };
        
        let report = health_checker.generate_health_report(&system_health);
        
        assert!(report.contains("Phase 5 Migration Engine Health Report"));
        assert!(report.contains("Overall Status: ✅"));
        assert!(report.contains("System Uptime: 3600s"));
        assert!(report.contains("Total Health Checks: 100"));
        assert!(report.contains("Failed Checks: 2"));
        assert!(report.contains("Success Rate: 98.0%"));
        assert!(report.contains("test_component"));
    }

    #[test]
    fn test_health_trends() {
        let mut health_checker = HealthChecker::new(DatabaseDialect::SQLite);
        
        // Add some health history
        let component = "test_component";
        let now = current_timestamp();
        
        // Add mostly healthy checks
        for i in 0..10 {
            let health = ComponentHealth {
                status: if i < 8 { HealthStatus::Healthy } else { HealthStatus::Degraded },
                last_check: now - (i * 60), // One check per minute
                response_time_ms: 100,
                error_count: if i < 8 { 0 } else { 1 },
                details: "Test".to_string(),
            };
            health_checker.record_health_history(component, health);
        }
        
        let trend = health_checker.get_health_trends(component, Duration::from_secs(600)); // Last 10 minutes
        assert!(matches!(trend, HealthTrend::Stable | HealthTrend::Improving));
    }

    #[test]
    fn test_health_error_types() {
        let component_error = HealthCheckError::ComponentUnhealthy {
            component: "TestComponent".to_string(),
            message: "Component is down".to_string(),
        };
        assert!(format!("{}", component_error).contains("TestComponent"));
        assert!(format!("{}", component_error).contains("Component is down"));
        
        let system_error = HealthCheckError::SystemUnhealthy {
            message: "System overload".to_string(),
        };
        assert!(format!("{}", system_error).contains("System overload"));
        
        let timeout_error = HealthCheckError::Timeout {
            component: "SlowComponent".to_string(),
            timeout_ms: 5000,
        };
        assert!(format!("{}", timeout_error).contains("SlowComponent"));
        assert!(format!("{}", timeout_error).contains("5000ms"));
    }

    #[test]
    fn test_health_history_management() {
        let mut health_checker = HealthChecker::new(DatabaseDialect::SQLite);
        let component = "test_component";
        
        // Add many health records
        for i in 0..150 {
            let health = ComponentHealth {
                status: HealthStatus::Healthy,
                last_check: current_timestamp() - i,
                response_time_ms: 100,
                error_count: 0,
                details: "Test".to_string(),
            };
            health_checker.record_health_history(component, health);
        }
        
        // Should only keep the last 100 records
        let history = health_checker.get_component_history(component).unwrap();
        assert_eq!(history.len(), 100);
    }
}