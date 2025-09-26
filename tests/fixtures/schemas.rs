/// Database schema builders for test fixtures
/// 
/// This module provides database-agnostic schema creation using sea-query
/// for consistent test environments across SQLite, PostgreSQL, and MySQL.

use d1_rs::dialects::DatabaseDialect;
use sea_query::{
    ColumnDef, ForeignKey, ForeignKeyAction,
    Index, Iden, Table, SqliteQueryBuilder,
};

#[cfg(feature = "postgres")]
use sea_query::PostgresQueryBuilder;

#[cfg(feature = "mysql")]
use sea_query::MysqlQueryBuilder;
use serde_json::Value;

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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
                    .col(ColumnDef::new(TestUsers::CreatedAt).date_time().default(sea_query::Expr::current_timestamp()))
                    .to_owned()
            },
            #[cfg(feature = "postgres")]
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
            #[cfg(feature = "mysql")]
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
                    .col(ColumnDef::new(TestUsers::CreatedAt).date_time().default(sea_query::Expr::current_timestamp()))
                    .to_owned()
            },
        };
        
        let sql = table.build(SqliteQueryBuilder);
        (sql, vec![])
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
                    .col(ColumnDef::new(TestPosts::CreatedAt).date_time().default(sea_query::Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_posts_user_id")
                            .from(TestPosts::Table, TestPosts::UserId)
                            .to(TestUsers::Table, TestUsers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned()
            },
            #[cfg(feature = "postgres")]
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
            #[cfg(feature = "mysql")]
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
                    .col(ColumnDef::new(TestPosts::CreatedAt).date_time().default(sea_query::Expr::current_timestamp()))
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
        
        let sql = table.build(SqliteQueryBuilder);
        (sql, vec![])
    }
    
    /// Create comprehensive type testing table
    #[allow(dead_code)]
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
                    .col(ColumnDef::new(TestTypeTest::RealCol).float())
                    .col(ColumnDef::new(TestTypeTest::DoubleCol).double())
                    .col(ColumnDef::new(TestTypeTest::BlobCol).blob())
                    .col(ColumnDef::new(TestTypeTest::BooleanCol).boolean())
                    .col(ColumnDef::new(TestTypeTest::DateCol).date())
                    .col(ColumnDef::new(TestTypeTest::DatetimeCol).date_time())
                    .col(ColumnDef::new(TestTypeTest::DecimalCol).decimal_len(10, 2))
                    .to_owned()
            },
            #[cfg(feature = "postgres")]
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
                    .col(ColumnDef::new(TestTypeTest::RealCol).float())
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
            #[cfg(feature = "mysql")]
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
                    .col(ColumnDef::new(TestTypeTest::RealCol).float())
                    .col(ColumnDef::new(TestTypeTest::DoubleCol).double())
                    .col(ColumnDef::new(TestTypeTest::BlobCol).blob())
                    .col(ColumnDef::new(TestTypeTest::BooleanCol).boolean())
                    .col(ColumnDef::new(TestTypeTest::DateCol).date())
                    .col(ColumnDef::new(TestTypeTest::DatetimeCol).date_time())
                    .col(ColumnDef::new(TestTypeTest::JsonCol).json())
                    .col(ColumnDef::new(TestTypeTest::DecimalCol).decimal_len(10, 2))
                    .to_owned()
            },
        };
        
        let sql = table.build(SqliteQueryBuilder);
        (sql, vec![])
    }
    
    /// Create performance testing table with indexes
    #[allow(dead_code)]
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
                    .col(ColumnDef::new(TestPerformance::Score).float())
                    .col(ColumnDef::new(TestPerformance::Category).string_len(50))
                    .col(ColumnDef::new(TestPerformance::CreatedAt).date_time().default(sea_query::Expr::current_timestamp()))
                    .col(ColumnDef::new(TestPerformance::UpdatedAt).date_time())
                    .to_owned()
            },
            #[cfg(feature = "postgres")]
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
            #[cfg(feature = "mysql")]
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
                    .col(ColumnDef::new(TestPerformance::CreatedAt).date_time().default(sea_query::Expr::current_timestamp()))
                    .col(ColumnDef::new(TestPerformance::UpdatedAt).date_time())
                    .to_owned()
            },
        };
        
        let sql = match self.dialect {
            DatabaseDialect::SQLite => table.build(SqliteQueryBuilder),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => table.build(PostgresQueryBuilder),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => table.build(MysqlQueryBuilder),
        };
        (sql, vec![])
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
        indexes.push((value_index.build(SqliteQueryBuilder), vec![]));
        
        // Category index
        let category_index = Index::create()
            .name("idx_performance_category")
            .table(TestPerformance::Table)
            .col(TestPerformance::Category)
            .to_owned();
        indexes.push((category_index.build(SqliteQueryBuilder), vec![]));
        
        // Composite index for complex queries
        let composite_index = Index::create()
            .name("idx_performance_composite")
            .table(TestPerformance::Table)
            .col(TestPerformance::Category)
            .col(TestPerformance::Value)
            .to_owned();
        indexes.push((composite_index.build(SqliteQueryBuilder), vec![]));
        
        indexes
    }
    
    /// Create categories table for hierarchical testing
    #[allow(dead_code)]
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
                    .col(ColumnDef::new(TestCategories::CreatedAt).date_time().default(sea_query::Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_categories_parent_id")
                            .from(TestCategories::Table, TestCategories::ParentId)
                            .to(TestCategories::Table, TestCategories::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                    )
                    .to_owned()
            },
            #[cfg(feature = "postgres")]
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
            #[cfg(feature = "mysql")]
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
                    .col(ColumnDef::new(TestCategories::CreatedAt).date_time().default(sea_query::Expr::current_timestamp()))
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
        
        let sql = table.build(SqliteQueryBuilder);
        (sql, vec![])
    }
    
    /// Get all drop table statements for cleanup
    pub fn drop_tables(&self) -> Vec<(String, Vec<Value>)> {
        let mut drops = Vec::new();
        
        // Drop tables in reverse dependency order
        drops.push({
            let drop = Table::drop().table(TestPosts::Table).if_exists().to_owned();
            (drop.build(SqliteQueryBuilder), vec![])
        });
        
        drops.push({
            let drop = Table::drop().table(TestCategories::Table).if_exists().to_owned();
            (drop.build(SqliteQueryBuilder), vec![])
        });
        
        drops.push({
            let drop = Table::drop().table(TestUsers::Table).if_exists().to_owned();
            (drop.build(SqliteQueryBuilder), vec![])
        });
        
        drops.push({
            let drop = Table::drop().table(TestTypeTest::Table).if_exists().to_owned();
            (drop.build(SqliteQueryBuilder), vec![])
        });
        
        drops.push({
            let drop = Table::drop().table(TestPerformance::Table).if_exists().to_owned();
            (drop.build(SqliteQueryBuilder), vec![])
        });
        
        drops
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
    #[cfg(feature = "postgres")]
    fn test_postgres_schema_creation() {
        let builder = TestSchemaBuilder::new(DatabaseDialect::PostgreSQL);
        let (sql, _) = builder.create_type_test_table();
        
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS"));
        assert!(sql.contains("test_type_test"));
        assert!(sql.contains("json_col"));
        assert!(sql.contains("uuid_col"));
    }
    
    #[test]
    #[cfg(feature = "mysql")]
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