# Sea-Query Migration Plan: Database Agnostic ORM

This document outlines the comprehensive plan to migrate d1-rs from SQLite-centric raw SQL to a database-agnostic architecture using SeaQL's sea-query.

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

### 1.1 Dependency Integration
- [ ] Add sea-query dependencies to Cargo.toml
- [ ] Add database-specific drivers (sqlx-postgres, sqlx-mysql)
- [ ] Set up feature flags for each database backend
- [ ] Configure conditional compilation for backends

### 1.2 Core Abstractions
- [ ] **Create `DatabaseBackend` trait** (`src/backends/mod.rs`)
- [ ] **Create `DatabaseDialect` trait** (`src/dialects/mod.rs`)
- [ ] **Create `SchemaIntrospector` trait** (`src/introspection/mod.rs`)
- [ ] **Create `QueryRenderer` trait** for dialect-specific SQL generation

### 1.3 Migration Preparation
- [ ] **Audit all raw SQL usage** across the codebase
- [ ] **Document SQL patterns** that need sea-query equivalents
- [ ] **Create compatibility matrix** for database features

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

### 2.1 Multi-Backend Client
- [ ] **Refactor `D1Client`** to `DatabaseClient<B: DatabaseBackend>`
- [ ] **Implement SQLite backend** (maintain current functionality)
- [ ] **Implement PostgreSQL backend** using sqlx
- [ ] **Implement MySQL backend** using sqlx
- [ ] **Keep D1 backend** for WASM target (Cloudflare Workers)

### 2.2 Configuration System
- [ ] **Create database configuration** structs for each backend
- [ ] **Implement connection pooling** for PostgreSQL/MySQL
- [ ] **Add database URL parsing** (postgres://, mysql://, sqlite://)
- [ ] **Environment-based configuration** for testing

### 2.3 Type System Overhaul
- [ ] **Database-specific type mapping** in each dialect
- [ ] **Cross-database type conversion** (e.g., JSON support)
- [ ] **Boolean handling** for all databases
- [ ] **DateTime handling** with timezone awareness

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

### 3.1 Sea-Query Integration
- [ ] **Replace Query struct** with sea-query builders
- [ ] **Implement type-safe SELECT builders** using sea-query
- [ ] **Implement INSERT/UPDATE/DELETE builders** using sea-query
- [ ] **Add complex query support** (JOINs, subqueries, CTEs)

### 3.2 Entity Query Builder Refactor
- [ ] **Update derive macro** to generate sea-query-based methods
- [ ] **Implement `where_field_eq`** methods using sea-query expressions
- [ ] **Add relation query builders** with proper JOIN generation
- [ ] **Implement COUNT queries** using sea-query aggregation

### 3.3 Advanced Query Features
- [ ] **Implement pagination** with LIMIT/OFFSET for all databases
- [ ] **Add UPSERT support** (INSERT...ON CONFLICT for PostgreSQL, etc.)
- [ ] **Implement bulk operations** (batch INSERT/UPDATE)
- [ ] **Add query optimization** hints for each database

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