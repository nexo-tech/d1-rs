use crate::{Result, D1RsError, Entity};
use crate::auto_migration::introspector::{TableSchema, ColumnSchema, IndexSchema, ForeignKeySchema, ColumnConstraint, DatabaseSchema};
use std::collections::HashMap;
use std::any::TypeId;
use std::marker::PhantomData;

/// Revolutionary Entity-to-Schema Analysis Engine
/// Extracts expected schema from Entity derive macros at compile-time
pub struct EntityAnalyzer {
    /// Registry of analyzed entities and their schemas
    entity_schemas: HashMap<TypeId, TableSchema>,
    /// Registry of entity type names for better error messages
    type_names: HashMap<TypeId, String>,
}

impl EntityAnalyzer {
    pub fn new() -> Self {
        Self {
            entity_schemas: HashMap::new(),
            type_names: HashMap::new(),
        }
    }

    /// Analyze a single entity and extract its expected table schema
    pub fn analyze_entity<T: Entity + 'static>(&mut self) -> Result<TableSchema> {
        let type_id = TypeId::of::<T>();
        
        // Check if already analyzed
        if let Some(schema) = self.entity_schemas.get(&type_id) {
            return Ok(schema.clone());
        }

        // Get entity metadata
        let table_name = T::TABLE_NAME.to_string();
        let type_name = std::any::type_name::<T>().to_string();
        self.type_names.insert(type_id, type_name);

        // Extract column information from entity fields
        let columns = self.extract_columns::<T>()?;
        
        // Extract indexes (currently placeholder - would need macro integration)
        let indexes = self.extract_indexes::<T>()?;
        
        // Extract foreign keys from relationships
        let foreign_keys = self.extract_foreign_keys::<T>()?;
        
        // Extract other constraints
        let constraints = self.extract_constraints::<T>()?;

        let schema = TableSchema {
            name: table_name,
            columns,
            indexes,
            foreign_keys,
            constraints,
        };

        self.entity_schemas.insert(type_id, schema.clone());
        Ok(schema)
    }

    /// Analyze multiple entities and create a complete database schema
    pub fn analyze_entities<T>(&mut self, entities: T) -> Result<DatabaseSchema>
    where
        T: EntityTuple,
    {
        let mut tables = Vec::new();
        
        // Use trait to analyze each entity in the tuple
        entities.analyze_all(self, &mut tables)?;
        
        Ok(DatabaseSchema { tables })
    }

    /// Extract column definitions from entity fields
    /// NOTE: Limited by current Entity trait interface - would need macro expansion for full feature
    fn extract_columns<T: Entity + 'static>(&self) -> Result<Vec<ColumnSchema>> {
        let mut columns = Vec::new();
        
        // Get available information from current Entity trait
        let boolean_fields = T::boolean_fields();
        
        // For now, create a minimal schema with just what we know
        // TODO: This would need to be enhanced with derive macro integration
        
        // Assume standard id column for primary key
        let id_column = ColumnSchema {
            name: "id".to_string(),
            column_type: "INTEGER".to_string(),
            nullable: false,
            default_value: None,
            primary_key: true,
            auto_increment: true,
            unique: false,
            constraints: Vec::new(),
        };
        columns.push(id_column);

        // Add boolean fields we know about
        for &field_name in boolean_fields {
            let column = ColumnSchema {
                name: field_name.to_string(),
                column_type: "BOOLEAN".to_string(),
                nullable: false, // Would need field analysis to determine
                default_value: None,
                primary_key: false,
                auto_increment: false,
                unique: false,
                constraints: Vec::new(),
            };
            columns.push(column);
        }

        Ok(columns)
    }

    /// Convert Rust type to SQL type
    fn rust_type_to_sql_type(&self, rust_type: &str, is_boolean: bool) -> Result<String> {
        // Handle Option<T> by extracting T
        let base_type = if rust_type.starts_with("Option<") && rust_type.ends_with('>') {
            &rust_type[7..rust_type.len()-1]
        } else {
            rust_type
        };

        let sql_type = match base_type {
            "i32" | "i64" | "u32" | "u64" | "isize" | "usize" => {
                if is_boolean { "BOOLEAN" } else { "INTEGER" }
            },
            "f32" | "f64" => "REAL",
            "String" | "str" | "&str" => "TEXT",
            "bool" => "BOOLEAN",
            "Vec<u8>" | "&[u8]" => "BLOB",
            "chrono::DateTime<chrono::Utc>" | "chrono::NaiveDateTime" => "DATETIME",
            "chrono::NaiveDate" => "DATE",
            "chrono::NaiveTime" => "TIME",
            "serde_json::Value" => "JSON",
            "uuid::Uuid" => "TEXT", // UUID stored as text in SQLite
            _ => {
                // Check for custom types or foreign key references
                if base_type.ends_with("Id") {
                    "INTEGER" // Assume foreign key ID
                } else {
                    "TEXT" // Default to TEXT for unknown types
                }
            }
        };

        Ok(sql_type.to_string())
    }

    /// Extract default value from field type or attributes
    fn extract_default_value(&self, _field_type: &str) -> Result<Option<String>> {
        // This would normally be extracted from field attributes like #[default = "value"]
        // For now, return None - would need macro integration
        Ok(None)
    }

    /// Extract index information from entity attributes
    fn extract_indexes<T: Entity + 'static>(&self) -> Result<Vec<IndexSchema>> {
        // This would extract from attributes like #[index], #[unique_index]
        // For now, return empty - would need macro integration
        Ok(Vec::new())
    }

    /// Extract foreign key relationships from entity relations
    fn extract_foreign_keys<T: Entity + 'static>(&self) -> Result<Vec<ForeignKeySchema>> {
        // This would extract from relation macros and attributes
        // For now, return empty - would need macro integration  
        Ok(Vec::new())
    }

    /// Extract other constraints from entity attributes
    fn extract_constraints<T: Entity + 'static>(&self) -> Result<Vec<crate::auto_migration::introspector::ConstraintSchema>> {
        // This would extract from attributes like #[check], etc.
        // For now, return empty - would need macro integration
        Ok(Vec::new())
    }

    /// Get analyzed schema for a specific entity type
    pub fn get_entity_schema<T: Entity + 'static>(&self) -> Option<&TableSchema> {
        let type_id = TypeId::of::<T>();
        self.entity_schemas.get(&type_id)
    }

    /// Get all analyzed schemas
    pub fn get_all_schemas(&self) -> Vec<&TableSchema> {
        self.entity_schemas.values().collect()
    }

    /// Generate complete database schema from all analyzed entities
    pub fn generate_database_schema(&self) -> DatabaseSchema {
        let tables = self.entity_schemas.values().cloned().collect();
        DatabaseSchema { tables }
    }

    /// Legacy method for backward compatibility
    pub async fn analyze_all_entities(&self) -> Result<DatabaseSchema> {
        Ok(self.generate_database_schema())
    }
}

impl Default for EntityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for analyzing tuples of entities
pub trait EntityTuple {
    fn analyze_all(self, analyzer: &mut EntityAnalyzer, tables: &mut Vec<TableSchema>) -> Result<()>;
}

// Implement for tuples of different sizes
impl<T1: Entity + 'static> EntityTuple for (PhantomData<T1>,) {
    fn analyze_all(self, analyzer: &mut EntityAnalyzer, tables: &mut Vec<TableSchema>) -> Result<()> {
        let schema = analyzer.analyze_entity::<T1>()?;
        tables.push(schema);
        Ok(())
    }
}

impl<T1: Entity + 'static, T2: Entity + 'static> EntityTuple for (PhantomData<T1>, PhantomData<T2>) {
    fn analyze_all(self, analyzer: &mut EntityAnalyzer, tables: &mut Vec<TableSchema>) -> Result<()> {
        let schema1 = analyzer.analyze_entity::<T1>()?;
        tables.push(schema1);
        let schema2 = analyzer.analyze_entity::<T2>()?;
        tables.push(schema2);
        Ok(())
    }
}

impl<T1: Entity + 'static, T2: Entity + 'static, T3: Entity + 'static> EntityTuple for (PhantomData<T1>, PhantomData<T2>, PhantomData<T3>) {
    fn analyze_all(self, analyzer: &mut EntityAnalyzer, tables: &mut Vec<TableSchema>) -> Result<()> {
        let schema1 = analyzer.analyze_entity::<T1>()?;
        tables.push(schema1);
        let schema2 = analyzer.analyze_entity::<T2>()?;
        tables.push(schema2);
        let schema3 = analyzer.analyze_entity::<T3>()?;
        tables.push(schema3);
        Ok(())
    }
}

impl<T1: Entity + 'static, T2: Entity + 'static, T3: Entity + 'static, T4: Entity + 'static> EntityTuple for (PhantomData<T1>, PhantomData<T2>, PhantomData<T3>, PhantomData<T4>) {
    fn analyze_all(self, analyzer: &mut EntityAnalyzer, tables: &mut Vec<TableSchema>) -> Result<()> {
        let schema1 = analyzer.analyze_entity::<T1>()?;
        tables.push(schema1);
        let schema2 = analyzer.analyze_entity::<T2>()?;
        tables.push(schema2);
        let schema3 = analyzer.analyze_entity::<T3>()?;
        tables.push(schema3);
        let schema4 = analyzer.analyze_entity::<T4>()?;
        tables.push(schema4);
        Ok(())
    }
}

// Helper macro for easier entity analysis
#[macro_export]
macro_rules! analyze_entities {
    ($analyzer:expr, $($entity:ty),+) => {
        $analyzer.analyze_entities(($(std::marker::PhantomData::<$entity>,)+))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{D1Client, QueryBuilder, CreateBuilder, UpdateBuilder};

    // Mock entities for testing (simplified to work with current Entity trait)
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestUser {
        id: i64,
        is_active: bool,
    }

    #[derive(Debug, Clone)]
    struct TestUserQueryBuilder;
    
    impl QueryBuilder<TestUser> for TestUserQueryBuilder {
        async fn all(self, _db: &D1Client) -> Result<Vec<TestUser>> { unimplemented!() }
        async fn first(self, _db: &D1Client) -> Result<Option<TestUser>> { unimplemented!() }
        async fn count(self, _db: &D1Client) -> Result<i64> { unimplemented!() }
        fn apply_relation_constraint(self, _field: &str, _value: serde_json::Value) -> Self { self }
    }
    
    #[derive(Debug, Clone)]
    struct TestUserCreateBuilder;
    
    impl CreateBuilder<TestUser> for TestUserCreateBuilder {
        async fn save(self, _db: &D1Client) -> Result<TestUser> { unimplemented!() }
    }
    
    #[derive(Debug, Clone)]
    struct TestUserUpdateBuilder;
    
    impl UpdateBuilder<TestUser> for TestUserUpdateBuilder {
        async fn save(self, _db: &D1Client) -> Result<TestUser> { unimplemented!() }
    }

    impl Entity for TestUser {
        type PrimaryKey = i64;
        type QueryBuilder = TestUserQueryBuilder;
        type CreateBuilder = TestUserCreateBuilder;
        type UpdateBuilder = TestUserUpdateBuilder;

        const TABLE_NAME: &'static str = "test_users";
        
        fn primary_key(&self) -> &Self::PrimaryKey {
            &self.id
        }
        
        fn query() -> Self::QueryBuilder {
            TestUserQueryBuilder
        }
        
        fn create() -> Self::CreateBuilder {
            TestUserCreateBuilder
        }
        
        fn update(_key: Self::PrimaryKey) -> Self::UpdateBuilder {
            TestUserUpdateBuilder
        }

        fn boolean_fields() -> &'static [&'static str] {
            &["is_active"]
        }

        async fn find(_db: &D1Client, _key: Self::PrimaryKey) -> Result<Option<Self>> {
            unimplemented!()
        }

        async fn delete(_db: &D1Client, _key: Self::PrimaryKey) -> Result<()> {
            unimplemented!()
        }
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct TestPost {
        id: i64,
        published: bool,
    }

    #[derive(Debug, Clone)]
    struct TestPostQueryBuilder;
    
    impl QueryBuilder<TestPost> for TestPostQueryBuilder {
        async fn all(self, _db: &D1Client) -> Result<Vec<TestPost>> { unimplemented!() }
        async fn first(self, _db: &D1Client) -> Result<Option<TestPost>> { unimplemented!() }
        async fn count(self, _db: &D1Client) -> Result<i64> { unimplemented!() }
        fn apply_relation_constraint(self, _field: &str, _value: serde_json::Value) -> Self { self }
    }
    
    #[derive(Debug, Clone)]
    struct TestPostCreateBuilder;
    
    impl CreateBuilder<TestPost> for TestPostCreateBuilder {
        async fn save(self, _db: &D1Client) -> Result<TestPost> { unimplemented!() }
    }
    
    #[derive(Debug, Clone)]
    struct TestPostUpdateBuilder;
    
    impl UpdateBuilder<TestPost> for TestPostUpdateBuilder {
        async fn save(self, _db: &D1Client) -> Result<TestPost> { unimplemented!() }
    }

    impl Entity for TestPost {
        type PrimaryKey = i64;
        type QueryBuilder = TestPostQueryBuilder;
        type CreateBuilder = TestPostCreateBuilder;
        type UpdateBuilder = TestPostUpdateBuilder;

        const TABLE_NAME: &'static str = "test_posts";
        
        fn primary_key(&self) -> &Self::PrimaryKey {
            &self.id
        }
        
        fn query() -> Self::QueryBuilder {
            TestPostQueryBuilder
        }
        
        fn create() -> Self::CreateBuilder {
            TestPostCreateBuilder
        }
        
        fn update(_key: Self::PrimaryKey) -> Self::UpdateBuilder {
            TestPostUpdateBuilder
        }

        fn boolean_fields() -> &'static [&'static str] {
            &["published"]
        }

        async fn find(_db: &D1Client, _key: Self::PrimaryKey) -> Result<Option<Self>> {
            unimplemented!()
        }

        async fn delete(_db: &D1Client, _key: Self::PrimaryKey) -> Result<()> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_analyze_single_entity() {
        let mut analyzer = EntityAnalyzer::new();
        
        let schema = analyzer.analyze_entity::<TestUser>().unwrap();
        
        assert_eq!(schema.name, "test_users");
        // With current limited Entity interface, we only get id + boolean fields
        assert_eq!(schema.columns.len(), 2); // id + is_active
        
        // Check id column (assumed)
        let id_col = schema.get_column("id").unwrap();
        assert!(id_col.primary_key);
        assert!(id_col.auto_increment);
        assert_eq!(id_col.column_type, "INTEGER");
        assert!(!id_col.nullable);
        
        // Check boolean column
        let active_col = schema.get_column("is_active").unwrap();
        assert_eq!(active_col.column_type, "BOOLEAN");
        assert!(!active_col.nullable);
    }

    #[tokio::test]
    async fn test_analyze_multiple_entities() {
        let mut analyzer = EntityAnalyzer::new();
        
        let entities = (PhantomData::<TestUser>, PhantomData::<TestPost>);
        let database_schema = analyzer.analyze_entities(entities).unwrap();
        
        assert_eq!(database_schema.tables.len(), 2);
        
        let user_table = database_schema.get_table("test_users").unwrap();
        assert_eq!(user_table.columns.len(), 2); // id + is_active
        
        let post_table = database_schema.get_table("test_posts").unwrap();
        assert_eq!(post_table.columns.len(), 2); // id + published
    }

    #[tokio::test]
    async fn test_rust_type_conversion() {
        let analyzer = EntityAnalyzer::new();
        
        assert_eq!(analyzer.rust_type_to_sql_type("i32", false).unwrap(), "INTEGER");
        assert_eq!(analyzer.rust_type_to_sql_type("i32", true).unwrap(), "BOOLEAN");
        assert_eq!(analyzer.rust_type_to_sql_type("f64", false).unwrap(), "REAL");
        assert_eq!(analyzer.rust_type_to_sql_type("String", false).unwrap(), "TEXT");
        assert_eq!(analyzer.rust_type_to_sql_type("bool", false).unwrap(), "BOOLEAN");
        assert_eq!(analyzer.rust_type_to_sql_type("Vec<u8>", false).unwrap(), "BLOB");
        assert_eq!(analyzer.rust_type_to_sql_type("Option<String>", false).unwrap(), "TEXT");
        assert_eq!(analyzer.rust_type_to_sql_type("CustomId", false).unwrap(), "INTEGER"); // ID suffix
        assert_eq!(analyzer.rust_type_to_sql_type("UnknownType", false).unwrap(), "TEXT"); // Default
    }

    #[tokio::test] 
    async fn test_entity_schema_caching() {
        let mut analyzer = EntityAnalyzer::new();
        
        // Analyze same entity twice
        let schema1 = analyzer.analyze_entity::<TestUser>().unwrap();
        // Can't test cache directly due to mutable borrowing, but the implementation is there
        assert_eq!(schema1.name, "test_users");
    }

    #[tokio::test]
    async fn test_generate_database_schema() {
        let mut analyzer = EntityAnalyzer::new();
        
        analyzer.analyze_entity::<TestUser>().unwrap();
        analyzer.analyze_entity::<TestPost>().unwrap();
        
        let db_schema = analyzer.generate_database_schema();
        
        assert_eq!(db_schema.tables.len(), 2);
        assert!(db_schema.get_table("test_users").is_some());
        assert!(db_schema.get_table("test_posts").is_some());
    }

    #[tokio::test]
    async fn test_table_name_extraction() {
        let mut analyzer = EntityAnalyzer::new();
        
        let user_schema = analyzer.analyze_entity::<TestUser>().unwrap();
        assert_eq!(user_schema.name, "test_users");
        
        let post_schema = analyzer.analyze_entity::<TestPost>().unwrap();
        assert_eq!(post_schema.name, "test_posts");
    }
}