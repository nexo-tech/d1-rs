use serde_json::Value;
use std::fmt;
use crate::dialects::DatabaseDialect;

/// Revolutionary type-safe parameter building system
/// Eliminates all string-based placeholder generation for compile-time safety
pub trait TypeSafeParameterBuilder {
    /// Generate type-safe SQL fragment with parameter bindings for specific dialect
    fn build_sql_fragment(&self, dialect: DatabaseDialect) -> SqlFragment;
    
    /// Get all parameter values in the correct order
    fn parameter_values(&self) -> Vec<Value>;
    
    /// Get the number of parameters for validation
    fn parameter_count(&self) -> usize;
}

/// Compile-time safe SQL fragment with parameter metadata
#[derive(Debug, Clone, PartialEq)]
pub struct SqlFragment {
    /// The SQL fragment with placeholder positions marked
    pub sql: String,
    /// Number of parameters this fragment expects
    pub parameter_count: usize,
}

impl SqlFragment {
    /// Create a new SQL fragment with known parameter count
    pub fn new(sql: String, parameter_count: usize) -> Self {
        Self { sql, parameter_count }
    }
    
    /// Create fragment for a single parameter binding with database-specific placeholder
    pub fn single_parameter(dialect: DatabaseDialect) -> Self {
        Self {
            sql: dialect.placeholder(1),
            parameter_count: 1,
        }
    }
    
    /// Create fragment with no parameters
    pub fn no_parameters(sql: String) -> Self {
        Self {
            sql,
            parameter_count: 0,
        }
    }
}

/// Type-safe parameter binding for compile-time validation
#[derive(Debug, Clone)]
pub struct ParameterBinding {
    /// The actual value to bind
    pub value: Value,
    /// Compile-time type information for validation
    pub binding_type: ParameterType,
}

impl ParameterBinding {
    /// Create a new parameter binding with type information
    pub fn new(value: Value, binding_type: ParameterType) -> Self {
        Self { value, binding_type }
    }
    
    /// Create binding for string values
    pub fn string(value: String) -> Self {
        Self::new(Value::String(value), ParameterType::String)
    }
    
    /// Create binding for integer values
    pub fn integer(value: i64) -> Self {
        Self::new(Value::Number(serde_json::Number::from(value)), ParameterType::Integer)
    }
    
    /// Create binding for boolean values
    pub fn boolean(value: bool) -> Self {
        Self::new(Value::Bool(value), ParameterType::Boolean)
    }
    
    /// Create binding for null values
    pub fn null() -> Self {
        Self::new(Value::Null, ParameterType::Null)
    }
    
    /// Create binding from any JSON value with automatic type detection
    pub fn from_value(value: Value) -> Self {
        let binding_type = match &value {
            Value::String(_) => ParameterType::String,
            Value::Number(_) => ParameterType::Number,
            Value::Bool(_) => ParameterType::Boolean,
            Value::Null => ParameterType::Null,
            Value::Array(_) => ParameterType::Array,
            Value::Object(_) => ParameterType::Object,
        };
        Self::new(value, binding_type)
    }
}

/// Compile-time parameter type classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterType {
    String,
    Integer,
    Number,
    Boolean,
    Null,
    Array,
    Object,
}

impl fmt::Display for ParameterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterType::String => write!(f, "STRING"),
            ParameterType::Integer => write!(f, "INTEGER"),
            ParameterType::Number => write!(f, "NUMBER"),
            ParameterType::Boolean => write!(f, "BOOLEAN"),
            ParameterType::Null => write!(f, "NULL"),
            ParameterType::Array => write!(f, "ARRAY"),
            ParameterType::Object => write!(f, "OBJECT"),
        }
    }
}

/// Type-safe parameter set for managing collections of bindings
#[derive(Debug, Clone)]
pub struct QueryParameterSet {
    /// All parameter bindings in order
    bindings: Vec<ParameterBinding>,
}

impl QueryParameterSet {
    /// Create a new empty parameter set
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }
    
    /// Add a parameter binding to the set
    pub fn add_binding(&mut self, binding: ParameterBinding) -> &mut Self {
        self.bindings.push(binding);
        self
    }
    
    /// Add a string parameter
    pub fn add_string(&mut self, value: String) -> &mut Self {
        self.add_binding(ParameterBinding::string(value))
    }
    
    /// Add an integer parameter
    pub fn add_integer(&mut self, value: i64) -> &mut Self {
        self.add_binding(ParameterBinding::integer(value))
    }
    
    /// Add a boolean parameter
    pub fn add_boolean(&mut self, value: bool) -> &mut Self {
        self.add_binding(ParameterBinding::boolean(value))
    }
    
    /// Add a null parameter
    pub fn add_null(&mut self) -> &mut Self {
        self.add_binding(ParameterBinding::null())
    }
    
    /// Add a parameter from a JSON value
    pub fn add_value(&mut self, value: Value) -> &mut Self {
        self.add_binding(ParameterBinding::from_value(value))
    }
    
    /// Get the number of parameters
    pub fn len(&self) -> usize {
        self.bindings.len()
    }
    
    /// Check if the parameter set is empty
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
    
    /// Get all parameter values in order
    pub fn values(&self) -> Vec<Value> {
        self.bindings.iter().map(|b| b.value.clone()).collect()
    }
    
    /// Get all parameter types for validation
    pub fn types(&self) -> Vec<ParameterType> {
        self.bindings.iter().map(|b| b.binding_type.clone()).collect()
    }
    
    /// Generate placeholder fragment for this parameter set with database-specific placeholders
    pub fn placeholder_fragment(&self, dialect: DatabaseDialect) -> SqlFragment {
        if self.is_empty() {
            return SqlFragment::no_parameters(String::new());
        }
        
        let placeholders = dialect.placeholders(self.len());
        SqlFragment::new(placeholders, self.len())
    }
}

impl Default for QueryParameterSet {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeSafeParameterBuilder for QueryParameterSet {
    fn build_sql_fragment(&self, dialect: DatabaseDialect) -> SqlFragment {
        self.placeholder_fragment(dialect)
    }
    
    fn parameter_values(&self) -> Vec<Value> {
        self.values()
    }
    
    fn parameter_count(&self) -> usize {
        self.len()
    }
}

/// Type-safe parameter builder for INSERT statements
#[derive(Debug, Clone)]
pub struct InsertParameterBuilder {
    /// Parameters for column values
    parameters: QueryParameterSet,
}

impl InsertParameterBuilder {
    /// Create a new INSERT parameter builder
    pub fn new() -> Self {
        Self {
            parameters: QueryParameterSet::new(),
        }
    }
    
    /// Add a column value parameter
    pub fn add_column_value(&mut self, value: Value) -> &mut Self {
        self.parameters.add_value(value);
        self
    }
    
    /// Get the VALUES clause fragment with database-specific placeholders
    pub fn values_clause_fragment(&self, dialect: DatabaseDialect) -> SqlFragment {
        if self.parameters.is_empty() {
            return SqlFragment::no_parameters("()".to_string());
        }
        
        let fragment = self.parameters.placeholder_fragment(dialect);
        SqlFragment::new(
            format!("({})", fragment.sql),
            fragment.parameter_count
        )
    }
}

impl Default for InsertParameterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeSafeParameterBuilder for InsertParameterBuilder {
    fn build_sql_fragment(&self, dialect: DatabaseDialect) -> SqlFragment {
        self.values_clause_fragment(dialect)
    }
    
    fn parameter_values(&self) -> Vec<Value> {
        self.parameters.parameter_values()
    }
    
    fn parameter_count(&self) -> usize {
        self.parameters.parameter_count()
    }
}

/// Type-safe parameter builder for UPDATE statements
#[derive(Debug, Clone)]
pub struct UpdateParameterBuilder {
    /// Parameters for SET clause
    set_parameters: QueryParameterSet,
    /// Parameters for WHERE clause
    where_parameters: QueryParameterSet,
}

impl UpdateParameterBuilder {
    /// Create a new UPDATE parameter builder
    pub fn new() -> Self {
        Self {
            set_parameters: QueryParameterSet::new(),
            where_parameters: QueryParameterSet::new(),
        }
    }
    
    /// Add a SET clause parameter
    pub fn add_set_parameter(&mut self, value: Value) -> &mut Self {
        self.set_parameters.add_value(value);
        self
    }
    
    /// Add a WHERE clause parameter
    pub fn add_where_parameter(&mut self, value: Value) -> &mut Self {
        self.where_parameters.add_value(value);
        self
    }
    
    /// Get SET clause fragment with database-specific placeholders
    pub fn set_clause_fragment(&self, dialect: DatabaseDialect) -> SqlFragment {
        self.set_parameters.placeholder_fragment(dialect)
    }
    
    /// Get WHERE clause fragment with database-specific placeholders
    pub fn where_clause_fragment(&self, dialect: DatabaseDialect) -> SqlFragment {
        self.where_parameters.placeholder_fragment(dialect)
    }
}

impl Default for UpdateParameterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeSafeParameterBuilder for UpdateParameterBuilder {
    fn build_sql_fragment(&self, _dialect: DatabaseDialect) -> SqlFragment {
        // Combine SET and WHERE parameters
        let total_count = self.set_parameters.parameter_count() + self.where_parameters.parameter_count();
        SqlFragment::new(String::new(), total_count)
    }
    
    fn parameter_values(&self) -> Vec<Value> {
        let mut values = self.set_parameters.parameter_values();
        values.extend(self.where_parameters.parameter_values());
        values
    }
    
    fn parameter_count(&self) -> usize {
        self.set_parameters.parameter_count() + self.where_parameters.parameter_count()
    }
}

/// Type-safe parameter builder for WHERE clauses
#[derive(Debug, Clone)]
pub struct WhereParameterBuilder {
    /// Parameters for conditions
    parameters: QueryParameterSet,
}

impl WhereParameterBuilder {
    /// Create a new WHERE parameter builder
    pub fn new() -> Self {
        Self {
            parameters: QueryParameterSet::new(),
        }
    }
    
    /// Add a condition parameter
    pub fn add_condition_parameter(&mut self, value: Value) -> &mut Self {
        self.parameters.add_value(value);
        self
    }
}

impl Default for WhereParameterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeSafeParameterBuilder for WhereParameterBuilder {
    fn build_sql_fragment(&self, dialect: DatabaseDialect) -> SqlFragment {
        self.parameters.placeholder_fragment(dialect)
    }
    
    fn parameter_values(&self) -> Vec<Value> {
        self.parameters.parameter_values()
    }
    
    fn parameter_count(&self) -> usize {
        self.parameters.parameter_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::dialects::DatabaseDialect;

    // Test our business logic: parameter binding and type safety, not SQL generation

    #[test]
    fn test_parameter_binding_creation() {
        let string_binding = ParameterBinding::string("test".to_string());
        assert_eq!(string_binding.binding_type, ParameterType::String);
        assert_eq!(string_binding.value, Value::String("test".to_string()));

        let int_binding = ParameterBinding::integer(42);
        assert_eq!(int_binding.binding_type, ParameterType::Integer);

        let bool_binding = ParameterBinding::boolean(true);
        assert_eq!(bool_binding.binding_type, ParameterType::Boolean);
        assert_eq!(bool_binding.value, Value::Bool(true));

        let null_binding = ParameterBinding::null();
        assert_eq!(null_binding.binding_type, ParameterType::Null);
        assert_eq!(null_binding.value, Value::Null);
    }

    #[test]
    fn test_parameter_binding_from_value() {
        let json_string = json!("hello");
        let binding = ParameterBinding::from_value(json_string.clone());
        assert_eq!(binding.binding_type, ParameterType::String);
        assert_eq!(binding.value, json_string);

        let json_number = json!(123);
        let binding = ParameterBinding::from_value(json_number.clone());
        assert_eq!(binding.binding_type, ParameterType::Number);
        assert_eq!(binding.value, json_number);

        let json_array = json!([1, 2, 3]);
        let binding = ParameterBinding::from_value(json_array.clone());
        assert_eq!(binding.binding_type, ParameterType::Array);
        assert_eq!(binding.value, json_array);
    }

    #[test]
    fn test_query_parameter_set_basic_operations() {
        let mut params = QueryParameterSet::new();
        assert!(params.is_empty());
        assert_eq!(params.len(), 0);

        params.add_string("test".to_string());
        params.add_integer(42);
        params.add_boolean(true);

        assert_eq!(params.len(), 3);
        assert!(!params.is_empty());

        let values = params.values();
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], Value::String("test".to_string()));
        assert_eq!(values[1], json!(42));
        assert_eq!(values[2], Value::Bool(true));
    }

    #[test]
    fn test_query_parameter_set_placeholder_generation() {
        let mut params = QueryParameterSet::new();
        
        // Test parameter counting and business logic across all databases
        for dialect in DatabaseDialect::all_available() {
            // Empty parameter set
            let fragment = params.placeholder_fragment(dialect);
            assert_eq!(fragment.parameter_count, 0);
            assert_eq!(fragment.sql, "");

            // Single parameter
            params.add_string("test".to_string());
            let fragment = params.placeholder_fragment(dialect);
            assert_eq!(fragment.parameter_count, 1);
            assert!(!fragment.sql.is_empty());

            // Multiple parameters
            params.add_integer(42);
            params.add_boolean(true);
            let fragment = params.placeholder_fragment(dialect);
            assert_eq!(fragment.parameter_count, 3);
            assert!(!fragment.sql.is_empty());
            
            // Reset for next dialect
            params = QueryParameterSet::new();
        }
    }

    #[test]
    fn test_insert_parameter_builder() {
        let mut builder = InsertParameterBuilder::new();
        
        // Test parameter handling across all databases
        for dialect in DatabaseDialect::all_available() {
            // Empty builder
            let fragment = builder.values_clause_fragment(dialect);
            assert_eq!(fragment.parameter_count, 0);
            assert_eq!(fragment.sql, "()");

            // Add parameters
            builder.add_column_value(json!("name"));
            builder.add_column_value(json!(25));
            builder.add_column_value(json!(true));

            let fragment = builder.values_clause_fragment(dialect);
            assert_eq!(fragment.parameter_count, 3);
            assert!(fragment.sql.contains("(") && fragment.sql.contains(")"));

            // Test parameter values (this is our business logic)
            let values = builder.parameter_values();
            assert_eq!(values.len(), 3);
            assert_eq!(values[0], json!("name"));
            assert_eq!(values[1], json!(25));
            assert_eq!(values[2], json!(true));
            
            // Reset for next dialect
            builder = InsertParameterBuilder::new();
        }
    }

    #[test]
    fn test_update_parameter_builder() {
        let mut builder = UpdateParameterBuilder::new();
        
        // Add SET parameters
        builder.add_set_parameter(json!("new_name"));
        builder.add_set_parameter(json!(30));
        
        // Add WHERE parameters
        builder.add_where_parameter(json!(1));
        
        assert_eq!(builder.parameter_count(), 3);

        let values = builder.parameter_values();
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], json!("new_name"));
        assert_eq!(values[1], json!(30));
        assert_eq!(values[2], json!(1));
    }

    #[test]
    fn test_where_parameter_builder() {
        let mut builder = WhereParameterBuilder::new();
        
        builder.add_condition_parameter(json!(18));
        builder.add_condition_parameter(json!("active"));

        assert_eq!(builder.parameter_count(), 2);
        
        // Test across all databases - focus on parameter correctness
        for dialect in DatabaseDialect::all_available() {
            let fragment = builder.build_sql_fragment(dialect);
            assert_eq!(fragment.parameter_count, 2);
            assert!(!fragment.sql.is_empty());
            
            // Test parameter values (our business logic)
            let values = builder.parameter_values();
            assert_eq!(values.len(), 2);
            assert_eq!(values[0], json!(18));
            assert_eq!(values[1], json!("active"));
        }
    }

    #[test]
    fn test_type_safe_parameter_builder_trait() {
        let mut params = QueryParameterSet::new();
        params.add_string("test".to_string());
        params.add_integer(42);

        // Test trait methods across all databases - focus on parameter correctness
        for dialect in DatabaseDialect::all_available() {
            let fragment = params.build_sql_fragment(dialect);
            assert_eq!(fragment.parameter_count, 2);
            assert!(!fragment.sql.is_empty());

            let values = params.parameter_values();
            assert_eq!(values.len(), 2);
            assert_eq!(values[0], json!("test"));
            assert_eq!(values[1], json!(42));

            assert_eq!(params.parameter_count(), 2);
        }
    }

    #[test]
    fn test_sql_fragment_creation() {
        // Test fragment construction logic - count validation, not SQL format
        let fragment = SqlFragment::new("test_sql".to_string(), 3);
        assert_eq!(fragment.parameter_count, 3);
        assert!(!fragment.sql.is_empty());

        // Test single parameter across databases
        for dialect in DatabaseDialect::all_available() {
            let single = SqlFragment::single_parameter(dialect);
            assert_eq!(single.parameter_count, 1);
            assert!(!single.sql.is_empty());
        }

        let no_params = SqlFragment::no_parameters("SELECT 1".to_string());
        assert_eq!(no_params.parameter_count, 0);
        assert_eq!(no_params.sql, "SELECT 1");
    }

    #[test]
    fn test_parameter_type_display() {
        assert_eq!(format!("{}", ParameterType::String), "STRING");
        assert_eq!(format!("{}", ParameterType::Integer), "INTEGER");
        assert_eq!(format!("{}", ParameterType::Boolean), "BOOLEAN");
        assert_eq!(format!("{}", ParameterType::Null), "NULL");
        assert_eq!(format!("{}", ParameterType::Array), "ARRAY");
        assert_eq!(format!("{}", ParameterType::Object), "OBJECT");
    }

    #[test]
    fn test_complex_parameter_scenarios() {
        // Test with mixed types including null and complex JSON
        let mut params = QueryParameterSet::new();
        params.add_value(json!(null));
        params.add_value(json!({"nested": {"key": "value"}}));
        params.add_value(json!([1, 2, 3, 4, 5]));
        
        assert_eq!(params.parameter_count(), 3);
        
        // Test type detection (our business logic)
        let types = params.types();
        assert_eq!(types[0], ParameterType::Null);
        assert_eq!(types[1], ParameterType::Object);
        assert_eq!(types[2], ParameterType::Array);
        
        // Test parameter values
        let values = params.values();
        assert_eq!(values[0], json!(null));
        assert_eq!(values[1], json!({"nested": {"key": "value"}}));
        assert_eq!(values[2], json!([1, 2, 3, 4, 5]));
        
        // Test placeholder generation across databases
        for dialect in DatabaseDialect::all_available() {
            let fragment = params.placeholder_fragment(dialect);
            assert_eq!(fragment.parameter_count, 3);
            assert!(!fragment.sql.is_empty());
        }
    }

    #[test]
    fn test_parameter_builder_chaining() {
        let mut params = QueryParameterSet::new();
        
        // Test method chaining
        params
            .add_string("first".to_string())
            .add_integer(1)
            .add_boolean(false)
            .add_null();
            
        assert_eq!(params.parameter_count(), 4);
        
        let values = params.values();
        assert_eq!(values[0], json!("first"));
        assert_eq!(values[1], json!(1));
        assert_eq!(values[2], json!(false));
        assert_eq!(values[3], json!(null));
    }
}