/// Database schema builders for test fixtures
/// 
/// This module provides database-agnostic schema creation using sea-query
/// for consistent test environments across SQLite, PostgreSQL, and MySQL.

use d1_rs::dialects::DatabaseDialect;
use sea_query::{
    ColumnDef, CreateTableStatement, DropTableStatement, ForeignKey, ForeignKeyAction,
    Index, Iden, Table, SqliteQueryBuilder,
};
use serde_json::Value;
use std::collections::HashMap;

/// Schema builder for test fixtures
pub struct TestSchemaBuilder {
    dialect: DatabaseDialect,
}

/// Table identifiers for sea-query
#[derive(Iden)]
pub enum TestUsers {
    Table,
    Id,
    Name,
    Email,
    IsActive,
    Score,
    CreatedAt,
}

#[derive(Iden)]
pub enum TestPosts {
    Table,
    Id,
    Title,
    Content,
    UserId,
    IsPublished,
    CreatedAt,
}

#[derive(Iden)]
pub enum TestTypeTest {
    Table,
    Id,
    TextCol,
    VarcharCol,
    IntegerCol,
    BigintCol,
    RealCol,
    DoubleCol,
    BlobCol,
    BinaryCol,
    BooleanCol,
    DateCol,
    TimeCol,
    DatetimeCol,
    TimestampCol,
    JsonCol,
    JsonbCol,
    UuidCol,
    DecimalCol,
    NumericCol,
}

#[derive(Iden)]
pub enum TestPerformance {
    Table,
    Id,
    Data,
    Value,
    Score,
    Category,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum TestCategories {
    Table,
    Id,
    Name,
    Description,
    ParentId,
    CreatedAt,
}

impl TestSchemaBuilder {
    /// Create new schema builder for specified dialect
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self { dialect }
    }
    
    /// Create comprehensive users table for all database types
    pub fn create_users_table(&self) -> (String, Vec<Value>) {
        let table = match self.dialect {
            DatabaseDialect::SQLite => {
                Table::create()
                    .table(TestUsers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestUsers::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestUsers::Name).text().not_null())
                    .col(ColumnDef::new(TestUsers::Email).text().not_null().unique_key())
                    .col(ColumnDef::new(TestUsers::IsActive).integer().not_null().default(1))
                    .col(ColumnDef::new(TestUsers::Score).integer())
                    .col(ColumnDef::new(TestUsers::CreatedAt).datetime().default(sea_query::Expr::current_timestamp()))
                    .to_owned()
            },
            DatabaseDialect::PostgreSQL => {
                Table::create()
                    .table(TestUsers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestUsers::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestUsers::Name).string_len(255).not_null())
                    .col(ColumnDef::new(TestUsers::Email).string_len(255).not_null().unique_key())
                    .col(ColumnDef::new(TestUsers::IsActive).boolean().not_null().default(true))
                    .col(ColumnDef::new(TestUsers::Score).integer())
                    .col(ColumnDef::new(TestUsers::CreatedAt).timestamp().default(sea_query::Expr::current_timestamp()))
                    .to_owned()
            },
            DatabaseDialect::MySQL => {
                Table::create()
                    .table(TestUsers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestUsers::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestUsers::Name).string_len(255).not_null())
                    .col(ColumnDef::new(TestUsers::Email).string_len(255).not_null().unique_key())
                    .col(ColumnDef::new(TestUsers::IsActive).boolean().not_null().default(true))
                    .col(ColumnDef::new(TestUsers::Score).integer())
                    .col(ColumnDef::new(TestUsers::CreatedAt).datetime().default(sea_query::Expr::current_timestamp()))
                    .to_owned()
            },
        };
        
        table.build_sqlx(SqliteQueryBuilder)
    }
    
    /// Create posts table with foreign key relationships
    pub fn create_posts_table(&self) -> (String, Vec<Value>) {
        let table = match self.dialect {
            DatabaseDialect::SQLite => {
                Table::create()
                    .table(TestPosts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestPosts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestPosts::Title).text().not_null())
                    .col(ColumnDef::new(TestPosts::Content).text())
                    .col(ColumnDef::new(TestPosts::UserId).integer().not_null())
                    .col(ColumnDef::new(TestPosts::IsPublished).integer().not_null().default(0))
                    .col(ColumnDef::new(TestPosts::CreatedAt).datetime().default(sea_query::Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posts_user_id")
                            .from(TestPosts::Table, TestPosts::UserId)
                            .to(TestUsers::Table, TestUsers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            },
            DatabaseDialect::PostgreSQL => {
                Table::create()
                    .table(TestPosts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestPosts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestPosts::Title).string_len(255).not_null())
                    .col(ColumnDef::new(TestPosts::Content).text())
                    .col(ColumnDef::new(TestPosts::UserId).integer().not_null())
                    .col(ColumnDef::new(TestPosts::IsPublished).boolean().not_null().default(false))
                    .col(ColumnDef::new(TestPosts::CreatedAt).timestamp().default(sea_query::Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posts_user_id")
                            .from(TestPosts::Table, TestPosts::UserId)
                            .to(TestUsers::Table, TestUsers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            },
            DatabaseDialect::MySQL => {
                Table::create()
                    .table(TestPosts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestPosts::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestPosts::Title).string_len(255).not_null())
                    .col(ColumnDef::new(TestPosts::Content).text())
                    .col(ColumnDef::new(TestPosts::UserId).integer().not_null())
                    .col(ColumnDef::new(TestPosts::IsPublished).boolean().not_null().default(false))
                    .col(ColumnDef::new(TestPosts::CreatedAt).datetime().default(sea_query::Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posts_user_id")
                            .from(TestPosts::Table, TestPosts::UserId)
                            .to(TestUsers::Table, TestUsers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            },
        };
        
        table.build_sqlx(SqliteQueryBuilder)
    }
    
    /// Create comprehensive type testing table
    pub fn create_type_test_table(&self) -> (String, Vec<Value>) {
        let table = match self.dialect {
            DatabaseDialect::SQLite => {
                Table::create()
                    .table(TestTypeTest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestTypeTest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestTypeTest::TextCol).text())
                    .col(ColumnDef::new(TestTypeTest::VarcharCol).string_len(100))
                    .col(ColumnDef::new(TestTypeTest::IntegerCol).integer())
                    .col(ColumnDef::new(TestTypeTest::BigintCol).big_integer())
                    .col(ColumnDef::new(TestTypeTest::RealCol).real())
                    .col(ColumnDef::new(TestTypeTest::DoubleCol).double())
                    .col(ColumnDef::new(TestTypeTest::BlobCol).blob())
                    .col(ColumnDef::new(TestTypeTest::BooleanCol).boolean())
                    .col(ColumnDef::new(TestTypeTest::DateCol).date())
                    .col(ColumnDef::new(TestTypeTest::DatetimeCol).datetime())
                    .col(ColumnDef::new(TestTypeTest::DecimalCol).decimal_len(10, 2))
                    .to_owned()
            },
            DatabaseDialect::PostgreSQL => {
                Table::create()
                    .table(TestTypeTest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestTypeTest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestTypeTest::TextCol).text())
                    .col(ColumnDef::new(TestTypeTest::VarcharCol).string_len(100))
                    .col(ColumnDef::new(TestTypeTest::IntegerCol).integer())
                    .col(ColumnDef::new(TestTypeTest::BigintCol).big_integer())
                    .col(ColumnDef::new(TestTypeTest::RealCol).real())
                    .col(ColumnDef::new(TestTypeTest::DoubleCol).double())
                    .col(ColumnDef::new(TestTypeTest::BinaryCol).binary_len(255))
                    .col(ColumnDef::new(TestTypeTest::BooleanCol).boolean())
                    .col(ColumnDef::new(TestTypeTest::DateCol).date())
                    .col(ColumnDef::new(TestTypeTest::TimeCol).time())
                    .col(ColumnDef::new(TestTypeTest::TimestampCol).timestamp())
                    .col(ColumnDef::new(TestTypeTest::JsonCol).json())
                    .col(ColumnDef::new(TestTypeTest::JsonbCol).json_binary())
                    .col(ColumnDef::new(TestTypeTest::UuidCol).uuid())
                    .col(ColumnDef::new(TestTypeTest::DecimalCol).decimal_len(10, 2))
                    .col(ColumnDef::new(TestTypeTest::NumericCol).decimal_len(15, 5))
                    .to_owned()
            },
            DatabaseDialect::MySQL => {
                Table::create()
                    .table(TestTypeTest::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestTypeTest::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestTypeTest::TextCol).text())
                    .col(ColumnDef::new(TestTypeTest::VarcharCol).string_len(100))
                    .col(ColumnDef::new(TestTypeTest::IntegerCol).integer())
                    .col(ColumnDef::new(TestTypeTest::BigintCol).big_integer())
                    .col(ColumnDef::new(TestTypeTest::RealCol).real())
                    .col(ColumnDef::new(TestTypeTest::DoubleCol).double())
                    .col(ColumnDef::new(TestTypeTest::BlobCol).blob())
                    .col(ColumnDef::new(TestTypeTest::BooleanCol).boolean())
                    .col(ColumnDef::new(TestTypeTest::DateCol).date())
                    .col(ColumnDef::new(TestTypeTest::DatetimeCol).datetime())
                    .col(ColumnDef::new(TestTypeTest::JsonCol).json())
                    .col(ColumnDef::new(TestTypeTest::DecimalCol).decimal_len(10, 2))
                    .to_owned()
            },
        };
        
        table.build_sqlx(SqliteQueryBuilder)
    }
    
    /// Create performance testing table with indexes
    pub fn create_performance_table(&self) -> (String, Vec<Value>) {
        let table = match self.dialect {
            DatabaseDialect::SQLite => {
                Table::create()
                    .table(TestPerformance::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestPerformance::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestPerformance::Data).text().not_null())
                    .col(ColumnDef::new(TestPerformance::Value).integer().not_null())
                    .col(ColumnDef::new(TestPerformance::Score).real())
                    .col(ColumnDef::new(TestPerformance::Category).string_len(50))
                    .col(ColumnDef::new(TestPerformance::CreatedAt).datetime().default(sea_query::Expr::current_timestamp()))
                    .col(ColumnDef::new(TestPerformance::UpdatedAt).datetime())
                    .to_owned()
            },
            DatabaseDialect::PostgreSQL => {
                Table::create()
                    .table(TestPerformance::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestPerformance::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestPerformance::Data).text().not_null())
                    .col(ColumnDef::new(TestPerformance::Value).integer().not_null())
                    .col(ColumnDef::new(TestPerformance::Score).double())
                    .col(ColumnDef::new(TestPerformance::Category).string_len(50))
                    .col(ColumnDef::new(TestPerformance::CreatedAt).timestamp().default(sea_query::Expr::current_timestamp()))
                    .col(ColumnDef::new(TestPerformance::UpdatedAt).timestamp())
                    .to_owned()
            },
            DatabaseDialect::MySQL => {
                Table::create()
                    .table(TestPerformance::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestPerformance::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestPerformance::Data).text().not_null())
                    .col(ColumnDef::new(TestPerformance::Value).integer().not_null())
                    .col(ColumnDef::new(TestPerformance::Score).double())
                    .col(ColumnDef::new(TestPerformance::Category).string_len(50))
                    .col(ColumnDef::new(TestPerformance::CreatedAt).datetime().default(sea_query::Expr::current_timestamp()))
                    .col(ColumnDef::new(TestPerformance::UpdatedAt).datetime())
                    .to_owned()
            },
        };
        
        table.build_sqlx(SqliteQueryBuilder)
    }
    
    /// Create performance table indexes
    pub fn create_performance_indexes(&self) -> Vec<(String, Vec<Value>)> {
        let mut indexes = Vec::new();
        
        // Value index for performance queries
        let value_index = Index::create()
            .name("idx_performance_value")
            .table(TestPerformance::Table)
            .col(TestPerformance::Value)
            .to_owned();
        indexes.push(value_index.build_sqlx(SqliteQueryBuilder));
        
        // Category index
        let category_index = Index::create()
            .name("idx_performance_category")
            .table(TestPerformance::Table)
            .col(TestPerformance::Category)
            .to_owned();
        indexes.push(category_index.build_sqlx(SqliteQueryBuilder));
        
        // Composite index for complex queries
        let composite_index = Index::create()
            .name("idx_performance_composite")
            .table(TestPerformance::Table)
            .col(TestPerformance::Category)
            .col(TestPerformance::Value)
            .to_owned();
        indexes.push(composite_index.build_sqlx(SqliteQueryBuilder));
        
        indexes
    }
    
    /// Create categories table for hierarchical testing
    pub fn create_categories_table(&self) -> (String, Vec<Value>) {
        let table = match self.dialect {
            DatabaseDialect::SQLite => {
                Table::create()
                    .table(TestCategories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestCategories::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestCategories::Name).string_len(255).not_null())
                    .col(ColumnDef::new(TestCategories::Description).text())
                    .col(ColumnDef::new(TestCategories::ParentId).integer())
                    .col(ColumnDef::new(TestCategories::CreatedAt).datetime().default(sea_query::Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_categories_parent_id")
                            .from(TestCategories::Table, TestCategories::ParentId)
                            .to(TestCategories::Table, TestCategories::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                    )
                    .to_owned()
            },
            DatabaseDialect::PostgreSQL => {
                Table::create()
                    .table(TestCategories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestCategories::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestCategories::Name).string_len(255).not_null())
                    .col(ColumnDef::new(TestCategories::Description).text())
                    .col(ColumnDef::new(TestCategories::ParentId).integer())
                    .col(ColumnDef::new(TestCategories::CreatedAt).timestamp().default(sea_query::Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_categories_parent_id")
                            .from(TestCategories::Table, TestCategories::ParentId)
                            .to(TestCategories::Table, TestCategories::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                    )
                    .to_owned()
            },
            DatabaseDialect::MySQL => {
                Table::create()
                    .table(TestCategories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TestCategories::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TestCategories::Name).string_len(255).not_null())
                    .col(ColumnDef::new(TestCategories::Description).text())
                    .col(ColumnDef::new(TestCategories::ParentId).integer())
                    .col(ColumnDef::new(TestCategories::CreatedAt).datetime().default(sea_query::Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_categories_parent_id")
                            .from(TestCategories::Table, TestCategories::ParentId)
                            .to(TestCategories::Table, TestCategories::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                    )
                    .to_owned()
            },
        };
        
        table.build_sqlx(SqliteQueryBuilder)
    }
    
    /// Get all drop table statements for cleanup
    pub fn drop_tables(&self) -> Vec<(String, Vec<Value>)> {
        let tables = vec![
            TestPosts::Table,
            TestUsers::Table,
            TestTypeTest::Table,
            TestPerformance::Table,
            TestCategories::Table,
        ];
        
        tables.into_iter().map(|table| {
            let drop = Table::drop().table(table).if_exists().to_owned();
            drop.build_sqlx(SqliteQueryBuilder)
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sqlite_schema_creation() {
        let builder = TestSchemaBuilder::new(DatabaseDialect::SQLite);
        let (sql, _) = builder.create_users_table();
        
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(sql.contains("test_users"));
        assert!(sql.contains("PRIMARY KEY"));
    }
    
    #[test]
    fn test_postgres_schema_creation() {
        let builder = TestSchemaBuilder::new(DatabaseDialect::PostgreSQL);
        let (sql, _) = builder.create_type_test_table();
        
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(sql.contains("test_type_test"));
        assert!(sql.contains("json_col"));
        assert!(sql.contains("uuid_col"));
    }
    
    #[test]
    fn test_mysql_schema_creation() {
        let builder = TestSchemaBuilder::new(DatabaseDialect::MySQL);
        let (sql, _) = builder.create_performance_table();
        
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(sql.contains("test_performance"));
        assert!(sql.contains("AUTO_INCREMENT"));
    }
    
    #[test]
    fn test_foreign_key_creation() {
        let builder = TestSchemaBuilder::new(DatabaseDialect::SQLite);
        let (sql, _) = builder.create_posts_table();
        
        assert!(sql.contains("FOREIGN KEY"));
        assert!(sql.contains("REFERENCES"));
        assert!(sql.contains("user_id"));
    }
    
    #[test]
    fn test_index_creation() {
        let builder = TestSchemaBuilder::new(DatabaseDialect::SQLite);
        let indexes = builder.create_performance_indexes();
        
        assert_eq!(indexes.len(), 3);
        assert!(indexes[0].0.contains("CREATE INDEX"));
        assert!(indexes[0].0.contains("idx_performance_value"));
    }
    
    #[test]
    fn test_drop_tables() {
        let builder = TestSchemaBuilder::new(DatabaseDialect::SQLite);
        let drops = builder.drop_tables();
        
        assert!(!drops.is_empty());
        for (sql, _) in drops {
            assert!(sql.contains("DROP TABLE IF EXISTS"));
        }
    }
}