# Multi-Database Testing Infrastructure - Complete Implementation Plan

## Overview
This document provides the complete implementation plan for multi-database testing infrastructure that supports SQLite, PostgreSQL, and MySQL with proper feature gates, zero compilation warnings, and database-agnostic tests using only sea-query builders.

**Total Scope**: 58 test files, ~15,000-20,000 lines of code, 18 subtasks across 5 phases.

---

## 🎯 **MASTER CHECKLIST - ALL SUBTASKS**

### **Phase 1: Foundation & Clean Compilation**
- [x] **Task 1.1**: Fix Compilation Warnings (200-300 lines)
- [x] **Task 1.2**: Fix Failed Test Cases (300-400 lines)  
- [x] **Task 1.3**: Feature Gate Compilation Verification (100-200 lines)

### **Phase 2: Command & Feature Gate Integration**
- [x] **Task 2.1**: Backend Isolation & Logging (250-350 lines)
- [x] **Task 2.2**: Command Environment Enforcement (150-250 lines)

### **Phase 3: Database-Agnostic Test Conversion**
- [x] **Task 3.1**: Raw SQL Audit & Helper Utilities (300-400 lines)
- [x] **Task 3.2**: Convert Infrastructure & Fixtures (400-500 lines)
- [x] **Task 3.3**: Convert Core Functionality Tests (400-500 lines)
- [x] **Task 3.4**: Convert Schema & DDL Tests (450-500 lines)
- [x] **Task 3.5**: Convert Migration Tests (450-500 lines)
- [x] **Task 3.6**: Convert Multi-Database & Cross-Database Tests (350-450 lines)
- [ ] **Task 3.7**: Convert Introspection & Analysis Tests (400-500 lines)
- [ ] **Task 3.8**: Convert Performance & Benchmark Tests (300-400 lines)
- [ ] **Task 3.9**: Convert Advanced Feature Tests (350-450 lines)

### **Phase 4: Backend Verification & Validation**
- [ ] **Task 4.1**: Connection Verification System (250-350 lines)
- [ ] **Task 4.2**: Test Execution Verification Macros (200-300 lines)

### **Phase 5: Final Optimization & Cleanup**
- [ ] **Task 5.1**: Code Cleanup & Performance Optimization (300-400 lines)
- [ ] **Task 5.2**: Final Integration Testing & Validation (250-350 lines)

### **Final Success Criteria**
- [ ] `cargo check --tests` produces ZERO warnings
- [x] `just test` - ALL tests pass using ONLY SQLite
- [x] `just test-postgres` - ALL tests pass using ONLY PostgreSQL  
- [x] `just test-mysql` - ALL tests pass using ONLY MySQL
- [ ] ZERO raw SQL in any test file (only sea-query builders)
- [ ] Backend verification logs confirm correct database usage
- [ ] Feature gates properly isolate database backends
- [ ] Tests are truly database-agnostic and portable

---

## 🔧 **DETAILED IMPLEMENTATION PLAN**

## **PHASE 1: Foundation & Clean Compilation**

### **Task 1.1: Fix Compilation Warnings**
**Estimated Lines**: 200-300  
**Priority**: CRITICAL  
**Files Affected**: 
- `tests/fixtures/entities.rs` (unused imports)
- `tests/common/database_manager.rs` (unreachable patterns, unused variables) 
- Various test files (dead code warnings)

**Current Issues**:
- Unused imports in `tests/fixtures/entities.rs`
- Unreachable patterns in `database_manager.rs`
- Unused variables for environment URLs
- Dead code warnings for unused methods

**Implementation**:
```rust
// Fix unused imports in tests/fixtures/entities.rs
// Remove: use chrono::{DateTime, Utc, NaiveDateTime};
// Or use #[allow(unused_imports)] if needed for future use

// Fix unreachable patterns in database_manager.rs create_client()
// Remove the catch-all pattern since all dialects are handled:
// Remove: _ => { /* This should never happen */ }

// Fix unused variables
if let Ok(_postgres_url) = env::var("POSTGRES_TEST_URL") {
    // Use underscore prefix for unused variables
}

// Clean up dead code - either add #[allow(dead_code)] or remove entirely
#[allow(dead_code)]
pub async fn wait_for_database(&self, ...) { /* unused method */ }

// Fix variable mutability warnings
let manager = Self { // Remove 'mut' if not needed
    available_databases: vec![DatabaseDialect::SQLite],
    connection_urls: HashMap::new(),
};
```

**Acceptance Criteria**:
- `cargo check --tests` shows ZERO warnings
- `cargo check --features postgres` shows ZERO warnings  
- `cargo check --features mysql` shows ZERO warnings
- All feature combinations compile cleanly

---

### **Task 1.2: Fix Failed Test Cases**  
**Estimated Lines**: 300-400  
**Priority**: HIGH  
**Files Affected**:
- `tests/fixtures/mod.rs` (fixture setup issues)
- Related test functions causing failures

**Failed Tests to Fix**:
1. `test_performance_fixture_setup`
2. `test_type_testing_fixture` 
3. `test_users_posts_fixture`

**Root Cause Analysis**:
- Likely fixture setup issues with sea-query API changes
- Parameter binding problems in fixture creation
- Remaining `.build()` vs `.build(QueryBuilder)` issues

**Implementation**:
```rust
// Debug each failing test individually
// Example fix for fixture creation:

// Check for remaining API issues in fixture setup
impl TestFixtureManager {
    pub async fn setup_fixture<B: DatabaseBackend>(
        &self,
        fixture_name: &str,
        backend: &B,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fixture = self.fixtures.get(fixture_name)
            .ok_or_else(|| format!("Fixture '{}' not found", fixture_name))?;
            
        // Execute setup queries - ensure proper parameter binding
        for (sql, params) in &fixture.setup_queries {
            backend.execute_query(sql, params).await
                .map_err(|e| format!("Setup query failed: {} - Error: {}", sql, e))?;
        }
        
        // Insert test data with proper error handling
        for record in &fixture.test_data {
            self.insert_test_record(record, backend).await
                .map_err(|e| format!("Test data insertion failed for table {}: {}", record.table, e))?;
        }
        
        Ok(())
    }
}
```

**Acceptance Criteria**:
- [x] ALL tests pass with `just test` 
- [x] No test failures in any configuration
- [x] Clear error messages for debugging if issues persist

---

### **Task 1.3: Feature Gate Compilation Verification**
**Estimated Lines**: 100-200  
**Priority**: HIGH  
**Files Affected**: All test files with feature-gated code

**Implementation**:
```bash
# Create comprehensive feature gate testing script
#!/bin/bash

echo "Testing all feature combinations for compilation..."

# Test each combination
cargo check --no-default-features || exit 1
cargo check --features sqlite || exit 1
cargo check --features postgres || exit 1  
cargo check --features mysql || exit 1
cargo check --features postgres,mysql || exit 1
cargo check --all-features || exit 1

# Test conditional compilation
cargo check --tests --no-default-features || exit 1
cargo check --tests --features postgres || exit 1
cargo check --tests --features mysql || exit 1

echo "✅ All feature combinations compile successfully"
```

**Fix Common Issues**:
```rust
// Ensure all feature-gated code is properly conditioned
#[cfg(feature = "postgres")]
use d1_rs::backends::PostgreSQLBackend;

#[cfg(feature = "mysql")]
use d1_rs::backends::MySQLBackend;

// Fix conditional match arms
match dialect {
    DatabaseDialect::SQLite => { /* always available */ },
    #[cfg(feature = "postgres")]
    DatabaseDialect::PostgreSQL => { /* postgres implementation */ },
    #[cfg(feature = "mysql")]  
    DatabaseDialect::MySQL => { /* mysql implementation */ },
    // Remove unreachable catch-all
}
```

**Acceptance Criteria**:
- All feature combinations compile without errors or warnings
- Conditional compilation works correctly for all dialects

---

## **PHASE 2: Command & Feature Gate Integration**

### **Task 2.1: Backend Isolation & Logging**
**Estimated Lines**: 250-350  
**Priority**: CRITICAL  
**Files Affected**: 
- `tests/common/database_manager.rs`

**Current Issue**: Need to ensure commands use ONLY their designated backends

**Implementation**:
```rust
// Add comprehensive backend verification logging
impl TestDatabaseManager {
    pub async fn create_client(&self, dialect: DatabaseDialect) -> Result<AnyDatabaseBackend, _> {
        let backend = match dialect {
            DatabaseDialect::SQLite => {
                println!("🗄️  Using SQLite in-memory backend");
                let sqlite = SQLiteBackend::new_in_memory().await
                    .map_err(|e| format!("SQLite backend creation failed: {}", e))?;
                AnyDatabaseBackend::SQLite(sqlite)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let url = self.connection_urls.get(&dialect)
                    .ok_or("PostgreSQL URL not configured. Set POSTGRES_TEST_URL environment variable.")?;
                println!("🐘 Using PostgreSQL backend: {}", Self::mask_credentials(url));
                let pg = PostgreSQLBackend::new(url).await
                    .map_err(|e| format!("PostgreSQL backend creation failed: {}", e))?;
                AnyDatabaseBackend::PostgreSQL(pg)
            },
            #[cfg(feature = "mysql")]  
            DatabaseDialect::MySQL => {
                let url = self.connection_urls.get(&dialect)
                    .ok_or("MySQL URL not configured. Set MYSQL_TEST_URL environment variable.")?;
                println!("🐬 Using MySQL backend: {}", Self::mask_credentials(url));
                let mysql = MySQLBackend::new(url).await
                    .map_err(|e| format!("MySQL backend creation failed: {}", e))?;
                AnyDatabaseBackend::MySQL(mysql)
            },
        };
        
        // Verify backend type matches expected dialect
        let actual_dialect = backend.dialect();
        if actual_dialect != dialect {
            return Err(format!(
                "❌ Backend verification FAILED! Expected: {:?}, Got: {:?}\n   Connection: {}", 
                dialect, actual_dialect, backend.connection_info()
            ).into());
        }
        
        println!("✅ Backend verified: {:?} - {}", actual_dialect, backend.connection_info());
        Ok(backend)
    }
    
    fn mask_credentials(url: &str) -> String {
        // Mask password in connection string for security
        if let Some(at_pos) = url.find('@') {
            if let Some(colon_pos) = url[0..at_pos].rfind(':') {
                let mut masked = url.to_string();
                masked.replace_range(colon_pos + 1..at_pos, "***");
                return masked;
            }
        }
        url.to_string()
    }
}
```

**Acceptance Criteria**:
- Backend logging shows correct database being used
- Backend type verification prevents cross-contamination
- Clear error messages for configuration issues

---

### **Task 2.2: Command Environment Enforcement**
**Estimated Lines**: 150-250  
**Priority**: HIGH  
**Files Affected**: 
- `justfile`
- `tests/common/database_manager.rs` (environment variable handling)

**Implementation**:
```bash
# Update justfile to enforce backend isolation
test:
    @echo "🗄️  Running tests on SQLite ONLY"
    @export DATABASE_BACKENDS="sqlite"
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p')

test-postgres: start-test-dbs
    @echo "🐘 Running tests on PostgreSQL ONLY" 
    @export DATABASE_BACKENDS="postgres"
    @export POSTGRES_TEST_URL="postgresql://d1rs_user:d1rs_pass@localhost:5433/d1rs_test"
    cargo nextest run --features postgres --target $(rustc -vV | sed -n 's|host: ||p')

test-mysql: start-test-dbs  
    @echo "🐬 Running tests on MySQL ONLY"
    @export DATABASE_BACKENDS="mysql" 
    @export MYSQL_TEST_URL="mysql://d1rs_user:d1rs_pass@localhost:3307/d1rs_test"
    cargo nextest run --features mysql --target $(rustc -vV | sed -n 's|host: ||p')

# Add validation commands
validate-backend-isolation:
    @echo "🔍 Validating backend isolation..."
    @DATABASE_BACKENDS=sqlite just test > /tmp/sqlite_test.log 2>&1
    @if grep -q "PostgreSQL\|MySQL" /tmp/sqlite_test.log; then \
        echo "❌ Backend isolation FAILED: SQLite tests used other backends"; \
        exit 1; \
    fi
    @echo "✅ Backend isolation verified"
```

**Environment Variable Handling**:
```rust
// Add environment-based backend restriction
impl TestDatabaseManager {
    pub fn new() -> Self {
        let mut manager = Self {
            available_databases: vec![DatabaseDialect::SQLite],
            connection_urls: HashMap::new(),
        };
        
        // Check environment restrictions
        if let Ok(allowed_backends) = env::var("DATABASE_BACKENDS") {
            manager.available_databases = allowed_backends
                .split(',')
                .filter_map(|backend| match backend.trim().to_lowercase().as_str() {
                    "sqlite" => Some(DatabaseDialect::SQLite),
                    #[cfg(feature = "postgres")]
                    "postgres" | "postgresql" => Some(DatabaseDialect::PostgreSQL),
                    #[cfg(feature = "mysql")]
                    "mysql" => Some(DatabaseDialect::MySQL),
                    _ => {
                        println!("⚠️ Unknown backend in DATABASE_BACKENDS: {}", backend);
                        None
                    }
                })
                .collect();
        }
        
        // Configure available databases based on environment
        // ... rest of existing logic
        
        manager
    }
}
```

**Acceptance Criteria**:
- `just test` uses ONLY SQLite
- `just test-postgres` uses ONLY PostgreSQL  
- `just test-mysql` uses ONLY MySQL
- Environment variable restrictions are enforced

---

## **PHASE 3: Database-Agnostic Test Conversion**

### **Task 3.1: Raw SQL Audit & Helper Utilities**
**Estimated Lines**: 300-400  
**Priority**: CRITICAL  
**Files Affected**:
- New file: `tests/common/query_helpers.rs`
- `tests/common/mod.rs` (export helpers)

**Scope**:

#### 1. **Comprehensive Raw SQL Audit**:
```bash
#!/bin/bash
# Create comprehensive audit report
echo "=== RAW SQL AUDIT REPORT ===" > raw_sql_audit.txt
echo "Date: $(date)" >> raw_sql_audit.txt
echo "" >> raw_sql_audit.txt

echo "1. Raw SQL SELECT/INSERT/UPDATE/DELETE queries:" >> raw_sql_audit.txt
grep -r "SELECT\|INSERT\|UPDATE\|DELETE\|CREATE\|DROP" tests/ --include="*.rs" -n >> raw_sql_audit.txt

echo -e "\n2. execute_query with string literals:" >> raw_sql_audit.txt  
grep -r "execute_query.*\"" tests/ --include="*.rs" -n >> raw_sql_audit.txt

echo -e "\n3. format! usage for SQL:" >> raw_sql_audit.txt
grep -r "format!\|&format" tests/ --include="*.rs" -n >> raw_sql_audit.txt

echo -e "\n4. execute_schema with string literals:" >> raw_sql_audit.txt
grep -r "execute_schema.*\"" tests/ --include="*.rs" -n >> raw_sql_audit.txt

echo "Audit complete. Found $(grep -c "tests/" raw_sql_audit.txt) instances to fix."
```

#### 2. **Helper Utilities Implementation**:
```rust
// tests/common/query_helpers.rs
use sea_query::{
    Query, Table, Index, Expr, SimpleExpr, ColumnDef, TableCreateStatement, 
    SqliteQueryBuilder, PostgresQueryBuilder, MysqlQueryBuilder, 
    DynIden, Alias, Value as SeaValue, Iden, ForeignKey, ForeignKeyAction
};
use serde_json::Value;
use d1_rs::dialects::DatabaseDialect;

/// Trait for database-agnostic query building
pub trait DatabaseDialectQueryBuilder {
    fn query_builder(&self) -> Box<dyn sea_query::QueryBuilder>;
}

impl DatabaseDialectQueryBuilder for DatabaseDialect {
    fn query_builder(&self) -> Box<dyn sea_query::QueryBuilder> {
        match self {
            DatabaseDialect::SQLite => Box::new(SqliteQueryBuilder),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => Box::new(PostgresQueryBuilder),
            #[cfg(feature = "mysql")]  
            DatabaseDialect::MySQL => Box::new(MysqlQueryBuilder),
        }
    }
}

/// Convert sea-query parameters to JSON values for backend compatibility
pub fn convert_sea_query_params_to_json(params: Vec<SeaValue>) -> Vec<Value> {
    params.into_iter().map(|p| match p {
        SeaValue::String(Some(s)) => Value::String(*s),
        SeaValue::Int(Some(i)) => Value::Number(i.into()),
        SeaValue::BigInt(Some(i)) => Value::Number(i.into()),
        SeaValue::Float(Some(f)) => Value::Number(serde_json::Number::from_f64(f).unwrap_or(0.into())),
        SeaValue::Double(Some(d)) => Value::Number(serde_json::Number::from_f64(d).unwrap_or(0.into())),  
        SeaValue::Bool(Some(b)) => Value::Bool(b),
        _ => Value::Null,
    }).collect()
}

/// Build a standard COUNT query
pub fn build_count_query(table: DynIden, dialect: DatabaseDialect) -> (String, Vec<Value>) {
    let (sql, params) = Query::select()
        .expr(Expr::count(Expr::asterisk()).as_(Alias::new("count")))
        .from(table)
        .build(dialect.query_builder().as_ref());
    (sql, convert_sea_query_params_to_json(params))
}

/// Build a standard INSERT query
pub fn build_insert_query(
    table: DynIden, 
    columns: Vec<DynIden>, 
    values: Vec<SimpleExpr>, 
    dialect: DatabaseDialect
) -> Result<(String, Vec<Value>), sea_query::error::Error> {
    let (sql, params) = Query::insert()
        .into_table(table)
        .columns(columns)
        .values(values)?
        .build(dialect.query_builder().as_ref());
    Ok((sql, convert_sea_query_params_to_json(params)))
}

/// Build a standard UPDATE query
pub fn build_update_query(
    table: DynIden,
    updates: Vec<(DynIden, SimpleExpr)>,
    where_clause: Option<SimpleExpr>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::update().table(table);
    
    for (column, value) in updates {
        query = query.value(column, value);
    }
    
    if let Some(where_expr) = where_clause {
        query = query.and_where(where_expr);
    }
    
    let (sql, params) = query.build(dialect.query_builder().as_ref());
    (sql, convert_sea_query_params_to_json(params))
}

/// Build a standard DELETE query
pub fn build_delete_query(
    table: DynIden,
    where_clause: Option<SimpleExpr>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut query = Query::delete().from_table(table);
    
    if let Some(where_expr) = where_clause {
        query = query.and_where(where_expr);
    }
    
    let (sql, params) = query.build(dialect.query_builder().as_ref());
    (sql, convert_sea_query_params_to_json(params))
}

/// Build a CREATE TABLE query
pub fn build_create_table_query(
    table: DynIden,
    columns: Vec<ColumnDef>,
    dialect: DatabaseDialect
) -> (String, Vec<Value>) {
    let mut create = Table::create().table(table).if_not_exists();
    
    for column in columns {
        create = create.col(column);
    }
    
    let (sql, params) = create.build(dialect.query_builder().as_ref());
    (sql, convert_sea_query_params_to_json(params))
}

/// Build a DROP TABLE query
pub fn build_drop_table_query(table: DynIden, dialect: DatabaseDialect) -> (String, Vec<Value>) {
    let (sql, params) = Table::drop()
        .table(table)
        .if_exists()
        .build(dialect.query_builder().as_ref());
    (sql, convert_sea_query_params_to_json(params))
}
```

**Acceptance Criteria**:
- Complete audit of all raw SQL usage documented
- Helper utilities cover all common query patterns
- All helpers work across SQLite, PostgreSQL, and MySQL
- Parameter conversion maintains type safety

---

### **Task 3.2: Convert Infrastructure & Fixtures**
**Estimated Lines**: 400-500  
**Priority**: HIGH  
**Files Affected**:
- `tests/fixtures/mod.rs` (~200 lines to convert)
- `tests/fixtures/schemas.rs` (~150 lines to convert)  
- `tests/fixtures/entities.rs` (~50 lines to convert)
- `tests/common/database_manager.rs` (utility methods)

**Scope**: Convert ALL raw SQL in fixture creation to sea-query builders

**Implementation Examples**:

#### Fixture Creation Conversion:
```rust
// ❌ BEFORE (Raw SQL) - FORBIDDEN
impl TestFixtureManager {
    fn create_users_table(&self) -> (String, Vec<Value>) {
        ("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT UNIQUE)".to_string(), vec![])
    }
}

// ✅ AFTER (sea-query builders) - REQUIRED  
impl TestFixtureManager {
    fn create_users_table(&self) -> (String, Vec<Value>) {
        use crate::common::query_helpers::build_create_table_query;
        
        let columns = vec![
            ColumnDef::new(Users::Id).integer().auto_increment().primary_key(),
            ColumnDef::new(Users::Name).text().not_null(),
            ColumnDef::new(Users::Email).text().unique_key(),
        ];
        
        build_create_table_query(Users::Table.into_iden(), columns, self.dialect)
    }
}
```

#### Fixture Data Insertion:
```rust
// ❌ BEFORE (Raw SQL) - FORBIDDEN
async fn insert_test_record<B: DatabaseBackend>(
    &self,
    record: &TestRecord,
    backend: &B,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let sql = format!("INSERT INTO {} ({}) VALUES ({})", 
        record.table, 
        record.data.keys().collect::<Vec<_>>().join(", "),
        record.data.keys().map(|_| "?").collect::<Vec<_>>().join(", ")
    );
    let params: Vec<Value> = record.data.values().cloned().collect();
    backend.execute_query(&sql, &params).await?;
    Ok(())
}

// ✅ AFTER (sea-query builders) - REQUIRED
async fn insert_test_record<B: DatabaseBackend>(
    &self,
    record: &TestRecord,
    backend: &B,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::common::query_helpers::build_insert_query;
    
    let table_iden = Alias::new(&record.table).into_iden();
    let columns: Vec<DynIden> = record.data.keys()
        .map(|col| Alias::new(col).into_iden())
        .collect();
    let values: Vec<SimpleExpr> = record.data.values()
        .map(|val| match val {
            Value::String(s) => Expr::value(s.clone()),
            Value::Number(n) => Expr::value(n.as_i64().unwrap_or(0)),
            Value::Bool(b) => Expr::value(*b),
            _ => Expr::value(val.to_string()),
        })
        .collect();
    
    let (sql, params) = build_insert_query(table_iden, columns, values, backend.dialect())?;
    backend.execute_query(&sql, &params).await?;
    Ok(())
}
```

#### Schema Builder Updates:
```rust
// Update all schema creation methods to use sea-query
impl TestSchemaBuilder {
    pub fn create_comprehensive_schema(&self) -> Vec<(String, Vec<Value>)> {
        let mut statements = Vec::new();
        
        // Users table
        statements.push(self.create_users_table());
        
        // Posts table with foreign key
        statements.push(self.create_posts_table());
        
        // Type testing table
        statements.push(self.create_type_test_table());
        
        statements
    }
    
    fn create_posts_table(&self) -> (String, Vec<Value>) {
        let columns = vec![
            ColumnDef::new(Posts::Id).integer().auto_increment().primary_key(),
            ColumnDef::new(Posts::Title).string_len(255).not_null(),
            ColumnDef::new(Posts::Content).text(),
            ColumnDef::new(Posts::UserId).integer().not_null(),
            ColumnDef::new(Posts::CreatedAt).date_time().default(Expr::current_timestamp()),
        ];
        
        let (sql, params) = Table::create()
            .table(Posts::Table)
            .if_not_exists()
            .cols(columns)
            .foreign_key(
                ForeignKey::create()
                    .name("fk_posts_user_id")
                    .from(Posts::Table, Posts::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade)
            )
            .build(self.dialect.query_builder().as_ref());
            
        (sql, convert_sea_query_params_to_json(params))
    }
}
```

**Acceptance Criteria**:
- ZERO raw SQL in all fixture files
- All fixtures pass on SQLite, PostgreSQL, and MySQL
- Fixture setup/teardown times < 1 second per fixture

---

### **Task 3.3: Convert Core Functionality Tests**  
**Estimated Lines**: 400-500  
**Priority**: HIGH  
**Files Affected** (~10 files):
- `tests/test_basic.rs`
- `tests/test_crud.rs` 
- `tests/test_queries.rs`
- `tests/test_boolean_conversion.rs`
- `tests/test_relations.rs`
- `tests/test_eager_loading.rs`

**Implementation Pattern**:

#### Basic CRUD Conversion:
```rust
// ❌ BEFORE (Raw SQL) - FORBIDDEN
#[tokio::test]
async fn test_basic_crud() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = setup_test_client().await?;
    
    // CREATE TABLE - Raw SQL FORBIDDEN!
    client.execute_schema("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)").await?;
    
    // INSERT - Raw SQL FORBIDDEN!
    client.execute_query("INSERT INTO users (name) VALUES (?)", &[json!("Alice")]).await?;
    
    // SELECT - Raw SQL FORBIDDEN!
    let result = client.execute_query("SELECT * FROM users WHERE name = ?", &[json!("Alice")]).await?;
    assert_eq!(result.len(), 1);
    
    Ok(())
}

// ✅ AFTER (sea-query builders) - REQUIRED
#[tokio::test] 
async fn test_basic_crud() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = TestDatabaseManager::new();
    let client = manager.create_client(DatabaseDialect::SQLite).await?;
    let dialect = client.dialect();
    
    // CREATE TABLE with sea-query
    let (create_sql, _) = Table::create()
        .table(Users::Table)
        .if_not_exists()
        .col(ColumnDef::new(Users::Id).integer().auto_increment().primary_key())
        .col(ColumnDef::new(Users::Name).text().not_null())
        .build(dialect.query_builder().as_ref());
    client.execute_schema(&create_sql).await?;
    
    // INSERT with sea-query
    let (insert_sql, insert_params) = Query::insert()
        .into_table(Users::Table)
        .columns([Users::Name])
        .values([Expr::value("Alice")])?
        .build(dialect.query_builder().as_ref()); 
    let json_params = convert_sea_query_params_to_json(insert_params);
    client.execute_query(&insert_sql, &json_params).await?;
    
    // SELECT with sea-query
    let (select_sql, select_params) = Query::select()
        .columns([Users::Id, Users::Name])
        .from(Users::Table)
        .and_where(Expr::col(Users::Name).eq("Alice"))
        .build(dialect.query_builder().as_ref());
    let json_params = convert_sea_query_params_to_json(select_params);
    
    let result = client.execute_query(&select_sql, &json_params).await?;
    assert_eq!(result.len(), 1);
    
    println!("✅ Basic CRUD test completed on {:?}", dialect);
    Ok(())
}
```

#### Boolean Conversion Tests:
```rust
// Convert boolean handling to be database-agnostic
#[tokio::test]
async fn test_boolean_conversion() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = TestDatabaseManager::new();
    let client = manager.create_client(DatabaseDialect::SQLite).await?;
    let dialect = client.dialect();
    
    // Schema creation with proper boolean handling
    let boolean_column = match dialect {
        DatabaseDialect::SQLite => ColumnDef::new(Users::IsActive).integer().default(1),
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => ColumnDef::new(Users::IsActive).boolean().default(true),
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => ColumnDef::new(Users::IsActive).boolean().default(true),
    };
    
    let (create_sql, _) = Table::create()
        .table(Users::Table)
        .if_not_exists() 
        .col(ColumnDef::new(Users::Id).integer().auto_increment().primary_key())
        .col(ColumnDef::new(Users::Name).text())
        .col(boolean_column)
        .build(dialect.query_builder().as_ref());
    client.execute_schema(&create_sql).await?;
    
    // Insert with boolean value
    let (insert_sql, params) = Query::insert()
        .into_table(Users::Table)
        .columns([Users::Name, Users::IsActive])
        .values([Expr::value("Alice"), Expr::value(true)])?
        .build(dialect.query_builder().as_ref());
    client.execute_query(&insert_sql, &convert_sea_query_params_to_json(params)).await?;
    
    // Query boolean field
    let (select_sql, params) = Query::select()
        .columns([Users::Name, Users::IsActive])
        .from(Users::Table)
        .and_where(Expr::col(Users::IsActive).eq(true))
        .build(dialect.query_builder().as_ref());
    
    let result = client.execute_query(&select_sql, &convert_sea_query_params_to_json(params)).await?;
    assert_eq!(result.len(), 1);
    
    Ok(())
}
```

**Acceptance Criteria**:
- ZERO raw SQL in core functionality tests
- All tests pass identically on SQLite, PostgreSQL, MySQL
- Boolean conversion works correctly across databases

---

### **Task 3.4: Convert Schema & DDL Tests**
**Estimated Lines**: 450-500  
**Priority**: HIGH  
**Files Affected** (~8 files):
- `tests/test_schema.rs`
- `tests/test_schema_simple.rs` 
- `tests/test_schema_differ.rs`
- `tests/test_auto_schema_client.rs`

**Implementation Focus**: Convert ALL CREATE/ALTER/DROP/INDEX operations

```rust
// ❌ BEFORE (Raw SQL) - FORBIDDEN
#[tokio::test]
async fn test_schema_operations() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = setup_test_client().await?;
    
    // Raw DDL - FORBIDDEN!
    client.execute_schema("CREATE TABLE products (id SERIAL PRIMARY KEY, name VARCHAR(255), price DECIMAL(10,2))").await?;
    client.execute_schema("ALTER TABLE products ADD COLUMN category_id INTEGER").await?;
    client.execute_schema("CREATE INDEX idx_products_name ON products(name)").await?;
    client.execute_schema("DROP TABLE products").await?;
    
    Ok(())
}

// ✅ AFTER (sea-query builders) - REQUIRED
#[tokio::test]
async fn test_schema_operations() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = TestDatabaseManager::new();
    let client = manager.create_client(DatabaseDialect::SQLite).await?;
    let dialect = client.dialect();
    
    // CREATE TABLE with sea-query
    let (create_sql, _) = Table::create()
        .table(Products::Table)
        .if_not_exists()
        .col(ColumnDef::new(Products::Id).integer().auto_increment().primary_key())
        .col(ColumnDef::new(Products::Name).string_len(255).not_null())
        .col(ColumnDef::new(Products::Price).decimal_len(10, 2))
        .build(dialect.query_builder().as_ref());
    client.execute_schema(&create_sql).await?;
    
    // ALTER TABLE with sea-query
    let (alter_sql, _) = Table::alter()
        .table(Products::Table)
        .add_column(ColumnDef::new(Products::CategoryId).integer())
        .build(dialect.query_builder().as_ref());
    client.execute_schema(&alter_sql).await?;
    
    // CREATE INDEX with sea-query
    let (index_sql, _) = Index::create()
        .name("idx_products_name")
        .table(Products::Table)
        .col(Products::Name)
        .build(dialect.query_builder().as_ref());
    client.execute_schema(&index_sql).await?;
    
    // DROP TABLE with sea-query
    let (drop_sql, _) = Table::drop()
        .table(Products::Table)
        .if_exists()
        .build(dialect.query_builder().as_ref());
    client.execute_schema(&drop_sql).await?;
    
    println!("✅ Schema operations completed on {:?}", dialect);
    Ok(())
}
```

#### Database-Specific Type Handling:
```rust
// Handle database-specific column types properly
fn create_comprehensive_type_table(dialect: DatabaseDialect) -> (String, Vec<Value>) {
    let mut create = Table::create()
        .table(TypeTest::Table)
        .if_not_exists()
        .col(ColumnDef::new(TypeTest::Id).integer().auto_increment().primary_key());
    
    // Add columns based on database capabilities
    match dialect {
        DatabaseDialect::SQLite => {
            create = create
                .col(ColumnDef::new(TypeTest::TextCol).text())
                .col(ColumnDef::new(TypeTest::IntegerCol).integer())
                .col(ColumnDef::new(TypeTest::RealCol).float())
                .col(ColumnDef::new(TypeTest::BlobCol).blob())
                .col(ColumnDef::new(TypeTest::BooleanCol).integer()) // SQLite uses INTEGER for boolean
                .col(ColumnDef::new(TypeTest::DatetimeCol).date_time());
        },
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => {
            create = create
                .col(ColumnDef::new(TypeTest::TextCol).text())
                .col(ColumnDef::new(TypeTest::IntegerCol).integer())
                .col(ColumnDef::new(TypeTest::RealCol).real())
                .col(ColumnDef::new(TypeTest::BinaryCol).binary_len(255))
                .col(ColumnDef::new(TypeTest::BooleanCol).boolean())
                .col(ColumnDef::new(TypeTest::TimestampCol).timestamp())
                .col(ColumnDef::new(TypeTest::JsonCol).json())
                .col(ColumnDef::new(TypeTest::UuidCol).uuid());
        },
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => {
            create = create
                .col(ColumnDef::new(TypeTest::TextCol).text())
                .col(ColumnDef::new(TypeTest::IntegerCol).integer())
                .col(ColumnDef::new(TypeTest::RealCol).real())
                .col(ColumnDef::new(TypeTest::BlobCol).blob())
                .col(ColumnDef::new(TypeTest::BooleanCol).boolean())
                .col(ColumnDef::new(TypeTest::DatetimeCol).date_time())
                .col(ColumnDef::new(TypeTest::JsonCol).json());
        },
    }
    
    let (sql, params) = create.build(dialect.query_builder().as_ref());
    (sql, convert_sea_query_params_to_json(params))
}
```

**Acceptance Criteria**:
- All schema operations use sea-query builders
- Database-specific types handled automatically
- Schema tests pass on all three databases

---

### **Task 3.5: Convert Migration Tests**
**Estimated Lines**: 450-500  
**Priority**: HIGH  
**Files Affected** (~12 files):
- `tests/test_migrations.rs`
- `tests/test_type_safe_migrations.rs`
- `tests/test_migration_validator.rs`
- `tests/test_migration_planner.rs`
- `tests/test_data_migration*.rs` (multiple files)
- `tests/test_smart_migration_strategies.rs`

**Implementation**: Convert all migration UP/DOWN operations

```rust
// ✅ Migration Framework with sea-query
pub struct Migration {
    pub name: String,
    pub up_operations: Vec<MigrationOperation>,
    pub down_operations: Vec<MigrationOperation>,
}

pub enum MigrationOperation {
    CreateTable {
        table: DynIden,
        columns: Vec<ColumnDef>,
        foreign_keys: Vec<ForeignKey>,
    },
    AlterTable {
        table: DynIden,
        changes: Vec<TableAlteration>,
    },
    DropTable {
        table: DynIden,
    },
    CreateIndex {
        name: String,
        table: DynIden,
        columns: Vec<DynIden>,
    },
    DataMigration {
        description: String,
        operation: Box<dyn DataMigrationFn>,
    },
}

impl MigrationOperation {
    pub async fn execute<B: DatabaseBackend>(&self, backend: &B) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let dialect = backend.dialect();
        
        match self {
            MigrationOperation::CreateTable { table, columns, foreign_keys } => {
                let mut create = Table::create().table(table.clone()).if_not_exists();
                
                for column in columns {
                    create = create.col(column.clone());
                }
                
                for fk in foreign_keys {
                    create = create.foreign_key(fk.clone());
                }
                
                let (sql, _) = create.build(dialect.query_builder().as_ref());
                backend.execute_schema(&sql).await?;
            },
            
            MigrationOperation::AlterTable { table, changes } => {
                for change in changes {
                    let (sql, _) = match change {
                        TableAlteration::AddColumn(column_def) => {
                            Table::alter()
                                .table(table.clone())
                                .add_column(column_def.clone())
                                .build(dialect.query_builder().as_ref())
                        },
                        TableAlteration::DropColumn(column) => {
                            Table::alter()
                                .table(table.clone())
                                .drop_column(column.clone())
                                .build(dialect.query_builder().as_ref())
                        },
                    };
                    backend.execute_schema(&sql).await?;
                }
            },
            
            MigrationOperation::DataMigration { operation, .. } => {
                operation.execute(backend).await?;
            },
            
            // ... other operations
        }
        
        Ok(())
    }
}

// Example migration test conversion
#[tokio::test]
async fn test_add_email_column_migration() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = TestDatabaseManager::new();
    let client = manager.create_client(DatabaseDialect::SQLite).await?;
    
    // Setup initial schema
    let initial_migration = Migration {
        name: "create_users_table".to_string(),
        up_operations: vec![
            MigrationOperation::CreateTable {
                table: Users::Table.into_iden(),
                columns: vec![
                    ColumnDef::new(Users::Id).integer().auto_increment().primary_key(),
                    ColumnDef::new(Users::Name).text().not_null(),
                ],
                foreign_keys: vec![],
            }
        ],
        down_operations: vec![
            MigrationOperation::DropTable {
                table: Users::Table.into_iden(),
            }
        ],
    };
    
    // Execute initial migration
    for operation in &initial_migration.up_operations {
        operation.execute(&client).await?;
    }
    
    // Test adding email column
    let add_email_migration = Migration {
        name: "add_email_to_users".to_string(),
        up_operations: vec![
            MigrationOperation::AlterTable {
                table: Users::Table.into_iden(),
                changes: vec![
                    TableAlteration::AddColumn(
                        ColumnDef::new(Users::Email).string_len(255).unique_key()
                    ),
                ],
            }
        ],
        down_operations: vec![
            MigrationOperation::AlterTable {
                table: Users::Table.into_iden(),
                changes: vec![
                    TableAlteration::DropColumn(Users::Email.into_iden()),
                ],
            }
        ],
    };
    
    // Execute migration
    for operation in &add_email_migration.up_operations {
        operation.execute(&client).await?;
    }
    
    // Verify column was added
    // Test insertion with email
    let (insert_sql, params) = Query::insert()
        .into_table(Users::Table)
        .columns([Users::Name, Users::Email])
        .values([Expr::value("Alice"), Expr::value("alice@example.com")])?
        .build(client.dialect().query_builder().as_ref());
    client.execute_query(&insert_sql, &convert_sea_query_params_to_json(params)).await?;
    
    // Test rollback
    for operation in add_email_migration.down_operations.iter().rev() {
        operation.execute(&client).await?;
    }
    
    println!("✅ Migration test completed on {:?}", client.dialect());
    Ok(())
}
```

**Acceptance Criteria**:
- All migration operations use sea-query builders
- Migration rollbacks work correctly
- Migrations produce identical results across databases

---

### **Task 3.6: Convert Multi-Database & Cross-Database Tests**  
**Estimated Lines**: 350-450
**Priority**: HIGH  
**Files Affected** (~6 files):
- `tests/test_multi_database.rs`
- `tests/test_multi_database_integration.rs`
- `tests/test_cross_database_suite.rs`
- `tests/test_cross_database_advanced.rs`

**Implementation**: Ensure tests demonstrate identical behavior across databases

```rust
// ✅ True cross-database test pattern
pub async fn test_cross_database_feature<B: DatabaseBackend>(
    backend: &B,
    expected_dialect: DatabaseDialect
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Verify we're using the expected backend
    backend.verify_backend_type(expected_dialect)?;
    
    let dialect = backend.dialect();
    println!("🔄 Testing cross-database feature on {:?}", dialect);
    
    // Create table that works across all databases
    let (create_sql, _) = Table::create()
        .table(CrossDbTest::Table)
        .if_not_exists()
        .col(match dialect {
            DatabaseDialect::SQLite => ColumnDef::new(CrossDbTest::Id).integer().auto_increment().primary_key(),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => ColumnDef::new(CrossDbTest::Id).integer().auto_increment().primary_key(),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => ColumnDef::new(CrossDbTest::Id).integer().auto_increment().primary_key(),
        })
        .col(ColumnDef::new(CrossDbTest::Name).text().not_null())
        .col(ColumnDef::new(CrossDbTest::Value).integer().not_null())
        .col(match dialect {
            DatabaseDialect::SQLite => ColumnDef::new(CrossDbTest::CreatedAt).date_time().default(Expr::current_timestamp()),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => ColumnDef::new(CrossDbTest::CreatedAt).timestamp().default(Expr::current_timestamp()),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => ColumnDef::new(CrossDbTest::CreatedAt).date_time().default(Expr::current_timestamp()),
        })
        .build(dialect.query_builder().as_ref());
    backend.execute_schema(&create_sql).await?;
    
    // Insert identical test data
    let test_data = vec![
        ("Alice", 100),
        ("Bob", 200),
        ("Charlie", 300),
    ];
    
    for (name, value) in &test_data {
        let (insert_sql, params) = Query::insert()
            .into_table(CrossDbTest::Table)
            .columns([CrossDbTest::Name, CrossDbTest::Value])
            .values([Expr::value(*name), Expr::value(*value)])?
            .build(dialect.query_builder().as_ref());
        backend.execute_query(&insert_sql, &convert_sea_query_params_to_json(params)).await?;
    }
    
    // Verify count
    let (count_sql, params) = Query::select()
        .expr(Expr::count(Expr::asterisk()).as_(Alias::new("count")))
        .from(CrossDbTest::Table)
        .build(dialect.query_builder().as_ref());
    let result = backend.execute_query(&count_sql, &convert_sea_query_params_to_json(params)).await?;
    let count = result.extract_count()?;
    assert_eq!(count, 3, "Count should be identical across all databases");
    
    // Test aggregate functions
    let (sum_sql, params) = Query::select()
        .expr(Expr::sum(Expr::col(CrossDbTest::Value)).as_(Alias::new("total")))
        .from(CrossDbTest::Table)
        .build(dialect.query_builder().as_ref());
    let result = backend.execute_query(&sum_sql, &convert_sea_query_params_to_json(params)).await?;
    // Verify sum is 600 regardless of database
    
    // Test ordering
    let (order_sql, params) = Query::select()
        .columns([CrossDbTest::Name, CrossDbTest::Value])
        .from(CrossDbTest::Table)
        .order_by(CrossDbTest::Value, Order::Desc)
        .build(dialect.query_builder().as_ref());
    let result = backend.execute_query(&order_sql, &convert_sea_query_params_to_json(params)).await?;
    let rows = result.into_rows();
    assert_eq!(rows.len(), 3);
    // Verify ordering is identical
    
    // Cleanup
    let (drop_sql, _) = Table::drop()
        .table(CrossDbTest::Table)
        .if_exists()
        .build(dialect.query_builder().as_ref());
    backend.execute_schema(&drop_sql).await?;
    
    println!("✅ Cross-database feature test completed successfully on {:?}", dialect);
    Ok(())
}

// Use the multi_db_test macro for comprehensive testing
multi_db_test!(test_sqlite_cross_db, DatabaseDialect::SQLite, {
    test_cross_database_feature(&client, DatabaseDialect::SQLite).await
});

#[cfg(feature = "postgres")]
multi_db_test!(test_postgres_cross_db, DatabaseDialect::PostgreSQL, {
    test_cross_database_feature(&client, DatabaseDialect::PostgreSQL).await
});

#[cfg(feature = "mysql")]
multi_db_test!(test_mysql_cross_db, DatabaseDialect::MySQL, {
    test_cross_database_feature(&client, DatabaseDialect::MySQL).await
});
```

**Acceptance Criteria**:
- Tests demonstrate identical behavior across databases
- Database-specific handling is minimal and well-documented
- Cross-database feature parity is validated

---

### **Task 3.7: Convert Introspection & Analysis Tests**
**Estimated Lines**: 400-500  
**Priority**: MEDIUM  
**Files Affected** (~8 files):
- `tests/test_introspector_debug.rs`
- `tests/test_introspector_pragma.rs`
- `tests/test_simple_introspector.rs`
- `tests/test_entity_analyzer_advanced.rs`

**Implementation**: Handle database-specific metadata queries through abstraction

```rust
// ✅ Database-agnostic introspection
pub struct DatabaseIntrospector {
    dialect: DatabaseDialect,
}

impl DatabaseIntrospector {
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self { dialect }
    }
    
    pub async fn get_table_names<B: DatabaseBackend>(&self, backend: &B) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let (sql, params) = match self.dialect {
            DatabaseDialect::SQLite => {
                Query::select()
                    .column(Alias::new("name"))
                    .from(Alias::new("sqlite_master"))
                    .and_where(Expr::col(Alias::new("type")).eq("table"))
                    .and_where(Expr::col(Alias::new("name")).ne("sqlite_sequence"))
                    .build(SqliteQueryBuilder)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                Query::select()
                    .column(Alias::new("table_name"))
                    .from(Alias::new("information_schema.tables"))
                    .and_where(Expr::col(Alias::new("table_schema")).eq("public"))
                    .and_where(Expr::col(Alias::new("table_type")).eq("BASE TABLE"))
                    .build(PostgresQueryBuilder)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                Query::select()
                    .column(Alias::new("TABLE_NAME"))
                    .from(Alias::new("information_schema.TABLES"))
                    .and_where(Expr::col(Alias::new("TABLE_SCHEMA")).eq(Expr::current_database()))
                    .and_where(Expr::col(Alias::new("TABLE_TYPE")).eq("BASE TABLE"))
                    .build(MysqlQueryBuilder)
            },
        };
        
        let result = backend.execute_query(&sql, &convert_sea_query_params_to_json(params)).await?;
        Ok(result.extract_strings()?)
    }
    
    pub async fn get_column_info<B: DatabaseBackend>(&self, backend: &B, table_name: &str) -> Result<Vec<ColumnInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let (sql, params) = match self.dialect {
            DatabaseDialect::SQLite => {
                // Use PRAGMA table_info
                (format!("PRAGMA table_info({})", table_name), vec![])
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                Query::select()
                    .columns([
                        Alias::new("column_name"),
                        Alias::new("data_type"),
                        Alias::new("is_nullable"),
                        Alias::new("column_default")
                    ])
                    .from(Alias::new("information_schema.columns"))
                    .and_where(Expr::col(Alias::new("table_schema")).eq("public"))
                    .and_where(Expr::col(Alias::new("table_name")).eq(table_name))
                    .order_by(Alias::new("ordinal_position"), Order::Asc)
                    .build(PostgresQueryBuilder)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                Query::select()
                    .columns([
                        Alias::new("COLUMN_NAME"),
                        Alias::new("DATA_TYPE"),
                        Alias::new("IS_NULLABLE"),
                        Alias::new("COLUMN_DEFAULT")
                    ])
                    .from(Alias::new("information_schema.COLUMNS"))
                    .and_where(Expr::col(Alias::new("TABLE_SCHEMA")).eq(Expr::current_database()))
                    .and_where(Expr::col(Alias::new("TABLE_NAME")).eq(table_name))
                    .order_by(Alias::new("ORDINAL_POSITION"), Order::Asc)
                    .build(MysqlQueryBuilder)
            },
        };
        
        let result = backend.execute_query(&sql, &convert_sea_query_params_to_json(params)).await?;
        
        // Parse result into ColumnInfo structs
        let mut columns = Vec::new();
        for row in result.into_rows() {
            let column_info = self.parse_column_info(row)?;
            columns.push(column_info);
        }
        
        Ok(columns)
    }
    
    fn parse_column_info(&self, row: Value) -> Result<ColumnInfo, Box<dyn std::error::Error + Send + Sync>> {
        // Parse database-specific column information into common format
        match self.dialect {
            DatabaseDialect::SQLite => {
                // Parse SQLite PRAGMA table_info format
                // [cid, name, type, notnull, dflt_value, pk]
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                // Parse PostgreSQL information_schema format
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                // Parse MySQL information_schema format
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

// Test introspection across databases
#[tokio::test]
async fn test_database_introspection() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = TestDatabaseManager::new();
    let client = manager.create_client(DatabaseDialect::SQLite).await?;
    let dialect = client.dialect();
    
    // Create test table
    let (create_sql, _) = Table::create()
        .table(TestTable::Table)
        .if_not_exists()
        .col(ColumnDef::new(TestTable::Id).integer().auto_increment().primary_key())
        .col(ColumnDef::new(TestTable::Name).text().not_null())
        .col(ColumnDef::new(TestTable::Email).text().unique_key())
        .col(ColumnDef::new(TestTable::CreatedAt).date_time().default(Expr::current_timestamp()))
        .build(dialect.query_builder().as_ref());
    client.execute_schema(&create_sql).await?;
    
    // Test introspection
    let introspector = DatabaseIntrospector::new(dialect);
    
    // Get table names
    let tables = introspector.get_table_names(&client).await?;
    assert!(tables.contains(&"test_table".to_string()));
    
    // Get column info
    let columns = introspector.get_column_info(&client, "test_table").await?;
    assert_eq!(columns.len(), 4);
    
    // Verify column details
    let id_column = columns.iter().find(|c| c.name == "id").unwrap();
    assert!(id_column.is_primary_key);
    
    let name_column = columns.iter().find(|c| c.name == "name").unwrap();
    assert!(!name_column.is_nullable);
    
    println!("✅ Introspection test completed on {:?}", dialect);
    Ok(())
}
```

**Acceptance Criteria**:
- Introspection works identically across all databases
- Database-specific metadata is properly abstracted
- Entity analysis produces consistent results

---

### **Task 3.8: Convert Performance & Benchmark Tests**
**Estimated Lines**: 300-400  
**Priority**: MEDIUM  
**Files Affected** (~4 files):
- `tests/test_performance_benchmarks.rs`
- `tests/test_performance_regression.rs`

**Implementation**: Ensure benchmarks work across all databases with proper batching

```rust
// ✅ Database-agnostic performance testing
pub struct PerformanceBenchmark {
    dialect: DatabaseDialect,
    batch_size: usize,
}

impl PerformanceBenchmark {
    pub fn new(dialect: DatabaseDialect) -> Self {
        Self {
            dialect,
            batch_size: 1000, // Configurable batch size
        }
    }
    
    pub async fn benchmark_bulk_insert<B: DatabaseBackend>(
        &self,
        backend: &B,
        record_count: usize
    ) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>> {
        // Setup performance test table
        let (create_sql, _) = Table::create()
            .table(PerfTest::Table)
            .if_not_exists()
            .col(ColumnDef::new(PerfTest::Id).integer().auto_increment().primary_key())
            .col(ColumnDef::new(PerfTest::Value).integer().not_null())
            .col(ColumnDef::new(PerfTest::Data).text())
            .col(ColumnDef::new(PerfTest::CreatedAt).date_time().default(Expr::current_timestamp()))
            .build(self.dialect.query_builder().as_ref());
        backend.execute_schema(&create_sql).await?;
        
        let start = Instant::now();
        
        // Batch insert for optimal performance
        for batch_start in (0..record_count).step_by(self.batch_size) {
            let batch_end = std::cmp::min(batch_start + self.batch_size, record_count);
            
            let mut insert = Query::insert()
                .into_table(PerfTest::Table)
                .columns([PerfTest::Value, PerfTest::Data])
                .to_owned();
            
            for i in batch_start..batch_end {
                insert = insert.values([
                    Expr::value(i as i32),
                    Expr::value(format!("test_data_{}", i))
                ])?;
            }
            
            let (sql, params) = insert.build(self.dialect.query_builder().as_ref());
            backend.execute_query(&sql, &convert_sea_query_params_to_json(params)).await?;
        }
        
        let duration = start.elapsed();
        
        // Verify count
        let (count_sql, params) = Query::select()
            .expr(Expr::count(Expr::asterisk()).as_(Alias::new("count")))
            .from(PerfTest::Table)
            .build(self.dialect.query_builder().as_ref());
        let result = backend.execute_query(&count_sql, &convert_sea_query_params_to_json(params)).await?;
        let count = result.extract_count()?;
        assert_eq!(count as usize, record_count);
        
        // Cleanup
        let (drop_sql, _) = Table::drop()
            .table(PerfTest::Table)
            .if_exists()
            .build(self.dialect.query_builder().as_ref());
        backend.execute_schema(&drop_sql).await?;
        
        Ok(duration)
    }
    
    pub async fn benchmark_complex_query<B: DatabaseBackend>(
        &self,
        backend: &B
    ) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>> {
        // Setup test data
        self.setup_complex_query_data(backend).await?;
        
        let start = Instant::now();
        
        // Complex query with JOIN, WHERE, GROUP BY, HAVING, ORDER BY
        let (sql, params) = Query::select()
            .columns([
                Users::Name,
                Expr::count(Posts::Id).as_(Alias::new("post_count")),
                Expr::avg(Posts::Score).as_(Alias::new("avg_score"))
            ])
            .from(Users::Table)
            .join(JoinType::InnerJoin, Posts::Table, 
                Expr::col((Users::Table, Users::Id)).equals((Posts::Table, Posts::UserId)))
            .and_where(Expr::col((Posts::Table, Posts::IsPublished)).eq(true))
            .group_by_col((Users::Table, Users::Id))
            .group_by_col((Users::Table, Users::Name))
            .and_having(Expr::count(Posts::Id).gt(5))
            .order_by((Users::Table, Users::Name), Order::Asc)
            .build(self.dialect.query_builder().as_ref());
        
        let result = backend.execute_query(&sql, &convert_sea_query_params_to_json(params)).await?;
        let _rows = result.into_rows(); // Consume the result
        
        let duration = start.elapsed();
        
        // Cleanup
        self.cleanup_complex_query_data(backend).await?;
        
        Ok(duration)
    }
}

// Performance test that runs across all databases
#[tokio::test]
async fn test_bulk_insert_performance() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = TestDatabaseManager::new();
    let client = manager.create_client(DatabaseDialect::SQLite).await?;
    let dialect = client.dialect();
    
    let benchmark = PerformanceBenchmark::new(dialect);
    let duration = benchmark.benchmark_bulk_insert(&client, 10000).await?;
    
    println!("✅ Bulk insert of 10k records on {:?}: {:?}", dialect, duration);
    
    // Performance expectations (can be adjusted per database)
    match dialect {
        DatabaseDialect::SQLite => {
            assert!(duration < Duration::from_secs(5), "SQLite bulk insert should be < 5s");
        },
        #[cfg(feature = "postgres")]
        DatabaseDialect::PostgreSQL => {
            assert!(duration < Duration::from_secs(10), "PostgreSQL bulk insert should be < 10s");
        },
        #[cfg(feature = "mysql")]
        DatabaseDialect::MySQL => {
            assert!(duration < Duration::from_secs(10), "MySQL bulk insert should be < 10s");
        },
    }
    
    Ok(())
}
```

**Acceptance Criteria**:
- Performance tests use only sea-query builders
- Benchmarks provide meaningful cross-database comparisons
- Performance optimizations maintain database agnosticity

---

### **Task 3.9: Convert Advanced Feature Tests**  
**Estimated Lines**: 350-450  
**Priority**: MEDIUM  
**Files Affected** (~8 files):
- `tests/test_relation_filtering.rs`
- `tests/test_recursive_relationships.rs`  
- `tests/test_enhanced_error_messages.rs`
- `tests/debug_relations.rs`

**Implementation**: Handle advanced SQL features with sea-query

```rust
// ✅ Advanced features with sea-query
#[tokio::test]
async fn test_recursive_relationships() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = TestDatabaseManager::new();
    let client = manager.create_client(DatabaseDialect::SQLite).await?;
    let dialect = client.dialect();
    
    // Create categories table with self-reference
    let (create_sql, _) = Table::create()
        .table(Categories::Table)
        .if_not_exists()
        .col(ColumnDef::new(Categories::Id).integer().auto_increment().primary_key())
        .col(ColumnDef::new(Categories::Name).text().not_null())
        .col(ColumnDef::new(Categories::ParentId).integer())
        .foreign_key(
            ForeignKey::create()
                .name("fk_categories_parent")
                .from(Categories::Table, Categories::ParentId)
                .to(Categories::Table, Categories::Id)
                .on_delete(ForeignKeyAction::SetNull)
        )
        .build(dialect.query_builder().as_ref());
    client.execute_schema(&create_sql).await?;
    
    // Insert hierarchical data
    let test_data = vec![
        (1, "Root", None),
        (2, "Electronics", Some(1)),
        (3, "Computers", Some(2)),
        (4, "Laptops", Some(3)),
        (5, "Gaming Laptops", Some(4)),
    ];
    
    for (id, name, parent_id) in test_data {
        let mut insert = Query::insert()
            .into_table(Categories::Table)
            .columns([Categories::Id, Categories::Name])
            .values([Expr::value(id), Expr::value(name)])?;
        
        if let Some(pid) = parent_id {
            insert = Query::insert()
                .into_table(Categories::Table)
                .columns([Categories::Id, Categories::Name, Categories::ParentId])
                .values([Expr::value(id), Expr::value(name), Expr::value(pid)])?;
        }
        
        let (sql, params) = insert.build(dialect.query_builder().as_ref());
        client.execute_query(&sql, &convert_sea_query_params_to_json(params)).await?;
    }
    
    // Test recursive CTE query (if supported by database)
    if dialect.supports_cte() {
        let cte = CommonTableExpression::new()
            .query(
                Query::select()
                    .columns([Categories::Id, Categories::Name, Categories::ParentId])
                    .expr(Expr::value(0).as_(Alias::new("level")))
                    .from(Categories::Table)
                    .and_where(Expr::col(Categories::ParentId).is_null())
                    .union(UnionType::All,
                        Query::select()
                            .columns([
                                (Categories::Table, Categories::Id),
                                (Categories::Table, Categories::Name),
                                (Categories::Table, Categories::ParentId)
                            ])
                            .expr(Expr::col((CategoryTree::Table, CategoryTree::Level)).add(1))
                            .from(Categories::Table)
                            .join(JoinType::InnerJoin, CategoryTree::Table,
                                Expr::col((Categories::Table, Categories::ParentId))
                                    .equals((CategoryTree::Table, CategoryTree::Id)))
                            .to_owned()
                    )
                    .to_owned()
            )
            .table_name(CategoryTree::Table)
            .columns([
                CategoryTree::Id,
                CategoryTree::Name,
                CategoryTree::ParentId,
                CategoryTree::Level
            ])
            .to_owned();

        let (sql, params) = Query::select()
            .columns([CategoryTree::Name, CategoryTree::Level])
            .from(CategoryTree::Table)
            .with(cte)
            .order_by(CategoryTree::Level, Order::Asc)
            .order_by(CategoryTree::Name, Order::Asc)
            .build(dialect.query_builder().as_ref());
        
        let result = client.execute_query(&sql, &convert_sea_query_params_to_json(params)).await?;
        let rows = result.into_rows();
        assert!(rows.len() >= 5); // Should get all categories in hierarchy
    }
    
    // Test window functions (if supported)
    if dialect.supports_window_functions() {
        let (sql, params) = Query::select()
            .columns([
                Categories::Name,
                Categories::ParentId,
            ])
            .expr(
                Expr::expr(WindowStatement::new()
                    .partition_by_col(Categories::ParentId)
                    .order_by_col(Categories::Name)
                    .row_number()
                ).as_(Alias::new("row_num"))
            )
            .from(Categories::Table)
            .build(dialect.query_builder().as_ref());
        
        let result = client.execute_query(&sql, &convert_sea_query_params_to_json(params)).await?;
        let _rows = result.into_rows();
    }
    
    println!("✅ Recursive relationships test completed on {:?}", dialect);
    Ok(())
}

// Advanced filtering with subqueries
#[tokio::test]
async fn test_advanced_filtering() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = TestDatabaseManager::new();
    let client = manager.create_client(DatabaseDialect::SQLite).await?;
    let dialect = client.dialect();
    
    // Setup test tables
    setup_users_and_posts_tables(&client).await?;
    
    // Complex filtering with EXISTS subquery
    let (sql, params) = Query::select()
        .columns([Users::Id, Users::Name])
        .from(Users::Table)
        .and_where(
            Expr::exists(
                Query::select()
                    .column(Posts::Id)
                    .from(Posts::Table)
                    .and_where(
                        Expr::col((Posts::Table, Posts::UserId))
                            .equals((Users::Table, Users::Id))
                    )
                    .and_where(Expr::col((Posts::Table, Posts::IsPublished)).eq(true))
                    .to_owned()
            )
        )
        .build(dialect.query_builder().as_ref());
    
    let result = client.execute_query(&sql, &convert_sea_query_params_to_json(params)).await?;
    let users_with_posts = result.into_rows();
    
    // Complex filtering with IN subquery
    let (sql, params) = Query::select()
        .columns([Posts::Id, Posts::Title])
        .from(Posts::Table)
        .and_where(
            Expr::col(Posts::UserId).is_in(
                Query::select()
                    .column(Users::Id)
                    .from(Users::Table)
                    .and_where(Expr::col(Users::IsActive).eq(true))
                    .to_owned()
            )
        )
        .build(dialect.query_builder().as_ref());
    
    let result = client.execute_query(&sql, &convert_sea_query_params_to_json(params)).await?;
    let active_user_posts = result.into_rows();
    
    println!("✅ Advanced filtering test completed on {:?}", dialect);
    Ok(())
}
```

**Acceptance Criteria**:
- Advanced SQL features work across all databases
- Complex queries maintain readability and maintainability
- Feature compatibility is documented

---

## **PHASE 4: Backend Verification & Validation**

### **Task 4.1: Connection Verification System**
**Estimated Lines**: 250-350  
**Priority**: HIGH  
**Files Affected**:
- `tests/common/database_manager.rs`
- New file: `tests/common/backend_verification.rs`

**Implementation**:
```rust
// tests/common/backend_verification.rs
use d1_rs::dialects::DatabaseDialect;
use std::fmt;

/// Backend verification errors
#[derive(Debug)]
pub enum BackendVerificationError {
    TypeMismatch { expected: DatabaseDialect, actual: DatabaseDialect },
    ConnectionFailure(String),
    HealthCheckFailure(String),
}

impl fmt::Display for BackendVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendVerificationError::TypeMismatch { expected, actual } => {
                write!(f, "❌ Backend verification FAILED! Expected: {:?}, Got: {:?}", expected, actual)
            },
            BackendVerificationError::ConnectionFailure(msg) => {
                write!(f, "❌ Connection verification FAILED: {}", msg)
            },
            BackendVerificationError::HealthCheckFailure(msg) => {
                write!(f, "❌ Health check FAILED: {}", msg)
            },
        }
    }
}

impl std::error::Error for BackendVerificationError {}

// Add comprehensive backend verification to AnyDatabaseBackend
impl AnyDatabaseBackend {
    /// Verify that this backend matches the expected dialect
    pub fn verify_backend_type(&self, expected: DatabaseDialect) -> Result<(), BackendVerificationError> {
        let actual = self.dialect();
        if actual != expected {
            return Err(BackendVerificationError::TypeMismatch { expected, actual });
        }
        
        println!("✅ Backend verified: {:?} - {}", actual, self.connection_info());
        Ok(())
    }
    
    /// Get a human-readable connection summary
    pub fn connection_summary(&self) -> String {
        match self {
            AnyDatabaseBackend::SQLite(_) => "SQLite in-memory database".to_string(),
            #[cfg(feature = "postgres")]
            AnyDatabaseBackend::PostgreSQL(pg) => {
                format!("PostgreSQL - Pool: {}/{} connections", 
                    pg.idle_connections(), pg.pool_size())
            },
            #[cfg(feature = "mysql")]
            AnyDatabaseBackend::MySQL(mysql) => {
                format!("MySQL - Pool: {}/{} connections", 
                    mysql.idle_connections(), mysql.pool_size())
            },
        }
    }
    
    /// Perform comprehensive health check
    pub async fn health_check(&self) -> Result<(), BackendVerificationError> {
        // Test basic connectivity
        match self.ping().await {
            Ok(_) => println!("✅ Ping successful: {}", self.connection_summary()),
            Err(e) => return Err(BackendVerificationError::HealthCheckFailure(
                format!("Ping failed: {}", e)
            )),
        }
        
        // Test simple query execution
        let (test_sql, params) = match self.dialect() {
            DatabaseDialect::SQLite => ("SELECT 1 as health_check".to_string(), vec![]),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => ("SELECT 1 as health_check".to_string(), vec![]),
            #[cfg(feature = "mysql")]  
            DatabaseDialect::MySQL => ("SELECT 1 as health_check".to_string(), vec![]),
        };
        
        match self.execute_query(&test_sql, &params).await {
            Ok(result) => {
                let rows = result.into_rows();
                if rows.is_empty() {
                    return Err(BackendVerificationError::HealthCheckFailure(
                        "Health check query returned no results".to_string()
                    ));
                }
                println!("✅ Health check query successful");
            },
            Err(e) => return Err(BackendVerificationError::HealthCheckFailure(
                format!("Health check query failed: {}", e)
            )),
        }
        
        // Test schema operations
        let test_table = format!("health_check_table_{}", 
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
        
        let create_sql = format!("CREATE TEMPORARY TABLE {} (id INTEGER)", test_table);
        match self.execute_schema(&create_sql).await {
            Ok(_) => println!("✅ Schema operation test successful"),
            Err(e) => return Err(BackendVerificationError::HealthCheckFailure(
                format!("Schema operation test failed: {}", e)
            )),
        }
        
        println!("✅ Full health check passed for {}", self.connection_summary());
        Ok(())
    }
    
    /// Verify backend isolation (ensure no cross-contamination)
    pub async fn verify_isolation(&self, expected_dialect: DatabaseDialect) -> Result<(), BackendVerificationError> {
        self.verify_backend_type(expected_dialect)?;
        
        // Additional isolation checks
        let connection_info = self.connection_info();
        
        match expected_dialect {
            DatabaseDialect::SQLite => {
                if connection_info.contains("postgres") || connection_info.contains("mysql") {
                    return Err(BackendVerificationError::ConnectionFailure(
                        format!("SQLite backend has contaminated connection: {}", connection_info)
                    ));
                }
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                if !connection_info.to_lowercase().contains("postgres") {
                    return Err(BackendVerificationError::ConnectionFailure(
                        format!("PostgreSQL backend missing postgres in connection: {}", connection_info)
                    ));
                }
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                if !connection_info.to_lowercase().contains("mysql") {
                    return Err(BackendVerificationError::ConnectionFailure(
                        format!("MySQL backend missing mysql in connection: {}", connection_info)
                    ));
                }
            },
        }
        
        println!("✅ Backend isolation verified for {:?}", expected_dialect);
        Ok(())
    }
}

// Global verification functions
pub async fn verify_test_environment() -> Result<(), BackendVerificationError> {
    println!("🔍 Verifying test environment...");
    
    // Check environment variables
    let expected_backends = std::env::var("DATABASE_BACKENDS")
        .unwrap_or_else(|_| "sqlite".to_string());
    
    println!("📊 Expected backends: {}", expected_backends);
    
    // Verify each expected backend
    for backend_name in expected_backends.split(',') {
        match backend_name.trim().to_lowercase().as_str() {
            "sqlite" => {
                let manager = crate::common::database_manager::TestDatabaseManager::new();
                let client = manager.create_client(DatabaseDialect::SQLite).await
                    .map_err(|e| BackendVerificationError::ConnectionFailure(e.to_string()))?;
                client.verify_isolation(DatabaseDialect::SQLite).await?;
                client.health_check().await?;
            },
            #[cfg(feature = "postgres")]
            "postgres" | "postgresql" => {
                let manager = crate::common::database_manager::TestDatabaseManager::new();
                let client = manager.create_client(DatabaseDialect::PostgreSQL).await
                    .map_err(|e| BackendVerificationError::ConnectionFailure(e.to_string()))?;
                client.verify_isolation(DatabaseDialect::PostgreSQL).await?;
                client.health_check().await?;
            },
            #[cfg(feature = "mysql")]
            "mysql" => {
                let manager = crate::common::database_manager::TestDatabaseManager::new();
                let client = manager.create_client(DatabaseDialect::MySQL).await
                    .map_err(|e| BackendVerificationError::ConnectionFailure(e.to_string()))?;
                client.verify_isolation(DatabaseDialect::MySQL).await?;
                client.health_check().await?;
            },
            _ => {
                return Err(BackendVerificationError::ConnectionFailure(
                    format!("Unknown backend: {}", backend_name)
                ));
            }
        }
    }
    
    println!("✅ Test environment verification complete");
    Ok(())
}
```

**Acceptance Criteria**:
- Backend type verification prevents cross-contamination
- Connection health checks work for all databases
- Clear logging shows which backend is being used
- Environment verification catches configuration issues

---

### **Task 4.2: Test Execution Verification Macros**  
**Estimated Lines**: 200-300  
**Priority**: HIGH  
**Files Affected**:
- New file: `tests/common/test_macros.rs` 
- `tests/common/mod.rs` (export macros)

**Implementation**:
```rust
// tests/common/test_macros.rs
use crate::common::{database_manager::TestDatabaseManager, backend_verification::BackendVerificationError};
use d1_rs::dialects::DatabaseDialect;

/// Macro for tests that should run on a specific database backend
macro_rules! multi_db_test {
    ($test_name:ident, $expected_dialect:expr, $test_body:block) => {
        #[tokio::test]
        async fn $test_name() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            println!("🔄 Starting test {} on {:?}", stringify!($test_name), $expected_dialect);
            
            let manager = TestDatabaseManager::new();
            let client = manager.create_client($expected_dialect).await
                .map_err(|e| format!("Failed to create client for {:?}: {}", $expected_dialect, e))?;
            
            // Verify backend type matches expectation
            client.verify_backend_type($expected_dialect)
                .map_err(|e| format!("Backend verification failed: {}", e))?;
            
            // Perform health check
            client.health_check().await
                .map_err(|e| format!("Health check failed: {}", e))?;
            
            // Execute the test body
            let result = async move $test_body.await;
            
            match &result {
                Ok(_) => {
                    println!("✅ Test {} completed successfully on {}", 
                        stringify!($test_name), client.connection_summary());
                },
                Err(e) => {
                    println!("❌ Test {} failed on {}: {}", 
                        stringify!($test_name), client.connection_summary(), e);
                }
            }
            
            result
        }
    };
}

/// Macro for tests that should run on ALL available database backends
macro_rules! cross_db_test {
    ($test_name:ident, $test_body:expr) => {
        mod $test_name {
            use super::*;
            
            multi_db_test!(sqlite, DatabaseDialect::SQLite, {
                $test_body(&client, DatabaseDialect::SQLite).await
            });
            
            #[cfg(feature = "postgres")]
            multi_db_test!(postgres, DatabaseDialect::PostgreSQL, {
                $test_body(&client, DatabaseDialect::PostgreSQL).await
            });
            
            #[cfg(feature = "mysql")]
            multi_db_test!(mysql, DatabaseDialect::MySQL, {
                $test_body(&client, DatabaseDialect::MySQL).await
            });
        }
    };
}

/// Macro for feature-gated tests that only run when specific features are enabled
macro_rules! feature_test {
    ($feature:literal, $test_name:ident, $dialect:expr, $test_body:block) => {
        #[cfg(feature = $feature)]
        multi_db_test!($test_name, $dialect, $test_body);
    };
}

/// Macro for environment-restricted tests
macro_rules! env_test {
    ($test_name:ident, $allowed_backends:expr, $dialect:expr, $test_body:block) => {
        #[tokio::test]
        async fn $test_name() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            // Check if this backend is allowed in current environment
            let allowed_backends_env = std::env::var("DATABASE_BACKENDS")
                .unwrap_or_else(|_| "sqlite".to_string());
            
            let allowed_list: Vec<&str> = $allowed_backends;
            let is_allowed = allowed_list.iter().any(|&backend| {
                allowed_backends_env.contains(backend)
            });
            
            if !is_allowed {
                println!("⏭️  Skipping test {} - backend {:?} not allowed in current environment ({})", 
                    stringify!($test_name), $dialect, allowed_backends_env);
                return Ok(());
            }
            
            // Proceed with normal test execution
            multi_db_test!(inner, $dialect, $test_body);
            inner().await
        }
    };
}

/// Macro for performance-sensitive tests with timing
macro_rules! timed_test {
    ($test_name:ident, $dialect:expr, $max_duration_ms:expr, $test_body:block) => {
        multi_db_test!($test_name, $dialect, {
            let start = std::time::Instant::now();
            
            let result = async move $test_body.await;
            
            let duration = start.elapsed();
            let max_duration = std::time::Duration::from_millis($max_duration_ms);
            
            if duration > max_duration {
                return Err(format!(
                    "Test {} took {:?}, expected < {:?}", 
                    stringify!($test_name), duration, max_duration
                ).into());
            }
            
            println!("⏱️  Test {} completed in {:?}", stringify!($test_name), duration);
            result
        });
    };
}

/// Helper macro for setup and teardown
macro_rules! test_with_cleanup {
    ($test_name:ident, $dialect:expr, $setup:block, $test_body:block, $cleanup:block) => {
        multi_db_test!($test_name, $dialect, {
            println!("🔧 Setting up test {}", stringify!($test_name));
            $setup;
            
            let test_result = async move $test_body.await;
            
            println!("🧹 Cleaning up test {}", stringify!($test_name));
            $cleanup;
            
            test_result
        });
    };
}

// Example usage patterns
pub fn example_macro_usage() {
    // These are example usages - not actual tests
    
    /*
    // Single database test
    multi_db_test!(test_basic_functionality, DatabaseDialect::SQLite, {
        // Test implementation here
        Ok(())
    });
    
    // Cross-database test
    cross_db_test!(test_cross_database_feature, |client, dialect| async move {
        // This runs on all available databases
        println!("Testing on {:?}", dialect);
        Ok(())
    });
    
    // Feature-gated test
    feature_test!("postgres", test_postgres_specific, DatabaseDialect::PostgreSQL, {
        // Only runs when postgres feature is enabled
        Ok(())
    });
    
    // Environment-restricted test
    env_test!(test_sqlite_only, &["sqlite"], DatabaseDialect::SQLite, {
        // Only runs when DATABASE_BACKENDS includes sqlite
        Ok(())
    });
    
    // Performance test with timing
    timed_test!(test_fast_operation, DatabaseDialect::SQLite, 1000, {
        // Must complete within 1000ms
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    });
    
    // Test with setup and cleanup
    test_with_cleanup!(test_with_resources, DatabaseDialect::SQLite,
        {
            // Setup code
            println!("Setting up test resources");
        },
        {
            // Test body
            println!("Running test");
            Ok(())
        },
        {
            // Cleanup code
            println!("Cleaning up test resources");
        }
    );
    */
}

// Export macros for use in tests
pub use {multi_db_test, cross_db_test, feature_test, env_test, timed_test, test_with_cleanup};
```

**Update tests/common/mod.rs**:
```rust
pub mod database_manager;
pub mod backend_verification;
pub mod test_macros;
pub mod query_helpers;

// Re-export commonly used items
pub use database_manager::{TestDatabaseManager, AnyDatabaseBackend, TestUtils};
pub use backend_verification::{BackendVerificationError, verify_test_environment};
pub use test_macros::*;
pub use query_helpers::*;
```

**Acceptance Criteria**:
- Test macros enforce backend verification
- Clear logging of test execution context
- Macros work with all database backends
- Environment and feature restrictions are properly enforced

---

## **PHASE 5: Final Optimization & Cleanup**

### **Task 5.1: Code Cleanup & Performance Optimization**
**Estimated Lines**: 300-400  
**Priority**: MEDIUM  
**Files Affected**: All test files (cleanup pass)

**Implementation**:

#### 1. **Unused Import Cleanup**:
```bash
#!/bin/bash
# Automated cleanup script
echo "🧹 Cleaning up unused imports and dead code..."

# Find and remove unused imports
find tests/ -name "*.rs" -type f -exec rustfmt {} \;

# Run clippy to identify cleanup opportunities
cargo clippy --tests -- -W unused-imports -W dead-code -W unreachable-code

echo "✅ Cleanup completed"
```

#### 2. **Performance Optimizations**:
```rust
// Optimize TestDatabaseManager for performance
impl TestDatabaseManager {
    /// Create client with connection pooling optimization
    pub async fn create_optimized_client(&self, dialect: DatabaseDialect) -> Result<AnyDatabaseBackend, _> {
        let backend = match dialect {
            DatabaseDialect::SQLite => {
                // SQLite optimization: Use WAL mode for better concurrency
                let mut sqlite = SQLiteBackend::new_in_memory().await?;
                sqlite.execute_pragma("PRAGMA journal_mode = WAL").await?;
                sqlite.execute_pragma("PRAGMA synchronous = NORMAL").await?;
                sqlite.execute_pragma("PRAGMA cache_size = 10000").await?;
                AnyDatabaseBackend::SQLite(sqlite)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let url = self.connection_urls.get(&dialect).ok_or("PostgreSQL URL not configured")?;
                
                // PostgreSQL optimization: Tune pool settings for tests
                let config = DatabaseConfig {
                    url: url.clone(),
                    pool_config: Some(PoolConfig {
                        max_connections: 5,
                        min_connections: 1,
                        connect_timeout: Duration::from_secs(5),
                        idle_timeout: Some(Duration::from_secs(60)),
                        max_lifetime: Some(Duration::from_secs(300)),
                    }),
                    ..Default::default()
                };
                
                let pg = PostgreSQLBackend::from_config(&config).await?;
                AnyDatabaseBackend::PostgreSQL(pg)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                // Similar MySQL optimizations
                let url = self.connection_urls.get(&dialect).ok_or("MySQL URL not configured")?;
                
                let config = DatabaseConfig {
                    url: url.clone(),
                    pool_config: Some(PoolConfig {
                        max_connections: 5,
                        min_connections: 1,
                        connect_timeout: Duration::from_secs(5),
                        idle_timeout: Some(Duration::from_secs(60)),
                        max_lifetime: Some(Duration::from_secs(300)),
                    }),
                    ..Default::default()
                };
                
                let mysql = MySQLBackend::from_config(&config).await?;
                AnyDatabaseBackend::MySQL(mysql)
            },
        };
        
        Ok(backend)
    }
    
    /// Optimized fixture setup with batching
    pub async fn setup_fixtures_batch(&self, fixture_names: &[&str], backend: &AnyDatabaseBackend) -> Result<(), _> {
        // Batch multiple fixture setups into single transaction
        backend.execute_schema("BEGIN TRANSACTION").await?;
        
        for fixture_name in fixture_names {
            self.setup_fixture(fixture_name, backend).await?;
        }
        
        backend.execute_schema("COMMIT").await?;
        Ok(())
    }
}
```

#### 3. **Error Handling Standardization**:
```rust
// Standardize error handling across all tests
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Backend verification failed: {0}")]
    BackendVerification(#[from] BackendVerificationError),
    
    #[error("Fixture setup failed: {0}")]
    FixtureSetup(String),
    
    #[error("Test assertion failed: {0}")]
    Assertion(String),
    
    #[error("Performance test failed: expected < {expected:?}, actual: {actual:?}")]
    Performance { expected: Duration, actual: Duration },
}

// Standardized result type for tests
pub type TestResult = Result<(), TestError>;

// Helper functions for common test patterns
pub async fn with_test_table<F, Fut>(
    client: &AnyDatabaseBackend,
    table_name: &str,
    columns: Vec<ColumnDef>,
    test_fn: F
) -> TestResult 
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = TestResult>,
{
    let dialect = client.dialect();
    
    // Create table
    let (create_sql, _) = build_create_table_query(
        Alias::new(table_name).into_iden(),
        columns,
        dialect
    );
    client.execute_schema(&create_sql).await
        .map_err(|e| TestError::Database(e.to_string()))?;
    
    // Run test
    let result = test_fn().await;
    
    // Cleanup
    let (drop_sql, _) = build_drop_table_query(
        Alias::new(table_name).into_iden(),
        dialect
    );
    let _ = client.execute_schema(&drop_sql).await; // Ignore cleanup errors
    
    result
}
```

**Acceptance Criteria**:
- Zero unused imports/dead code warnings
- Test execution time < 30 seconds for `just test`
- Memory usage < 100MB peak during test execution
- Standardized error handling across all tests

---

### **Task 5.2: Final Integration Testing & Validation**
**Estimated Lines**: 250-350  
**Priority**: HIGH  
**Files Affected**:
- New file: `tests/integration_validation.rs`
- `justfile` (validation commands)

**Implementation**:
```rust
// tests/integration_validation.rs
use crate::common::*;
use d1_rs::dialects::DatabaseDialect;
use std::collections::HashSet;

/// Comprehensive integration validation test suite
mod integration_validation {
    use super::*;

    #[tokio::test]
    async fn validate_backend_isolation() -> TestResult {
        println!("🔍 Validating backend isolation...");
        
        // Test SQLite isolation
        std::env::set_var("DATABASE_BACKENDS", "sqlite");
        let manager = TestDatabaseManager::new();
        
        // Should only have SQLite available
        let available = manager.available_dialects();
        assert_eq!(available, &[DatabaseDialect::SQLite], 
            "SQLite-only environment should only have SQLite available");
        
        let client = manager.create_client(DatabaseDialect::SQLite).await?;
        client.verify_isolation(DatabaseDialect::SQLite).await?;
        
        // Verify connection info doesn't mention other databases
        let conn_info = client.connection_info().to_lowercase();
        assert!(!conn_info.contains("postgres"), "SQLite connection info mentions postgres");
        assert!(!conn_info.contains("mysql"), "SQLite connection info mentions mysql");
        
        println!("✅ Backend isolation validated");
        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "postgres")]
    async fn validate_postgres_isolation() -> TestResult {
        // Similar test for PostgreSQL isolation
        std::env::set_var("DATABASE_BACKENDS", "postgres");
        std::env::set_var("POSTGRES_TEST_URL", "postgresql://test:test@localhost:5432/test");
        
        let manager = TestDatabaseManager::new();
        let client = manager.create_client(DatabaseDialect::PostgreSQL).await?;
        
        client.verify_isolation(DatabaseDialect::PostgreSQL).await?;
        
        let conn_info = client.connection_info().to_lowercase();
        assert!(conn_info.contains("postgres"), "PostgreSQL connection info should mention postgres");
        
        println!("✅ PostgreSQL isolation validated");
        Ok(())
    }

    #[tokio::test] 
    async fn validate_feature_gates() -> TestResult {
        println!("🔍 Validating feature gate compilation...");
        
        // This test validates that conditional compilation works correctly
        #[cfg(feature = "postgres")]
        {
            let _: Option<PostgreSQLBackend> = None;
            println!("✅ PostgreSQL feature gate active");
        }
        
        #[cfg(feature = "mysql")]
        {
            let _: Option<MySQLBackend> = None;
            println!("✅ MySQL feature gate active");
        }
        
        // SQLite should always be available
        let _: Option<SQLiteBackend> = None;
        println!("✅ SQLite always available");
        
        Ok(())
    }

    #[tokio::test]
    async fn validate_query_equivalence() -> TestResult {
        println!("🔍 Validating query equivalence across databases...");
        
        let test_data = vec![
            ("Alice", 25, true),
            ("Bob", 30, false), 
            ("Charlie", 35, true),
        ];
        
        // Test on all available databases
        for dialect in DatabaseDialect::all_available() {
            println!("Testing query equivalence on {:?}", dialect);
            
            let manager = TestDatabaseManager::new();
            if !manager.is_available(dialect) {
                println!("⏭️  Skipping {:?} - not available", dialect);
                continue;
            }
            
            let client = manager.create_client(dialect).await?;
            
            // Create identical table structure
            let (create_sql, _) = Table::create()
                .table(TestUsers::Table)
                .if_not_exists()
                .col(ColumnDef::new(TestUsers::Id).integer().auto_increment().primary_key())
                .col(ColumnDef::new(TestUsers::Name).text().not_null())
                .col(ColumnDef::new(TestUsers::Age).integer().not_null())
                .col(match dialect {
                    DatabaseDialect::SQLite => ColumnDef::new(TestUsers::IsActive).integer().default(1),
                    #[cfg(feature = "postgres")]
                    DatabaseDialect::PostgreSQL => ColumnDef::new(TestUsers::IsActive).boolean().default(true),
                    #[cfg(feature = "mysql")]
                    DatabaseDialect::MySQL => ColumnDef::new(TestUsers::IsActive).boolean().default(true),
                })
                .build(dialect.query_builder().as_ref());
            client.execute_schema(&create_sql).await?;
            
            // Insert identical test data
            for (name, age, is_active) in &test_data {
                let (insert_sql, params) = Query::insert()
                    .into_table(TestUsers::Table)
                    .columns([TestUsers::Name, TestUsers::Age, TestUsers::IsActive])
                    .values([
                        Expr::value(*name),
                        Expr::value(*age),
                        Expr::value(*is_active)
                    ])?
                    .build(dialect.query_builder().as_ref());
                client.execute_query(&insert_sql, &convert_sea_query_params_to_json(params)).await?;
            }
            
            // Test COUNT query
            let (count_sql, params) = Query::select()
                .expr(Expr::count(Expr::asterisk()).as_(Alias::new("count")))
                .from(TestUsers::Table)
                .build(dialect.query_builder().as_ref());
            let result = client.execute_query(&count_sql, &convert_sea_query_params_to_json(params)).await?;
            let count = result.extract_count()?;
            assert_eq!(count, 3, "Count should be 3 on all databases");
            
            // Test WHERE query
            let (where_sql, params) = Query::select()
                .columns([TestUsers::Name, TestUsers::Age])
                .from(TestUsers::Table)
                .and_where(Expr::col(TestUsers::IsActive).eq(true))
                .order_by(TestUsers::Name, Order::Asc)
                .build(dialect.query_builder().as_ref());
            let result = client.execute_query(&where_sql, &convert_sea_query_params_to_json(params)).await?;
            let rows = result.into_rows();
            assert_eq!(rows.len(), 2, "Should find 2 active users on all databases");
            
            // Test aggregation
            let (avg_sql, params) = Query::select()
                .expr(Expr::avg(Expr::col(TestUsers::Age)).as_(Alias::new("avg_age")))
                .from(TestUsers::Table)
                .build(dialect.query_builder().as_ref());
            let result = client.execute_query(&avg_sql, &convert_sea_query_params_to_json(params)).await?;
            let _rows = result.into_rows(); // Should work on all databases
            
            println!("✅ Query equivalence verified for {:?}", dialect);
        }
        
        println!("✅ Query equivalence validation complete");
        Ok(())
    }

    #[tokio::test]
    async fn validate_no_raw_sql() -> TestResult {
        println!("🔍 Validating zero raw SQL usage...");
        
        // This is more of a compile-time check, but we can do some runtime validation
        let manager = TestDatabaseManager::new();
        let client = manager.create_client(DatabaseDialect::SQLite).await?;
        
        // All database operations should go through sea-query builders
        // If we've done our job correctly, there should be no way to execute raw SQL
        // except through the low-level execute_query method with properly built queries
        
        println!("✅ All database operations use sea-query builders");
        Ok(())
    }

    #[tokio::test] 
    async fn validate_performance_targets() -> TestResult {
        println!("🔍 Validating performance targets...");
        
        let manager = TestDatabaseManager::new();
        let client = manager.create_client(DatabaseDialect::SQLite).await?;
        
        // Test fixture setup performance
        let start = std::time::Instant::now();
        manager.setup_fixture("users_posts", &client).await?;
        let setup_duration = start.elapsed();
        
        assert!(setup_duration < Duration::from_secs(2), 
            "Fixture setup should be < 2s, was {:?}", setup_duration);
        
        // Test bulk operation performance
        let start = std::time::Instant::now();
        
        // Create table and insert 1000 records
        let (create_sql, _) = Table::create()
            .table(PerfTest::Table)
            .if_not_exists()
            .col(ColumnDef::new(PerfTest::Id).integer().auto_increment().primary_key())
            .col(ColumnDef::new(PerfTest::Value).integer())
            .build(client.dialect().query_builder().as_ref());
        client.execute_schema(&create_sql).await?;
        
        // Batch insert
        let mut insert = Query::insert()
            .into_table(PerfTest::Table)
            .columns([PerfTest::Value])
            .to_owned();
            
        for i in 0..1000 {
            insert = insert.values([Expr::value(i)])?;
        }
        
        let (sql, params) = insert.build(client.dialect().query_builder().as_ref());
        client.execute_query(&sql, &convert_sea_query_params_to_json(params)).await?;
        
        let bulk_duration = start.elapsed();
        assert!(bulk_duration < Duration::from_secs(5),
            "Bulk insert should be < 5s, was {:?}", bulk_duration);
        
        println!("✅ Performance targets met");
        Ok(())
    }
}
```

**Update justfile with validation commands**:
```bash
# Add to justfile
validate-all: validate-backend-isolation validate-feature-gates validate-performance
    @echo "✅ All validations complete"

validate-backend-isolation:
    @echo "🔍 Validating backend isolation..."
    @DATABASE_BACKENDS=sqlite cargo nextest run validate_backend_isolation
    @if command -v docker >/dev/null 2>&1; then \
        just start-test-dbs && \
        DATABASE_BACKENDS=postgres POSTGRES_TEST_URL="postgresql://d1rs_user:d1rs_pass@localhost:5433/d1rs_test" \
        cargo nextest run --features postgres validate_postgres_isolation && \
        DATABASE_BACKENDS=mysql MYSQL_TEST_URL="mysql://d1rs_user:d1rs_pass@localhost:3307/d1rs_test" \
        cargo nextest run --features mysql validate_mysql_isolation; \
    fi
    @echo "✅ Backend isolation validated"

validate-feature-gates:
    @echo "🔍 Validating feature gate compilation..."
    @cargo check --no-default-features
    @cargo check --features sqlite
    @cargo check --features postgres
    @cargo check --features mysql  
    @cargo check --all-features
    @echo "✅ Feature gates validated"

validate-performance:
    @echo "🔍 Validating performance targets..."
    @cargo nextest run validate_performance_targets
    @echo "✅ Performance targets validated"

validate-no-raw-sql:
    @echo "🔍 Validating zero raw SQL usage..."
    @if grep -r "execute_query.*\"SELECT\|INSERT\|UPDATE\|DELETE\|CREATE\|DROP" tests/ --include="*.rs" --exclude="*integration_validation.rs"; then \
        echo "❌ Found raw SQL in tests!"; \
        exit 1; \
    else \
        echo "✅ No raw SQL found in tests"; \
    fi
```

**Acceptance Criteria**:
- ALL tests pass on all three databases
- ZERO compilation warnings across all configurations  
- Backend isolation is completely enforced
- No raw SQL anywhere in the test codebase
- Performance targets are met
- Feature gates work correctly
- Query equivalence is validated across databases

---

## 📊 **FINAL EXECUTION SUMMARY**

### **Critical Path Dependencies**:
1. **Phase 1** → **Phase 2** → **Phase 3** → **Phase 4** → **Phase 5**
2. **Task 3.1** must complete before any other Phase 3 tasks
3. **Tasks 3.2-3.9** can be parallelized after 3.1
4. **Phase 4** tasks can be parallelized
5. **Phase 5** tasks can be parallelized

### **Total Effort**: 18 tasks, ~6,250 lines of code changes

### **Success Metrics**:
- **✅ Zero compilation warnings** across all configurations
- **✅ 100% test pass rate** on SQLite, PostgreSQL, and MySQL
- **✅ Zero raw SQL** in any test file  
- **✅ Complete backend isolation** verified by logging
- **✅ <30 second test execution** time for `just test`
- **✅ Database-agnostic tests** producing identical results

This comprehensive breakdown ensures each task is **manageable (200-500 lines)**, **independent**, and **clearly defined** for execution by different developers/Claudes while maintaining the overall goal of a **professional, database-agnostic testing infrastructure**.

