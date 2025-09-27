/// Comprehensive Performance Benchmarking Suite
///
/// This module provides systematic performance testing and regression detection
/// for the d1-rs ORM across all supported database backends.

use d1_rs::backends::QueryResult;
use d1_rs::dialects::DatabaseDialect;
use serde_json::Value;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use sea_query::{Query, Expr, Order, Alias, SqliteQueryBuilder, IntoIden, Func};

#[cfg(feature = "postgres")]
use sea_query::PostgresQueryBuilder;

#[cfg(feature = "mysql")]
use sea_query::MysqlQueryBuilder;

mod common;
use common::multi_db::*;
use common::query_helpers::*;

/// Performance metrics collection and analysis
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub database: String,
    pub operation: String,
    pub duration: Duration,
    pub memory_usage: Option<u64>,
    pub throughput: Option<f64>,
    pub success: bool,
}

/// Performance benchmark thresholds for regression detection
#[derive(Debug, Clone)]
pub struct BenchmarkThresholds {
    pub max_query_time_ms: u64,
    pub max_insert_time_ms: u64,
    pub max_schema_time_ms: u64,
    pub max_migration_time_ms: u64,
    pub max_memory_usage_mb: u64,
    pub min_throughput_ops_sec: f64,
}

impl BenchmarkThresholds {
    pub fn default() -> Self {
        Self {
            max_query_time_ms: 100,      // 100ms for single query
            max_insert_time_ms: 50,      // 50ms for single insert
            max_schema_time_ms: 500,     // 500ms for schema operations
            max_migration_time_ms: 1000, // 1s for migration operations
            max_memory_usage_mb: 100,    // 100MB memory limit
            min_throughput_ops_sec: 100.0, // 100 ops/second minimum
        }
    }
    
    pub fn lenient() -> Self {
        Self {
            max_query_time_ms: 500,      // More lenient for CI
            max_insert_time_ms: 200,     
            max_schema_time_ms: 2000,    
            max_migration_time_ms: 5000, 
            max_memory_usage_mb: 500,    
            min_throughput_ops_sec: 50.0,
        }
    }
}

/// Performance benchmarking framework
pub struct PerformanceBenchmark {
    thresholds: BenchmarkThresholds,
    results: Vec<PerformanceMetrics>,
}

impl PerformanceBenchmark {
    pub fn new(thresholds: BenchmarkThresholds) -> Self {
        Self {
            thresholds,
            results: Vec::new(),
        }
    }
    
    pub fn record_metric(&mut self, metric: PerformanceMetrics) {
        self.results.push(metric);
    }
    
    pub fn validate_thresholds(&self) -> Vec<String> {
        let mut violations = Vec::new();
        
        for metric in &self.results {
            if !metric.success {
                violations.push(format!("Operation failed: {} on {}", metric.operation, metric.database));
                continue;
            }
            
            let duration_ms = metric.duration.as_millis() as u64;
            
            match metric.operation.as_str() {
                "single_query" => {
                    if duration_ms > self.thresholds.max_query_time_ms {
                        violations.push(format!(
                            "Query performance violation on {}: {}ms > {}ms threshold",
                            metric.database, duration_ms, self.thresholds.max_query_time_ms
                        ));
                    }
                },
                "single_insert" => {
                    if duration_ms > self.thresholds.max_insert_time_ms {
                        violations.push(format!(
                            "Insert performance violation on {}: {}ms > {}ms threshold",
                            metric.database, duration_ms, self.thresholds.max_insert_time_ms
                        ));
                    }
                },
                "schema_operation" => {
                    if duration_ms > self.thresholds.max_schema_time_ms {
                        violations.push(format!(
                            "Schema operation violation on {}: {}ms > {}ms threshold",
                            metric.database, duration_ms, self.thresholds.max_schema_time_ms
                        ));
                    }
                },
                "migration_operation" => {
                    if duration_ms > self.thresholds.max_migration_time_ms {
                        violations.push(format!(
                            "Migration operation violation on {}: {}ms > {}ms threshold",
                            metric.database, duration_ms, self.thresholds.max_migration_time_ms
                        ));
                    }
                },
                _ => {}
            }
            
            if let Some(memory_mb) = metric.memory_usage {
                if memory_mb > self.thresholds.max_memory_usage_mb {
                    violations.push(format!(
                        "Memory usage violation on {}: {}MB > {}MB threshold",
                        metric.database, memory_mb, self.thresholds.max_memory_usage_mb
                    ));
                }
            }
            
            if let Some(throughput) = metric.throughput {
                if throughput < self.thresholds.min_throughput_ops_sec {
                    violations.push(format!(
                        "Throughput violation on {}: {:.2} ops/sec < {:.2} ops/sec threshold",
                        metric.database, throughput, self.thresholds.min_throughput_ops_sec
                    ));
                }
            }
        }
        
        violations
    }
    
    pub fn generate_report(&self) -> String {
        let mut report = String::from("=== Performance Benchmark Report ===\n\n");
        
        // Group results by database
        let mut db_results: HashMap<String, Vec<&PerformanceMetrics>> = HashMap::new();
        for metric in &self.results {
            db_results.entry(metric.database.clone()).or_default().push(metric);
        }
        
        for (db_name, metrics) in db_results {
            report.push_str(&format!("## {} Performance\n", db_name));
            
            for metric in metrics {
                let status = if metric.success { "✅" } else { "❌" };
                let duration_ms = metric.duration.as_millis();
                
                report.push_str(&format!(
                    "{} {}: {}ms",
                    status, metric.operation, duration_ms
                ));
                
                if let Some(throughput) = metric.throughput {
                    report.push_str(&format!(" ({:.1} ops/sec)", throughput));
                }
                
                if let Some(memory) = metric.memory_usage {
                    report.push_str(&format!(" [{}MB]", memory));
                }
                
                report.push('\n');
            }
            report.push('\n');
        }
        
        let violations = self.validate_thresholds();
        if !violations.is_empty() {
            report.push_str("## ⚠️  Performance Violations\n");
            for violation in violations {
                report.push_str(&format!("- {}\n", violation));
            }
        } else {
            report.push_str("## ✅ All Performance Thresholds Met\n");
        }
        
        report
    }
}

/// Single query performance benchmarking
#[tokio::test]
async fn test_single_query_performance() {
    let databases = TestDatabase::available();
    let mut benchmark = PerformanceBenchmark::new(BenchmarkThresholds::lenient());
    
    for database in databases {
        let client = TestUtils::setup_test_client_with_data(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        // Benchmark simple SELECT query using sea-query helper
        let start = Instant::now();
        let (count_sql, count_params) = build_count_query(
            Alias::new("test_users").into_iden(), 
            database.dialect()
        );
        let result = client.query(&count_sql, &count_params).await;
        let duration = start.elapsed();
        
        let success = result.is_ok();
        if let Ok(query_result) = result {
            let rows = query_result.into_rows();
            assert!(!rows.is_empty(), "Query should return results");
        }
        
        benchmark.record_metric(PerformanceMetrics {
            database: database.name().to_string(),
            operation: "single_query".to_string(),
            duration,
            memory_usage: None,
            throughput: None,
            success,
        });
        
        println!("🔍 {} single query: {:?}", database.name(), duration);
    }
    
    // Validate performance requirements
    let violations = benchmark.validate_thresholds();
    if !violations.is_empty() {
        println!("{}", benchmark.generate_report());
        panic!("Performance violations detected: {:?}", violations);
    }
    
    println!("✅ Single query performance benchmarks passed");
}

/// Bulk insert performance benchmarking
#[tokio::test]
async fn test_bulk_insert_performance() {
    let databases = TestDatabase::available();
    let mut benchmark = PerformanceBenchmark::new(BenchmarkThresholds::lenient());
    
    for database in databases {
        let client = TestUtils::setup_test_client(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        let seeder = TestDataSeeder::new(client);
        
        // Benchmark bulk insert operations
        let insert_count = 100;
        let start = Instant::now();
        
        let mut success = true;
        for i in 0..insert_count {
            let params = vec![
                Value::String(format!("bulk_user_{}@example.com", i)),
                Value::String(format!("Bulk User {}", i)),
                seeder.bool_value(true),
                Value::Number(i.into()),
            ];
            
            if seeder.client.execute(&seeder.get_insert_user_sql(), &params).await.is_err() {
                success = false;
                break;
            }
        }
        
        let total_duration = start.elapsed();
        let avg_duration = total_duration / insert_count;
        let throughput = insert_count as f64 / total_duration.as_secs_f64();
        
        benchmark.record_metric(PerformanceMetrics {
            database: database.name().to_string(),
            operation: "single_insert".to_string(),
            duration: avg_duration,
            memory_usage: None,
            throughput: Some(throughput),
            success,
        });
        
        println!("📊 {} bulk insert ({} records): total {:?}, avg {:?}, {:.1} ops/sec", 
            database.name(), insert_count, total_duration, avg_duration, throughput);
    }
    
    // Validate performance requirements  
    let violations = benchmark.validate_thresholds();
    if !violations.is_empty() {
        println!("{}", benchmark.generate_report());
        panic!("Performance violations detected: {:?}", violations);
    }
    
    println!("✅ Bulk insert performance benchmarks passed");
}

/// Complex query performance benchmarking
#[tokio::test]
async fn test_complex_query_performance() {
    let databases = TestDatabase::available();
    let mut benchmark = PerformanceBenchmark::new(BenchmarkThresholds::lenient());
    
    for database in databases {
        let client = TestUtils::setup_test_client_with_data(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        let seeder = TestDataSeeder::new(client);
        
        // Benchmark complex JOIN query with aggregations using helper
        let start = Instant::now();
        let (complex_sql, complex_params) = build_complex_join_query_with_aggregations(database.dialect());
        let result = seeder.client.query(&complex_sql, &complex_params).await;
        let duration = start.elapsed();
        
        let success = result.is_ok();
        if let Ok(query_result) = result {
            let rows = query_result.into_rows();
            assert!(!rows.is_empty(), "Complex query should return results");
        }
        
        benchmark.record_metric(PerformanceMetrics {
            database: database.name().to_string(),
            operation: "single_query".to_string(),
            duration,
            memory_usage: None,
            throughput: None,
            success,
        });
        
        println!("🔗 {} complex query: {:?}", database.name(), duration);
    }
    
    // Validate performance requirements
    let violations = benchmark.validate_thresholds();
    if !violations.is_empty() {
        println!("{}", benchmark.generate_report());
        panic!("Performance violations detected: {:?}", violations);
    }
    
    println!("✅ Complex query performance benchmarks passed");
}

/// Schema operation performance benchmarking
#[tokio::test]
async fn test_schema_operation_performance() {
    let databases = TestDatabase::available();
    let mut benchmark = PerformanceBenchmark::new(BenchmarkThresholds::lenient());
    
    for database in databases {
        let client = TestUtils::setup_test_client(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        // Benchmark CREATE TABLE operation using sea-query helper
        let (create_sql, create_params) = build_create_table_query_with_columns(
            "perf_test",
            vec![
                ("id", "INTEGER", true, Some("AUTOINCREMENT"), false),
                ("name", "TEXT", false, None, false),
                ("score", "INTEGER", false, None, true),
                ("active", "INTEGER", false, Some("1"), true)
            ],
            client.dialect()
        );
        
        let start = Instant::now();
        let result = client.execute(&create_sql, &create_params).await;
        let create_duration = start.elapsed();
        let create_success = result.is_ok();
        
        // Benchmark CREATE INDEX operation using sea-query helper  
        let index_start = Instant::now();
        let (index_sql, index_params) = build_create_index_query(
            "idx_perf_name",
            "perf_test", 
            vec!["name"],
            false,
            client.dialect()
        );
        let index_result = client.execute(&index_sql, &index_params).await;
        let index_duration = index_start.elapsed();
        let index_success = index_result.is_ok();
        
        // Benchmark DROP operations using sea-query helper
        let drop_start = Instant::now();
        let (drop_table_sql, drop_table_params) = build_drop_table_query("perf_test", true, client.dialect());
        let _ = client.execute(&drop_table_sql, &drop_table_params).await;
        let drop_duration = drop_start.elapsed();
        
        let total_duration = create_duration + index_duration + drop_duration;
        let overall_success = create_success && index_success;
        
        benchmark.record_metric(PerformanceMetrics {
            database: database.name().to_string(),
            operation: "schema_operation".to_string(),
            duration: total_duration,
            memory_usage: None,
            throughput: None,
            success: overall_success,
        });
        
        println!("🏗️  {} schema ops: CREATE {:?}, INDEX {:?}, DROP {:?}", 
            database.name(), create_duration, index_duration, drop_duration);
    }
    
    // Validate performance requirements
    let violations = benchmark.validate_thresholds();
    if !violations.is_empty() {
        println!("{}", benchmark.generate_report());
        panic!("Performance violations detected: {:?}", violations);
    }
    
    println!("✅ Schema operation performance benchmarks passed");
}

/// Memory usage benchmarking with large datasets
#[tokio::test]
async fn test_memory_usage_benchmarks() {
    let databases = TestDatabase::available();
    let mut benchmark = PerformanceBenchmark::new(BenchmarkThresholds::lenient());
    
    for database in databases {
        let client = TestUtils::setup_test_client(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        let seeder = TestDataSeeder::new(client);
        
        // Insert substantial dataset
        let record_count = 1000;
        for i in 0..record_count {
            let params = vec![
                Value::String(format!("memory_test_{}@example.com", i)),
                Value::String(format!("Memory Test User {}", i)),
                seeder.bool_value(i % 2 == 0),
                Value::Number(i.into()),
            ];
            
            let _ = seeder.client.execute(&seeder.get_insert_user_sql(), &params).await;
        }
        
        // Benchmark large result set query using sea-query helper
        let start = Instant::now();
        let (select_sql, select_params) = build_select_query_with_order(
            Alias::new("test_users").into_iden(),
            vec![], // Empty means SELECT *
            vec![(Alias::new("id").into_iden(), Order::Asc)],
            database.dialect()
        );
        let result = seeder.client.query(&select_sql, &select_params).await;
        let duration = start.elapsed();
        
        let success = result.is_ok();
        let row_count = if let Ok(query_result) = result {
            let rows = query_result.into_rows();
            rows.len()
        } else {
            0
        };
        
        // Estimate memory usage (rough approximation)
        let estimated_memory_mb = (row_count * 200) / (1024 * 1024); // ~200 bytes per row estimate
        
        benchmark.record_metric(PerformanceMetrics {
            database: database.name().to_string(),
            operation: "memory_query".to_string(),
            duration,
            memory_usage: Some(estimated_memory_mb as u64),
            throughput: None,
            success,
        });
        
        println!("💾 {} memory test: {} rows in {:?}, ~{}MB estimated", 
            database.name(), row_count, duration, estimated_memory_mb);
    }
    
    println!("✅ Memory usage benchmarks completed");
}

/// Throughput benchmarking with concurrent operations
#[tokio::test] 
async fn test_throughput_benchmarks() {
    let databases = TestDatabase::available();
    let mut benchmark = PerformanceBenchmark::new(BenchmarkThresholds::lenient());
    
    for database in databases {
        let client = TestUtils::setup_test_client_with_data(database.clone()).await
            .expect(&format!("Failed to setup {} client", database.name()));
        
        // Benchmark read throughput
        let read_operations = 200;
        let start = Instant::now();
        
        let mut success_count = 0;
        for i in 0..read_operations {
            let (query_sql, query_params) = if i % 2 == 0 {
                build_count_query(Alias::new("test_users").into_iden(), database.dialect())
            } else {
                build_count_query(Alias::new("test_posts").into_iden(), database.dialect())
            };
            
            if client.query(&query_sql, &query_params).await.is_ok() {
                success_count += 1;
            }
        }
        
        let duration = start.elapsed();
        let throughput = success_count as f64 / duration.as_secs_f64();
        let success = success_count == read_operations;
        
        benchmark.record_metric(PerformanceMetrics {
            database: database.name().to_string(),
            operation: "read_throughput".to_string(),
            duration,
            memory_usage: None,
            throughput: Some(throughput),
            success,
        });
        
        println!("⚡ {} read throughput: {}/{} ops in {:?}, {:.1} ops/sec", 
            database.name(), success_count, read_operations, duration, throughput);
    }
    
    // Validate performance requirements
    let violations = benchmark.validate_thresholds();
    if !violations.is_empty() {
        println!("{}", benchmark.generate_report());
        panic!("Performance violations detected: {:?}", violations);
    }
    
    println!("✅ Throughput benchmarks passed");
}

/// Cross-database performance comparison analysis
#[tokio::test]
async fn test_cross_database_performance_comparison() {
    // For now, test with SQLite only since it's the available database in test environment
    // Future: expand to include PostgreSQL and MySQL when available in CI
    let client = TestUtils::setup_test_client_with_data(TestDatabase::SQLite).await
        .expect("Failed to setup SQLite test client");
    
    let mut performance_data: HashMap<String, Vec<Duration>> = HashMap::new();
    let mut timings = Vec::new();
    
    // Run multiple identical operations to get performance variance data using sea-query helpers
    let operations = vec![
        ("COUNT users", {
            let (sql, params) = build_count_query(Alias::new("test_users").into_iden(), DatabaseDialect::SQLite);
            (sql, params)
        }),
        ("SELECT users", {
            let (sql, params) = Query::select()
                .columns([Alias::new("id"), Alias::new("name"), Alias::new("email")])
                .from(Alias::new("test_users"))
                .limit(10)
                .build(SqliteQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        }),
        ("SELECT with WHERE", {
            let (sql, params) = build_select_query_with_where(
                Alias::new("test_users").into_iden(),
                vec![], // SELECT *
                vec![Expr::col(Alias::new("id")).lt(100)],
                DatabaseDialect::SQLite
            );
            (sql, params)
        }),
        ("Complex query", {
            let (sql, params) = Query::select()
                .expr_as(Func::count(Expr::col(Alias::new("id"))), Alias::new("user_count"))
                .from(Alias::new("test_users"))
                .and_where(Expr::col(Alias::new("created_at")).gt("2020-01-01"))
                .build(SqliteQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        }),
    ];
    
    for (op_name, (operation_sql, operation_params)) in &operations {
        let start = Instant::now();
        let result = client.execute(operation_sql, &operation_params).await;
        let duration = start.elapsed();
        
        // Verify operation succeeded
        assert!(result.is_ok(), "Operation '{}' failed: {:?}", op_name, result.err());
        timings.push(duration);
    }
    
    performance_data.insert("SQLite".to_string(), timings);
    
    // Generate comparison analysis
    println!("\n📊 Cross-Database Performance Comparison:");
    println!("{:<12} {:<10} {:<10} {:<10} {:<10}", "Database", "Avg (ms)", "Min (ms)", "Max (ms)", "Std Dev");
    println!("{}", "-".repeat(60));
    
    for (db_name, timings) in &performance_data {
        let durations_ms: Vec<f64> = timings.iter().map(|d| d.as_millis() as f64).collect();
        let avg = durations_ms.iter().sum::<f64>() / durations_ms.len() as f64;
        let min = durations_ms.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = durations_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        let variance = durations_ms.iter().map(|x| (x - avg).powi(2)).sum::<f64>() / durations_ms.len() as f64;
        let std_dev = variance.sqrt();
        
        println!("{:<12} {:<10.1} {:<10.1} {:<10.1} {:<10.1}", 
            db_name, avg, min, max, std_dev);
        
        // Validate performance is reasonable (operations should complete quickly)
        assert!(avg < 100.0, "Average operation time too high: {:.1}ms", avg);
        assert!(max < 500.0, "Maximum operation time too high: {:.1}ms", max);
    }
    
    println!("✅ Cross-database performance comparison completed (SQLite baseline established)");
}