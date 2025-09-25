/// Multi-database test utilities for running tests across SQLite, PostgreSQL, and MySQL
/// 
/// This module provides database-agnostic test infrastructure to ensure the ORM
/// works consistently across all supported database backends.

use d1_rs::db::SQLiteClient;
use d1_rs::backends::QueryResult;
use d1_rs::dialects::DatabaseDialect;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum TestDatabase {
    SQLite,
    #[cfg(feature = "postgres")]
    PostgreSQL,
    #[cfg(feature = "mysql")]
    MySQL,
}

impl TestDatabase {
    pub fn dialect(&self) -> DatabaseDialect {
        match self {
            TestDatabase::SQLite => DatabaseDialect::SQLite,
            #[cfg(feature = "postgres")]
            TestDatabase::PostgreSQL => DatabaseDialect::PostgreSQL,
            #[cfg(feature = "mysql")]
            TestDatabase::MySQL => DatabaseDialect::MySQL,
        }
    }

    pub fn connection_url(&self) -> String {
        match self {
            TestDatabase::SQLite => "sqlite::memory:".to_string(),
            #[cfg(feature = "postgres")]
            TestDatabase::PostgreSQL => {
                std::env::var("POSTGRES_TEST_URL")
                    .unwrap_or_else(|_| "postgresql://d1rs_user:d1rs_pass@localhost:5434/d1rs_test".to_string())
            },
            #[cfg(feature = "mysql")]
            TestDatabase::MySQL => {
                std::env::var("MYSQL_TEST_URL")
                    .unwrap_or_else(|_| "mysql://d1rs_user:d1rs_pass@localhost:3308/d1rs_test".to_string())
            },
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            TestDatabase::SQLite => "SQLite",
            #[cfg(feature = "postgres")]
            TestDatabase::PostgreSQL => "PostgreSQL", 
            #[cfg(feature = "mysql")]
            TestDatabase::MySQL => "MySQL",
        }
    }

    pub fn available() -> Vec<TestDatabase> {
        vec![TestDatabase::SQLite]
    }
}

/// Database client wrapper that provides unified interface across all backends
#[derive(Clone)]
pub struct TestClient {
    database: TestDatabase,
    client: SQLiteClient,
}

impl TestClient {
    pub async fn new(database: TestDatabase) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let client = match database {
            TestDatabase::SQLite => {
                SQLiteClient::new_in_memory().await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
            },
            #[cfg(feature = "postgres")]
            TestDatabase::PostgreSQL => {
                return Err("PostgreSQL support not yet implemented".into());
            },
            #[cfg(feature = "mysql")]
            TestDatabase::MySQL => {
                return Err("MySQL support not yet implemented".into());
            },
        };

        Ok(TestClient {
            database,
            client,
        })
    }

    pub fn database(&self) -> &TestDatabase {
        &self.database
    }

    pub fn dialect(&self) -> DatabaseDialect {
        self.database.dialect()
    }

    pub async fn execute(&self, sql: &str, params: &[Value]) -> std::result::Result<impl QueryResult, Box<dyn std::error::Error>> {
        self.client.execute(sql, params).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    pub async fn query(&self, sql: &str, params: &[Value]) -> std::result::Result<impl QueryResult, Box<dyn std::error::Error>> {
        self.client.execute(sql, params).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

}

/// Database-agnostic DDL generator for test schemas
pub struct TestSchemaBuilder {
    dialect: DatabaseDialect,
}

impl TestSchemaBuilder {
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self { dialect }
    }

    pub fn create_users_table(&self) -> String {
        match self.dialect {
            DatabaseDialect::SQLite => {
                r#"
                CREATE TABLE test_users (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    email TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    is_active INTEGER NOT NULL DEFAULT 1,
                    score INTEGER,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )
                "#.trim().to_string()
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                r#"
                CREATE TABLE test_users (
                    id BIGSERIAL PRIMARY KEY,
                    email VARCHAR(255) NOT NULL UNIQUE,
                    name VARCHAR(255) NOT NULL,
                    is_active BOOLEAN NOT NULL DEFAULT TRUE,
                    score INTEGER,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
                )
                "#.trim().to_string()
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                r#"
                CREATE TABLE test_users (
                    id BIGINT AUTO_INCREMENT PRIMARY KEY,
                    email VARCHAR(255) NOT NULL UNIQUE,
                    name VARCHAR(255) NOT NULL,
                    is_active BOOLEAN NOT NULL DEFAULT TRUE,
                    score INT,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )
                "#.trim().to_string()
            },
        }
    }

    pub fn create_posts_table(&self) -> String {
        match self.dialect {
            DatabaseDialect::SQLite => {
                r#"
                CREATE TABLE test_posts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    content TEXT NOT NULL,
                    is_published INTEGER NOT NULL DEFAULT 0,
                    views INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (user_id) REFERENCES test_users(id)
                )
                "#.trim().to_string()
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                r#"
                CREATE TABLE test_posts (
                    id BIGSERIAL PRIMARY KEY,
                    user_id BIGINT NOT NULL REFERENCES test_users(id),
                    title VARCHAR(255) NOT NULL,
                    content TEXT NOT NULL,
                    is_published BOOLEAN NOT NULL DEFAULT FALSE,
                    views INTEGER NOT NULL DEFAULT 0,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
                )
                "#.trim().to_string()
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                r#"
                CREATE TABLE test_posts (
                    id BIGINT AUTO_INCREMENT PRIMARY KEY,
                    user_id BIGINT NOT NULL,
                    title VARCHAR(255) NOT NULL,
                    content TEXT NOT NULL,
                    is_published BOOLEAN NOT NULL DEFAULT FALSE,
                    views INT NOT NULL DEFAULT 0,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (user_id) REFERENCES test_users(id)
                )
                "#.trim().to_string()
            },
        }
    }

    pub fn drop_tables(&self) -> Vec<String> {
        vec![
            "DROP TABLE IF EXISTS test_posts".to_string(),
            "DROP TABLE IF EXISTS test_users".to_string(),
        ]
    }
}

/// Test data seeder that works across all database backends
pub struct TestDataSeeder {
    pub client: TestClient,
}

impl TestDataSeeder {
    pub fn new(client: TestClient) -> Self {
        Self { client }
    }

    pub async fn seed_test_users(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let insert_sql = self.get_insert_user_sql();
        
        // Alice - active user with score
        self.client.execute(&insert_sql, &[
            Value::String("alice@example.com".to_string()),
            Value::String("Alice".to_string()),
            self.bool_value(true),
            Value::Number(100.into()),
        ]).await?;

        // Bob - inactive user with score
        self.client.execute(&insert_sql, &[
            Value::String("bob@example.com".to_string()),
            Value::String("Bob".to_string()),
            self.bool_value(false),
            Value::Number(50.into()),
        ]).await?;

        // Charlie - active user without score
        self.client.execute(&insert_sql, &[
            Value::String("charlie@example.com".to_string()),
            Value::String("Charlie".to_string()),
            self.bool_value(true),
            Value::Null,
        ]).await?;

        Ok(())
    }

    pub async fn seed_test_posts(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let insert_sql = self.get_insert_post_sql();

        // Alice's published post
        self.client.execute(&insert_sql, &[
            Value::Number(1.into()),
            Value::String("First Post".to_string()),
            Value::String("Content of first post".to_string()),
            self.bool_value(true),
            Value::Number(100.into()),
        ]).await?;

        // Alice's draft post
        self.client.execute(&insert_sql, &[
            Value::Number(1.into()),
            Value::String("Second Post".to_string()),
            Value::String("Content of second post".to_string()),
            self.bool_value(false),
            Value::Number(0.into()),
        ]).await?;

        // Bob's published post
        self.client.execute(&insert_sql, &[
            Value::Number(2.into()),
            Value::String("Bob's Post".to_string()),
            Value::String("Content of Bob's post".to_string()),
            self.bool_value(true),
            Value::Number(25.into()),
        ]).await?;

        Ok(())
    }

    pub async fn seed_all_data(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        self.seed_test_users().await?;
        self.seed_test_posts().await?;
        Ok(())
    }

    pub fn get_insert_user_sql(&self) -> String {
        "INSERT INTO test_users (email, name, is_active, score) VALUES (?, ?, ?, ?)".to_string()
    }

    pub fn get_insert_post_sql(&self) -> String {
        "INSERT INTO test_posts (user_id, title, content, is_published, views) VALUES (?, ?, ?, ?, ?)".to_string()
    }

    pub fn bool_value(&self, value: bool) -> Value {
        match self.client.dialect() {
            DatabaseDialect::SQLite => Value::Number(if value { 1 } else { 0 }.into()),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => Value::Bool(value),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => Value::Bool(value),
        }
    }
}

/// Multi-database test runner that executes tests across all available backends
pub struct MultiDatabaseTestRunner {
    databases: Vec<TestDatabase>,
}

impl MultiDatabaseTestRunner {
    pub fn new() -> Self {
        Self {
            databases: vec![TestDatabase::SQLite],
        }
    }

    pub fn with_databases(databases: Vec<TestDatabase>) -> Self {
        Self { databases }
    }

    pub async fn run_test<F, Fut>(&self, test_name: &str, test_fn: F) -> std::result::Result<(), Box<dyn std::error::Error>>
    where
        F: Fn(TestClient) -> Fut + Clone,
        Fut: std::future::Future<Output = std::result::Result<(), Box<dyn std::error::Error>>>,
    {
        for database in &self.databases {
            println!("Running test '{}' on {}", test_name, database.name());
            
            let client = TestClient::new(database.clone()).await?;
            
            // Setup test schema
            let schema_builder = TestSchemaBuilder::new(client.dialect());
            client.execute(&schema_builder.create_users_table(), &[]).await?;
            client.execute(&schema_builder.create_posts_table(), &[]).await?;
            
            // Run the test
            match test_fn.clone()(client).await {
                Ok(()) => println!("✅ {} - {}", database.name(), test_name),
                Err(e) => {
                    eprintln!("❌ {} - {}: {}", database.name(), test_name, e);
                    return Err(e);
                }
            }
        }
        
        Ok(())
    }

}

/// Test utilities for common operations across databases
pub struct TestUtils;

impl TestUtils {
    pub async fn setup_test_client(database: TestDatabase) -> std::result::Result<TestClient, Box<dyn std::error::Error>> {
        let client = TestClient::new(database.clone()).await?;
        
        // Create test schema
        let schema_builder = TestSchemaBuilder::new(client.dialect());
        client.execute(&schema_builder.create_users_table(), &[]).await?;
        client.execute(&schema_builder.create_posts_table(), &[]).await?;
        
        Ok(client)
    }

    pub async fn setup_test_client_with_data(database: TestDatabase) -> std::result::Result<TestClient, Box<dyn std::error::Error>> {
        let client = Self::setup_test_client(database).await?;
        
        // Seed test data
        let seeder = TestDataSeeder::new(client);
        seeder.seed_all_data().await?;
        
        Ok(seeder.client)
    }

    pub async fn cleanup_test_client(client: &TestClient) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema_builder = TestSchemaBuilder::new(client.dialect());
        for drop_sql in schema_builder.drop_tables() {
            let _ = client.execute(&drop_sql, &[]).await; // Ignore errors for cleanup
        }
        Ok(())
    }

    pub async fn count_records(client: &TestClient, table: &str) -> std::result::Result<i64, Box<dyn std::error::Error>> {
        let sql = format!("SELECT COUNT(*) as count FROM {}", table);
        let result = client.query(&sql, &[]).await?;
        
        if let Some(row) = result.into_rows().into_iter().next() {
            if let Some(count_value) = row.get("count") {
                match count_value {
                    Value::Number(n) => return Ok(n.as_i64().unwrap_or(0)),
                    _ => return Ok(0),
                }
            }
        }
        
        Ok(0)
    }

    pub async fn verify_table_exists(client: &TestClient, table: &str) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        let sql = match client.dialect() {
            DatabaseDialect::SQLite => {
                "SELECT name FROM sqlite_master WHERE type='table' AND name=?"
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name = $1"
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?"
            },
        };
        
        let result = client.query(sql, &[Value::String(table.to_string())]).await?;
        Ok(!result.into_rows().is_empty())
    }
}

/// Macro for running a test across all available databases
#[macro_export]
macro_rules! multi_db_test {
    ($test_name:ident, $test_fn:expr) => {
        #[tokio::test]
        async fn $test_name() {
            use crate::common::multi_db::*;
            
            let runner = MultiDatabaseTestRunner::new();
            runner.run_test(stringify!($test_name), $test_fn)
                .await
                .expect("Multi-database test failed");
        }
    };
}

/// Macro for setting up a test client with data
#[macro_export]
macro_rules! setup_test_db_with_data {
    ($database:expr) => {
        TestUtils::setup_test_client_with_data($database).await.expect("Failed to setup test database")
    };
}

/// Macro for setting up a test client without data
#[macro_export]
macro_rules! setup_test_db {
    ($database:expr) => {
        TestUtils::setup_test_client($database).await.expect("Failed to setup test database")
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_connection_urls() {
        let sqlite = TestDatabase::SQLite;
        assert_eq!(sqlite.connection_url(), "sqlite::memory:");
        assert_eq!(sqlite.name(), "SQLite");
        assert_eq!(sqlite.dialect(), DatabaseDialect::SQLite);
    }

    #[tokio::test]
    async fn test_schema_builder_sqlite() {
        let builder = TestSchemaBuilder::new(DatabaseDialect::SQLite);
        let users_ddl = builder.create_users_table();
        assert!(users_ddl.contains("INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(users_ddl.contains("test_users"));
        
        let posts_ddl = builder.create_posts_table();
        assert!(posts_ddl.contains("test_posts"));
        assert!(posts_ddl.contains("FOREIGN KEY"));
    }

    #[tokio::test]
    async fn test_test_client_creation() {
        let client = TestClient::new(TestDatabase::SQLite).await.unwrap();
        assert_eq!(client.database(), &TestDatabase::SQLite);
        assert_eq!(client.dialect(), DatabaseDialect::SQLite);
    }

    #[tokio::test]
    async fn test_test_utils_setup() {
        let client = TestUtils::setup_test_client(TestDatabase::SQLite).await.unwrap();
        
        // Verify tables were created
        assert!(TestUtils::verify_table_exists(&client, "test_users").await.unwrap());
        assert!(TestUtils::verify_table_exists(&client, "test_posts").await.unwrap());
        
        // Verify no data initially
        assert_eq!(TestUtils::count_records(&client, "test_users").await.unwrap(), 0);
        assert_eq!(TestUtils::count_records(&client, "test_posts").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_test_utils_setup_with_data() {
        let client = TestUtils::setup_test_client_with_data(TestDatabase::SQLite).await.unwrap();
        
        // Verify data was seeded
        assert_eq!(TestUtils::count_records(&client, "test_users").await.unwrap(), 3);
        assert_eq!(TestUtils::count_records(&client, "test_posts").await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_data_seeder() {
        let client = TestClient::new(TestDatabase::SQLite).await.unwrap();
        
        // Setup schema
        let schema_builder = TestSchemaBuilder::new(client.dialect());
        client.execute(&schema_builder.create_users_table(), &[]).await.unwrap();
        client.execute(&schema_builder.create_posts_table(), &[]).await.unwrap();
        
        // Seed data
        let seeder = TestDataSeeder::new(client);
        seeder.seed_test_users().await.unwrap();
        seeder.seed_test_posts().await.unwrap();
        
        // Verify data
        assert_eq!(TestUtils::count_records(&seeder.client, "test_users").await.unwrap(), 3);
        assert_eq!(TestUtils::count_records(&seeder.client, "test_posts").await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_multi_database_test_runner() {
        let runner = MultiDatabaseTestRunner::with_databases(vec![TestDatabase::SQLite]);
        
        runner.run_test("test_basic_functionality", |client| async move {
            // Test that we can insert and query data
            let seeder = TestDataSeeder::new(client);
            seeder.seed_test_users().await?;
            
            let count = TestUtils::count_records(&seeder.client, "test_users").await?;
            assert_eq!(count, 3);
            
            Ok(())
        }).await.unwrap();
    }

    #[tokio::test] 
    async fn test_cleanup_test_client() {
        let client = TestUtils::setup_test_client_with_data(TestDatabase::SQLite).await.unwrap();
        
        // Verify data exists
        assert_eq!(TestUtils::count_records(&client, "test_users").await.unwrap(), 3);
        
        // Cleanup
        TestUtils::cleanup_test_client(&client).await.unwrap();
        
        // Verify tables are dropped (this should fail)
        assert!(!TestUtils::verify_table_exists(&client, "test_users").await.unwrap());
    }
}