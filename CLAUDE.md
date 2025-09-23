# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

d1-rs is a database-agnostic, type-safe ORM that supports SQLite, PostgreSQL, and MySQL. Originally designed for Cloudflare D1, it has evolved into a comprehensive ORM solution using sea-query for database-agnostic SQL generation. It uses conditional compilation and feature flags to support multiple backends while providing zero runtime overhead by only including necessary database drivers.

## 🚀 CRITICAL: Performance & Memory Efficiency

**ALWAYS prioritize performance and memory efficiency:**

- **COUNT queries MUST use sea-query COUNT aggregation** - NEVER load all records into memory to count with `.len()`
- **Use LIMIT 1 for first() queries** - don't load all records then take first  
- **Generate efficient SQL via sea-query** - avoid N+1 queries through proper eager loading and JOINs
- **Memory usage should be minimal** - don't keep unnecessary data in memory
- **Zero-copy optimizations where possible**
- **Database-optimized SQL generation** - Use sea-query to generate optimal SQL for each database dialect
- **Leverage database-specific features** - Use PostgreSQL arrays, MySQL JSON functions, etc. through sea-query
- **Connection pooling efficiency** - Optimize for each database backend (PostgreSQL/MySQL pools, SQLite connections)

## 🚨 CRITICAL: Zero Warnings Policy

**The codebase MUST have ZERO compilation warnings at all times:**

- **Before committing any code** - Run `cargo check` and ensure no warnings
- **All warnings must be fixed** - No warnings of any type are acceptable
- **Tests must pass cleanly** - Run `just test` and ensure clean output without warnings
- **Lint-clean code only** - Use `cargo clippy` if available and fix all issues
- **Quality gates** - All code must compile and test without warnings before merge

**Common warning types to avoid:**
- Unused imports, variables, or functions
- Unnecessary mutable variables
- Ambiguous glob re-exports
- Dead code or unreachable patterns
- Type inference failures

## 🚨 CRITICAL: Sea-Query Only Policy

**NO RAW SQL ALLOWED - ALL database operations MUST use sea-query:**

- **❌ FORBIDDEN**: Hand-written SQL strings, string concatenation, format!() for SQL
- **❌ FORBIDDEN**: Direct use of `sqlite_master`, `pragma_*`, `information_schema` queries
- **❌ FORBIDDEN**: Database-specific SQL syntax in business logic
- **✅ REQUIRED**: All queries MUST use sea-query builders (SelectStatement, InsertStatement, etc.)
- **✅ REQUIRED**: Database-agnostic code that works on SQLite, PostgreSQL, AND MySQL
- **✅ REQUIRED**: Dialect-aware SQL generation through DatabaseBackend abstraction

**Migration from Raw SQL:**
- **Replace existing raw SQL** with sea-query equivalents during any code changes
- **Use introspection traits** instead of direct schema queries
- **Generate SQL through DatabaseDialect** renderers, never manual string building
- **Schema operations** must use sea-query schema builders (CreateTableStatement, etc.)

**Enforcement:**
- **Code reviews MUST reject** any raw SQL additions
- **No exceptions** for "quick fixes" or "temporary solutions"
- **Migration plan** in `SEA_QUERY.md` MUST be followed for systematic replacement
- **All database operations** MUST be testable across SQLite, PostgreSQL, and MySQL

**Why this matters:**
- **Type safety**: Compile-time query validation vs runtime SQL errors
- **Database portability**: Single codebase works on multiple databases
- **Security**: Built-in SQL injection prevention
- **Performance**: Database-optimized SQL generation for each dialect
- **Maintainability**: Refactorable, IDE-friendly query building

## Development Commands

### Testing
**MANDATORY: Use nextest for all testing - much faster than cargo test**

```bash
# Run all tests (FAST parallel execution)
just test

# Run specific test (FASTEST for iteration)  
just test-one test_name

# Run tests matching pattern
just test-filter pattern

# Debug single test with output
just test-debug test_name

# NEVER use cargo test directly - always use nextest via just commands
```

### Building
```bash
# Build for native target (default with sqlite feature)
cargo build

# Build for WASM target
cargo build --target wasm32-unknown-unknown --features d1

# Check for compilation errors without building
cargo check
```

## Architecture

### Dual Backend System
- **WASM Target**: Uses `worker::d1::D1Database` for Cloudflare Workers
- **Native Target**: Uses `rusqlite` with in-memory databases for testing
- Conditional compilation (`cfg(target_arch = "wasm32")`) ensures only relevant code is included

### Core Components
- **Entity Trait** (`src/lib.rs:45`): Central trait implemented by derive macro
- **D1Client** (`src/db.rs:17`): Database client abstraction that wraps either D1Database or rusqlite::Connection
- **Query System** (`src/query.rs`): SQL query building with type-safe methods
- **Derive Macro** (`d1-rs-derive/src/lib.rs`): Code generation for Entity implementations

### Boolean Handling
SQLite stores booleans as integers (0/1). The ORM automatically converts:
- Rust `bool` → SQLite `INTEGER` when inserting/updating (`convert_to_sqlite`)
- SQLite `INTEGER` → Rust `bool` when querying (`convert_from_sqlite`)
- Boolean field metadata is generated by derive macro and stored in `boolean_fields()`

### Entity System
Entities are defined using the `#[derive(Entity)]` macro which generates:
- Query builders with type-safe where clauses (`UserQueryBuilder`)
- Create builders for inserting new records (`UserCreateBuilder`)
- Update builders for modifying existing records (`UserUpdateBuilder`)
- Automatic table name generation (PascalCase → snake_case + pluralization)

### Project Structure
```
src/
├── lib.rs          # Main exports, Entity trait, error types
├── db.rs           # D1Client abstraction (dual backend)
├── entity.rs       # Entity re-exports
├── query.rs        # SQL query building
├── types.rs        # Type conversion utilities
├── migrations.rs   # Migration system
└── schema.rs       # Schema utilities

d1-rs-derive/       # Proc macro crate
└── src/lib.rs      # Entity derive implementation

tests/              # Integration tests
├── common/mod.rs   # Test utilities
├── test_basic.rs   # Basic functionality tests
├── test_crud.rs    # CRUD operation tests
├── test_boolean_conversion.rs  # Boolean handling tests
└── ...
```

## Key Features to Understand

### Conditional Compilation
The codebase uses `#[cfg(target_arch = "wasm32")]` extensively to provide different implementations for WASM vs native targets.

### Type-Safe Query Building
The derive macro generates methods like:
- `where_field_name_eq(value)` for equality comparisons
- `where_field_name_like(pattern)` for string pattern matching
- `order_by_field_name_asc()` for ordering

## CRITICAL DESIGN PRINCIPLES

### 🚨 ZERO STRING LITERALS POLICY 🚨
**NEVER use string literals for field names, relation names, or operators in the public API.**

❌ **FORBIDDEN** (Error-prone, runtime failures):
```rust
Predicate::field("is_published", "=", true)  // Runtime errors!
user.posts().where_eq("title", "foo")        // Typos cause crashes!
Predicate::has("posts")                       // No compile-time validation!
```

✅ **REQUIRED** (Compile-time safe, impossible to have errors):
```rust
user.posts().query().where_is_published_eq(true)  // Type-safe!
user.posts().query().where_title_eq("foo")        // Auto-completed!
User::query().has_posts()                         // Compile-time validated!
```

### Compile-Time Safety Requirements:
1. **All field access must be auto-generated** - Use derive macros to generate `where_field_name_eq()` methods
2. **All relation access must be type-safe** - Relations generate methods like `has_posts()`, `has_posts_with()`
3. **Zero runtime field/relation name validation** - Everything validated at compile time
4. **Impossible to typo field/relation names** - IDE auto-completion prevents errors
5. **Superior to any string-based ORM** - Even better than hand-written SQL in terms of safety

### Implementation Strategy:
- **Leverage Rust's type system** - Use generics and traits for compile-time safety
- **Generate specific methods** - `where_name_eq()` instead of `where_eq("name")`  
- **Use phantom types** - Encode entity relationships in the type system
- **Macro-generated code** - All repetitive code should be auto-generated
- **Zero-cost abstractions** - No runtime overhead compared to hand-written SQL
- **❌ NO HEURISTICS EVER** - Never use unreliable pattern matching or guessing for type detection
- **❌ NO HARDCODED TYPE LISTS** - Never hardcode specific type names like "UserProfile", "Address" etc
- **❌ NO CULTURAL/LANGUAGE BIAS** - Must work with Russian, Chinese, or any naming convention
- **Trait-based or attribute-based detection only** - Use Rust's type system properly
- **User-configurable type mapping** - Allow explicit override of default behavior

## 🚨 DEVELOPMENT PHILOSOPHY: LEAN & CLEAN CODE

### **NO BACKWARD COMPATIBILITY CONCERNS**
**This library is in heavy development with no external users. Prioritize clean, lean code over compatibility:**

- **❌ NO COMPATIBILITY LAYERS** - Don't create deprecated API wrappers or migration paths
- **❌ NO LEGACY CODE MAINTENANCE** - If an API is deprecated, DELETE it completely
- **✅ CLEAN BREAKS** - Make breaking changes freely to achieve better design
- **✅ LEAN CODEBASE** - Remove any code that's no longer needed immediately
- **✅ FOCUS ON QUALITY** - Prioritize revolutionary design over maintaining old patterns

**Principle: If something becomes obsolete, remove it entirely. Don't accumulate technical debt.**

### 🚨 ZERO PLACEHOLDER POLICY 🚨
**NEVER write placeholder implementations, TODOs, or "for now" shortcuts. ALWAYS implement the complete solution.**

❌ **FORBIDDEN** (Accumulates technical debt):
```rust
// TODO: Implement proper validation
fn validate() -> Result<()> { Ok(()) }

// For now, just return empty
fn get_users() -> Vec<User> { vec![] }

// Placeholder implementation  
fn execute_migration() -> Result<()> {
    // Will implement later
    Ok(())
}
```

✅ **REQUIRED** (Complete implementations only):
```rust
fn validate(&self) -> Result<()> {
    // Full validation logic with proper error handling
    if self.name.is_empty() {
        return Err(ValidationError::EmptyName);
    }
    if !self.email.contains('@') {
        return Err(ValidationError::InvalidEmail);
    }
    Ok(())
}

fn get_users(&self) -> Result<Vec<User>> {
    // Complete implementation with proper error handling
    let sql = "SELECT id, name, email FROM users WHERE active = ?";
    self.db.query(sql, &[&true])
        .map_err(|e| UserError::DatabaseError(e))
        .and_then(|rows| rows.into_iter().map(User::from_row).collect())
}
```

### Implementation Requirements:
1. **COMPLETE IMPLEMENTATIONS ONLY** - Every function must be fully implemented before committing
2. **NO TODO COMMENTS** - If you identify work needed, implement it immediately or don't add the function
3. **NO "FOR NOW" LOGIC** - Don't add temporary shortcuts that will need replacement later
4. **NO PLACEHOLDER RETURNS** - Don't return empty collections, None, or Ok(()) without proper logic
5. **THINK THROUGH THE COMPLETE SOLUTION** - Consider all edge cases, error handling, and performance requirements upfront

### When You Need to Implement Something:
- **Research the requirements thoroughly** - Understand what the complete implementation needs
- **Design the full solution** - Don't start coding until you understand the complete requirements  
- **Implement all edge cases** - Handle errors, validation, and boundary conditions
- **Add comprehensive tests** - Ensure the implementation works correctly in all scenarios
- **Optimize for performance** - Consider memory usage, query efficiency, and compilation time

**Principle: If you can't implement it completely right now, don't add it to the codebase. Wait until you can deliver a complete, production-ready implementation.**

## 🚨 MANDATORY: File Modularity Policy

**CRITICAL:** Large implementations must be split into focused, maintainable modules.

### File Size and Complexity Limits:
- **Maximum file size**: 800 lines of actual code (excluding tests)
- **Maximum function size**: 100 lines of actual code
- **Complexity threshold**: If a module handles >3 distinct concerns, split it

### When to Split Files:
❌ **FORBIDDEN** (Monolithic, unmaintainable):
```rust
// data_migration.rs - 2000+ lines mixing concerns
impl DataMigrator {
    async fn execute_type_conversion() { /* 200 lines */ }
    async fn execute_normalization() { /* 300 lines */ } 
    async fn execute_aggregation() { /* 250 lines */ }
    async fn execute_value_mapping() { /* 200 lines */ }
    async fn execute_format_transformation() { /* 200 lines */ }
    // + 1000 more lines of helper functions
}
```

✅ **REQUIRED** (Modular, focused, maintainable):
```rust
// src/auto_migration/
//   mod.rs                    # Public exports and coordination
//   data_migration.rs         # Core orchestration (< 400 lines)
//   transformations/
//     mod.rs                  # Transformation exports
//     type_conversion.rs      # Type conversion logic only
//     normalization.rs        # Data normalization logic only  
//     aggregation.rs          # Data aggregation logic only
//     value_mapping.rs        # Value mapping logic only
//     format_transformation.rs # Format transformation logic only
//   validators/
//     mod.rs                  # Validation exports
//     integrity_checker.rs    # Data integrity validation
//     schema_validator.rs     # Schema validation logic
//   utils/
//     mod.rs                  # Utility exports
//     batch_processor.rs      # Batch processing utilities
//     backup_manager.rs       # Backup and rollback utilities
```

### Modular Design Requirements:
1. **Single Responsibility** - Each file handles exactly one concern
2. **Clear Interfaces** - Well-defined public APIs between modules
3. **Minimal Dependencies** - Modules only depend on what they actually need
4. **Testable Units** - Each module can be tested independently
5. **Logical Organization** - Related functionality grouped together

### Implementation Strategy:
1. **Plan the module structure FIRST** - Design the file organization before implementing
2. **Create focused trait abstractions** - Define clear interfaces between modules
3. **Implement one module at a time** - Complete each module fully before moving to next
4. **Keep coordination logic minimal** - Main orchestration file should mostly delegate
5. **Comprehensive module tests** - Each module needs its own test suite

**Principle: If implementing a feature would make any single file >800 lines or add >3 concerns to a module, create a proper modular structure instead.**

### Automatic CRUD Operations
Entities get `find()`, `delete()`, `create()`, and `update()` methods automatically generated with proper SQL handling.

### Testing Strategy
Tests run on the native target using in-memory SQLite databases, allowing full integration testing without requiring actual D1 databases.

### Testing Requirements
**MANDATORY: Use nextest for all testing operations**

Every feature MUST have:
- **Unit tests**: `just test-filter unit`
- **Integration tests**: `just test-filter integration` 
- **Single test debug**: `just test-debug test_name`
- **Performance tests**: `just test-filter performance`

**NEVER use `cargo test` - always use `just test` commands for speed**

## 🚨 MANDATORY: Comprehensive Cleanup Plan

**CRITICAL:** This codebase currently has major violations of the design principles above. Follow the comprehensive cleanup plan in `CLEANUP_PLAN.md` EXACTLY as specified.

### Implementation Execution Rules:

1. **FOLLOW THE PLAN**: Implement phases 1-7 in `CLEANUP_PLAN.md` in exact order
2. **TRACK PROGRESS**: Update checklists in `CLEANUP_PLAN.md` as items are completed
3. **COMPLETION CRITERIA**: An item is only complete when:
   - ✅ Fully tested for ALL edge cases
   - ✅ Fully satisfies the task requirements
   - ✅ Generates ZERO compilation warnings
   - ✅ All existing tests still pass
   - ✅ Performance requirements are met

### Quality Gates Before Any Work:
- [ ] Run `cargo check` - must show ZERO warnings
- [ ] Run `just test` - all tests must pass cleanly
- [ ] Review current violations in `CLEANUP_PLAN.md`

### Quality Gates After Each Item:
- [ ] Run `cargo check` - must still show ZERO warnings  
- [ ] Run `just test` - all tests must still pass
- [ ] Update the specific checklist item in `CLEANUP_PLAN.md` as complete
- [ ] Verify the implementation fully addresses the violation

### Phase Completion Requirements:
Each phase in `CLEANUP_PLAN.md` must meet ALL acceptance criteria before proceeding to the next phase.

**NEVER skip items or phases. The codebase must be systematically fixed to achieve true type safety and eliminate all violations.**

