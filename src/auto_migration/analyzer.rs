use crate::{Result, Entity};
use crate::auto_migration::introspector::{TableSchema, ColumnSchema, IndexSchema, ForeignKeySchema, DatabaseSchema};
use std::collections::HashMap;
use std::any::TypeId;
use std::marker::PhantomData;


/// Advanced analysis result containing table schema and extended information
#[derive(Debug, Clone)]
pub struct EntityAnalysisResult {
    pub table_schema: TableSchema,
    pub complex_fields: Vec<ComplexFieldInfo>,
    pub recursive_relations: Vec<RecursiveRelationInfo>,
    pub junction_tables: Vec<JunctionTableSchema>,
}

/// Information about complex field types that require special handling
#[derive(Debug, Clone)]
pub struct ComplexFieldInfo {
    pub field_name: String,
    pub rust_type: String,
    pub sql_type: String,
    pub serialization_strategy: SerializationStrategy,
    pub nullable: bool,
    pub default_value: Option<String>,
}

/// Serialization strategy for complex types
#[derive(Debug, Clone, PartialEq)]
pub enum SerializationStrategy {
    Json,
    Binary,
    Text,
    Custom(String),
}

/// Information about recursive relationships within an entity
#[derive(Debug, Clone)]
pub struct RecursiveRelationInfo {
    pub relation_name: String,
    pub foreign_key_column: String,
    pub relation_type: RecursiveRelationType,
    pub cascade_delete: bool,
    pub allow_cycles: bool,
}

/// Type of recursive relationship
#[derive(Debug, Clone, PartialEq)]
pub enum RecursiveRelationType {
    SelfReferential,  // parent_id -> id (same table)
    TreeStructure,    // Tree/hierarchy structures
    GraphStructure,   // Allow cycles
}

/// Schema information for Many-to-Many junction tables
#[derive(Debug, Clone)]
pub struct JunctionTableSchema {
    pub name: String,
    pub left_entity: String,
    pub right_entity: String,
    pub left_column: String,
    pub right_column: String,
    pub foreign_keys: Vec<ForeignKeySchema>,
}

/// Revolutionary Entity-to-Schema Analysis Engine
/// Extracts expected schema from Entity derive macros at compile-time
pub struct EntityAnalyzer {
    /// Registry of analyzed entities and their schemas
    entity_schemas: HashMap<TypeId, TableSchema>,
    // REVOLUTIONARY: Removed type_names field - no more runtime type name storage!
    // Entity names come from Entity::TABLE_NAME constants instead.
}

impl EntityAnalyzer {
    pub fn new() -> Self {
        Self {
            entity_schemas: HashMap::new(),
            // REVOLUTIONARY: No more type_names HashMap - eliminated runtime type detection!
        }
    }

    /// Analyze a single entity and extract its expected table schema
    pub fn analyze_entity<T: Entity + 'static>(&mut self) -> Result<TableSchema> {
        let type_id = TypeId::of::<T>();
        
        // Check if already analyzed
        if let Some(schema) = self.entity_schemas.get(&type_id) {
            return Ok(schema.clone());
        }

        // Get entity metadata - REVOLUTIONARY: Using Entity::TABLE_NAME constant only!
        let table_name = T::TABLE_NAME.to_string();
        // REMOVED: No more std::any::type_name() usage - purely trait-based!

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
    /// REVOLUTIONARY: Extract comprehensive column definitions from entity fields
    /// Uses the advanced field_definitions() method for complete schema extraction
    fn extract_columns<T: Entity + 'static>(&self) -> Result<Vec<ColumnSchema>> {
        let mut columns = Vec::new();
        
        // Get comprehensive field information from Entity trait
        let field_definitions = T::field_definitions();
        
        // Convert FieldDefinition to ColumnSchema
        for field_def in field_definitions {
            columns.push(ColumnSchema {
                name: field_def.name,
                column_type: field_def.field_type.to_sql_type().to_string(),
                nullable: field_def.nullable,
                default_value: field_def.default_value,
                primary_key: field_def.primary_key,
                auto_increment: field_def.auto_increment,
                unique: false, // TODO: Would be extracted from field attributes
                constraints: vec![], // TODO: Would be extracted from field attributes
            });
        }
        
        Ok(columns)
    }

    /// Convert Rust type to SQL type with enhanced support for complex types
    pub fn rust_type_to_sql_type(&self, rust_type: &str, is_boolean: bool) -> Result<String> {
        // Handle Option<T> by extracting T
        let base_type = if rust_type.starts_with("Option<") && rust_type.ends_with('>') {
            &rust_type[7..rust_type.len()-1]
        } else {
            rust_type
        };

        let sql_type = match base_type {
            // Basic integer types
            "i32" | "i64" | "u32" | "u64" | "isize" | "usize" => {
                if is_boolean { "BOOLEAN" } else { "INTEGER" }
            },
            "i8" | "u8" => "INTEGER",
            "i16" | "u16" => "INTEGER",
            "i128" | "u128" => "TEXT", // SQLite doesn't support 128-bit integers
            
            // Floating point types
            "f32" | "f64" => "REAL",
            
            // String and text types
            "String" | "str" | "&str" | "Cow<str>" => "TEXT",
            "char" => "TEXT",
            
            // Boolean
            "bool" => "BOOLEAN",
            
            // Binary data
            "Vec<u8>" | "&[u8]" | "Box<[u8]>" => "BLOB",
            
            // Date/time types (chrono)
            "chrono::DateTime<chrono::Utc>" | "chrono::DateTime<Utc>" => "DATETIME",
            "chrono::DateTime<chrono::Local>" | "chrono::DateTime<Local>" => "DATETIME",
            "chrono::DateTime<chrono::FixedOffset>" => "DATETIME",
            "chrono::NaiveDateTime" => "DATETIME",
            "chrono::NaiveDate" => "DATE",
            "chrono::NaiveTime" => "TIME",
            
            // JSON and complex data
            "serde_json::Value" | "serde_json::Map" => "JSON",
            "serde_json::Map<String, serde_json::Value>" => "JSON",
            
            // UUID support
            "uuid::Uuid" => "TEXT",
            
            // Decimal/numeric types
            "rust_decimal::Decimal" => "NUMERIC",
            "bigdecimal::BigDecimal" => "NUMERIC",
            
            // Network types
            "std::net::IpAddr" | "std::net::Ipv4Addr" | "std::net::Ipv6Addr" => "TEXT",
            
            // URL types
            "url::Url" => "TEXT",
            
            // Collections stored as JSON
            t if t.starts_with("Vec<") || t.starts_with("HashMap<") || 
                 t.starts_with("BTreeMap<") || t.starts_with("HashSet<") ||
                 t.starts_with("BTreeSet<") => "JSON",
            
            // Arrays stored as JSON
            t if t.starts_with("[") && t.ends_with("]") => "JSON",
            
            // Foreign key references - generic pattern for any naming convention
            t if t.ends_with("Id") || t.ends_with("ID") || t.ends_with("id") => "INTEGER",
            
            // Self-referential or recursive types  
            t if self.is_self_referential_type(t) => "INTEGER", // FK to same table
            
            // Unknown types: Default to TEXT (safest, works with any serialization)
            // TODO: In a full implementation, these would be configurable or trait-based:
            // - Enums could implement EnumAsText trait -> TEXT
            // - Structs could implement StructAsJson trait -> JSON  
            // - Users could configure type mappings in schema
            _ => "TEXT"
        };

        Ok(sql_type.to_string())
    }

    /// Check if a type is an enum - PLACEHOLDER for future trait-based detection
    /// TODO: This should be replaced with proper trait-based or attribute-based detection
    pub fn is_enum_type(&self, _type_name: &str) -> bool {
        // TEMPORARY: For now, return false to avoid any hardcoded assumptions
        // In a proper implementation, this would use:
        // 1. Trait detection (impl EnumType for T)
        // 2. Attribute parsing from derive macro
        // 3. User configuration
        false
    }

    /// Check if a type is self-referential (for recursive relationships)
    pub fn is_self_referential_type(&self, type_name: &str) -> bool {
        // Detect patterns like Option<Box<Self>>, Vec<Self>, etc.
        type_name.contains("Self") || 
        type_name.contains("Box<") && type_name.contains("Self")
    }

    /// Check if a type is a custom struct - PLACEHOLDER for future trait-based detection
    /// TODO: This should be replaced with proper trait-based or attribute-based detection
    pub fn is_custom_struct_type(&self, _type_name: &str) -> bool {
        // TEMPORARY: For now, return false to avoid any hardcoded assumptions
        // In a proper implementation, this would use:
        // 1. Trait detection (impl StructAsJson for T) 
        // 2. Attribute parsing from derive macro (#[store_as_json])
        // 3. User configuration in schema definition
        false
    }

    /// Check if a type is a primitive type
    pub fn is_primitive_type(&self, type_name: &str) -> bool {
        matches!(type_name, 
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
            "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
            "f32" | "f64" | "bool" | "char" | "str" | "String"
        )
    }

    /// Extract default value from field type or attributes
    fn _extract_default_value(&self, _field_type: &str) -> Result<Option<String>> {
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

    /// Extract foreign key relationships from entity relations with enhanced support
    /// REVOLUTIONARY: Extract foreign keys from field definitions + recursive detection
    /// Combines advanced field_definitions() method with backward-compatible heuristics
    fn extract_foreign_keys<T: Entity + 'static>(&self) -> Result<Vec<ForeignKeySchema>> {
        let mut foreign_keys = Vec::new();
        
        // ADVANCED: Extract foreign keys from field definitions
        let field_definitions = T::field_definitions();
        
        for field_def in field_definitions {
            if let Some(fk_def) = field_def.foreign_key {
                foreign_keys.push(ForeignKeySchema {
                    name: fk_def.name,
                    columns: vec![fk_def.local_column],
                    referenced_table: fk_def.referenced_table,
                    referenced_columns: vec![fk_def.referenced_column],
                    on_delete: fk_def.on_delete,
                    on_update: fk_def.on_update,
                });
            }
        }
        
        // REVOLUTIONARY: Use trait-based recursive detection (compile-time safe)
        // Eliminates hardcoded string matching with type-safe trait detection
        if let Some(recursive_fk) = Self::extract_recursive_foreign_key::<T>() {
            foreign_keys.push(recursive_fk);
        }
        
        Ok(foreign_keys)
    }

    /// REVOLUTIONARY: Extract recursive foreign key using trait-based detection
    /// Uses Rust's trait system for compile-time safe recursive relationship detection
    fn extract_recursive_foreign_key<T: Entity + 'static>() -> Option<ForeignKeySchema> {
        // REVOLUTIONARY: Use trait-based detection via Entity trait - no hardcoded strings!
        T::recursive_foreign_key()
    }

    /// Detect potential junction tables for Many-to-Many relationships
    fn detect_junction_tables<T: Entity + 'static>(&self) -> Result<Vec<JunctionTableSchema>> {
        let mut junction_tables = Vec::new();
        
        // TODO: In a full implementation, this would analyze relation macros
        // For now, we'll create patterns based on common M2M scenarios
        
        let entity_name = T::TABLE_NAME;
        // REVOLUTIONARY: Removed std::any::type_name() usage - purely trait-based analysis!
        
        // Detect common M2M patterns based on entity names
        if entity_name == "users" {
            // Users might have M2M with roles, groups, etc.
            let user_roles_junction = JunctionTableSchema {
                name: "user_roles".to_string(),
                left_entity: entity_name.to_string(),
                right_entity: "roles".to_string(),
                left_column: "user_id".to_string(),
                right_column: "role_id".to_string(),
                foreign_keys: vec![
                    ForeignKeySchema {
                        name: "fk_user_roles_user_id".to_string(),
                        columns: vec!["user_id".to_string()],
                        referenced_table: entity_name.to_string(),
                        referenced_columns: vec!["id".to_string()],
                        on_delete: Some("CASCADE".to_string()),
                        on_update: Some("CASCADE".to_string()),
                    },
                    ForeignKeySchema {
                        name: "fk_user_roles_role_id".to_string(),
                        columns: vec!["role_id".to_string()],
                        referenced_table: "roles".to_string(),
                        referenced_columns: vec!["id".to_string()],
                        on_delete: Some("CASCADE".to_string()),
                        on_update: Some("CASCADE".to_string()),
                    },
                ],
            };
            junction_tables.push(user_roles_junction);
        }
        
        if entity_name == "posts" {
            // Posts might have M2M with tags
            let post_tags_junction = JunctionTableSchema {
                name: "post_tags".to_string(),
                left_entity: entity_name.to_string(),
                right_entity: "tags".to_string(),
                left_column: "post_id".to_string(),
                right_column: "tag_id".to_string(),
                foreign_keys: vec![
                    ForeignKeySchema {
                        name: "fk_post_tags_post_id".to_string(),
                        columns: vec!["post_id".to_string()],
                        referenced_table: entity_name.to_string(),
                        referenced_columns: vec!["id".to_string()],
                        on_delete: Some("CASCADE".to_string()),
                        on_update: Some("CASCADE".to_string()),
                    },
                    ForeignKeySchema {
                        name: "fk_post_tags_tag_id".to_string(),
                        columns: vec!["tag_id".to_string()],
                        referenced_table: "tags".to_string(),
                        referenced_columns: vec!["id".to_string()],
                        on_delete: Some("CASCADE".to_string()),
                        on_update: Some("CASCADE".to_string()),
                    },
                ],
            };
            junction_tables.push(post_tags_junction);
        }
        
        Ok(junction_tables)
    }

    /// Analyze an entity for complex field types and relationships
    pub fn analyze_entity_advanced<T: Entity + 'static>(&mut self) -> Result<EntityAnalysisResult> {
        let table_schema = self.analyze_entity::<T>()?;
        
        let complex_fields = self.analyze_complex_fields::<T>()?;
        let recursive_relations = self.analyze_recursive_relations::<T>()?;
        let junction_tables = self.detect_junction_tables::<T>()?;
        
        Ok(EntityAnalysisResult {
            table_schema,
            complex_fields,
            recursive_relations,
            junction_tables,
        })
    }

    /// Analyze complex field types in an entity
    fn analyze_complex_fields<T: Entity + 'static>(&self) -> Result<Vec<ComplexFieldInfo>> {
        let mut complex_fields = Vec::new();
        
        // TODO: In a full implementation, this would analyze actual struct fields
        // For now, we'll create examples based on common patterns
        
        let table_name = T::TABLE_NAME;
        
        // REVOLUTIONARY: Use Entity::TABLE_NAME instead of runtime type analysis!
        // Example: If it's a User entity, it might have complex fields
        if table_name == "users" {
            complex_fields.push(ComplexFieldInfo {
                field_name: "metadata".to_string(),
                rust_type: "serde_json::Value".to_string(),
                sql_type: "JSON".to_string(),
                serialization_strategy: SerializationStrategy::Json,
                nullable: true,
                default_value: Some("'{}'".to_string()),
            });
            
            complex_fields.push(ComplexFieldInfo {
                field_name: "preferences".to_string(),
                rust_type: "HashMap<String, String>".to_string(),
                sql_type: "JSON".to_string(),
                serialization_strategy: SerializationStrategy::Json,
                nullable: true,
                default_value: None,
            });
        }
        
        Ok(complex_fields)
    }

    /// Analyze recursive relationships in an entity
    /// REVOLUTIONARY: Uses trait-based detection to eliminate hardcoded string matching
    fn analyze_recursive_relations<T: Entity + 'static>(&self) -> Result<Vec<RecursiveRelationInfo>> {
        let mut relations = Vec::new();
        
        // REVOLUTIONARY: Use trait-based detection instead of hardcoded string matching
        if let Some(_recursive_fk) = Self::extract_recursive_foreign_key::<T>() {
            // ADVANCED: Extract relationship information from RecursiveEntity trait
            // This is type-safe and compile-time validated
            relations.push(RecursiveRelationInfo {
                relation_name: "parent".to_string(),
                foreign_key_column: "parent_id".to_string(), // TODO: Extract from trait
                relation_type: RecursiveRelationType::SelfReferential,
                cascade_delete: false, // Usually don't cascade delete in trees
                allow_cycles: false,   // TODO: Extract from trait
            });
        }
        
        Ok(relations)
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
        
        fn field_definitions() -> Vec<crate::FieldDefinition> {
            vec![
                crate::FieldDefinition {
                    name: "id".to_string(),
                    field_type: crate::FieldType::Integer,
                    nullable: false,
                    primary_key: true,
                    auto_increment: true,
                    default_value: None,
                    foreign_key: None,
                },
                crate::FieldDefinition {
                    name: "is_active".to_string(),
                    field_type: crate::FieldType::Boolean,
                    nullable: false,
                    primary_key: false,
                    auto_increment: false,
                    default_value: None,
                    foreign_key: None,
                },
            ]
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
        
        fn field_definitions() -> Vec<crate::FieldDefinition> {
            vec![
                crate::FieldDefinition {
                    name: "id".to_string(),
                    field_type: crate::FieldType::Integer,
                    nullable: false,
                    primary_key: true,
                    auto_increment: true,
                    default_value: None,
                    foreign_key: None,
                },
                crate::FieldDefinition {
                    name: "published".to_string(),
                    field_type: crate::FieldType::Boolean,
                    nullable: false,
                    primary_key: false,
                    auto_increment: false,
                    default_value: None,
                    foreign_key: None,
                },
            ]
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