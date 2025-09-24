# Raw SQL Audit Report

Generated on: 2024-01-XX  
Purpose: Complete inventory of raw SQL usage for sea-query migration  
Status: **COMPREHENSIVE AUDIT**

## Executive Summary

- **Total SQL usages found**: 287 occurrences
- **Query building**: 89 occurrences (HIGH priority)
- **Schema introspection**: 23 occurrences (HIGH priority)
- **DDL operations**: 145 occurrences (MEDIUM priority)
- **Migration operations**: 30 occurrences (HIGH priority)

## Priority Classification

- **CRITICAL**: Core query building in public APIs (must migrate first)
- **HIGH**: Schema introspection and migration operations (required for multi-DB support)
- **MEDIUM**: DDL operations and complex schema changes (can be migrated incrementally)
- **LOW**: Test fixtures and debugging queries (migrate last)

---

## Detailed Findings by File

### 🚨 CRITICAL Priority Files

#### src/query/queries.rs (CRITICAL - Public API)
**19 SQL usages** - Core query building that powers the public ORM API

```
Lines 72, 108, 158, 199: Core SELECT/INSERT/UPDATE query building
```

- **Current**: Direct `format!("SELECT * FROM {}", table)` string building
- **Sea-query equivalent**: 
  ```rust
  sea_query::Query::select()
      .columns([sea_query::Asterisk])
      .from(sea_query::Alias::new(table))
  ```
- **Estimated effort**: 8-12 hours
- **Migration strategy**: Replace Query struct methods with sea-query builders

#### src/migrations.rs (CRITICAL - Migration System)
**12 SQL usages** - Migration system infrastructure

```
Lines 143-156: Migration table creation
Lines 165, 173-174, 183, 198, 204, 217, 236, 243, 254: Migration metadata queries
```

- **Current**: Hardcoded migration table DDL and queries
- **Sea-query equivalent**: Use `sea_query::Table::create()` and query builders
- **Estimated effort**: 6-8 hours  
- **Migration strategy**: Abstract migration system to use sea-query builders

### 🔥 HIGH Priority Files

#### src/auto_migration/introspector.rs (HIGH - Schema Discovery)
**11 SQL usages** - Critical for multi-database schema introspection

```
Lines 34, 69: sqlite_master queries
Lines 304, 321, 344, 367, 410: pragma_* function queries  
Lines 392: Index introspection
```

- **Current**: SQLite-specific `pragma_table_info()` and `sqlite_master` queries
- **Sea-query equivalent**: Custom `SchemaIntrospector` trait with database-specific implementations
- **Estimated effort**: 15-20 hours
- **Migration strategy**: Replace with trait-based abstraction supporting all databases

#### src/edges.rs (HIGH - Relation Queries)
**34 SQL usages** - Relation system that powers Entity relationships

```
Lines 236, 259, 272, 289, 337, 372, 396, 415, 457, 478, 503, 515, 535, 585, 621, 641: Relation queries
Lines 811, 821: EXISTS subqueries for relation filtering
Lines 1130: Dynamic SELECT field building
```

- **Current**: Dynamic SQL building for relations and joins
- **Sea-query equivalent**: 
  ```rust
  sea_query::Query::select()
      .from(related_table)
      .and_where(sea_query::Expr::col(foreign_key).eq(entity_id))
  ```
- **Estimated effort**: 12-15 hours
- **Migration strategy**: Replace relation queries with sea-query join builders

#### src/schema.rs (HIGH - Schema Operations)  
**27 SQL usages** - Table and index creation system

```
Lines 174, 178: Foreign key constraint SQL
Lines 310: CREATE TABLE building
Lines 721, 728, 755, 760: Index creation
Lines 738, 742, 767, 777, 799: DDL operations (DROP, ALTER)
```

- **Current**: Manual DDL string construction
- **Sea-query equivalent**: 
  ```rust
  sea_query::Table::create()
      .table(table_name)
      .col(sea_query::ColumnDef::new(column_name).integer())
  ```
- **Estimated effort**: 10-12 hours
- **Migration strategy**: Replace DDL builders with sea-query schema builders

### ⚠️ MEDIUM Priority Files

#### src/auto_migration/complex_schema_changes.rs (MEDIUM - Complex DDL)
**45 SQL usages** - Complex schema migration operations

```
Lines 421, 454, 464, 557, 561, 607, 621, 680: Table manipulation queries
Lines 1124, 1133-1134, 1270, 1279-1280, 1287: Advanced table restructuring
```

- **Current**: Complex DDL for table recreation and data migration
- **Sea-query equivalent**: Combination of schema and query builders
- **Estimated effort**: 20-25 hours
- **Migration strategy**: Migrate after core query builders are complete

#### src/type_safe_migrations.rs (MEDIUM - Type-Safe DDL)
**2 SQL usages** - Type-safe migration helpers

```
Lines 169, 284: CREATE/DROP TABLE generation
```

- **Current**: Type-safe wrappers around manual DDL
- **Sea-query equivalent**: Enhanced type-safe wrappers around sea-query builders
- **Estimated effort**: 4-6 hours
- **Migration strategy**: Extend existing API to use sea-query internally

### 📝 MEDIUM-LOW Priority Files

#### src/auto_migration/executor.rs (MEDIUM-LOW - Migration Execution)
**11 SQL usages** - Migration plan execution

```
Lines 30, 42, 68, 107, 111, 123, 139: DDL execution and logging
```

- **Current**: Direct DDL execution with manual string building
- **Sea-query equivalent**: Execute sea-query generated DDL
- **Estimated effort**: 6-8 hours
- **Migration strategy**: Update after core schema builders are migrated

#### Various rollback and validation modules (LOW - Advanced Features)
**120+ SQL usages** - Advanced migration features

```
Multiple files in auto_migration/rollback/*, auto_migration/validators/*
```

- **Current**: Complex SQL for rollback generation and validation
- **Sea-query equivalent**: Advanced sea-query patterns and custom SQL
- **Estimated effort**: 25-30 hours
- **Migration strategy**: Migrate last, after core functionality is stable

---

## Migration Roadmap

### Phase 1: Core Query Builders (Week 1-2)
1. **src/query/queries.rs** - Replace Query struct with sea-query builders
2. **src/migrations.rs** - Abstract migration system
3. **Basic test coverage** - Ensure existing functionality works

### Phase 2: Schema Operations (Week 3-4)  
1. **src/schema.rs** - Replace DDL builders
2. **src/edges.rs** - Replace relation queries
3. **Multi-database testing** - Verify PostgreSQL/MySQL compatibility

### Phase 3: Schema Introspection (Week 5-6)
1. **src/auto_migration/introspector.rs** - Database-agnostic introspection
2. **Database-specific implementations** - PostgreSQL/MySQL introspectors  
3. **Cross-database schema compatibility**

### Phase 4: Advanced Features (Week 7-8)
1. **Complex schema changes** - Table recreation with sea-query
2. **Type-safe migrations** - Enhanced API wrappers
3. **Migration execution** - Sea-query integration

### Phase 5: Migration & Validation Systems (Week 9-10)
1. **Rollback generation** - Sea-query based rollback SQL
2. **Validation systems** - Database-agnostic validation
3. **Performance optimization** - Query plan analysis

---

## Technical Debt Analysis

### High-Risk SQL Patterns

1. **SQLite-specific pragma queries** (23 occurrences)
   - Risk: Hard dependency on SQLite prevents PostgreSQL/MySQL support
   - Fix: Abstract behind SchemaIntrospector trait

2. **Manual string concatenation** (89 occurrences)  
   - Risk: SQL injection potential, type safety issues
   - Fix: Replace with sea-query builders

3. **Database-specific SQL syntax** (45 occurrences)
   - Risk: Breaks cross-database compatibility  
   - Fix: Use sea-query dialect-specific generation

### Code Quality Issues

1. **No SQL injection protection** in 34 locations
2. **Hardcoded table/column names** in 67 locations  
3. **No query plan optimization** in complex queries
4. **Inconsistent error handling** across SQL operations

---

## Migration Guidelines

### Code Transformation Patterns

#### Before (Raw SQL):
```rust
let sql = format!("SELECT * FROM {} WHERE {} = ?", table, column);
let result = db.execute(&sql, &[value]).await?;
```

#### After (Sea-Query):
```rust
let query = sea_query::Query::select()
    .columns([sea_query::Asterisk])
    .from(sea_query::Alias::new(table))
    .and_where(sea_query::Expr::col(column).eq(value))
    .to_string(dialect);
let result = db.execute(&query, &[]).await?;
```

### Testing Requirements

1. **Cross-database compatibility tests** for each migrated component
2. **SQL injection prevention tests** for all query builders  
3. **Performance benchmarks** to ensure no regressions
4. **Migration safety tests** for schema operations

### Success Metrics

- [ ] **Zero raw SQL strings** in core query building (src/query/, src/edges/)
- [ ] **All databases supported** (SQLite, PostgreSQL, MySQL)  
- [ ] **100% test coverage** maintained throughout migration
- [ ] **No performance regressions** in benchmark tests
- [ ] **Full backward compatibility** for public APIs

---

## Risk Assessment

### Migration Risks

1. **API Breaking Changes** (MEDIUM)
   - Mitigation: Maintain facade pattern during transition
   
2. **Performance Degradation** (LOW-MEDIUM)  
   - Mitigation: Benchmark each component migration
   
3. **Database Compatibility Issues** (HIGH)
   - Mitigation: Comprehensive cross-database test suite

4. **Development Velocity Impact** (MEDIUM)
   - Mitigation: Incremental migration with working software at each phase

### Rollback Strategy

1. **Feature flags** for sea-query vs raw SQL
2. **Comprehensive test suite** for both implementations  
3. **Performance monitoring** to detect regressions
4. **Gradual rollout** with fallback mechanisms

---

## Conclusion

The audit reveals **287 raw SQL usages** across the codebase, with the highest concentration in core query building (src/query/queries.rs) and relation systems (src/edges.rs). The migration to sea-query is **essential** for achieving database-agnostic functionality and eliminating SQLite-specific dependencies.

**Recommended approach**: Incremental migration starting with CRITICAL priority files, maintaining full test coverage and backward compatibility throughout the process.

**Timeline estimate**: 8-10 weeks for complete migration including testing and validation.

**Success criteria**: Zero raw SQL in core components, full PostgreSQL/MySQL support, maintained API compatibility, and no performance regressions.