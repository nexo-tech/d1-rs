use criterion::{black_box, criterion_group, criterion_main, Criterion};
use d1_rs::backends::sqlite::SQLiteClient;
use d1_rs::backends::DatabaseBackend;
use std::time::Duration;
use tokio::runtime::Runtime;

// Simple benchmark for basic database operations
async fn setup_test_db() -> SQLiteClient {
    let client = SQLiteClient::new_in_memory().await.unwrap();
    
    // Create test table
    client.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)", &[]).await.unwrap();
    
    // Insert test data
    for i in 1..=1000 {
        let query = "INSERT INTO users (name, email) VALUES (?, ?)";
        let params = [
            serde_json::Value::String(format!("User {}", i)),
            serde_json::Value::String(format!("user{}@example.com", i)),
        ];
        client.execute(query, &params).await.unwrap();
    }
    
    client
}

fn benchmark_select_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_test_db());
    
    c.bench_function("select_all_users", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = client.execute("SELECT * FROM users", &[]).await.unwrap();
                black_box(result);
            })
        })
    });
}

fn benchmark_select_with_where(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_test_db());
    
    c.bench_function("select_users_with_where", |b| {
        b.iter(|| {
            rt.block_on(async {
                let params = [serde_json::Value::Number(500.into())];
                let result = client.execute("SELECT * FROM users WHERE id < ?", &params).await.unwrap();
                black_box(result);
            })
        })
    });
}

fn benchmark_count_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_test_db());
    
    c.bench_function("count_users", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = client.execute("SELECT COUNT(*) FROM users", &[]).await.unwrap();
                black_box(result);
            })
        })
    });
}

fn benchmark_insert_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_test_db());
    
    let mut counter = 2000;
    c.bench_function("insert_user", |b| {
        b.iter(|| {
            counter += 1;
            rt.block_on(async {
                let params = [
                    serde_json::Value::String(format!("Benchmark User {}", counter)),
                    serde_json::Value::String(format!("bench{}@example.com", counter)),
                ];
                let result = client.execute("INSERT INTO users (name, email) VALUES (?, ?)", &params).await.unwrap();
                black_box(result);
            })
        })
    });
}

fn benchmark_update_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let client = rt.block_on(setup_test_db());
    
    c.bench_function("update_user", |b| {
        b.iter(|| {
            rt.block_on(async {
                let params = [
                    serde_json::Value::String("Updated User".to_string()),
                    serde_json::Value::Number(1.into()),
                ];
                let result = client.execute("UPDATE users SET name = ? WHERE id = ?", &params).await.unwrap();
                black_box(result);
            })
        })
    });
}

// Configure benchmark group with reasonable settings for CI
criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(50)  // Reduce sample size for faster CI execution
        .measurement_time(Duration::from_secs(10))  // Limit measurement time
        .warm_up_time(Duration::from_secs(2));  // Quick warm-up
    targets = 
        benchmark_select_query,
        benchmark_select_with_where, 
        benchmark_count_query,
        benchmark_insert_query,
        benchmark_update_query
}

criterion_main!(benches);