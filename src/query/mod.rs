/// Revolutionary type-safe query building system
/// 
/// This module provides compile-time safe SQL query building with zero string literals
/// for parameter generation. All parameter binding is validated at compile time.
pub mod parameter_builder;
pub mod queries;

// Re-export parameter building types
pub use parameter_builder::{
    TypeSafeParameterBuilder,
    ParameterBinding,
    ParameterType,
    QueryParameterSet,
    SqlFragment,
    InsertParameterBuilder,
    UpdateParameterBuilder,
    WhereParameterBuilder,
};

// Re-export query types
pub use queries::{
    Query,
    InsertQuery,
    UpdateQuery,
    WhereClause,
    OrderBy,
};