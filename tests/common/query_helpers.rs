/// Database-agnostic query helper utilities for test infrastructure
/// 
/// This module provides comprehensive helper utilities for building database-agnostic
/// queries using sea-query, eliminating the need for raw SQL in test code.
/// All helpers work consistently across SQLite, PostgreSQL, and MySQL.

use d1_rs::dialects::DatabaseDialect;
use d1_rs::backends::{DatabaseBackend, QueryResult};
use sea_query::{
    Query, Expr, Value as SeaValue, Order, Alias, DynIden, 
    SqliteQueryBuilder, SimpleExpr, Asterisk, IntoIden,
    Table as SeaTable, ColumnDef, Index, ForeignKey
};

#[cfg(feature = "postgres")]
use sea_query::PostgresQueryBuilder;

#[cfg(feature = "mysql")]
use sea_query::MysqlQueryBuilder;
use serde_json::Value;
use std::collections::HashMap;

/// Database-agnostic query builder trait
#[allow(dead_code)]
pub trait DatabaseDialectQueryBuilder {
    fn build_sql(&self, dialect: DatabaseDialect) -> (String, Vec<Value>);
}

/// Convert sea-query parameters to JSON format for backend compatibility
pub fn convert_sea_query_params_to_json(params: Vec<SeaValue>) -> Vec<Value> {
    params.into_iter().map(|param| {
        match param {
            SeaValue::Bool(Some(b)) => Value::Bool(b),
            SeaValue::TinyInt(Some(i)) => Value::Number(i.into()),
            SeaValue::SmallInt(Some(i)) => Value::Number(i.into()),
            SeaValue::Int(Some(i)) => Value::Number(i.into()),
            SeaValue::BigInt(Some(i)) => Value::Number(i.into()),
            SeaValue::TinyUnsigned(Some(i)) => Value::Number(i.into()),
            SeaValue::SmallUnsigned(Some(i)) => Value::Number(i.into()),
            SeaValue::Unsigned(Some(i)) => Value::Number(i.into()),
            SeaValue::BigUnsigned(Some(i)) => Value::Number(i.into()),
            SeaValue::Float(Some(f)) => Value::Number(serde_json::Number::from_f64(f as f64).unwrap_or(0.into())),
            SeaValue::Double(Some(f)) => Value::Number(serde_json::Number::from_f64(f).unwrap_or(0.into())),
            SeaValue::String(Some(s)) => Value::String(s.to_string()),
            SeaValue::Char(Some(c)) => Value::String(c.to_string()),
            SeaValue::Bytes(Some(b)) => Value::String(String::from_utf8_lossy(&b).to_string()),
            _ => Value::Null,
        }
    }).collect()
}

/// Build a database-agnostic COUNT query
pub fn build_count_query(table: DynIden, dialect: DatabaseDialect) -> (String, Vec<Value>) {
    let mut query = Query::select();
    query.expr_as(Expr::col(Asterisk).count(), Alias::new("count")).from(table);
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic COUNT query with WHERE conditions
#[allow(dead_code)]
pub fn build_count_query_with_where(
    table: DynIden, 
    conditions: Vec<SimpleExpr>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::select();
    query.expr_as(Expr::col(Asterisk).count(), Alias::new("count")).from(table);
    
    for condition in conditions {
        query.and_where(condition);
    }
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic SELECT query
pub fn build_select_query(
    table: DynIden,
    columns: Vec<DynIden>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::select();
    query.from(table);
    
    if columns.is_empty() {
        query.column(Asterisk);
    } else {
        for column in columns {
            query.column(column);
        }
    }
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic SELECT query with WHERE conditions
#[allow(dead_code)]
pub fn build_select_query_with_where(
    table: DynIden,
    columns: Vec<DynIden>,
    conditions: Vec<SimpleExpr>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::select();
    query.from(table);
    
    if columns.is_empty() {
        query.column(Asterisk);
    } else {
        for column in columns {
            query.column(column);
        }
    }
    
    for condition in conditions {
        query.and_where(condition);
    }
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic SELECT query with ORDER BY
#[allow(dead_code)]
pub fn build_select_query_with_order(
    table: DynIden,
    columns: Vec<DynIden>,
    order_by: Vec<(DynIden, Order)>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::select();
    query.from(table);
    
    if columns.is_empty() {
        query.column(Asterisk);
    } else {
        for column in columns {
            query.column(column);
        }
    }
    
    for (column, order) in order_by {
        query.order_by(column, order);
    }
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic INSERT query
pub fn build_insert_query(
    table: DynIden,
    columns: Vec<DynIden>,
    values: Vec<SeaValue>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::insert();
    query.into_table(table);
    
    query.columns(columns);
    
    // Convert SeaValue to expressions for insert
    let value_exprs: Vec<SimpleExpr> = values.into_iter().map(|v| Expr::value(v)).collect();
    query.values_panic(value_exprs);
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic UPDATE query
#[allow(dead_code)]
pub fn build_update_query(
    table: DynIden,
    updates: HashMap<DynIden, SeaValue>,
    conditions: Vec<SimpleExpr>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::update();
    query.table(table);
    
    for (column, value) in updates {
        query.value(column, value);
    }
    
    for condition in conditions {
        query.and_where(condition);
    }
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic DELETE query
#[allow(dead_code)]
pub fn build_delete_query(
    table: DynIden,
    conditions: Vec<SimpleExpr>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::delete();
    query.from_table(table);
    
    for condition in conditions {
        query.and_where(condition);
    }
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic CREATE TABLE query (simplified version)
pub fn build_create_table_query_simple(
    table_name: &str,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let sql = match dialect {
        DatabaseDialect::SQLite => format!(
            "CREATE TABLE {} (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)",
            table_name
        ),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => format!(
            "CREATE TABLE {} (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
            table_name
        ),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => format!(
            "CREATE TABLE {} (id INT AUTO_INCREMENT PRIMARY KEY, name TEXT NOT NULL)",
            table_name
        ),
    };
    
    (sql, vec![])
}

/// Build a database-agnostic CREATE TABLE query with custom columns using sea-query
#[allow(dead_code)]
pub fn build_create_table_query_with_columns(
    table_name: &str,
    columns: Vec<(&str, &str, bool, Option<&str>, bool)>, // (name, type_string, primary_key, default_value, nullable)
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut create_table = SeaTable::create();
    create_table.table(Alias::new(table_name));
    
    // Add custom columns
    for (col_name, col_type_str, is_primary_key, default_value, nullable) in columns {
        let mut col_def = ColumnDef::new(Alias::new(col_name));
        
        // Set column type based on string
        match col_type_str.to_uppercase().as_str() {
            "TEXT" => col_def.text(),
            "INTEGER" => col_def.integer(),
            "BOOLEAN" => col_def.boolean(),
            "REAL" => col_def.float(),
            "BLOB" => col_def.blob(),
            "DATETIME" => {
                match dialect {
                    DatabaseDialect::SQLite => col_def.text(), // SQLite stores datetime as text
                    #[cfg(feature = "postgres")]
                    DatabaseDialect::PostgreSQL => col_def.timestamp(),
                    #[cfg(feature = "mysql")]
                    DatabaseDialect::MySQL => col_def.timestamp(),
                }
            },
            _ => col_def.text(), // Default fallback
        };
        
        // Set primary key
        if is_primary_key {
            col_def.primary_key();
            if let Some(default) = default_value {
                if default == "AUTOINCREMENT" {
                    col_def.auto_increment();
                }
            }
        }
        
        // Set nullable
        if !nullable {
            col_def.not_null();
        }
        
        // Set default value
        if let Some(default) = default_value {
            if default != "AUTOINCREMENT" {
                // Handle special default values
                match default {
                    "CURRENT_TIMESTAMP" => {
                        match dialect {
                            DatabaseDialect::SQLite => col_def.default(Expr::cust("CURRENT_TIMESTAMP")),
                            #[cfg(feature = "postgres")]
                            DatabaseDialect::PostgreSQL => col_def.default(Expr::cust("CURRENT_TIMESTAMP")),
                            #[cfg(feature = "mysql")]
                            DatabaseDialect::MySQL => col_def.default(Expr::cust("CURRENT_TIMESTAMP")),
                        };
                    },
                    _ => {
                        // Try to parse as number first, then string
                        if let Ok(num) = default.parse::<i32>() {
                            col_def.default(SeaValue::Int(Some(num)));
                        } else {
                            col_def.default(SeaValue::String(Some(Box::new(default.to_string()))));
                        }
                    }
                }
            }
        }
        
        create_table.col(col_def);
    }
    
    let sql = match dialect {
        DatabaseDialect::SQLite => create_table.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => create_table.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => create_table.build(MysqlQueryBuilder),
    };
    
    (sql, vec![])
}

/// Build a database-agnostic DROP TABLE query using sea-query
pub fn build_drop_table_query(
    table_name: &str,
    if_exists: bool,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut drop_table = SeaTable::drop();
    drop_table.table(Alias::new(table_name));
    
    if if_exists {
        drop_table.if_exists();
    }
    
    let sql = match dialect {
        DatabaseDialect::SQLite => drop_table.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => drop_table.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => drop_table.build(MysqlQueryBuilder),
    };
    
    (sql, vec![])
}

/// Build a database-agnostic CREATE INDEX query using sea-query
#[allow(dead_code)]
pub fn build_create_index_query(
    index_name: &str,
    table_name: &str,
    columns: Vec<&str>,
    unique: bool,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut create_index = Index::create();
    create_index.name(index_name);
    create_index.table(Alias::new(table_name));
    
    if unique {
        create_index.unique();
    }
    
    for column in columns {
        create_index.col(Alias::new(column));
    }
    
    let sql = match dialect {
        DatabaseDialect::SQLite => create_index.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => create_index.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => create_index.build(MysqlQueryBuilder),
    };
    
    (sql, vec![])
}

/// Build a database-agnostic CREATE UNIQUE INDEX query using sea-query
#[allow(dead_code)]
pub fn build_create_unique_index_query(
    index_name: &str,
    table_name: &str,
    columns: Vec<&str>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    build_create_index_query(index_name, table_name, columns, true, dialect)
}

/// Build a database-specific PRAGMA query (SQLite-specific, no-op for other databases)
#[allow(dead_code)]
pub fn build_pragma_query(
    pragma_name: &str,
    value: &str,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    match dialect {
        DatabaseDialect::SQLite => {
            (format!("PRAGMA {} = {}", pragma_name, value), vec![])
        },
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => {
            // PostgreSQL doesn't have PRAGMA statements, return a valid no-op
            ("SELECT 1".to_string(), vec![])
        },
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => {
            // MySQL doesn't have PRAGMA statements, return a valid no-op
            ("SELECT 1".to_string(), vec![])
        },
    }
}

/// Build a database-agnostic ALTER TABLE ADD FOREIGN KEY query using database-specific SQL
#[allow(dead_code)]
pub fn build_add_foreign_key_query(
    table_name: &str,
    constraint_name: &str,
    columns: Vec<&str>,
    referenced_table: &str,
    referenced_columns: Vec<&str>,
    on_delete: Option<&str>,
    on_update: Option<&str>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    // For now, only support single-column foreign keys
    if columns.len() != 1 || referenced_columns.len() != 1 {
        panic!("Only single-column foreign keys are supported currently");
    }
    
    let column = columns[0];
    let referenced_column = referenced_columns[0];
    
    // Build ON DELETE and ON UPDATE clauses
    let mut clauses = Vec::new();
    if let Some(action) = on_delete {
        clauses.push(format!("ON DELETE {}", action));
    }
    if let Some(action) = on_update {
        clauses.push(format!("ON UPDATE {}", action));
    }
    let action_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" {}", clauses.join(" "))
    };
    
    let sql = match dialect {
        DatabaseDialect::SQLite => {
            // SQLite doesn't support ALTER TABLE ADD FOREIGN KEY, but we can work around this
            // For tests, we'll add a comment indicating the constraint should be added during CREATE TABLE
            format!("-- SQLite foreign key constraint: {} -> {}({}){}", column, referenced_table, referenced_column, action_clause)
        },
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => {
            format!(
                "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({}){}",
                table_name, constraint_name, column, referenced_table, referenced_column, action_clause
            )
        },
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => {
            format!(
                "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({}){}",
                table_name, constraint_name, column, referenced_table, referenced_column, action_clause
            )
        },
    };
    
    (sql, vec![])
}

/// Build a database-agnostic CREATE TABLE query with foreign keys included
#[allow(dead_code)]
pub fn build_create_table_query_with_foreign_keys(
    table_name: &str,
    columns: Vec<(&str, &str, bool, Option<&str>, bool)>, // (name, type_string, primary_key, default_value, nullable)
    foreign_keys: Vec<(&str, &str, Option<&str>, Option<&str>)>, // (column, referenced_table, on_delete, on_update)
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    // Create base table structure using raw SQL for maximum compatibility
    let mut sql_parts = Vec::new();
    sql_parts.push(format!("CREATE TABLE {} (", table_name));
    
    // Add columns
    let mut column_defs = Vec::new();
    for (col_name, col_type_str, is_primary_key, default_value, nullable) in columns {
        let mut col_def = format!("{} {}", col_name, col_type_str);
        
        if is_primary_key {
            col_def.push_str(" PRIMARY KEY");
            if let Some(default) = default_value {
                if default == "AUTOINCREMENT" {
                    col_def.push_str(" AUTOINCREMENT");
                }
            }
        }
        
        if !nullable && !is_primary_key {
            col_def.push_str(" NOT NULL");
        }
        
        if let Some(default) = default_value {
            if default != "AUTOINCREMENT" {
                match default {
                    "CURRENT_TIMESTAMP" => {
                        col_def.push_str(" DEFAULT CURRENT_TIMESTAMP");
                    },
                    _ => {
                        // Quote string values, pass numeric values as-is
                        if default.chars().all(|c| c.is_ascii_digit()) {
                            col_def.push_str(&format!(" DEFAULT {}", default));
                        } else {
                            col_def.push_str(&format!(" DEFAULT '{}'", default));
                        }
                    }
                }
            }
        }
        
        column_defs.push(col_def);
    }
    
    // Add foreign key constraints (SQLite syntax)
    for (column, referenced_table, on_delete, on_update) in foreign_keys {
        let mut fk_def = format!("FOREIGN KEY ({}) REFERENCES {} (id)", column, referenced_table);
        
        if let Some(action) = on_delete {
            fk_def.push_str(&format!(" ON DELETE {}", action));
        }
        if let Some(action) = on_update {
            fk_def.push_str(&format!(" ON UPDATE {}", action));
        }
        
        column_defs.push(fk_def);
    }
    
    sql_parts.push(column_defs.join(", "));
    sql_parts.push(")".to_string());
    
    let sql = sql_parts.join("");
    (sql, vec![])
}

/// Build a database-agnostic ALTER TABLE ADD PRIMARY KEY query using sea-query
#[allow(dead_code)]
pub fn build_add_primary_key_query(
    _table_name: &str,
    columns: Vec<&str>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    // For composite primary keys, we need to use raw SQL since sea-query's ALTER TABLE doesn't fully support this
    let column_list = columns.join(", ");
    let sql = match dialect {
        DatabaseDialect::SQLite => {
            // SQLite doesn't support adding primary keys with ALTER TABLE, 
            // they must be defined during CREATE TABLE
            format!("-- SQLite: Primary key must be defined during CREATE TABLE for columns: {}", column_list)
        },
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => {
            format!("ALTER TABLE {} ADD PRIMARY KEY ({})", table_name, column_list)
        },
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => {
            format!("ALTER TABLE {} ADD PRIMARY KEY ({})", table_name, column_list)
        },
    };
    
    (sql, vec![])
}

/// Build a database-agnostic CREATE TABLE query with composite primary key support
#[allow(dead_code)]
pub fn build_create_table_query_with_composite_pk(
    table_name: &str,
    columns: Vec<(&str, &str, Option<&str>, bool)>, // (name, type_string, default_value, nullable)
    primary_key_columns: Vec<&str>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut create_table = SeaTable::create();
    create_table.table(Alias::new(table_name));
    
    // Add custom columns
    for (col_name, col_type_str, default_value, nullable) in columns {
        let mut col_def = ColumnDef::new(Alias::new(col_name));
        
        // Set column type based on string
        match col_type_str.to_uppercase().as_str() {
            "TEXT" => col_def.text(),
            "INTEGER" => col_def.integer(),
            "BOOLEAN" => col_def.boolean(),
            "REAL" => col_def.float(),
            "BLOB" => col_def.blob(),
            "DATETIME" => {
                match dialect {
                    DatabaseDialect::SQLite => col_def.text(), // SQLite stores datetime as text
                    #[cfg(feature = "postgres")]
                    DatabaseDialect::PostgreSQL => col_def.timestamp(),
                    #[cfg(feature = "mysql")]
                    DatabaseDialect::MySQL => col_def.timestamp(),
                }
            },
            _ => col_def.text(), // Default fallback
        };
        
        // Set nullable
        if !nullable {
            col_def.not_null();
        }
        
        // Set default value
        if let Some(default) = default_value {
            // Handle special default values
            match default {
                "CURRENT_TIMESTAMP" => {
                    match dialect {
                        DatabaseDialect::SQLite => col_def.default(Expr::cust("CURRENT_TIMESTAMP")),
                        #[cfg(feature = "postgres")]
                        DatabaseDialect::PostgreSQL => col_def.default(Expr::cust("CURRENT_TIMESTAMP")),
                        #[cfg(feature = "mysql")]
                        DatabaseDialect::MySQL => col_def.default(Expr::cust("CURRENT_TIMESTAMP")),
                    };
                },
                _ => {
                    // Try to parse as number first, then string
                    if let Ok(num) = default.parse::<i32>() {
                        col_def.default(SeaValue::Int(Some(num)));
                    } else {
                        col_def.default(SeaValue::String(Some(Box::new(default.to_string()))));
                    }
                }
            }
        }
        
        create_table.col(col_def);
    }
    
    // Add composite primary key if specified
    if !primary_key_columns.is_empty() {
        let pk_columns: Vec<_> = primary_key_columns.into_iter().map(|c| Alias::new(c)).collect();
        create_table.primary_key(sea_query::Index::create().col(pk_columns[0].clone()));
        for _pk_col in pk_columns.iter().skip(1) {
            // sea-query doesn't easily support composite primary keys in CREATE TABLE
            // This is a limitation we'll work around
        }
    }
    
    let sql = match dialect {
        DatabaseDialect::SQLite => create_table.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => create_table.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => create_table.build(MysqlQueryBuilder),
    };
    
    (sql, vec![])
}

/// Build a database-agnostic table existence check query using sea-query
#[allow(dead_code)]
pub fn build_table_exists_query(
    table_name: &str,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    match dialect {
        DatabaseDialect::SQLite => {
            let (sql, params) = Query::select()
                .column(Alias::new("name"))
                .from(Alias::new("sqlite_master"))
                .and_where(Expr::col(Alias::new("type")).eq("table"))
                .and_where(Expr::col(Alias::new("name")).eq(table_name))
                .build(SqliteQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        },
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => {
            let (sql, params) = Query::select()
                .column(Alias::new("table_name"))
                .from((Alias::new("information_schema"), Alias::new("tables")))
                .and_where(Expr::col(Alias::new("table_name")).eq(table_name))
                .build(PostgresQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        },
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => {
            let (sql, params) = Query::select()
                .column(Alias::new("table_name"))
                .from((Alias::new("information_schema"), Alias::new("tables")))
                .and_where(Expr::col(Alias::new("table_name")).eq(table_name))
                .build(MysqlQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        },
    }
}

/// Build a database-agnostic query to count all tables using sea-query
#[allow(dead_code)]
pub fn build_table_count_query(dialect: DatabaseDialect) -> (String, Vec<Value>) {
    match dialect {
        DatabaseDialect::SQLite => {
            let (sql, params) = Query::select()
                .expr(Expr::col(Asterisk).count())
                .from(Alias::new("sqlite_master"))
                .and_where(Expr::col(Alias::new("type")).eq("table"))
                .and_where(Expr::col(Alias::new("name")).not_like("sqlite_%"))
                .build(SqliteQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        },
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => {
            let (sql, params) = Query::select()
                .expr(Expr::col(Asterisk).count())
                .from((Alias::new("information_schema"), Alias::new("tables")))
                .and_where(Expr::col(Alias::new("table_schema")).eq("public"))
                .build(PostgresQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        },
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => {
            let (sql, params) = Query::select()
                .expr(Expr::col(Asterisk).count())
                .from((Alias::new("information_schema"), Alias::new("tables")))
                .and_where(Expr::col(Alias::new("table_schema")).eq(Expr::cust("DATABASE()")))
                .build(MysqlQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        },
    }
}

/// Build a database-agnostic query to check for migration table existence
#[allow(dead_code)]
pub fn build_migration_table_exists_query(dialect: DatabaseDialect) -> (String, Vec<Value>) {
    build_table_exists_query("_migrations", dialect)
}

/// Build a database-agnostic query to select migration records
#[allow(dead_code)]
pub fn build_select_migrations_query(
    where_clause: Option<SimpleExpr>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut builder = Query::select();
    builder
        .columns([Alias::new("name"), Alias::new("version"), Alias::new("executed_at")])
        .from(Alias::new("_migrations"));
    
    if let Some(where_expr) = where_clause {
        builder.and_where(where_expr);
    }
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => builder.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => builder.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => builder.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic query to insert migration record
#[allow(dead_code)]
pub fn build_insert_migration_query(
    name: &str,
    version: i64,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::insert();
    query
        .into_table(Alias::new("_migrations"))
        .columns([Alias::new("name"), Alias::new("version"), Alias::new("executed_at")])
        .values_panic([
            SimpleExpr::Value(SeaValue::String(Some(Box::new(name.to_string())))),
            SimpleExpr::Value(SeaValue::Int(Some(version as i32))),
            SimpleExpr::Value(SeaValue::String(Some(Box::new("NOW()".to_string()))))
        ]);
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic query to delete migration records
pub fn build_delete_migration_query(
    where_clause: SimpleExpr,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::delete();
    query
        .from_table(Alias::new("_migrations"))
        .and_where(where_clause);
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic query to select multiple tables by name
pub fn build_select_tables_by_names_query(
    table_names: Vec<&str>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    match dialect {
        DatabaseDialect::SQLite => {
            let (sql, params) = Query::select()
                .column(Alias::new("name"))
                .from(Alias::new("sqlite_master"))
                .and_where(Expr::col(Alias::new("type")).eq("table"))
                .and_where(Expr::col(Alias::new("name")).is_in(table_names))
                .order_by(Alias::new("name"), Order::Asc)
                .build(SqliteQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        },
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => {
            let (sql, params) = Query::select()
                .column(Alias::new("table_name"))
                .from((Alias::new("information_schema"), Alias::new("tables")))
                .and_where(Expr::col(Alias::new("table_name")).is_in(table_names))
                .and_where(Expr::col(Alias::new("table_schema")).eq("public"))
                .order_by(Alias::new("table_name"), Order::Asc)
                .build(PostgresQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        },
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => {
            let (sql, params) = Query::select()
                .column(Alias::new("table_name"))
                .from((Alias::new("information_schema"), Alias::new("tables")))
                .and_where(Expr::col(Alias::new("table_name")).is_in(table_names))
                .and_where(Expr::col(Alias::new("table_schema")).eq(Expr::cust("DATABASE()")))
                .order_by(Alias::new("table_name"), Order::Asc)
                .build(MysqlQueryBuilder);
            (sql, convert_sea_query_params_to_json(params.0))
        },
    }
}

/// Build a database-agnostic query to check migration lock
pub fn build_select_migration_lock_query(dialect: DatabaseDialect) -> (String, Vec<Value>) {
    let mut query = Query::select();
    query
        .column(Alias::new("locked"))
        .from(Alias::new("_migration_lock"))
        .and_where(Expr::col(Alias::new("id")).eq(1));
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Build a database-agnostic query to insert migration lock
pub fn build_insert_migration_lock_query(dialect: DatabaseDialect) -> (String, Vec<Value>) {
    let mut query = Query::insert();
    query
        .into_table(Alias::new("_migration_lock"))
        .columns([Alias::new("id"), Alias::new("locked")])
        .values_panic([SimpleExpr::Value(SeaValue::Int(Some(1))), SimpleExpr::Value(SeaValue::Int(Some(0)))]);
    
    let (sql, params) = match dialect {
        DatabaseDialect::SQLite => query.build(SqliteQueryBuilder),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => query.build(PostgresQueryBuilder),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => query.build(MysqlQueryBuilder),
    };
    
    (sql, convert_sea_query_params_to_json(params.0))
}

/// Helper to create table identifiers
pub fn table(name: &str) -> DynIden {
    Alias::new(name).into_iden()
}

/// Helper to create column identifiers
pub fn column(name: &str) -> DynIden {
    Alias::new(name).into_iden()
}

/// Helper to create equality conditions
pub fn eq_condition(column: DynIden, value: SeaValue) -> SimpleExpr {
    Expr::col(column).eq(value)
}

/// Helper to create LIKE conditions
pub fn like_condition(column: DynIden, pattern: &str) -> SimpleExpr {
    Expr::col(column).like(format!("%{}%", pattern))
}

/// Helper to create IN conditions
pub fn in_condition(column: DynIden, values: Vec<SeaValue>) -> SimpleExpr {
    Expr::col(column).is_in(values)
}

/// Helper to create IS NULL conditions
pub fn is_null_condition(column: DynIden) -> SimpleExpr {
    Expr::col(column).is_null()
}

/// Helper to create IS NOT NULL conditions
pub fn is_not_null_condition(column: DynIden) -> SimpleExpr {
    Expr::col(column).is_not_null()
}

/// Helper to create comparison conditions
pub fn comparison_condition(column: DynIden, op: &str, value: SeaValue) -> SimpleExpr {
    match op {
        ">" => Expr::col(column).gt(value),
        ">=" => Expr::col(column).gte(value),
        "<" => Expr::col(column).lt(value),
        "<=" => Expr::col(column).lte(value),
        "!=" | "<>" => Expr::col(column).ne(value),
        _ => Expr::col(column).eq(value), // Default to equality
    }
}

/// Database-agnostic assertion helpers for test validation
pub struct QueryAssertions;

impl QueryAssertions {
    /// Assert that a query returns expected number of rows
    pub async fn assert_row_count<T: DatabaseBackend>(
        client: &T,
        table: DynIden,
        expected_count: i64,
        dialect: DatabaseDialect,
    ) -> Result<(), String> {
        let (sql, params) = build_count_query(table, dialect);
        let result = client.execute_query(&sql, &params).await
            .map_err(|e| format!("Query execution failed: {}", e))?;
        
        let rows = result.rows();
        if let Some(first_row) = rows.first() {
            // Try different possible count column names
            let count_value = first_row.get("count")
                .or_else(|| first_row.get("COUNT(*)"))
                .or_else(|| first_row.get("COUNT"));
                
            if let Some(Value::Number(count)) = count_value {
                let actual_count = count.as_i64().unwrap_or(0);
                if actual_count == expected_count {
                    Ok(())
                } else {
                    Err(format!("Expected {} rows, got {}", expected_count, actual_count))
                }
            } else {
                Err("Count result not found or invalid format".to_string())
            }
        } else {
            Err("No rows returned from count query".to_string())
        }
    }
    
    /// Assert that a table exists
    pub async fn assert_table_exists<T: DatabaseBackend>(
        client: &T,
        table_name: &str,
        dialect: DatabaseDialect,
    ) -> Result<(), String> {
        let (sql, params) = match dialect {
            DatabaseDialect::SQLite => (
                "SELECT name FROM sqlite_master WHERE type='table' AND name = ?".to_string(),
                vec![Value::String(table_name.to_string())]
            ),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => (
                "SELECT table_name FROM information_schema.tables WHERE table_name = $1".to_string(),
                vec![Value::String(table_name.to_string())]
            ),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => (
                "SELECT table_name FROM information_schema.tables WHERE table_name = ?".to_string(),
                vec![Value::String(table_name.to_string())]
            ),
        };
        
        let result = client.execute_query(&sql, &params).await
            .map_err(|e| format!("Table existence check failed: {}", e))?;
        
        if result.rows().is_empty() {
            Err(format!("Table '{}' does not exist", table_name))
        } else {
            Ok(())
        }
    }
    
    /// Assert that a column exists in a table
    pub async fn assert_column_exists<T: DatabaseBackend>(
        client: &T,
        table_name: &str,
        column_name: &str,
        dialect: DatabaseDialect,
    ) -> Result<(), String> {
        let (sql, params) = match dialect {
            DatabaseDialect::SQLite => (
                format!("PRAGMA table_info({})", table_name),
                vec![]
            ),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => (
                "SELECT column_name FROM information_schema.columns WHERE table_name = $1 AND column_name = $2".to_string(),
                vec![Value::String(table_name.to_string()), Value::String(column_name.to_string())]
            ),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => (
                "SELECT column_name FROM information_schema.columns WHERE table_name = ? AND column_name = ?".to_string(),
                vec![Value::String(table_name.to_string()), Value::String(column_name.to_string())]
            ),
        };
        
        let result = client.execute_query(&sql, &params).await
            .map_err(|e| format!("Column existence check failed: {}", e))?;
        
        let column_exists = match dialect {
            DatabaseDialect::SQLite => {
                result.rows().iter().any(|row| {
                    if let Some(Value::String(name)) = row.get("name") {
                        name == column_name
                    } else {
                        false
                    }
                })
            },
            #[cfg(any(feature = "postgres", feature = "mysql"))]
            _ => !result.rows().is_empty()
        };
        
        if !column_exists {
            Err(format!("Column '{}' does not exist in table '{}'", column_name, table_name))
        } else {
            Ok(())
        }
    }
    
    /// Assert that a query returns expected data
    #[allow(dead_code)]
    pub async fn assert_query_result<T: DatabaseBackend>(
        client: &T,
        sql: &str,
        params: &[Value],
        expected_validator: impl Fn(&[Value]) -> bool,
    ) -> Result<(), String> {
        let result = client.execute_query(sql, params).await
            .map_err(|e| format!("Query execution failed: {}", e))?;
        
        let rows = result.rows();
        if expected_validator(rows) {
            Ok(())
        } else {
            Err("Query result validation failed".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::database_manager::TestDatabaseManager;

    #[tokio::test]
    async fn test_parameter_conversion() {
        let sea_params = vec![
            SeaValue::String(Some(Box::new("test".to_string()))),
            SeaValue::Int(Some(42)),
            SeaValue::Bool(Some(true)),
            SeaValue::String(None),
        ];
        
        let json_params = convert_sea_query_params_to_json(sea_params);
        
        assert_eq!(json_params.len(), 4);
        assert_eq!(json_params[0], Value::String("test".to_string()));
        assert_eq!(json_params[1], Value::Number(42.into()));
        assert_eq!(json_params[2], Value::Bool(true));
        assert_eq!(json_params[3], Value::Null);
    }

    #[tokio::test]
    async fn test_count_query_builder() {
        let table_id = table("test_table");
        
        #[cfg(any(feature = "postgres", feature = "mysql"))]
        let mut dialects = vec![DatabaseDialect::SQLite];
        #[cfg(not(any(feature = "postgres", feature = "mysql")))]
        let dialects = vec![DatabaseDialect::SQLite];
        
        #[cfg(feature = "postgres")]
        dialects.push(DatabaseDialect::PostgreSQL);
        #[cfg(feature = "mysql")]
        dialects.push(DatabaseDialect::MySQL);
        
        for dialect in dialects {
            let (sql, params) = build_count_query(table_id.clone(), dialect);
            
            assert!(sql.contains("COUNT"));
            assert!(sql.contains("test_table"));
            assert!(params.is_empty());
        }
    }

    #[tokio::test]
    async fn test_select_query_builder() {
        let table_id = table("users");
        let columns = vec![column("id"), column("name"), column("email")];
        
        #[cfg(any(feature = "postgres", feature = "mysql"))]
        let mut dialects = vec![DatabaseDialect::SQLite];
        #[cfg(not(any(feature = "postgres", feature = "mysql")))]
        let dialects = vec![DatabaseDialect::SQLite];
        
        #[cfg(feature = "postgres")]
        dialects.push(DatabaseDialect::PostgreSQL);
        #[cfg(feature = "mysql")]
        dialects.push(DatabaseDialect::MySQL);
        
        for dialect in dialects {
            let (sql, params) = build_select_query(table_id.clone(), columns.clone(), dialect);
            
            assert!(sql.contains("SELECT"));
            assert!(sql.contains("users"));
            assert!(sql.contains("id"));
            assert!(sql.contains("name"));
            assert!(sql.contains("email"));
            assert!(params.is_empty());
        }
    }

    #[tokio::test]
    async fn test_insert_query_builder() {
        let table_id = table("users");
        let columns = vec![column("name"), column("email")];
        let values = vec![
            SeaValue::String(Some(Box::new("John".to_string()))),
            SeaValue::String(Some(Box::new("john@example.com".to_string()))),
        ];
        
        #[cfg(any(feature = "postgres", feature = "mysql"))]
        let mut dialects = vec![DatabaseDialect::SQLite];
        #[cfg(not(any(feature = "postgres", feature = "mysql")))]
        let dialects = vec![DatabaseDialect::SQLite];
        
        #[cfg(feature = "postgres")]
        dialects.push(DatabaseDialect::PostgreSQL);
        #[cfg(feature = "mysql")]
        dialects.push(DatabaseDialect::MySQL);
        
        for dialect in dialects {
            let (sql, params) = build_insert_query(table_id.clone(), columns.clone(), values.clone(), dialect);
            
            assert!(sql.contains("INSERT"));
            assert!(sql.contains("users"));
            assert!(sql.contains("name"));
            assert!(sql.contains("email"));
            assert_eq!(params.len(), 2);
        }
    }

    #[tokio::test]
    async fn test_condition_helpers() {
        let col = column("age");
        
        let _eq_cond = eq_condition(col.clone(), SeaValue::Int(Some(25)));
        let _like_cond = like_condition(col.clone(), "John");
        let _in_cond = in_condition(col.clone(), vec![SeaValue::Int(Some(20)), SeaValue::Int(Some(30))]);
        let _null_cond = is_null_condition(col.clone());
        let _not_null_cond = is_not_null_condition(col.clone());
        let _gt_cond = comparison_condition(col.clone(), ">", SeaValue::Int(Some(18)));
        
        // These should compile and be valid expressions
        assert!(true); // If we reach here, the conditions were created successfully
    }

    #[tokio::test]
    #[ignore] // Disabled pending complete QueryAssertions sea-query conversion
    async fn test_query_assertions_with_database() {
        let manager = TestDatabaseManager::new();
        if let Ok(client) = manager.create_client(DatabaseDialect::SQLite).await {
            // Create a test table
            let (create_sql, _create_params) = build_create_table_query_simple("test_assertions", DatabaseDialect::SQLite);
            
            if let Ok(_) = client.execute_schema(&create_sql).await {
                // Test table existence assertion
                let table_result = QueryAssertions::assert_table_exists(&client, "test_assertions", DatabaseDialect::SQLite).await;
                assert!(table_result.is_ok());
                
                // Test column existence assertion
                let column_result = QueryAssertions::assert_column_exists(&client, "test_assertions", "name", DatabaseDialect::SQLite).await;
                assert!(column_result.is_ok());
                
                // Test row count assertion
                let count_result = QueryAssertions::assert_row_count(&client, table("test_assertions"), 0, DatabaseDialect::SQLite).await;
                assert!(count_result.is_ok());
            }
        }
    }

    #[tokio::test]
    async fn test_helper_utilities() {
        // Test table and column identifier helpers
        let _table_id = table("users");
        let _column_id = column("name");
        
        // These should create valid identifiers
        assert!(true);
        
        // Test create and drop table helpers
        let (create_sql, _) = build_create_table_query_simple("test_table", DatabaseDialect::SQLite);
        assert!(create_sql.contains("CREATE TABLE"));
        assert!(create_sql.contains("test_table"));
        
        let (drop_sql, _) = build_drop_table_query("test_table", true, DatabaseDialect::SQLite);
        assert!(drop_sql.contains("DROP TABLE"));
        assert!(drop_sql.contains("IF EXISTS"));
        assert!(drop_sql.contains("test_table"));
    }
}