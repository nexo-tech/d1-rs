use d1_rs::auto_migration::{EntityAnalyzer, SerializationStrategy, RecursiveRelationType};
use d1_rs::*;

/// Comprehensive test suite for Phase 1.2 EntityAnalyzer enhancements
/// Tests complex field types, recursive relationships, and M2M junction tables

#[tokio::test]
async fn test_complex_field_type_conversion() {
    let analyzer = EntityAnalyzer::new();
    
    // Test enhanced integer types
    assert_eq!(analyzer.rust_type_to_sql_type("i8", false).unwrap(), "INTEGER");
    assert_eq!(analyzer.rust_type_to_sql_type("u16", false).unwrap(), "INTEGER");
    assert_eq!(analyzer.rust_type_to_sql_type("i128", false).unwrap(), "TEXT"); // 128-bit not supported
    assert_eq!(analyzer.rust_type_to_sql_type("u128", false).unwrap(), "TEXT");
    
    // Test floating point types
    assert_eq!(analyzer.rust_type_to_sql_type("f32", false).unwrap(), "REAL");
    assert_eq!(analyzer.rust_type_to_sql_type("f64", false).unwrap(), "REAL");
    
    // Test string types
    assert_eq!(analyzer.rust_type_to_sql_type("String", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("&str", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("Cow<str>", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("char", false).unwrap(), "TEXT");
    
    // Test binary data types
    assert_eq!(analyzer.rust_type_to_sql_type("Vec<u8>", false).unwrap(), "BLOB");
    assert_eq!(analyzer.rust_type_to_sql_type("&[u8]", false).unwrap(), "BLOB");
    assert_eq!(analyzer.rust_type_to_sql_type("Box<[u8]>", false).unwrap(), "BLOB");
    
    // Test date/time types
    assert_eq!(analyzer.rust_type_to_sql_type("chrono::DateTime<Utc>", false).unwrap(), "DATETIME");
    assert_eq!(analyzer.rust_type_to_sql_type("chrono::DateTime<Local>", false).unwrap(), "DATETIME");
    assert_eq!(analyzer.rust_type_to_sql_type("chrono::NaiveDateTime", false).unwrap(), "DATETIME");
    assert_eq!(analyzer.rust_type_to_sql_type("chrono::NaiveDate", false).unwrap(), "DATE");
    assert_eq!(analyzer.rust_type_to_sql_type("chrono::NaiveTime", false).unwrap(), "TIME");
    
    // Test JSON types
    assert_eq!(analyzer.rust_type_to_sql_type("serde_json::Value", false).unwrap(), "JSON");
    assert_eq!(analyzer.rust_type_to_sql_type("serde_json::Map<String, serde_json::Value>", false).unwrap(), "JSON");
    
    // Test UUID
    assert_eq!(analyzer.rust_type_to_sql_type("uuid::Uuid", false).unwrap(), "TEXT");
    
    // Test decimal types
    assert_eq!(analyzer.rust_type_to_sql_type("rust_decimal::Decimal", false).unwrap(), "NUMERIC");
    assert_eq!(analyzer.rust_type_to_sql_type("bigdecimal::BigDecimal", false).unwrap(), "NUMERIC");
    
    // Test network types
    assert_eq!(analyzer.rust_type_to_sql_type("std::net::IpAddr", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("std::net::Ipv4Addr", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("std::net::Ipv6Addr", false).unwrap(), "TEXT");
    
    // Test URL types
    assert_eq!(analyzer.rust_type_to_sql_type("url::Url", false).unwrap(), "TEXT");
}

#[tokio::test]
async fn test_collection_types() {
    let analyzer = EntityAnalyzer::new();
    
    // Test collection types stored as JSON
    assert_eq!(analyzer.rust_type_to_sql_type("Vec<String>", false).unwrap(), "JSON");
    assert_eq!(analyzer.rust_type_to_sql_type("HashMap<String, i32>", false).unwrap(), "JSON");
    assert_eq!(analyzer.rust_type_to_sql_type("BTreeMap<String, String>", false).unwrap(), "JSON");
    assert_eq!(analyzer.rust_type_to_sql_type("HashSet<i32>", false).unwrap(), "JSON");
    assert_eq!(analyzer.rust_type_to_sql_type("BTreeSet<String>", false).unwrap(), "JSON");
    
    // Test array types
    assert_eq!(analyzer.rust_type_to_sql_type("[i32; 10]", false).unwrap(), "JSON");
    assert_eq!(analyzer.rust_type_to_sql_type("[String; 5]", false).unwrap(), "JSON");
}

#[tokio::test]
async fn test_option_types() {
    let analyzer = EntityAnalyzer::new();
    
    // Test Option<T> unwrapping
    assert_eq!(analyzer.rust_type_to_sql_type("Option<String>", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("Option<i32>", false).unwrap(), "INTEGER");
    assert_eq!(analyzer.rust_type_to_sql_type("Option<bool>", false).unwrap(), "BOOLEAN");
    assert_eq!(analyzer.rust_type_to_sql_type("Option<Vec<u8>>", false).unwrap(), "BLOB");
    assert_eq!(analyzer.rust_type_to_sql_type("Option<serde_json::Value>", false).unwrap(), "JSON");
}

#[tokio::test]
async fn test_custom_type_detection() {
    let analyzer = EntityAnalyzer::new();
    
    // Test enum type detection - currently returns false (no hardcoded detection)
    // TODO: In future, this would be trait-based or attribute-based
    assert!(!analyzer.is_enum_type("Status"));
    assert!(!analyzer.is_enum_type("UserRole"));
    assert!(!analyzer.is_enum_type("Color"));
    assert!(!analyzer.is_enum_type("Option<String>"));
    assert!(!analyzer.is_enum_type("Vec<i32>"));
    assert!(!analyzer.is_enum_type("std::collections::HashMap"));
    
    // Test self-referential type detection
    assert!(analyzer.is_self_referential_type("Option<Box<Self>>"));
    assert!(analyzer.is_self_referential_type("Vec<Self>"));
    assert!(analyzer.is_self_referential_type("Box<Self>"));
    assert!(!analyzer.is_self_referential_type("String"));
    assert!(!analyzer.is_self_referential_type("i32"));
    
    // Test custom struct detection - currently returns false (no hardcoded detection)
    // TODO: In future, this would be trait-based or attribute-based
    assert!(!analyzer.is_custom_struct_type("UserProfile"));
    assert!(!analyzer.is_custom_struct_type("Address"));
    assert!(!analyzer.is_custom_struct_type("std::string::String"));
    assert!(!analyzer.is_custom_struct_type("chrono::DateTime"));
    assert!(!analyzer.is_custom_struct_type("Option<String>"));
    
    // Test primitive type detection
    assert!(analyzer.is_primitive_type("i32"));
    assert!(analyzer.is_primitive_type("String"));
    assert!(analyzer.is_primitive_type("bool"));
    assert!(!analyzer.is_primitive_type("UserProfile"));
    assert!(!analyzer.is_primitive_type("Vec<i32>"));
}

#[tokio::test]
async fn test_foreign_key_id_detection() {
    let analyzer = EntityAnalyzer::new();
    
    // Test foreign key ID detection with different casing conventions
    assert_eq!(analyzer.rust_type_to_sql_type("UserId", false).unwrap(), "INTEGER");
    assert_eq!(analyzer.rust_type_to_sql_type("PostID", false).unwrap(), "INTEGER");
    assert_eq!(analyzer.rust_type_to_sql_type("category_id", false).unwrap(), "INTEGER");
    assert_eq!(analyzer.rust_type_to_sql_type("CustomEntityId", false).unwrap(), "INTEGER");
    
    // Test that it works with any naming convention
    assert_eq!(analyzer.rust_type_to_sql_type("ПользовательId", false).unwrap(), "INTEGER");
    assert_eq!(analyzer.rust_type_to_sql_type("用户ID", false).unwrap(), "INTEGER");
}

// Test entities for advanced analysis
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TestTreeNode {
    id: i64,
    parent_id: Option<i64>,
    name: String,
    is_active: bool,
}

#[derive(Debug, Clone)]
struct TestTreeNodeQueryBuilder;
impl QueryBuilder<TestTreeNode> for TestTreeNodeQueryBuilder {
    async fn all(self, _db: &D1Client) -> Result<Vec<TestTreeNode>> { unimplemented!() }
    async fn first(self, _db: &D1Client) -> Result<Option<TestTreeNode>> { unimplemented!() }
    async fn count(self, _db: &D1Client) -> Result<i64> { unimplemented!() }
    fn apply_relation_constraint(self, _field: &str, _value: serde_json::Value) -> Self { self }
}

#[derive(Debug, Clone)]
struct TestTreeNodeCreateBuilder;
impl CreateBuilder<TestTreeNode> for TestTreeNodeCreateBuilder {
    async fn save(self, _db: &D1Client) -> Result<TestTreeNode> { unimplemented!() }
}

#[derive(Debug, Clone)]
struct TestTreeNodeUpdateBuilder;
impl UpdateBuilder<TestTreeNode> for TestTreeNodeUpdateBuilder {
    async fn save(self, _db: &D1Client) -> Result<TestTreeNode> { unimplemented!() }
}

impl Entity for TestTreeNode {
    type PrimaryKey = i64;
    type QueryBuilder = TestTreeNodeQueryBuilder;
    type CreateBuilder = TestTreeNodeCreateBuilder;
    type UpdateBuilder = TestTreeNodeUpdateBuilder;

    const TABLE_NAME: &'static str = "test_tree_nodes";
    
    fn primary_key(&self) -> &Self::PrimaryKey { &self.id }
    fn query() -> Self::QueryBuilder { TestTreeNodeQueryBuilder }
    fn create() -> Self::CreateBuilder { TestTreeNodeCreateBuilder }
    fn update(_key: Self::PrimaryKey) -> Self::UpdateBuilder { TestTreeNodeUpdateBuilder }
    fn boolean_fields() -> &'static [&'static str] { &["is_active"] }

    async fn find(_db: &D1Client, _key: Self::PrimaryKey) -> Result<Option<Self>> { unimplemented!() }
    async fn delete(_db: &D1Client, _key: Self::PrimaryKey) -> Result<()> { unimplemented!() }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TestUser {
    id: i64,
    username: String,
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

    const TABLE_NAME: &'static str = "users";
    
    fn primary_key(&self) -> &Self::PrimaryKey { &self.id }
    fn query() -> Self::QueryBuilder { TestUserQueryBuilder }
    fn create() -> Self::CreateBuilder { TestUserCreateBuilder }
    fn update(_key: Self::PrimaryKey) -> Self::UpdateBuilder { TestUserUpdateBuilder }
    fn boolean_fields() -> &'static [&'static str] { &["is_active"] }

    async fn find(_db: &D1Client, _key: Self::PrimaryKey) -> Result<Option<Self>> { unimplemented!() }
    async fn delete(_db: &D1Client, _key: Self::PrimaryKey) -> Result<()> { unimplemented!() }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TestPost {
    id: i64,
    title: String,
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

    const TABLE_NAME: &'static str = "posts";
    
    fn primary_key(&self) -> &Self::PrimaryKey { &self.id }
    fn query() -> Self::QueryBuilder { TestPostQueryBuilder }
    fn create() -> Self::CreateBuilder { TestPostCreateBuilder }
    fn update(_key: Self::PrimaryKey) -> Self::UpdateBuilder { TestPostUpdateBuilder }
    fn boolean_fields() -> &'static [&'static str] { &["published"] }

    async fn find(_db: &D1Client, _key: Self::PrimaryKey) -> Result<Option<Self>> { unimplemented!() }
    async fn delete(_db: &D1Client, _key: Self::PrimaryKey) -> Result<()> { unimplemented!() }
}

#[tokio::test]
async fn test_recursive_relationship_detection() {
    let mut analyzer = EntityAnalyzer::new();
    
    // Test tree node entity (should detect recursive relationship)
    let advanced_result = analyzer.analyze_entity_advanced::<TestTreeNode>().unwrap();
    
    assert_eq!(advanced_result.table_schema.name, "test_tree_nodes");
    assert!(!advanced_result.recursive_relations.is_empty());
    
    let recursive_relation = &advanced_result.recursive_relations[0];
    assert_eq!(recursive_relation.relation_name, "parent");
    assert_eq!(recursive_relation.foreign_key_column, "parent_id");
    assert_eq!(recursive_relation.relation_type, RecursiveRelationType::SelfReferential);
    assert!(!recursive_relation.cascade_delete); // Trees usually don't cascade delete
    assert!(!recursive_relation.allow_cycles);
}

#[tokio::test]
async fn test_junction_table_detection() {
    let mut analyzer = EntityAnalyzer::new();
    
    // Test user entity (should detect user_roles junction table)
    let user_result = analyzer.analyze_entity_advanced::<TestUser>().unwrap();
    
    assert_eq!(user_result.table_schema.name, "users");
    assert!(!user_result.junction_tables.is_empty());
    
    let junction_table = &user_result.junction_tables[0];
    assert_eq!(junction_table.name, "user_roles");
    assert_eq!(junction_table.left_entity, "users");
    assert_eq!(junction_table.right_entity, "roles");
    assert_eq!(junction_table.left_column, "user_id");
    assert_eq!(junction_table.right_column, "role_id");
    assert_eq!(junction_table.foreign_keys.len(), 2);
    
    // Check foreign key constraints
    let user_fk = &junction_table.foreign_keys[0];
    assert_eq!(user_fk.columns, vec!["user_id"]);
    assert_eq!(user_fk.referenced_table, "users");
    assert_eq!(user_fk.referenced_columns, vec!["id"]);
    assert_eq!(user_fk.on_delete, Some("CASCADE".to_string()));
    
    let role_fk = &junction_table.foreign_keys[1];
    assert_eq!(role_fk.columns, vec!["role_id"]);
    assert_eq!(role_fk.referenced_table, "roles");
    assert_eq!(role_fk.referenced_columns, vec!["id"]);
    assert_eq!(role_fk.on_delete, Some("CASCADE".to_string()));
}

#[tokio::test]
async fn test_post_tags_junction_table() {
    let mut analyzer = EntityAnalyzer::new();
    
    // Test post entity (should detect post_tags junction table)
    let post_result = analyzer.analyze_entity_advanced::<TestPost>().unwrap();
    
    assert_eq!(post_result.table_schema.name, "posts");
    assert!(!post_result.junction_tables.is_empty());
    
    let junction_table = &post_result.junction_tables[0];
    assert_eq!(junction_table.name, "post_tags");
    assert_eq!(junction_table.left_entity, "posts");
    assert_eq!(junction_table.right_entity, "tags");
    assert_eq!(junction_table.left_column, "post_id");
    assert_eq!(junction_table.right_column, "tag_id");
    assert_eq!(junction_table.foreign_keys.len(), 2);
}

#[tokio::test]
async fn test_complex_field_analysis() {
    let mut analyzer = EntityAnalyzer::new();
    
    // Test user entity (should detect complex fields like metadata, preferences)
    let user_result = analyzer.analyze_entity_advanced::<TestUser>().unwrap();
    
    assert_eq!(user_result.table_schema.name, "users");
    assert!(!user_result.complex_fields.is_empty());
    
    // Should detect metadata field
    let metadata_field = user_result.complex_fields.iter()
        .find(|f| f.field_name == "metadata");
    assert!(metadata_field.is_some());
    
    let metadata = metadata_field.unwrap();
    assert_eq!(metadata.rust_type, "serde_json::Value");
    assert_eq!(metadata.sql_type, "JSON");
    assert_eq!(metadata.serialization_strategy, SerializationStrategy::Json);
    assert!(metadata.nullable);
    assert_eq!(metadata.default_value, Some("'{}'".to_string()));
    
    // Should detect preferences field
    let preferences_field = user_result.complex_fields.iter()
        .find(|f| f.field_name == "preferences");
    assert!(preferences_field.is_some());
    
    let preferences = preferences_field.unwrap();
    assert_eq!(preferences.rust_type, "HashMap<String, String>");
    assert_eq!(preferences.sql_type, "JSON");
    assert_eq!(preferences.serialization_strategy, SerializationStrategy::Json);
    assert!(preferences.nullable);
    assert_eq!(preferences.default_value, None);
}

#[tokio::test]
async fn test_foreign_key_extraction_with_recursion() {
    let mut analyzer = EntityAnalyzer::new();
    
    // Analyze tree node which should have recursive foreign key
    let schema = analyzer.analyze_entity::<TestTreeNode>().unwrap();
    
    // Should have recursive foreign key
    assert!(!schema.foreign_keys.is_empty());
    
    let recursive_fk = &schema.foreign_keys[0];
    assert_eq!(recursive_fk.name, "test_tree_nodes_parent_fk");
    assert_eq!(recursive_fk.columns, vec!["parent_id"]);
    assert_eq!(recursive_fk.referenced_table, "test_tree_nodes");
    assert_eq!(recursive_fk.referenced_columns, vec!["id"]);
    assert_eq!(recursive_fk.on_delete, Some("SET NULL".to_string()));
    assert_eq!(recursive_fk.on_update, Some("CASCADE".to_string()));
}

#[tokio::test]
async fn test_advanced_entity_analysis_complete() {
    let mut analyzer = EntityAnalyzer::new();
    
    // Test complete advanced analysis
    let user_result = analyzer.analyze_entity_advanced::<TestUser>().unwrap();
    let post_result = analyzer.analyze_entity_advanced::<TestPost>().unwrap();
    let tree_result = analyzer.analyze_entity_advanced::<TestTreeNode>().unwrap();
    
    // Verify all results have expected structure
    assert!(user_result.complex_fields.len() >= 2); // metadata, preferences
    assert!(user_result.junction_tables.len() >= 1); // user_roles
    assert!(user_result.recursive_relations.is_empty()); // No recursion in users
    
    assert!(post_result.junction_tables.len() >= 1); // post_tags
    assert!(post_result.recursive_relations.is_empty()); // No recursion in posts
    
    assert!(tree_result.recursive_relations.len() >= 1); // parent relationship
    assert!(tree_result.junction_tables.is_empty()); // No M2M in tree nodes
    
    println!("✅ All advanced entity analysis features working correctly!");
}

#[tokio::test]
async fn test_enum_type_sql_conversion() {
    let analyzer = EntityAnalyzer::new();
    
    // Test unknown enum types default to TEXT (safest, works with string serialization)
    // TODO: In future, users could configure these or use traits like EnumAsText/EnumAsInteger
    assert_eq!(analyzer.rust_type_to_sql_type("Status", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("UserRole", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("Priority", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("Color", false).unwrap(), "TEXT");
}

#[tokio::test]
async fn test_custom_struct_sql_conversion() {
    let analyzer = EntityAnalyzer::new();
    
    // Test unknown custom types default to TEXT (safest, works with any serialization)
    // TODO: In future, users could configure these or use traits like StructAsJson
    assert_eq!(analyzer.rust_type_to_sql_type("UserProfile", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("Address", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("Settings", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("Configuration", false).unwrap(), "TEXT");
    
    // Test that it works with any language/naming convention
    assert_eq!(analyzer.rust_type_to_sql_type("Пользователь", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("用户配置", false).unwrap(), "TEXT");
    assert_eq!(analyzer.rust_type_to_sql_type("MyCustomType123", false).unwrap(), "TEXT");
}