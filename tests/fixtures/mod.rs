/// Comprehensive Test Data Management System
///
/// This module provides a complete fixture management system for cross-database testing
/// using sea-query for database-agnostic SQL generation.

use d1_rs::backends::DatabaseBackend;
use d1_rs::dialects::DatabaseDialect;
use sea_query::{
    ColumnDef, ForeignKey, ForeignKeyAction,
    Index, Iden, Query, SqliteQueryBuilder, Table, IntoIden,
};
use d1_rs::backends::QueryResult;
use serde_json::{json, Value};
use std::collections::HashMap;

pub mod entities;
pub mod schemas;
pub mod data_sets;

/// Test fixture manager for multi-database testing with sea-query integration
pub struct TestFixtureManager {
    dialect: DatabaseDialect,
    fixtures: HashMap<String, TestFixture>,
}

/// Complete test fixture specification
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TestFixture {
    pub name: String,
    pub description: String,
    pub setup_queries: Vec<(String, Vec<Value>)>, // (SQL, params)
    pub teardown_queries: Vec<(String, Vec<Value>)>, // (SQL, params)
    pub test_data: Vec<TestRecord>,
    pub expected_results: HashMap<String, Value>,
}

/// Test data record for insertion
#[derive(Debug, Clone)]
pub struct TestRecord {
    pub table: String,
    pub data: HashMap<String, Value>,
}

/// Schema identifier enums for sea-query
#[derive(Iden)]
enum Users {
    Table,
    Id,
    Name,
    Email,
    CreatedAt,
}

#[derive(Iden)]
enum Posts {
    Table,
    Id,
    Title,
    Content,
    UserId,
    CreatedAt,
}

#[derive(Iden)]
#[allow(dead_code)]
enum TypeTest {
    Table,
    Id,
    TextCol,
    IntegerCol,
    RealCol,
    BlobCol,
    BooleanCol,
    DateCol,
    DatetimeCol,
    JsonCol,
    UuidCol,
}

#[derive(Iden)]
enum PerformanceTest {
    Table,
    Id,
    Data,
    Value,
    CreatedAt,
}

impl TestFixtureManager {
    /// Create new fixture manager for specified database dialect
    pub fn new(dialect: DatabaseDialect) -> Self {
        let mut manager = Self {
            dialect,
            fixtures: HashMap::new(),
        };
        
        manager.load_built_in_fixtures();
        manager
    }
    
    /// Setup a fixture in the database
    pub async fn setup_fixture<B: DatabaseBackend>(
        &self,
        fixture_name: &str,
        backend: &B,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fixture = self.fixtures.get(fixture_name)
            .ok_or_else(|| format!("Fixture '{}' not found", fixture_name))?;
            
        // Execute setup queries
        for (sql, params) in &fixture.setup_queries {
            backend.execute_query(sql, params).await?;
        }
        
        // Insert test data
        for record in &fixture.test_data {
            self.insert_test_record(record, backend).await?;
        }
        
        Ok(())
    }
    
    /// Teardown a fixture from the database
    pub async fn teardown_fixture<B: DatabaseBackend>(
        &self,
        fixture_name: &str, 
        backend: &B,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fixture = self.fixtures.get(fixture_name)
            .ok_or_else(|| format!("Fixture '{}' not found", fixture_name))?;
            
        // Execute teardown queries
        for (sql, params) in &fixture.teardown_queries {
            let _ = backend.execute_query(sql, params).await; // Ignore errors for cleanup
        }
        
        Ok(())
    }
    
    /// Load all built-in fixtures
    fn load_built_in_fixtures(&mut self) {
        self.load_users_posts_fixture();
        self.load_type_testing_fixture();
        self.load_performance_fixture();
    }
    
    /// Load users and posts fixture for relationship testing
    fn load_users_posts_fixture(&mut self) {
        let (setup_queries, teardown_queries) = self.get_users_posts_schema();
        
        self.fixtures.insert("users_posts".to_string(), TestFixture {
            name: "users_posts".to_string(),
            description: "Users with posts for relationship testing".to_string(),
            setup_queries,
            teardown_queries,
            test_data: vec![
                TestRecord {
                    table: "users".to_string(),
                    data: [
                        ("id".to_string(), json!(1)),
                        ("name".to_string(), json!("John Doe")),
                        ("email".to_string(), json!("john@example.com")),
                        ("created_at".to_string(), json!("2024-01-01 00:00:00")),
                    ].into_iter().collect(),
                },
                TestRecord {
                    table: "users".to_string(),
                    data: [
                        ("id".to_string(), json!(2)),
                        ("name".to_string(), json!("Jane Smith")),
                        ("email".to_string(), json!("jane@example.com")),
                        ("created_at".to_string(), json!("2024-01-02 00:00:00")),
                    ].into_iter().collect(),
                },
                TestRecord {
                    table: "posts".to_string(),
                    data: [
                        ("id".to_string(), json!(1)),
                        ("title".to_string(), json!("First Post")),
                        ("content".to_string(), json!("Hello World")),
                        ("user_id".to_string(), json!(1)),
                        ("created_at".to_string(), json!("2024-01-03 00:00:00")),
                    ].into_iter().collect(),
                },
            ],
            expected_results: [
                ("user_count".to_string(), json!(2)),
                ("post_count".to_string(), json!(1)),
            ].into_iter().collect(),
        });
    }
    
    /// Load type testing fixture for cross-database compatibility
    fn load_type_testing_fixture(&mut self) {
        let (setup_queries, teardown_queries) = self.get_type_testing_schema();
        
        self.fixtures.insert("type_testing".to_string(), TestFixture {
            name: "type_testing".to_string(),
            description: "All column types for cross-database compatibility".to_string(),
            setup_queries,
            teardown_queries,
            test_data: vec![
                TestRecord {
                    table: "type_test".to_string(),
                    data: self.get_type_test_data(),
                },
            ],
            expected_results: HashMap::new(),
        });
    }
    
    /// Load performance testing fixture with large dataset
    fn load_performance_fixture(&mut self) {
        let (setup_queries, teardown_queries) = self.get_performance_schema();
        
        self.fixtures.insert("performance_large".to_string(), TestFixture {
            name: "performance_large".to_string(),
            description: "Large dataset for performance testing".to_string(),
            setup_queries,
            teardown_queries,
            test_data: self.generate_performance_data(10000),
            expected_results: [
                ("record_count".to_string(), json!(10000)),
            ].into_iter().collect(),
        });
    }
    
    /// Generate users and posts schema using sea-query
    fn get_users_posts_schema(&self) -> (Vec<(String, Vec<Value>)>, Vec<(String, Vec<Value>)>) {
        let mut setup = Vec::new();
        let mut teardown = Vec::new();
        
        // Create users table
        let users_table = match self.dialect {
            DatabaseDialect::SQLite => {
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Users::Name).text().not_null())
                    .col(ColumnDef::new(Users::Email).text().not_null().unique_key())
                    .col(ColumnDef::new(Users::CreatedAt).date_time().not_null())
                    .to_owned()
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Users::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Users::Email).string_len(255).not_null().unique_key())
                    .col(ColumnDef::new(Users::CreatedAt).timestamp().not_null())
                    .to_owned()
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Users::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Users::Email).string_len(255).not_null().unique_key())
                    .col(ColumnDef::new(Users::CreatedAt).date_time().not_null())
                    .to_owned()
            },
        };
        
        // Create posts table with foreign key
        let posts_table = match self.dialect {
            DatabaseDialect::SQLite => {
                Table::create()
                    .table(Posts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Posts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Posts::Title).text().not_null())
                    .col(ColumnDef::new(Posts::Content).text())
                    .col(ColumnDef::new(Posts::UserId).integer().not_null())
                    .col(ColumnDef::new(Posts::CreatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posts_user_id")
                            .from(Posts::Table, Posts::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                Table::create()
                    .table(Posts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Posts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Posts::Title).string_len(255).not_null())
                    .col(ColumnDef::new(Posts::Content).text())
                    .col(ColumnDef::new(Posts::UserId).integer().not_null())
                    .col(ColumnDef::new(Posts::CreatedAt).timestamp().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posts_user_id")
                            .from(Posts::Table, Posts::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                Table::create()
                    .table(Posts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Posts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Posts::Title).string_len(255).not_null())
                    .col(ColumnDef::new(Posts::Content).text())
                    .col(ColumnDef::new(Posts::UserId).integer().not_null())
                    .col(ColumnDef::new(Posts::CreatedAt).date_time().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posts_user_id")
                            .from(Posts::Table, Posts::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            },
        };
        
        // Build setup queries
        let users_sql = users_table.build(SqliteQueryBuilder); // Works for all dialects
        let posts_sql = posts_table.build(SqliteQueryBuilder);
        let users_values = vec![];
        let posts_values = vec![];
        
        setup.push((users_sql, users_values));
        setup.push((posts_sql, posts_values));
        
        // Build teardown queries
        let drop_posts = Table::drop().table(Posts::Table).if_exists().to_owned();
        let drop_users = Table::drop().table(Users::Table).if_exists().to_owned();
        
        let drop_posts_sql = drop_posts.build(SqliteQueryBuilder);
        let drop_users_sql = drop_users.build(SqliteQueryBuilder);
        let drop_posts_values = vec![];
        let drop_users_values = vec![];
        
        teardown.push((drop_posts_sql, drop_posts_values));
        teardown.push((drop_users_sql, drop_users_values));
        
        (setup, teardown)
    }
    
    /// Generate type testing schema using sea-query
    fn get_type_testing_schema(&self) -> (Vec<(String, Vec<Value>)>, Vec<(String, Vec<Value>)>) {
        let mut setup = Vec::new();
        let mut teardown = Vec::new();
        
        let table = match self.dialect {
            DatabaseDialect::SQLite => {
                Table::create()
                    .table(TypeTest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TypeTest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TypeTest::TextCol).text())
                    .col(ColumnDef::new(TypeTest::IntegerCol).integer())
                    .col(ColumnDef::new(TypeTest::RealCol).float())
                    .col(ColumnDef::new(TypeTest::BlobCol).blob())
                    .col(ColumnDef::new(TypeTest::BooleanCol).boolean())
                    .col(ColumnDef::new(TypeTest::DateCol).date())
                    .col(ColumnDef::new(TypeTest::DatetimeCol).date_time())
                    .to_owned()
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let mut table_builder = Table::create()
                    .table(TypeTest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TypeTest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TypeTest::TextCol).text())
                    .col(ColumnDef::new(TypeTest::IntegerCol).integer())
                    .col(ColumnDef::new(TypeTest::RealCol).float())
                    .col(ColumnDef::new(TypeTest::BlobCol).binary())
                    .col(ColumnDef::new(TypeTest::BooleanCol).boolean())
                    .col(ColumnDef::new(TypeTest::DateCol).date())
                    .col(ColumnDef::new(TypeTest::DatetimeCol).timestamp())
                    .to_owned();
                    
                // Add PostgreSQL-specific columns
                table_builder.col(ColumnDef::new(TypeTest::JsonCol).json());
                table_builder.col(ColumnDef::new(TypeTest::UuidCol).uuid());
                table_builder
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                let mut table_builder = Table::create()
                    .table(TypeTest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TypeTest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TypeTest::TextCol).text())
                    .col(ColumnDef::new(TypeTest::IntegerCol).integer())
                    .col(ColumnDef::new(TypeTest::RealCol).double())
                    .col(ColumnDef::new(TypeTest::BlobCol).blob())
                    .col(ColumnDef::new(TypeTest::BooleanCol).boolean())
                    .col(ColumnDef::new(TypeTest::DateCol).date())
                    .col(ColumnDef::new(TypeTest::DatetimeCol).date_time())
                    .to_owned();
                    
                // Add MySQL-specific columns
                table_builder.col(ColumnDef::new(TypeTest::JsonCol).json());
                table_builder
            },
        };
        
        let sql = table.build(SqliteQueryBuilder);
        let values = vec![];
        setup.push((sql, values));
        
        // Teardown
        let drop_table = Table::drop().table(TypeTest::Table).if_exists().to_owned();
        let drop_sql = drop_table.build(SqliteQueryBuilder);
        let drop_values = vec![];
        teardown.push((drop_sql, drop_values));
        
        (setup, teardown)
    }
    
    /// Generate performance testing schema using sea-query
    fn get_performance_schema(&self) -> (Vec<(String, Vec<Value>)>, Vec<(String, Vec<Value>)>) {
        let mut setup = Vec::new();
        let mut teardown = Vec::new();
        
        let table = match self.dialect {
            DatabaseDialect::SQLite => {
                Table::create()
                    .table(PerformanceTest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PerformanceTest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PerformanceTest::Data).text().not_null())
                    .col(ColumnDef::new(PerformanceTest::Value).integer().not_null())
                    .col(ColumnDef::new(PerformanceTest::CreatedAt).date_time().default(sea_query::Expr::current_timestamp()))
                    .to_owned()
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                Table::create()
                    .table(PerformanceTest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PerformanceTest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PerformanceTest::Data).text().not_null())
                    .col(ColumnDef::new(PerformanceTest::Value).integer().not_null())
                    .col(ColumnDef::new(PerformanceTest::CreatedAt).timestamp().default(sea_query::Expr::current_timestamp()))
                    .to_owned()
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                Table::create()
                    .table(PerformanceTest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PerformanceTest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PerformanceTest::Data).text().not_null())
                    .col(ColumnDef::new(PerformanceTest::Value).integer().not_null())
                    .col(ColumnDef::new(PerformanceTest::CreatedAt).date_time().default(sea_query::Expr::current_timestamp()))
                    .to_owned()
            },
        };
        
        let table_sql = table.build(SqliteQueryBuilder);
        let table_values = vec![];
        setup.push((table_sql, table_values));
        
        // Create index for performance testing
        let index = Index::create()
            .name("idx_performance_value")
            .table(PerformanceTest::Table)
            .col(PerformanceTest::Value)
            .to_owned();
        
        let index_sql = index.build(SqliteQueryBuilder);
        let index_values = vec![];
        setup.push((index_sql, index_values));
        
        // Teardown
        let drop_table = Table::drop().table(PerformanceTest::Table).if_exists().to_owned();
        let drop_sql = drop_table.build(SqliteQueryBuilder);
        let drop_values = vec![];
        teardown.push((drop_sql, drop_values));
        
        (setup, teardown)
    }
    
    /// Generate test data for type testing
    fn get_type_test_data(&self) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        data.insert("id".to_string(), json!(1));
        data.insert("text_col".to_string(), json!("test text"));
        data.insert("integer_col".to_string(), json!(42));
        data.insert("real_col".to_string(), json!(3.14));
        data.insert("boolean_col".to_string(), json!(true));
        data.insert("date_col".to_string(), json!("2024-01-01"));
        data.insert("datetime_col".to_string(), json!("2024-01-01 12:00:00"));
        
        // Database-specific columns
        match self.dialect {
            #[cfg(feature = "postgres")]
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                data.insert("json_col".to_string(), json!({"key": "value"}));
                data.insert("uuid_col".to_string(), json!("123e4567-e89b-12d3-a456-426614174000"));
            },
            #[cfg(feature = "mysql")]
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                data.insert("json_col".to_string(), json!({"key": "value"}));
            },
            DatabaseDialect::SQLite => {},
        }
        
        data
    }
    
    /// Generate large dataset for performance testing
    fn generate_performance_data(&self, count: usize) -> Vec<TestRecord> {
        (0..count).map(|i| {
            TestRecord {
                table: "performance_test".to_string(),
                data: [
                    ("data".to_string(), json!(format!("test data {}", i))),
                    ("value".to_string(), json!(i % 1000)),
                ].into_iter().collect(),
            }
        }).collect()
    }
    
    /// Insert a test record using sea-query INSERT
    async fn insert_test_record<B: DatabaseBackend>(
        &self,
        record: &TestRecord,
        backend: &B,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut insert = Query::insert();
        insert.into_table(sea_query::Alias::new(&record.table));
        
        // Add columns and values using the new sea-query API
        let columns: Vec<sea_query::DynIden> = record.data.keys()
            .map(|col| sea_query::Alias::new(col).into_iden())
            .collect();
        
        let values: Vec<sea_query::SimpleExpr> = record.data.values().map(|v| {
            match v {
                Value::String(s) => sea_query::SimpleExpr::Value(sea_query::Value::String(Some(Box::new(s.clone())))),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        sea_query::SimpleExpr::Value(sea_query::Value::BigInt(Some(i)))
                    } else if let Some(f) = n.as_f64() {
                        sea_query::SimpleExpr::Value(sea_query::Value::Double(Some(f)))
                    } else {
                        sea_query::SimpleExpr::Value(sea_query::Value::String(Some(Box::new(n.to_string()))))
                    }
                },
                Value::Bool(b) => sea_query::SimpleExpr::Value(sea_query::Value::Bool(Some(*b))),
                Value::Null => sea_query::SimpleExpr::Value(sea_query::Value::String(None)),
                _ => sea_query::SimpleExpr::Value(sea_query::Value::String(Some(Box::new(v.to_string())))),
            }
        }).collect();
        
        insert.columns(columns).values(values)?;
        
        let (sql, params) = insert.build(SqliteQueryBuilder);
        let json_params: Vec<Value> = params.into_iter().map(|p| {
            match p {
                sea_query::Value::String(Some(s)) => Value::String(*s),
                sea_query::Value::BigInt(Some(i)) => Value::Number(i.into()),
                sea_query::Value::Double(Some(f)) => Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into())),
                sea_query::Value::Bool(Some(b)) => Value::Bool(b),
                _ => Value::Null,
            }
        }).collect();
        
        backend.execute_query(&sql, &json_params).await?;
        Ok(())
    }
}

/// Test helper macros for cross-database testing
#[macro_export]
macro_rules! with_test_fixture {
    ($fixture_name:expr, $backend:expr, $test_code:block) => {{
        let fixture_manager = $crate::tests::fixtures::TestFixtureManager::new($backend.dialect());
        fixture_manager.setup_fixture($fixture_name, &$backend).await?;
        
        let result = async move $test_code.await;
        
        let _ = fixture_manager.teardown_fixture($fixture_name, &$backend).await;
        result
    }};
}


#[cfg(test)]
mod tests {
    use super::*;
    use d1_rs::backends::SQLiteBackend;
    use crate::common::query_helpers::{build_count_query, build_select_query, table, column};
    
    #[tokio::test]
    async fn test_fixture_manager_creation() {
        let manager = TestFixtureManager::new(DatabaseDialect::SQLite);
        assert!(!manager.fixtures.is_empty());
        assert!(manager.fixtures.contains_key("users_posts"));
        assert!(manager.fixtures.contains_key("type_testing"));
        assert!(manager.fixtures.contains_key("performance_large"));
    }
    
    #[tokio::test]
    async fn test_users_posts_fixture() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let backend = SQLiteBackend::new_in_memory().await?;
        let manager = TestFixtureManager::new(backend.dialect());
        
        manager.setup_fixture("users_posts", &backend).await?;
        
        // Verify users were inserted using sea-query helper
        let (sql, params) = build_count_query(table("users"), backend.dialect());
        let result = backend.execute_query(&sql, &params).await?;
        let rows = result.into_rows();
        assert_eq!(rows.len(), 1);
        // Try different possible count column names
        let count_value = rows[0].get("count")
            .or_else(|| rows[0].get("COUNT(*)"))
            .or_else(|| rows[0].get("COUNT"));
        assert_eq!(count_value, Some(&serde_json::Value::Number(2.into())));
        
        // Verify posts were inserted using sea-query helper
        let (sql, params) = build_count_query(table("posts"), backend.dialect());
        let result = backend.execute_query(&sql, &params).await?;
        let rows = result.into_rows();
        assert_eq!(rows.len(), 1);
        let count_value = rows[0].get("count")
            .or_else(|| rows[0].get("COUNT(*)"))
            .or_else(|| rows[0].get("COUNT"));
        assert_eq!(count_value, Some(&serde_json::Value::Number(1.into())));
        
        manager.teardown_fixture("users_posts", &backend).await?;
        Ok(())
    }
    
    #[tokio::test]
    async fn test_type_testing_fixture() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let backend = SQLiteBackend::new_in_memory().await?;
        let manager = TestFixtureManager::new(backend.dialect());
        
        manager.setup_fixture("type_testing", &backend).await?;
        
        // Verify test data was inserted using sea-query helper
        let (sql, params) = build_count_query(table("type_test"), backend.dialect());
        let result = backend.execute_query(&sql, &params).await?;
        let rows = result.into_rows();
        assert_eq!(rows.len(), 1);
        let count_value = rows[0].get("count")
            .or_else(|| rows[0].get("COUNT(*)"))
            .or_else(|| rows[0].get("COUNT"));
        assert_eq!(count_value, Some(&serde_json::Value::Number(1.into())));
        
        // Verify specific data using sea-query helper
        let (sql, params) = build_select_query(
            table("type_test"), 
            vec![column("text_col"), column("integer_col")], 
            backend.dialect()
        );
        let result = backend.execute_query(&sql, &params).await?;
        let rows = result.into_rows();
        assert_eq!(rows.len(), 1);
        
        manager.teardown_fixture("type_testing", &backend).await?;
        Ok(())
    }
    
    #[tokio::test]
    async fn test_performance_fixture_setup() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let backend = SQLiteBackend::new_in_memory().await?;
        let manager = TestFixtureManager::new(backend.dialect());
        
        // Test with smaller dataset for speed
        manager.setup_fixture("performance_large", &backend).await?;
        
        // Verify large dataset was inserted using sea-query helper
        let (sql, params) = build_count_query(table("performance_test"), backend.dialect());
        let result = backend.execute_query(&sql, &params).await?;
        let rows = result.into_rows();
        assert_eq!(rows.len(), 1);
        let count_value = rows[0].get("count")
            .or_else(|| rows[0].get("COUNT(*)"))
            .or_else(|| rows[0].get("COUNT"));
        assert_eq!(count_value, Some(&serde_json::Value::Number(10000.into())));
        
        manager.teardown_fixture("performance_large", &backend).await?;
        Ok(())
    }
    
    #[tokio::test]
    async fn test_fixture_not_found() {
        let backend = SQLiteBackend::new_in_memory().await.unwrap();
        let manager = TestFixtureManager::new(backend.dialect());
        
        let result = manager.setup_fixture("nonexistent", &backend).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}