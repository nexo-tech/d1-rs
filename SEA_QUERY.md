# Sea-Query Migration Plan: Database Agnostic ORM

This document outlines the comprehensive plan to migrate d1-rs from SQLite-centric raw SQL to a database-agnostic architecture using SeaQL's sea-query.

## 📋 **TASK PROGRESS CHECKLIST**

**Track your progress by checking off completed tasks:**

### **Phase 1: Foundation & Dependencies (Week 1)**
- [ ] **Task 1.1**: Basic Dependencies Setup (~200 lines, 2-3 hours)
- [ ] **Task 1.2**: DatabaseDialect Enum (~150 lines, 1-2 hours)
- [ ] **Task 1.3**: QueryResult Trait (~250 lines, 3-4 hours)
- [ ] **Task 1.4**: Basic DatabaseBackend Trait (~200 lines, 2-3 hours)
- [ ] **Task 1.5**: Raw SQL Audit & Documentation (~300 lines, 4-5 hours)
- [ ] **Task 1.6**: Update lib.rs Exports (~100 lines, 1 hour)

### **Phase 2: Database Client Refactor (Week 2)**
- [ ] **Task 2.1**: SQLite Backend Implementation (~400 lines, 6-8 hours)
- [ ] **Task 2.2**: Generic DatabaseClient Wrapper (~300 lines, 4-5 hours)
- [ ] **Task 2.3**: Database Configuration Structs (~250 lines, 3-4 hours)
- [ ] **Task 2.4**: PostgreSQL Backend Stub (~350 lines, 5-6 hours)
- [ ] **Task 2.5**: Update lib.rs with Backend Exports (~100 lines, 1-2 hours)
- [ ] **Task 2.6**: MySQL Backend Stub (~350 lines, 5-6 hours)

### **Phase 3: Query Builder Migration (Week 3-4)**
- [ ] **Task 3.1**: Basic Sea-Query Integration (~300 lines, 4-5 hours)
- [ ] **Task 3.2**: Replace Query Struct in queries.rs (~400 lines, 6-7 hours)
- [ ] **Task 3.3**: Basic SELECT Builder (~350 lines, 5-6 hours)
- [ ] **Task 3.4**: Basic INSERT Builder (~300 lines, 4-5 hours)
- [ ] **Task 3.5**: Basic UPDATE Builder (~250 lines, 3-4 hours)
- [ ] **Task 3.6**: Basic DELETE Builder (~200 lines, 2-3 hours)

### **Phase 4: Schema Introspection (Week 5)**
- [ ] **Task 4.1**: SchemaIntrospector Trait Definition (~200 lines, 3-4 hours)
- [ ] **Task 4.2**: SQLite Schema Introspector (~400 lines, 6-8 hours)
- [ ] **Task 4.3**: PostgreSQL Schema Introspector (~450 lines, 7-9 hours)
- [ ] **Task 4.4**: MySQL Schema Introspector (~450 lines, 7-9 hours)
- [ ] **Task 4.5**: Unified Schema Representation (~300 lines, 4-5 hours)
- [ ] **Task 4.6**: Cross-Database Type Mapping (~250 lines, 3-4 hours)

### **Phase 5: Migration System Overhaul (Week 6-7)**
- [ ] **Task 5.1**: Schema Diffing Engine (~400 lines, 6-8 hours)
- [ ] **Task 5.2**: Database-Agnostic DDL Generation (~450 lines, 7-9 hours)
- [ ] **Task 5.3**: Migration Plan Execution (~350 lines, 5-6 hours)
- [ ] **Task 5.4**: Auto-Migration Integration (~400 lines, 6-8 hours)
- [ ] **Task 5.5**: Migration Rollback System (~300 lines, 4-5 hours)
- [ ] **Task 5.6**: Data Migration Support (~350 lines, 5-6 hours)

### **Phase 6: Testing Infrastructure (Week 8)**
- [ ] **Task 6.1**: Multi-Database Test Setup (~300 lines, 4-5 hours)
- [ ] **Task 6.2**: Cross-Database Test Suite (~400 lines, 6-8 hours)
- [ ] **Task 6.3**: Performance Benchmarking (~250 lines, 3-4 hours)
- [ ] **Task 6.4**: CI/CD Integration (~200 lines, 2-3 hours)
- [ ] **Task 6.5**: Test Data Management (~300 lines, 4-5 hours)

### **Phase 7: Documentation & Optimization (Week 9)**
- [ ] **Task 7.1**: Documentation Updates (~200 lines, 2-3 hours)
- [ ] **Task 7.2**: Performance Optimization (~300 lines, 4-5 hours)
- [ ] **Task 7.3**: Security Review (~150 lines, 1-2 hours)
- [ ] **Task 7.4**: Production Readiness (~200 lines, 2-3 hours)

**Total Tasks: 40 | Total Estimated Effort: ~220 hours (5-6 weeks)**

---

## 🎯 **PHASE COMPLETION TRACKING**

**Mark phases complete only when ALL tasks in that phase are done:**

- [ ] ✅ **Phase 1 Complete** - Foundation & Dependencies
- [ ] ✅ **Phase 2 Complete** - Database Client Refactor  
- [ ] ✅ **Phase 3 Complete** - Query Builder Migration
- [ ] ✅ **Phase 4 Complete** - Schema Introspection
- [ ] ✅ **Phase 5 Complete** - Migration System Overhaul
- [ ] ✅ **Phase 6 Complete** - Testing Infrastructure
- [ ] ✅ **Phase 7 Complete** - Documentation & Optimization

🎉 **Project Complete**: All phases finished, full database-agnostic ORM ready!

---

## 🎯 Project Goals

**Primary Objective**: Transform d1-rs into a truly database-agnostic ORM that supports:
- ✅ **SQLite** (existing, for testing & embedded use)
- ✅ **PostgreSQL** (production web applications)
- ✅ **MySQL** (enterprise compatibility)

**Architecture Goals**:
- **Zero Raw SQL**: Replace all hand-written SQL with sea-query builders
- **Dialect-Aware**: Automatic SQL generation for each database backend
- **Schema Introspection**: Database-agnostic schema discovery and migration
- **Type Safety**: Maintain compile-time safety across all databases
- **Performance**: Generate optimized SQL for each database dialect
- **Testing**: Unified test suite that runs against all supported databases

---

## 📋 Current State Analysis

### Critical Issues to Address:

1. **Raw SQL Everywhere**: Query generation uses string concatenation
2. **SQLite-Only Introspection**: Heavy use of `sqlite_master` and `pragma_*` functions
3. **Hardcoded Syntax**: No dialect abstraction for SQL differences
4. **Single Backend**: D1Client only supports SQLite/D1
5. **Type Mapping**: Boolean/type conversion only works for SQLite
6. **Migration System**: Auto-migration system is SQLite-specific

### Affected Components:
- **Query Building** (`src/query/`)
- **Database Client** (`src/db.rs`)
- **Schema Introspection** (`src/auto_migration/introspector.rs`)
- **Migration System** (`src/auto_migration/`, `src/migrations.rs`)
- **Type System** (`src/types.rs`)
- **Derive Macro** (`d1-rs-derive/src/lib.rs`)

---

## 🏗️ Architecture Design

### New Component Structure:

```
src/
├── backends/
│   ├── mod.rs              # Backend trait definitions
│   ├── sqlite.rs           # SQLite implementation
│   ├── postgres.rs         # PostgreSQL implementation
│   ├── mysql.rs            # MySQL implementation
│   └── d1.rs              # Cloudflare D1 implementation (WASM)
├── query_builder/
│   ├── mod.rs              # Re-exports and common types
│   ├── select.rs           # Type-safe SELECT builders
│   ├── insert.rs           # Type-safe INSERT builders
│   ├── update.rs           # Type-safe UPDATE builders
│   ├── delete.rs           # Type-safe DELETE builders
│   └── schema.rs           # DDL operations (CREATE/ALTER/DROP)
├── introspection/
│   ├── mod.rs              # Database-agnostic introspection trait
│   ├── sqlite.rs           # SQLite introspection (pragma_*)
│   ├── postgres.rs         # PostgreSQL introspection (information_schema)
│   └── mysql.rs            # MySQL introspection (information_schema)
├── dialects/
│   ├── mod.rs              # Dialect trait and common types
│   ├── sqlite.rs           # SQLite-specific type mappings
│   ├── postgres.rs         # PostgreSQL-specific type mappings
│   └── mysql.rs            # MySQL-specific type mappings
└── migration_engine/
    ├── mod.rs              # Database-agnostic migration engine
    ├── differ.rs           # Cross-database schema diffing
    ├── planner.rs          # Database-aware migration planning
    └── executor.rs         # Backend-specific execution
```

---

## 🚀 Implementation Phases

## Phase 1: Foundation & Dependencies (Week 1)

**Goal**: Establish the foundation for database-agnostic architecture without breaking existing functionality.

### Task 1.1: Basic Dependencies Setup (~200 lines)
**Estimated effort**: 2-3 hours | **Files**: `Cargo.toml`, `d1-rs-derive/Cargo.toml`

**Changes to make**:
- [ ] Add `sea-query = "0.31"` to main Cargo.toml dependencies
- [ ] Add `sea-query = "0.31"` to derive crate dependencies  
- [ ] Add feature flags: `postgres = ["sqlx/postgres"]`, `mysql = ["sqlx/mysql"]`, `sqlite = ["sqlx/sqlite"]`
- [ ] Add `sqlx = { version = "0.7", optional = true }` with feature flags
- [ ] Add `async-trait = "0.1"` dependency
- [ ] Update feature matrix in Cargo.toml comments

**Acceptance criteria**:
- [ ] `cargo check` passes with no warnings
- [ ] `cargo check --features postgres` compiles
- [ ] `cargo check --features mysql` compiles  
- [ ] Default features still work (existing SQLite tests pass)

**Testing**: `just test` must pass without any feature flags (existing functionality)

---

### Task 1.2: DatabaseDialect Enum (~150 lines)
**Estimated effort**: 1-2 hours | **Files**: `src/dialects/mod.rs` (new)

**Create the core dialect enum**:
```rust
// src/dialects/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatabaseDialect {
    SQLite,
    #[cfg(feature = "postgres")]
    PostgreSQL,
    #[cfg(feature = "mysql")]
    MySQL,
}

impl std::fmt::Display for DatabaseDialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseDialect::SQLite => write!(f, "SQLite"),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => write!(f, "PostgreSQL"),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => write!(f, "MySQL"),
        }
    }
}

impl DatabaseDialect {
    pub fn supports_returning(&self) -> bool {
        match self {
            DatabaseDialect::SQLite => true,
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => true,
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => false,
        }
    }
    
    pub fn quote_identifier(&self, name: &str) -> String {
        match self {
            DatabaseDialect::SQLite => format!("`{}`", name),
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => format!("\"{}\"", name),
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => format!("`{}`", name),
        }
    }
}
```

**Acceptance criteria**:
- [ ] Enum compiles with all feature combinations
- [ ] Display trait works correctly
- [ ] Helper methods provide database-specific behavior
- [ ] Zero compilation warnings

**Testing**: Add unit tests for each method and feature combination

---

### Task 1.3: QueryResult Trait (~250 lines)
**Estimated effort**: 3-4 hours | **Files**: `src/backends/mod.rs` (new)

**Create the core QueryResult abstraction**:
```rust
// src/backends/mod.rs
use serde_json::Value;
use std::collections::HashMap;

pub trait QueryResult: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    
    // Core data access
    fn rows(&self) -> &[Value];
    fn into_rows(self) -> Vec<Value>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    
    // Entity conversion (using existing logic)
    fn into_entities<T>(self) -> Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + crate::Entity;
        
    fn into_entity<T>(self) -> Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + crate::Entity;
        
    // Simple deserialization (non-Entity types)
    fn into_simple_entities<T>(self) -> Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned;
        
    fn into_simple_entity<T>(self) -> Result<Option<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned;
        
    // Utility methods for common patterns
    fn extract_count(&self) -> Result<i64, Self::Error>;
    fn extract_id(&self) -> Result<Option<i64>, Self::Error>;
    fn extract_strings(&self) -> Result<Vec<String>, Self::Error>;
}

// Implement for existing D1QueryResult to maintain compatibility
impl QueryResult for crate::db::D1QueryResult {
    type Error = crate::D1RsError;
    
    fn rows(&self) -> &[Value] {
        &self.rows
    }
    
    fn into_rows(self) -> Vec<Value> {
        self.rows
    }
    
    fn len(&self) -> usize {
        self.rows.len()
    }
    
    fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    
    // ... implement all methods using existing D1QueryResult logic
}
```

**Acceptance criteria**:
- [ ] Trait compiles and is well-documented
- [ ] D1QueryResult implementation maintains full compatibility
- [ ] All existing functionality works through trait methods
- [ ] Zero compilation warnings

**Testing**: Ensure all existing tests pass using trait methods

---

### Task 1.4: Basic DatabaseBackend Trait (~200 lines)
**Estimated effort**: 2-3 hours | **Files**: `src/backends/mod.rs` (extend)

**Add the core backend trait**:
```rust
// Add to src/backends/mod.rs
use async_trait::async_trait;
use crate::dialects::DatabaseDialect;

#[async_trait]
pub trait DatabaseBackend: Send + Sync + Clone {
    type QueryResult: QueryResult;
    type Error: std::error::Error + Send + Sync + 'static;
    
    // Core query execution
    async fn execute_query(
        &self, 
        sql: &str, 
        params: &[Value]
    ) -> Result<Self::QueryResult, Self::Error>;
    
    // Schema operations (DDL)
    async fn execute_schema(&self, sql: &str) -> Result<(), Self::Error>;
    
    // Metadata
    fn dialect(&self) -> DatabaseDialect;
    
    // Connection info (for debugging/monitoring)
    fn connection_info(&self) -> String;
    
    // Health check
    async fn ping(&self) -> Result<(), Self::Error>;
}

// Backend-specific error types
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Query error: {0}")]
    Query(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}
```

**Acceptance criteria**:
- [ ] Trait compiles with good documentation
- [ ] Error types are comprehensive
- [ ] Trait design allows for all database operations
- [ ] Zero compilation warnings

**Testing**: Create mock implementation for testing

---

### Task 1.5: Raw SQL Audit & Documentation (~300 lines)
**Estimated effort**: 4-5 hours | **Files**: `RAW_SQL_AUDIT.md` (new)

**Create comprehensive audit of all raw SQL usage**:

1. **Scan all `.rs` files** for SQL strings, format!() with SQL, pragma usage
2. **Categorize each usage**:
   - Query building (SELECT, INSERT, UPDATE, DELETE)
   - Schema introspection (sqlite_master, pragma_*)
   - DDL operations (CREATE, ALTER, DROP)
   - Database-specific operations

3. **Document in `RAW_SQL_AUDIT.md`**:
```markdown
# Raw SQL Audit Report

## Summary
- Total SQL usages found: XXX
- Query building: XX occurrences  
- Schema introspection: XX occurrences
- DDL operations: XX occurrences

## By File:

### src/query/queries.rs (Priority: HIGH)
- Lines 71-101: SELECT query building
- Sea-query equivalent: Query::select().from(table).where_()...
- Estimated effort: 2-3 hours

### src/auto_migration/introspector.rs (Priority: HIGH)  
- Lines 34, 69: sqlite_master queries
- Lines 301, 321: pragma_table_info usage  
- Sea-query equivalent: SchemaIntrospector trait
- Estimated effort: 8-10 hours
```

**Acceptance criteria**:
- [ ] All SQL usage documented with file/line numbers
- [ ] Each usage categorized by type and priority
- [ ] Sea-query equivalent identified for each usage
- [ ] Effort estimation provided for replacements
- [ ] Migration strategy outlined

**Testing**: Grep verification that all SQL patterns are captured

---

### Task 1.6: Update lib.rs Exports (~100 lines)
**Estimated effort**: 1 hour | **Files**: `src/lib.rs`

**Add new module exports without breaking existing APIs**:
```rust
// Add to src/lib.rs
pub mod dialects;
pub mod backends;

// Re-export core types for convenience
pub use dialects::{DatabaseDialect};
pub use backends::{DatabaseBackend, QueryResult, BackendError};

// Feature-gated exports
#[cfg(any(feature = "postgres", feature = "mysql"))]
pub use backends::sqlx_support;
```

**Acceptance criteria**:
- [ ] New modules exported correctly
- [ ] Existing exports remain unchanged
- [ ] Feature flags work correctly
- [ ] Zero compilation warnings

**Testing**: Ensure all existing imports still work

### Implementation Details:

#### `DatabaseBackend` Trait (`src/backends/mod.rs`):
```rust
#[async_trait]
pub trait DatabaseBackend: Send + Sync + Clone {
    type Connection: Send + Sync;
    type QueryResult: QueryResult;
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn execute_query(&self, sql: &str, params: &[Value]) -> Result<Self::QueryResult, Self::Error>;
    async fn execute_schema(&self, sql: &str) -> Result<(), Self::Error>;
    fn dialect(&self) -> DatabaseDialect;
    fn connection(&self) -> &Self::Connection;
}
```

#### `DatabaseDialect` Enum (`src/dialects/mod.rs`):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseDialect {
    SQLite,
    PostgreSQL,
    MySQL,
}

pub trait DialectRenderer {
    fn render_query(&self, query: &sea_query::SelectStatement) -> (String, Vec<Value>);
    fn render_insert(&self, query: &sea_query::InsertStatement) -> (String, Vec<Value>);
    fn render_update(&self, query: &sea_query::UpdateStatement) -> (String, Vec<Value>);
    fn render_delete(&self, query: &sea_query::DeleteStatement) -> (String, Vec<Value>);
    fn render_schema(&self, schema: &sea_query::SchemaStatement) -> String;
}
```

**Acceptance Criteria**:
- All core traits defined and documented
- Feature flags working correctly
- Zero compilation warnings
- Basic trait implementations compile successfully

---

## Phase 2: Database Client Refactor (Week 2)

**Goal**: Create concrete backend implementations while maintaining 100% compatibility with existing D1Client.

### Task 2.1: SQLite Backend Implementation (~400 lines)
**Estimated effort**: 6-8 hours | **Files**: `src/backends/sqlite.rs` (new)

**Create SQLite backend that wraps existing D1Client logic**:
```rust
// src/backends/sqlite.rs
use super::{DatabaseBackend, QueryResult, BackendError};
use crate::dialects::DatabaseDialect;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use worker::d1::D1Database;
#[cfg(not(target_arch = "wasm32"))]
use {rusqlite::Connection, tokio::sync::Mutex};

#[derive(Clone)]
pub struct SQLiteBackend {
    #[cfg(target_arch = "wasm32")]
    db: Arc<D1Database>,
    #[cfg(not(target_arch = "wasm32"))]
    db: Arc<Mutex<Connection>>,
}

// SQLite-specific QueryResult that wraps existing D1QueryResult
pub struct SQLiteQueryResult {
    inner: crate::db::D1QueryResult,
}

impl QueryResult for SQLiteQueryResult {
    type Error = crate::D1RsError;
    
    fn rows(&self) -> &[Value] {
        &self.inner.rows
    }
    
    fn into_rows(self) -> Vec<Value> {
        self.inner.rows
    }
    
    // ... implement all QueryResult methods by delegating to inner
}

#[async_trait]
impl DatabaseBackend for SQLiteBackend {
    type QueryResult = SQLiteQueryResult;
    type Error = crate::D1RsError;
    
    async fn execute_query(&self, sql: &str, params: &[Value]) -> Result<Self::QueryResult, Self::Error> {
        // Create temporary D1Client and use existing logic
        #[cfg(target_arch = "wasm32")]
        let client = crate::D1Client::new((*self.db).clone());
        #[cfg(not(target_arch = "wasm32"))]
        let client = /* construct from self.db */;
        
        let result = client.execute(sql, params).await?;
        Ok(SQLiteQueryResult { inner: result })
    }
    
    fn dialect(&self) -> DatabaseDialect {
        DatabaseDialect::SQLite
    }
    
    // ... implement remaining methods
}
```

**Acceptance criteria**:
- [ ] Backend compiles for both WASM and native targets
- [ ] All existing D1Client functionality accessible through backend
- [ ] QueryResult trait implementation works correctly
- [ ] Zero compilation warnings

**Testing**: Create test that verifies existing D1Client tests pass through SQLiteBackend

---

### Task 2.2: Generic DatabaseClient Wrapper (~300 lines)
**Estimated effort**: 4-5 hours | **Files**: `src/db.rs` (extend)

**Add generic DatabaseClient that works with any backend**:
```rust
// Add to src/db.rs (keep existing D1Client for compatibility)

#[derive(Clone)]
pub struct DatabaseClient<B: DatabaseBackend> {
    backend: B,
    dialect: DatabaseDialect,
}

impl<B: DatabaseBackend> DatabaseClient<B> {
    pub fn new(backend: B) -> Self {
        let dialect = backend.dialect();
        Self { backend, dialect }
    }
    
    pub fn dialect(&self) -> DatabaseDialect {
        self.dialect
    }
    
    // Direct query execution (for raw SQL during migration period)
    pub async fn execute(&self, sql: &str, params: &[Value]) -> Result<B::QueryResult, B::Error> {
        self.backend.execute_query(sql, params).await
    }
    
    pub async fn execute_returning_one(&self, sql: &str, params: &[Value]) -> Result<Option<HashMap<String, Value>>, B::Error> {
        let result = self.execute(sql, params).await?;
        // Convert first row to HashMap
        if let Some(row) = result.rows().first() {
            if let Value::Object(obj) = row {
                let hashmap: HashMap<String, Value> = obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                return Ok(Some(hashmap));
            }
        }
        Ok(None)
    }
    
    pub async fn execute_returning_count(&self, sql: &str, params: &[Value]) -> Result<i64, B::Error> {
        let result = self.execute(sql, params).await?;
        result.extract_count()
    }
    
    // Schema operations
    pub async fn execute_schema(&self, sql: &str) -> Result<(), B::Error> {
        self.backend.execute_schema(sql).await
    }
    
    // Health check
    pub async fn ping(&self) -> Result<(), B::Error> {
        self.backend.ping().await
    }
}

// Type alias for SQLite backend (most common case)
pub type SQLiteClient = DatabaseClient<crate::backends::SQLiteBackend>;

// Compatibility: Keep D1Client as type alias during migration
pub type D1Client = SQLiteClient;
```

**Acceptance criteria**:
- [ ] Generic client compiles and works with SQLiteBackend
- [ ] Existing D1Client type alias maintains compatibility
- [ ] All current D1Client methods available on new client
- [ ] Zero compilation warnings

**Testing**: Replace one test to use DatabaseClient<SQLiteBackend> instead of D1Client

---

### Task 2.3: Database Configuration Structs (~250 lines)
**Estimated effort**: 3-4 hours | **Files**: `src/backends/config.rs` (new)

**Create configuration system for all database types**:
```rust
// src/backends/config.rs
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub pool_config: Option<PoolConfig>,
    pub ssl_config: Option<SslConfig>,
    pub timeout_config: TimeoutConfig,
}

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct SslConfig {
    pub mode: SslMode,
    pub ca_cert_path: Option<String>,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SslMode {
    Disable,
    Allow,
    Prefer,
    Require,
    VerifyCA,
    VerifyFull,
}

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub connect_timeout: Duration,
    pub query_timeout: Duration,
    pub transaction_timeout: Duration,
}

impl DatabaseConfig {
    // Constructors for each database type
    pub fn sqlite_memory() -> Self {
        Self {
            url: ":memory:".to_string(),
            pool_config: None,
            ssl_config: None,
            timeout_config: TimeoutConfig::default(),
        }
    }
    
    pub fn sqlite_file(path: &str) -> Self {
        Self {
            url: format!("sqlite:{}", path),
            pool_config: None,
            ssl_config: None,
            timeout_config: TimeoutConfig::default(),
        }
    }
    
    #[cfg(feature = "postgres")]
    pub fn postgres(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool_config: Some(PoolConfig::postgres_default()),
            ssl_config: Some(SslConfig::default()),
            timeout_config: TimeoutConfig::default(),
        }
    }
    
    #[cfg(feature = "mysql")]
    pub fn mysql(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool_config: Some(PoolConfig::mysql_default()),
            ssl_config: Some(SslConfig::default()),
            timeout_config: TimeoutConfig::default(),
        }
    }
    
    // Parse from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingUrl)?;
            
        if url.starts_with("postgres://") {
            #[cfg(feature = "postgres")]
            return Ok(Self::postgres(&url));
            #[cfg(not(feature = "postgres"))]
            return Err(ConfigError::UnsupportedDatabase("PostgreSQL"));
        }
        
        if url.starts_with("mysql://") {
            #[cfg(feature = "mysql")]
            return Ok(Self::mysql(&url));
            #[cfg(not(feature = "mysql"))]
            return Err(ConfigError::UnsupportedDatabase("MySQL"));
        }
        
        if url.starts_with("sqlite:") || !url.contains("://") {
            return Ok(Self::sqlite_file(&url));
        }
        
        Err(ConfigError::InvalidUrl(url))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("DATABASE_URL environment variable not found")]
    MissingUrl,
    #[error("Unsupported database type: {0}")]
    UnsupportedDatabase(&'static str),
    #[error("Invalid database URL: {0}")]
    InvalidUrl(String),
}
```

**Acceptance criteria**:
- [ ] Configuration structs compile and work with feature flags
- [ ] Environment variable parsing works correctly
- [ ] Default configurations are sensible for each database
- [ ] Zero compilation warnings

**Testing**: Test configuration parsing for all supported URL formats

---

### Task 2.4: PostgreSQL Backend Stub (~350 lines)
**Estimated effort**: 5-6 hours | **Files**: `src/backends/postgres.rs` (new)

**Create basic PostgreSQL backend (feature-gated)**:
```rust
// src/backends/postgres.rs
#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use super::{DatabaseBackend, QueryResult, BackendError};
#[cfg(feature = "postgres")]
use crate::dialects::DatabaseDialect;
#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use serde_json::Value;
#[cfg(feature = "postgres")]
use std::sync::Arc;

#[cfg(feature = "postgres")]
#[derive(Clone)]
pub struct PostgreSQLBackend {
    pool: Arc<PgPool>,
}

#[cfg(feature = "postgres")]
pub struct PostgreSQLQueryResult {
    rows: Vec<Value>,
}

#[cfg(feature = "postgres")]
impl QueryResult for PostgreSQLQueryResult {
    type Error = crate::D1RsError;
    
    fn rows(&self) -> &[Value] {
        &self.rows
    }
    
    fn into_rows(self) -> Vec<Value> {
        self.rows
    }
    
    fn len(&self) -> usize {
        self.rows.len()
    }
    
    fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    
    fn into_entities<T>(self) -> Result<Vec<T>, Self::Error>
    where
        T: serde::de::DeserializeOwned + crate::Entity,
    {
        self.rows
            .into_iter()
            .map(|row| {
                let converted = T::convert_from_sqlite(row);
                serde_json::from_value(converted)
                    .map_err(|e| crate::D1RsError::SerializationError(e.to_string()))
            })
            .collect()
    }
    
    // ... implement remaining QueryResult methods
}

#[cfg(feature = "postgres")]
#[async_trait]
impl DatabaseBackend for PostgreSQLBackend {
    type QueryResult = PostgreSQLQueryResult;
    type Error = crate::D1RsError;
    
    async fn execute_query(&self, sql: &str, params: &[Value]) -> Result<Self::QueryResult, Self::Error> {
        // Convert serde_json::Value params to sqlx values
        let sqlx_params = convert_params_to_sqlx(params);
        
        // Execute query using sqlx
        let rows = sqlx::query(sql)
            .bind_all(sqlx_params)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| crate::D1RsError::Database(e.to_string()))?;
            
        // Convert sqlx rows to serde_json::Value
        let json_rows = rows.into_iter()
            .map(|row| convert_sqlx_row_to_json(row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| crate::D1RsError::SerializationError(e.to_string()))?;
            
        Ok(PostgreSQLQueryResult { rows: json_rows })
    }
    
    async fn execute_schema(&self, sql: &str) -> Result<(), Self::Error> {
        sqlx::query(sql)
            .execute(&*self.pool)
            .await
            .map_err(|e| crate::D1RsError::Database(e.to_string()))?;
        Ok(())
    }
    
    fn dialect(&self) -> DatabaseDialect {
        DatabaseDialect::PostgreSQL
    }
    
    fn connection_info(&self) -> String {
        format!("PostgreSQL pool with {} connections", self.pool.size())
    }
    
    async fn ping(&self) -> Result<(), Self::Error> {
        sqlx::query("SELECT 1")
            .execute(&*self.pool)
            .await
            .map_err(|e| crate::D1RsError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(feature = "postgres")]
impl PostgreSQLBackend {
    pub async fn new(url: &str) -> Result<Self, crate::D1RsError> {
        let pool = PgPool::connect(url)
            .await
            .map_err(|e| crate::D1RsError::Database(e.to_string()))?;
        Ok(Self {
            pool: Arc::new(pool),
        })
    }
    
    pub async fn from_config(config: &crate::backends::DatabaseConfig) -> Result<Self, crate::D1RsError> {
        // Use config to set up connection pool
        Self::new(&config.url).await
    }
}

// Helper functions for type conversion
#[cfg(feature = "postgres")]
fn convert_params_to_sqlx(params: &[Value]) -> Vec<sqlx::types::Json<Value>> {
    // Convert serde_json::Value to sqlx parameters
    // This is a simplified implementation - would need proper type handling
    params.iter().map(|v| sqlx::types::Json(v.clone())).collect()
}

#[cfg(feature = "postgres")]
fn convert_sqlx_row_to_json(row: sqlx::postgres::PgRow) -> Result<Value, sqlx::Error> {
    // Convert PostgreSQL row to serde_json::Value
    // This is a simplified implementation - would need proper column type handling
    todo!("Implement proper row conversion")
}

// Re-export for when feature is disabled
#[cfg(not(feature = "postgres"))]
pub struct PostgreSQLBackend;
```

**Acceptance criteria**:
- [ ] Backend compiles with postgres feature flag
- [ ] Basic connection and query execution works
- [ ] Proper error handling and type conversion
- [ ] Graceful compilation when feature is disabled
- [ ] Zero compilation warnings

**Testing**: Create basic connection test when postgres feature is enabled

---

### Task 2.5: Update lib.rs with Backend Exports (~100 lines)
**Estimated effort**: 1-2 hours | **Files**: `src/lib.rs`

**Add backend module exports and type aliases**:
```rust
// Add to src/lib.rs after existing exports

// Backend system exports
pub mod backends {
    pub mod config;
    pub mod sqlite;
    
    #[cfg(feature = "postgres")]
    pub mod postgres;
    
    #[cfg(feature = "mysql")]
    pub mod mysql;
    
    // Re-export core types
    pub use super::backends::{DatabaseBackend, QueryResult, BackendError};
    pub use config::*;
    pub use sqlite::*;
    
    #[cfg(feature = "postgres")]
    pub use postgres::*;
    
    #[cfg(feature = "mysql")]
    pub use mysql::*;
}

// Convenient type aliases for common backends
pub type SQLiteClient = DatabaseClient<backends::SQLiteBackend>;

#[cfg(feature = "postgres")]
pub type PostgreSQLClient = DatabaseClient<backends::PostgreSQLBackend>;

#[cfg(feature = "mysql")]
pub type MySQLClient = DatabaseClient<backends::MySQLBackend>;

// Maintain compatibility during transition
pub use SQLiteClient as D1Client;
```

**Acceptance criteria**:
- [ ] All backends properly exported with feature flags
- [ ] Type aliases work correctly
- [ ] Existing imports remain unbroken
- [ ] Zero compilation warnings

**Testing**: Verify imports work in external code

---

### Task 2.6: MySQL Backend Stub (~350 lines)
**Estimated effort**: 5-6 hours | **Files**: `src/backends/mysql.rs` (new)

**Create basic MySQL backend (similar to PostgreSQL)**:
```rust
// src/backends/mysql.rs
#[cfg(feature = "mysql")]
use sqlx::MySqlPool;
// ... similar structure to PostgreSQL backend

#[cfg(feature = "mysql")]
#[derive(Clone)]
pub struct MySQLBackend {
    pool: Arc<MySqlPool>,
}

// ... implement similar to PostgreSQL but with MySQL-specific details
```

**Acceptance criteria**: Same as PostgreSQL backend task

**Testing**: Basic connection test when mysql feature is enabled

### Implementation Details:

#### New `DatabaseClient` (`src/db.rs`):
```rust
#[derive(Clone)]
pub struct DatabaseClient<B: DatabaseBackend> {
    backend: B,
    dialect: DatabaseDialect,
}

impl<B: DatabaseBackend> DatabaseClient<B> {
    pub fn new(backend: B) -> Self {
        let dialect = backend.dialect();
        Self { backend, dialect }
    }
    
    pub async fn execute_sea_query<Q>(&self, query: Q) -> Result<B::QueryResult, B::Error> 
    where
        Q: QueryRenderer,
    {
        let (sql, params) = query.render_for_dialect(self.dialect);
        self.backend.execute_query(&sql, &params).await
    }
}
```

#### Backend Implementations:

**SQLite Backend** (`src/backends/sqlite.rs`):
```rust
#[derive(Clone)]
pub struct SQLiteBackend {
    #[cfg(target_arch = "wasm32")]
    db: Arc<D1Database>,
    #[cfg(not(target_arch = "wasm32"))]
    db: Arc<Mutex<Connection>>,
}

#[async_trait]
impl DatabaseBackend for SQLiteBackend {
    type Connection = /* ... */;
    type QueryResult = SQLiteQueryResult;
    type Error = SQLiteError;
    
    async fn execute_query(&self, sql: &str, params: &[Value]) -> Result<Self::QueryResult, Self::Error> {
        // Current D1Client logic here
    }
    
    fn dialect(&self) -> DatabaseDialect {
        DatabaseDialect::SQLite
    }
}
```

**PostgreSQL Backend** (`src/backends/postgres.rs`):
```rust
#[derive(Clone)]
pub struct PostgreSQLBackend {
    pool: Arc<sqlx::PgPool>,
}

#[async_trait]
impl DatabaseBackend for PostgreSQLBackend {
    type Connection = sqlx::PgPool;
    type QueryResult = PostgreSQLQueryResult;
    type Error = PostgreSQLError;
    
    async fn execute_query(&self, sql: &str, params: &[Value]) -> Result<Self::QueryResult, Self::Error> {
        // sqlx::query implementation
    }
    
    fn dialect(&self) -> DatabaseDialect {
        DatabaseDialect::PostgreSQL
    }
}
```

**Acceptance Criteria**:
- All backend implementations working
- Existing SQLite functionality maintained
- PostgreSQL and MySQL connections established
- Type conversion working for all databases
- Zero compilation warnings

---

## Phase 3: Query Builder Migration (Week 3-4)

**Goal**: Replace all raw SQL query building with type-safe sea-query builders while maintaining API compatibility.

### Task 3.1: Basic Sea-Query Integration (~300 lines)
**Estimated effort**: 4-5 hours | **Files**: `src/query_builder/mod.rs` (new)

**Create foundation for sea-query integration**:
```rust
// src/query_builder/mod.rs
use sea_query::{Query, SelectStatement, InsertStatement, UpdateStatement, DeleteStatement};
use sea_query::{Expr, Value as SeaValue};
use crate::backends::{DatabaseBackend, DatabaseClient};
use crate::dialects::DatabaseDialect;
use serde_json::Value;
use std::marker::PhantomData;

pub mod select;
pub mod insert;
pub mod update;
pub mod delete;

// Convert serde_json::Value to sea_query::Value
pub fn json_to_sea_value(value: &Value) -> SeaValue {
    match value {
        Value::Null => SeaValue::Null,
        Value::Bool(b) => SeaValue::Bool(Some(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SeaValue::BigInt(Some(i))
            } else if let Some(f) = n.as_f64() {
                SeaValue::Double(Some(f))
            } else {
                SeaValue::Null
            }
        },
        Value::String(s) => SeaValue::String(Some(Box::new(s.clone()))),
        Value::Array(_) | Value::Object(_) => {
            // Convert to JSON string for database storage
            SeaValue::String(Some(Box::new(value.to_string())))
        }
    }
}

// Convert sea_query::Value back to serde_json::Value
pub fn sea_value_to_json(value: &SeaValue) -> Value {
    match value {
        SeaValue::Null => Value::Null,
        SeaValue::Bool(Some(b)) => Value::Bool(*b),
        SeaValue::TinyInt(Some(i)) => Value::Number((*i as i64).into()),
        SeaValue::SmallInt(Some(i)) => Value::Number((*i as i64).into()),
        SeaValue::Int(Some(i)) => Value::Number((*i as i64).into()),
        SeaValue::BigInt(Some(i)) => Value::Number((*i).into()),
        SeaValue::Float(Some(f)) => Value::Number(serde_json::Number::from_f64(*f as f64).unwrap_or(0.into())),
        SeaValue::Double(Some(f)) => Value::Number(serde_json::Number::from_f64(*f).unwrap_or(0.into())),
        SeaValue::String(Some(s)) => Value::String(s.as_ref().clone()),
        _ => Value::Null,
    }
}

// Trait for rendering sea-query statements to SQL for specific dialects
pub trait QueryRenderer {
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>);
}

impl QueryRenderer for SelectStatement {
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        match dialect {
            DatabaseDialect::SQLite => {
                let (sql, values) = self.to_string(sea_query::SqliteQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let (sql, values) = self.to_string(sea_query::PostgresQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                let (sql, values) = self.to_string(sea_query::MysqlQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
        }
    }
}

impl QueryRenderer for InsertStatement {
    fn render_for_dialect(&self, dialect: DatabaseDialect) -> (String, Vec<Value>) {
        match dialect {
            DatabaseDialect::SQLite => {
                let (sql, values) = self.to_string(sea_query::SqliteQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "postgres")]
            DatabaseDialect::PostgreSQL => {
                let (sql, values) = self.to_string(sea_query::PostgresQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
            #[cfg(feature = "mysql")]
            DatabaseDialect::MySQL => {
                let (sql, values) = self.to_string(sea_query::MysqlQueryBuilder);
                let json_values = values.iter().map(sea_value_to_json).collect();
                (sql, json_values)
            },
        }
    }
}

// Similar implementations for UpdateStatement and DeleteStatement...
```

**Acceptance criteria**:
- [ ] Basic sea-query integration compiles with all feature combinations
- [ ] Value conversion functions work correctly
- [ ] QueryRenderer trait works for all database dialects
- [ ] Zero compilation warnings

**Testing**: Unit tests for value conversion and query rendering

---

### Task 3.2: Replace Query Struct in queries.rs (~400 lines)
**Estimated effort**: 6-7 hours | **Files**: `src/query/queries.rs` (replace existing)

**Replace existing Query struct with sea-query SelectStatement wrapper**:
```rust
// Replace content in src/query/queries.rs
use crate::query_builder::{QueryRenderer, json_to_sea_value};
use sea_query::{Query as SeaQuery, SelectStatement, Expr, Order};
use serde_json::Value;

// Keep same public API but use sea-query internally
#[derive(Debug)]
pub struct Query {
    inner: SelectStatement,
    table: String,
}

impl Query {
    pub fn new(table: String) -> Self {
        let mut select = SeaQuery::select();
        select.from(sea_query::Alias::new(&table))
              .column(sea_query::Asterisk);
        
        Self {
            inner: select,
            table,
        }
    }

    pub fn where_clause(&mut self, column: &str, operator: &str, value: Value) -> &mut Self {
        let sea_value = json_to_sea_value(&value);
        let column_expr = Expr::col(sea_query::Alias::new(column));
        
        match operator {
            "=" => { self.inner.and_where(column_expr.eq(sea_value)); },
            "!=" => { self.inner.and_where(column_expr.ne(sea_value)); },
            ">" => { self.inner.and_where(column_expr.gt(sea_value)); },
            ">=" => { self.inner.and_where(column_expr.gte(sea_value)); },
            "<" => { self.inner.and_where(column_expr.lt(sea_value)); },
            "<=" => { self.inner.and_where(column_expr.lte(sea_value)); },
            "LIKE" => { self.inner.and_where(column_expr.like(sea_value)); },
            "IN" => {
                // Handle IN operator specially
                if let Value::Array(values) = value {
                    let sea_values: Vec<_> = values.iter().map(json_to_sea_value).collect();
                    self.inner.and_where(column_expr.is_in(sea_values));
                }
            },
            _ => {
                // Default to equality for unknown operators
                self.inner.and_where(column_expr.eq(sea_value));
            }
        }
        self
    }

    pub fn order_by(&mut self, column: &str, ascending: bool) -> &mut Self {
        let order = if ascending { Order::Asc } else { Order::Desc };
        self.inner.order_by(sea_query::Alias::new(column), order);
        self
    }

    pub fn limit(&mut self, limit: i64) -> &mut Self {
        self.inner.limit(limit as u64);
        self
    }

    pub fn offset(&mut self, offset: i64) -> &mut Self {
        self.inner.offset(offset as u64);
        self
    }

    // Keep compatibility with existing to_sql method
    pub fn to_sql(&self) -> (String, Vec<Value>) {
        // Default to SQLite for backward compatibility
        self.inner.render_for_dialect(crate::dialects::DatabaseDialect::SQLite)
    }
    
    // New method that respects database dialect
    pub fn to_sql_for_dialect(&self, dialect: crate::dialects::DatabaseDialect) -> (String, Vec<Value>) {
        self.inner.render_for_dialect(dialect)
    }
}

// Implement QueryRenderer for our Query wrapper
impl QueryRenderer for Query {
    fn render_for_dialect(&self, dialect: crate::dialects::DatabaseDialect) -> (String, Vec<Value>) {
        self.inner.render_for_dialect(dialect)
    }
}
```

**Acceptance criteria**:
- [ ] All existing Query methods work identically
- [ ] Generated SQL is equivalent to previous implementation
- [ ] Works correctly with all database dialects
- [ ] Zero compilation warnings
- [ ] All existing tests pass

**Testing**: Run existing query tests and verify identical SQL output

---

### Task 3.3: Basic SELECT Builder (~350 lines)
**Estimated effort**: 5-6 hours | **Files**: `src/query_builder/select.rs` (new)

**Create type-safe SELECT builder for entities**:
```rust
// src/query_builder/select.rs
use super::QueryRenderer;
use crate::backends::{DatabaseBackend, DatabaseClient};
use crate::{Entity, Result};
use sea_query::{Query, SelectStatement, Expr, JoinType, Order};
use std::marker::PhantomData;
use serde_json::Value;

pub struct TypeSafeSelect<T: Entity> {
    query: SelectStatement,
    _phantom: PhantomData<T>,
}

impl<T: Entity> TypeSafeSelect<T> {
    pub fn new() -> Self {
        let mut query = Query::select();
        query.from(sea_query::Alias::new(T::TABLE_NAME))
             .column(sea_query::Asterisk);
        
        Self {
            query,
            _phantom: PhantomData,
        }
    }
    
    // Basic WHERE methods
    pub fn where_eq<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = crate::query_builder::json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).eq(sea_value));
        self
    }
    
    pub fn where_ne<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = crate::query_builder::json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).ne(sea_value));
        self
    }
    
    pub fn where_like<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = crate::query_builder::json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).like(sea_value));
        self
    }
    
    pub fn where_in<V>(mut self, column: &str, values: Vec<V>) -> Self 
    where
        V: Into<Value>,
    {
        let sea_values: Vec<_> = values.into_iter()
            .map(|v| crate::query_builder::json_to_sea_value(&v.into()))
            .collect();
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).is_in(sea_values));
        self
    }
    
    // Ordering
    pub fn order_by_asc(mut self, column: &str) -> Self {
        self.query.order_by(sea_query::Alias::new(column), Order::Asc);
        self
    }
    
    pub fn order_by_desc(mut self, column: &str) -> Self {
        self.query.order_by(sea_query::Alias::new(column), Order::Desc);
        self
    }
    
    // Pagination
    pub fn limit(mut self, limit: u64) -> Self {
        self.query.limit(limit);
        self
    }
    
    pub fn offset(mut self, offset: u64) -> Self {
        self.query.offset(offset);
        self
    }
    
    // Execution methods
    pub async fn all<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<Vec<T>> {
        let (sql, params) = self.query.render_for_dialect(client.dialect());
        let result = client.execute(&sql, &params).await
            .map_err(|e| crate::D1RsError::Database(format!("{:?}", e)))?;
        result.into_entities()
    }
    
    pub async fn first<B: DatabaseBackend>(mut self, client: &DatabaseClient<B>) -> Result<Option<T>> {
        self.query.limit(1);
        let (sql, params) = self.query.render_for_dialect(client.dialect());
        let result = client.execute(&sql, &params).await
            .map_err(|e| crate::D1RsError::Database(format!("{:?}", e)))?;
        result.into_entity()
    }
    
    pub async fn count<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<i64> {
        // Create a new count query
        let mut count_query = Query::select();
        count_query.from(sea_query::Alias::new(T::TABLE_NAME))
                   .expr(Expr::count(Expr::col(sea_query::Asterisk)));
        
        // Copy WHERE conditions from the original query
        // This is simplified - would need proper WHERE clause copying
        let (sql, params) = count_query.render_for_dialect(client.dialect());
        let result = client.execute(&sql, &params).await
            .map_err(|e| crate::D1RsError::Database(format!("{:?}", e)))?;
        result.extract_count()
    }
}

impl<T: Entity> QueryRenderer for TypeSafeSelect<T> {
    fn render_for_dialect(&self, dialect: crate::dialects::DatabaseDialect) -> (String, Vec<Value>) {
        self.query.render_for_dialect(dialect)
    }
}
```

**Acceptance criteria**:
- [ ] Type-safe SELECT builder compiles and works
- [ ] Basic WHERE, ORDER BY, LIMIT/OFFSET functionality works
- [ ] Query execution methods work with all backends
- [ ] COUNT queries generate optimal SQL
- [ ] Zero compilation warnings

**Testing**: Create comprehensive tests for all SELECT builder methods

---

### Task 3.4: Basic INSERT Builder (~300 lines)
**Estimated effort**: 4-5 hours | **Files**: `src/query_builder/insert.rs` (new)

**Create type-safe INSERT builder**:
```rust
// src/query_builder/insert.rs
use super::QueryRenderer;
use crate::backends::{DatabaseBackend, DatabaseClient};
use crate::{Entity, Result};
use sea_query::{Query, InsertStatement, Expr};
use std::marker::PhantomData;
use serde_json::Value;
use std::collections::HashMap;

pub struct TypeSafeInsert<T: Entity> {
    query: InsertStatement,
    _phantom: PhantomData<T>,
}

impl<T: Entity> TypeSafeInsert<T> {
    pub fn new() -> Self {
        let query = Query::insert()
            .into_table(sea_query::Alias::new(T::TABLE_NAME))
            .to_owned();
        
        Self {
            query,
            _phantom: PhantomData,
        }
    }
    
    pub fn values(mut self, data: HashMap<String, Value>) -> Self {
        // Convert HashMap to sea-query values
        let columns: Vec<_> = data.keys().map(|k| sea_query::Alias::new(k)).collect();
        let values: Vec<_> = data.values()
            .map(|v| crate::query_builder::json_to_sea_value(v))
            .collect();
            
        self.query.columns(columns).values(values).unwrap();
        self
    }
    
    pub fn value<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = crate::query_builder::json_to_sea_value(&value.into());
        self.query.value(sea_query::Alias::new(column), sea_value);
        self
    }
    
    // RETURNING clause for databases that support it
    pub fn returning_all(mut self) -> Self {
        // Only add RETURNING for databases that support it
        self.query.returning_all();
        self
    }
    
    pub fn returning_col(mut self, column: &str) -> Self {
        self.query.returning_col(sea_query::Alias::new(column));
        self
    }
    
    // Execution methods
    pub async fn save<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<T> {
        let dialect = client.dialect();
        let mut query = self.query;
        
        // Add RETURNING clause for databases that support it
        if dialect.supports_returning() {
            query.returning_all();
        }
        
        let (sql, params) = query.render_for_dialect(dialect);
        
        if dialect.supports_returning() {
            // Use returning result
            let result = client.execute(&sql, &params).await
                .map_err(|e| crate::D1RsError::Database(format!("{:?}", e)))?;
            let entity = result.into_entity()?
                .ok_or(crate::D1RsError::NotFound)?;
            Ok(entity)
        } else {
            // For MySQL, execute insert then select by ID
            let result = client.execute(&sql, &params).await
                .map_err(|e| crate::D1RsError::Database(format!("{:?}", e)))?;
            
            let id = result.extract_id()?
                .ok_or(crate::D1RsError::Database("No ID returned from INSERT".to_string()))?;
            
            // Query back the inserted record
            let select_query = TypeSafeSelect::<T>::new().where_eq("id", id);
            let entity = select_query.first(client).await?
                .ok_or(crate::D1RsError::NotFound)?;
            Ok(entity)
        }
    }
    
    pub async fn save_returning_id<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<i64> {
        let dialect = client.dialect();
        let mut query = self.query;
        
        if dialect.supports_returning() {
            query.returning_col(sea_query::Alias::new("id"));
        }
        
        let (sql, params) = query.render_for_dialect(dialect);
        let result = client.execute(&sql, &params).await
            .map_err(|e| crate::D1RsError::Database(format!("{:?}", e)))?;
            
        result.extract_id()?.ok_or(crate::D1RsError::Database("No ID returned".to_string()))
    }
}

impl<T: Entity> QueryRenderer for TypeSafeInsert<T> {
    fn render_for_dialect(&self, dialect: crate::dialects::DatabaseDialect) -> (String, Vec<Value>) {
        self.query.render_for_dialect(dialect)
    }
}
```

**Acceptance criteria**:
- [ ] INSERT builder compiles and works with all databases
- [ ] RETURNING clause handling works correctly per database
- [ ] Value insertion and type conversion work properly
- [ ] Error handling for missing IDs works
- [ ] Zero compilation warnings

**Testing**: Test INSERT operations on all supported databases

---

### Task 3.5: Basic UPDATE Builder (~250 lines)
**Estimated effort**: 3-4 hours | **Files**: `src/query_builder/update.rs` (new)

**Create type-safe UPDATE builder**:
```rust
// src/query_builder/update.rs
use super::QueryRenderer;
use crate::backends::{DatabaseBackend, DatabaseClient};
use crate::{Entity, Result};
use sea_query::{Query, UpdateStatement, Expr};
use std::marker::PhantomData;
use serde_json::Value;

pub struct TypeSafeUpdate<T: Entity> {
    query: UpdateStatement,
    _phantom: PhantomData<T>,
}

impl<T: Entity> TypeSafeUpdate<T> {
    pub fn new() -> Self {
        let query = Query::update()
            .table(sea_query::Alias::new(T::TABLE_NAME))
            .to_owned();
        
        Self {
            query,
            _phantom: PhantomData,
        }
    }
    
    pub fn set<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = crate::query_builder::json_to_sea_value(&value.into());
        self.query.value(sea_query::Alias::new(column), sea_value);
        self
    }
    
    pub fn where_eq<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = crate::query_builder::json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).eq(sea_value));
        self
    }
    
    pub fn returning_all(mut self) -> Self {
        self.query.returning_all();
        self
    }
    
    pub async fn save<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<T> {
        let dialect = client.dialect();
        let mut query = self.query;
        
        if dialect.supports_returning() {
            query.returning_all();
        }
        
        let (sql, params) = query.render_for_dialect(dialect);
        
        if dialect.supports_returning() {
            let result = client.execute(&sql, &params).await
                .map_err(|e| crate::D1RsError::Database(format!("{:?}", e)))?;
            let entity = result.into_entity()?
                .ok_or(crate::D1RsError::NotFound)?;
            Ok(entity)
        } else {
            // For MySQL, we need to implement a different strategy
            // This is simplified - would need proper implementation
            todo!("Implement UPDATE without RETURNING for MySQL")
        }
    }
    
    pub async fn execute<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<u64> {
        let (sql, params) = self.query.render_for_dialect(client.dialect());
        let _result = client.execute(&sql, &params).await
            .map_err(|e| crate::D1RsError::Database(format!("{:?}", e)))?;
        
        // Return affected rows count
        // This is simplified - would need proper affected rows extraction
        Ok(1)
    }
}

impl<T: Entity> QueryRenderer for TypeSafeUpdate<T> {
    fn render_for_dialect(&self, dialect: crate::dialects::DatabaseDialect) -> (String, Vec<Value>) {
        self.query.render_for_dialect(dialect)
    }
}
```

**Acceptance criteria**:
- [ ] UPDATE builder compiles and works
- [ ] SET and WHERE clauses work correctly
- [ ] RETURNING handling per database works
- [ ] Affected rows counting works
- [ ] Zero compilation warnings

**Testing**: Test UPDATE operations on all databases

---

### Task 3.6: Basic DELETE Builder (~200 lines)
**Estimated effort**: 2-3 hours | **Files**: `src/query_builder/delete.rs` (new)

**Create type-safe DELETE builder**:
```rust
// src/query_builder/delete.rs
use super::QueryRenderer;
use crate::backends::{DatabaseBackend, DatabaseClient};
use crate::{Entity, Result};
use sea_query::{Query, DeleteStatement, Expr};
use std::marker::PhantomData;
use serde_json::Value;

pub struct TypeSafeDelete<T: Entity> {
    query: DeleteStatement,
    _phantom: PhantomData<T>,
}

impl<T: Entity> TypeSafeDelete<T> {
    pub fn new() -> Self {
        let query = Query::delete()
            .from_table(sea_query::Alias::new(T::TABLE_NAME))
            .to_owned();
        
        Self {
            query,
            _phantom: PhantomData,
        }
    }
    
    pub fn where_eq<V>(mut self, column: &str, value: V) -> Self 
    where
        V: Into<Value>,
    {
        let sea_value = crate::query_builder::json_to_sea_value(&value.into());
        self.query.and_where(Expr::col(sea_query::Alias::new(column)).eq(sea_value));
        self
    }
    
    pub async fn execute<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<u64> {
        let (sql, params) = self.query.render_for_dialect(client.dialect());
        let _result = client.execute(&sql, &params).await
            .map_err(|e| crate::D1RsError::Database(format!("{:?}", e)))?;
        
        // Return affected rows count
        Ok(1)
    }
}

impl<T: Entity> QueryRenderer for TypeSafeDelete<T> {
    fn render_for_dialect(&self, dialect: crate::dialects::DatabaseDialect) -> (String, Vec<Value>) {
        self.query.render_for_dialect(dialect)
    }
}
```

**Acceptance criteria**:
- [ ] DELETE builder compiles and works
- [ ] WHERE clauses work correctly  
- [ ] Affected rows counting works
- [ ] Zero compilation warnings

**Testing**: Test DELETE operations on all databases

### Implementation Details:

#### New Query Builders (`src/query_builder/select.rs`):
```rust
pub struct TypeSafeSelect<T: Entity> {
    query: sea_query::SelectStatement,
    _phantom: PhantomData<T>,
}

impl<T: Entity> TypeSafeSelect<T> {
    pub fn new() -> Self {
        let mut query = sea_query::Query::select();
        query.from(T::table_iden());
        query.columns(T::column_idens());
        
        Self {
            query,
            _phantom: PhantomData,
        }
    }
    
    pub fn where_eq<V>(mut self, column: T::Column, value: V) -> Self 
    where
        V: Into<sea_query::Value>,
    {
        self.query.and_where(Expr::col(column.iden()).eq(value.into()));
        self
    }
    
    pub async fn all<B: DatabaseBackend>(self, client: &DatabaseClient<B>) -> Result<Vec<T>, B::Error> {
        let result = client.execute_sea_query(self.query).await?;
        result.into_entities()
    }
}
```

#### Updated Derive Macro Output:
```rust
// Generated for User entity
impl User {
    pub fn query() -> TypeSafeSelect<User> {
        TypeSafeSelect::new()
    }
    
    // Generated where methods
    pub fn where_name_eq(self, value: impl Into<String>) -> Self {
        self.where_eq(UserColumn::Name, value.into())
    }
    
    pub fn where_email_like(self, pattern: impl Into<String>) -> Self {
        self.query.and_where(Expr::col(UserColumn::Email).like(pattern.into()));
        self
    }
}
```

**Acceptance Criteria**:
- All existing query functionality works with sea-query
- Generated SQL is optimal for each database
- Type safety maintained throughout
- Performance equals or exceeds current implementation
- Zero compilation warnings

---

## Phase 4: Schema Introspection (Week 5)

### 4.1 Database-Agnostic Introspection
- [ ] **Implement SQLite introspector** using existing pragma logic
- [ ] **Implement PostgreSQL introspector** using information_schema
- [ ] **Implement MySQL introspector** using information_schema
- [ ] **Create unified schema representation** across databases

### 4.2 Cross-Database Schema Mapping
- [ ] **Type mapping tables** for each database
- [ ] **Constraint discovery** (foreign keys, unique constraints)
- [ ] **Index introspection** for all databases
- [ ] **Default value handling** across different syntax

### 4.3 Schema Validation
- [ ] **Cross-database compatibility checks**
- [ ] **Feature support matrix** (JSON columns, etc.)
- [ ] **Type conversion validation**
- [ ] **Performance impact analysis**

### Implementation Details:

#### Introspection Trait (`src/introspection/mod.rs`):
```rust
#[async_trait]
pub trait SchemaIntrospector {
    type Error: std::error::Error + Send + Sync + 'static;
    
    async fn list_tables(&self) -> Result<Vec<String>, Self::Error>;
    async fn describe_table(&self, table_name: &str) -> Result<TableSchema, Self::Error>;
    async fn list_columns(&self, table_name: &str) -> Result<Vec<ColumnSchema>, Self::Error>;
    async fn list_indexes(&self, table_name: &str) -> Result<Vec<IndexSchema>, Self::Error>;
    async fn list_foreign_keys(&self, table_name: &str) -> Result<Vec<ForeignKeySchema>, Self::Error>;
}
```

#### PostgreSQL Introspector (`src/introspection/postgres.rs`):
```rust
pub struct PostgreSQLIntrospector<'a> {
    client: &'a DatabaseClient<PostgreSQLBackend>,
}

#[async_trait]
impl SchemaIntrospector for PostgreSQLIntrospector<'_> {
    type Error = PostgreSQLError;
    
    async fn list_tables(&self) -> Result<Vec<String>, Self::Error> {
        let query = sea_query::Query::select()
            .column(InformationSchema::TableName)
            .from(InformationSchema::Tables)
            .and_where(Expr::col(InformationSchema::TableSchema).eq("public"))
            .and_where(Expr::col(InformationSchema::TableType).eq("BASE TABLE"));
            
        let result = self.client.execute_sea_query(query).await?;
        result.extract_strings()
    }
    
    async fn list_columns(&self, table_name: &str) -> Result<Vec<ColumnSchema>, Self::Error> {
        let query = sea_query::Query::select()
            .columns([
                InformationSchema::ColumnName,
                InformationSchema::DataType,
                InformationSchema::IsNullable,
                InformationSchema::ColumnDefault,
            ])
            .from(InformationSchema::Columns)
            .and_where(Expr::col(InformationSchema::TableName).eq(table_name))
            .and_where(Expr::col(InformationSchema::TableSchema).eq("public"));
            
        let result = self.client.execute_sea_query(query).await?;
        result.into_column_schemas()
    }
}
```

#### MySQL Introspector (`src/introspection/mysql.rs`):
```rust
// Similar to PostgreSQL but with MySQL-specific information_schema queries
```

**Acceptance Criteria**:
- Schema introspection works for all three databases
- Unified schema representation is accurate
- Type mapping is correct across databases
- Foreign key and index discovery works
- Zero compilation warnings

---

## Phase 5: Migration System Overhaul (Week 6-7)

### 5.1 Database-Agnostic Migrations
- [ ] **Refactor migration system** to use sea-query schema builders
- [ ] **Implement cross-database DDL generation**
- [ ] **Add database-specific optimizations**
- [ ] **Support migration rollback** for all databases

### 5.2 Auto-Migration Engine
- [ ] **Refactor auto-migration** to work with all databases
- [ ] **Implement schema diffing** across database types
- [ ] **Add safety checks** for destructive operations
- [ ] **Generate migration scripts** for each database

### 5.3 Data Migration Support
- [ ] **Cross-database data transformation**
- [ ] **Type conversion during migration**
- [ ] **Bulk data operations** using database-specific optimizations
- [ ] **Migration validation** and rollback safety

### Implementation Details:

#### Migration Engine (`src/migration_engine/mod.rs`):
```rust
pub struct MigrationEngine<B: DatabaseBackend> {
    client: DatabaseClient<B>,
    introspector: Box<dyn SchemaIntrospector<Error = B::Error>>,
}

impl<B: DatabaseBackend> MigrationEngine<B> {
    pub async fn plan_migration(
        &self,
        current_schema: &DatabaseSchema,
        target_schema: &DatabaseSchema,
    ) -> Result<MigrationPlan, B::Error> {
        let differ = SchemaDiffer::new(self.client.dialect());
        differ.diff_schemas(current_schema, target_schema)
    }
    
    pub async fn execute_migration(
        &self,
        plan: &MigrationPlan,
    ) -> Result<MigrationResult, B::Error> {
        let executor = MigrationExecutor::new(&self.client);
        executor.execute_plan(plan).await
    }
}
```

#### Schema Diffing (`src/migration_engine/differ.rs`):
```rust
pub struct SchemaDiffer {
    dialect: DatabaseDialect,
}

impl SchemaDiffer {
    pub fn diff_schemas(
        &self,
        current: &DatabaseSchema,
        target: &DatabaseSchema,
    ) -> Result<MigrationPlan, SchemaError> {
        let mut operations = Vec::new();
        
        // Table operations
        operations.extend(self.diff_tables(current, target)?);
        
        // Column operations  
        operations.extend(self.diff_columns(current, target)?);
        
        // Index operations
        operations.extend(self.diff_indexes(current, target)?);
        
        // Foreign key operations
        operations.extend(self.diff_foreign_keys(current, target)?);
        
        Ok(MigrationPlan {
            operations,
            dialect: self.dialect,
            safety_level: self.assess_safety_level(&operations),
        })
    }
}
```

**Acceptance Criteria**:
- Migrations work correctly for all databases
- Auto-migration detects differences accurately
- DDL generation is optimal for each database
- Rollback functionality is safe and reliable
- Zero compilation warnings

---

## Phase 6: Testing Infrastructure (Week 8)

### 6.1 Multi-Database Testing
- [ ] **Set up test databases** (PostgreSQL, MySQL, SQLite)
- [ ] **Create database-agnostic test suite**
- [ ] **Implement parallel testing** across all databases
- [ ] **Add integration tests** for each backend

### 6.2 Test Data Management
- [ ] **Database-specific test fixtures**
- [ ] **Cross-database data consistency tests**
- [ ] **Performance benchmarks** for each database
- [ ] **Memory usage testing** for different backends

### 6.3 CI/CD Integration
- [ ] **Docker containers** for test databases
- [ ] **GitHub Actions** matrix for all databases
- [ ] **Performance regression tests**
- [ ] **Coverage reports** per database

### Implementation Details:

#### Test Configuration (`tests/common/mod.rs`):
```rust
pub fn setup_test_databases() -> HashMap<DatabaseDialect, Box<dyn DatabaseBackend>> {
    let mut backends = HashMap::new();
    
    // SQLite in-memory
    let sqlite = SQLiteBackend::new_in_memory().await.unwrap();
    backends.insert(DatabaseDialect::SQLite, Box::new(sqlite) as Box<dyn DatabaseBackend>);
    
    // PostgreSQL test database
    if let Ok(postgres_url) = env::var("POSTGRES_TEST_URL") {
        let postgres = PostgreSQLBackend::new(&postgres_url).await.unwrap();
        backends.insert(DatabaseDialect::PostgreSQL, Box::new(postgres) as Box<dyn DatabaseBackend>);
    }
    
    // MySQL test database
    if let Ok(mysql_url) = env::var("MYSQL_TEST_URL") {
        let mysql = MySQLBackend::new(&mysql_url).await.unwrap();
        backends.insert(DatabaseDialect::MySQL, Box::new(mysql) as Box<dyn DatabaseBackend>);
    }
    
    backends
}

pub async fn run_cross_database_test<F, Fut>(test_fn: F) 
where
    F: Fn(Box<dyn DatabaseBackend>) -> Fut + Clone,
    Fut: Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    let backends = setup_test_databases();
    
    for (dialect, backend) in backends {
        println!("Running test for {:?}", dialect);
        test_fn(backend).await.unwrap();
    }
}
```

#### Cross-Database Tests:
```rust
#[tokio::test]
async fn test_basic_crud_all_databases() {
    run_cross_database_test(|backend| async move {
        let client = DatabaseClient::new(backend);
        
        // Setup test schema
        User::create_table(&client).await?;
        
        // Test CRUD operations
        let user = User::create()
            .name("Test User")
            .email("test@example.com")
            .save(&client)
            .await?;
            
        let found_user = User::query()
            .where_id_eq(user.id)
            .first(&client)
            .await?;
            
        assert_eq!(found_user.unwrap().name, "Test User");
        
        Ok(())
    }).await;
}
```

**Acceptance Criteria**:
- All tests pass on SQLite, PostgreSQL, and MySQL
- Test suite runs efficiently in parallel
- CI/CD pipeline validates all databases
- Performance benchmarks meet requirements
- Zero compilation warnings

---

## Phase 7: Documentation & Optimization (Week 9)

### 7.1 Documentation Update
- [ ] **Update README** with multi-database examples
- [ ] **Create migration guide** from SQLite-only to multi-database
- [ ] **Database-specific guides** (PostgreSQL setup, MySQL configuration)
- [ ] **Performance tuning guides** for each database

### 7.2 Performance Optimization
- [ ] **Query optimization** for each database dialect
- [ ] **Connection pooling** tuning
- [ ] **Batch operation** optimization
- [ ] **Memory usage** optimization

### 7.3 Production Readiness
- [ ] **Error handling** review and improvement
- [ ] **Logging and observability** for each backend
- [ ] **Configuration validation**
- [ ] **Security review** (SQL injection prevention)

---

## 🔧 Implementation Guidelines

### Code Quality Standards

#### 1. Zero Warnings Policy
- **Every phase must complete with zero compilation warnings**
- Run `cargo check` before marking any task complete
- Use `just test` for all testing operations

#### 2. Type Safety Requirements
- **No raw SQL strings in public APIs**
- **All database operations must be type-safe**
- **Compile-time validation of queries where possible**

#### 3. Performance Standards
- **Generated SQL must be optimal for each database**
- **Query performance must equal or exceed current implementation**
- **Memory usage should be minimal**

#### 4. Testing Requirements
- **All features must work on all supported databases**
- **Cross-database compatibility tests required**
- **Performance regression tests mandatory**

### Development Process

#### Phase Completion Criteria
Each phase must meet ALL requirements before proceeding:
- [ ] ✅ **All tasks completed** as specified in checklists
- [ ] ✅ **Zero compilation warnings** (`cargo check`)
- [ ] ✅ **All tests passing** (`just test`)
- [ ] ✅ **Documentation updated** for new features
- [ ] ✅ **Performance requirements met**

#### Quality Gates
- **Before starting each phase**: Review and validate previous phase completion
- **During development**: Continuous integration testing
- **After completion**: Full regression testing across all databases

---

## 📈 Success Metrics

### Technical Metrics
- **Database Support**: SQLite, PostgreSQL, MySQL all working
- **Query Performance**: Equal or better than current implementation
- **Type Safety**: Zero raw SQL in public APIs
- **Test Coverage**: >95% across all database backends
- **Memory Usage**: Minimal overhead from abstraction layer

### Developer Experience Metrics
- **API Compatibility**: Existing code continues to work
- **Error Messages**: Clear, helpful error messages for all databases
- **Documentation**: Complete guides for all supported databases
- **Migration Path**: Smooth upgrade from SQLite-only to multi-database

### Production Readiness Metrics
- **Reliability**: Zero known bugs in any database backend
- **Performance**: Production-ready performance on all databases
- **Security**: No SQL injection vulnerabilities
- **Monitoring**: Comprehensive observability for all backends

---

## 🚨 Risk Mitigation

### Technical Risks

#### 1. Breaking Changes
- **Risk**: Existing applications break during migration
- **Mitigation**: Maintain backward compatibility, provide migration tools
- **Rollback Plan**: Keep SQLite-only version available

#### 2. Performance Regression
- **Risk**: Sea-query overhead impacts performance
- **Mitigation**: Comprehensive benchmarking, optimization for each database
- **Monitoring**: Continuous performance testing

#### 3. Database Feature Gaps
- **Risk**: Some database features not available in sea-query
- **Mitigation**: Identify gaps early, contribute to sea-query or create abstractions
- **Fallback**: Raw SQL escape hatch for advanced features

### Implementation Risks

#### 1. Complexity Explosion
- **Risk**: Codebase becomes too complex to maintain
- **Mitigation**: Clean architecture, clear separation of concerns
- **Review**: Regular code reviews, refactoring

#### 2. Testing Complexity
- **Risk**: Testing across multiple databases becomes unmanageable
- **Mitigation**: Automated test infrastructure, Docker containers
- **Strategy**: Parallel testing, clear test organization

---

## 📅 Timeline

| Phase | Duration | Focus Area | Key Deliverables |
|-------|----------|------------|------------------|
| **Phase 1** | Week 1 | Foundation | Core traits, dependencies |
| **Phase 2** | Week 2 | Database Clients | Multi-backend client |
| **Phase 3** | Week 3-4 | Query Builders | Sea-query integration |
| **Phase 4** | Week 5 | Schema Introspection | Cross-database schema |
| **Phase 5** | Week 6-7 | Migration System | Auto-migrations |
| **Phase 6** | Week 8 | Testing Infrastructure | Multi-database tests |
| **Phase 7** | Week 9 | Documentation | Production readiness |

**Total Timeline**: ~9 weeks for complete migration

---

## 🎯 Getting Started

To begin implementation:

1. **Review this plan** with the development team
2. **Set up development environment** with all three databases
3. **Create feature branch** for sea-query migration
4. **Begin Phase 1** implementation
5. **Follow phase completion criteria** strictly

This migration will transform d1-rs from a SQLite-centric ORM into a truly database-agnostic, production-ready solution that can compete with any ORM in any language.