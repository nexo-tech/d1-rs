mod common;

use d1orm::*;

#[tokio::test]
async fn test_modern_schema_boolean_api() {
    // Test the modern schema API with boolean fields
    let users_table = TableDefinition::new("users")
        .integer("id").primary_key().auto_increment()
        .text("email").not_null().unique()
        .text("name").not_null()
        .boolean("is_active").default_true().not_null()  // Boolean field!
        .boolean("is_verified").default_false()           // Another boolean!
        .integer("age").check("age >= 0")
        .datetime("created_at").default(DefaultValue::CurrentTimestamp)
        .json("metadata")  // JSON field
        .build();
    
    println!("Generated SQL: {}", users_table.to_sql());
    
    // Verify boolean columns are detected properly
    let boolean_cols = users_table.boolean_columns();
    assert_eq!(boolean_cols, vec!["is_active", "is_verified"]);
    
    // Verify the SQL contains proper boolean handling (in correct SQL syntax order)
    let sql = users_table.to_sql();
    assert!(sql.contains("is_active INTEGER NOT NULL DEFAULT 1"));  // Correct SQL order: NOT NULL before DEFAULT
    assert!(sql.contains("is_verified INTEGER DEFAULT 0"));
}

#[tokio::test]
async fn test_schema_column_types() {
    // Test all column types
    let test_table = TableDefinition::new("test_types")
        .integer("int_col").not_null()
        .text("text_col").unique()
        .boolean("bool_col").default_false()  // Boolean becomes INTEGER
        .datetime("date_col").default(DefaultValue::CurrentTimestamp)
        .json("json_col")
        .build();
    
    let sql = test_table.to_sql();
    println!("Type test SQL: {}", sql);
    
    // Verify each type is converted correctly
    assert!(sql.contains("int_col INTEGER NOT NULL"));
    assert!(sql.contains("text_col TEXT UNIQUE"));
    assert!(sql.contains("bool_col INTEGER DEFAULT 0"));  // Boolean false = 0
    assert!(sql.contains("date_col DATETIME DEFAULT CURRENT_TIMESTAMP"));
    assert!(sql.contains("json_col TEXT"));
    
    // Verify boolean metadata
    let boolean_fields = test_table.boolean_columns();
    assert_eq!(boolean_fields, vec!["bool_col"]);
}

#[tokio::test]
async fn test_schema_constraints() {
    // Test various constraints
    let constrained_table = TableDefinition::new("constrained")
        .integer("id").primary_key().auto_increment()
        .text("username").not_null().unique()
        .integer("score").default(DefaultValue::Integer(100)).check("score >= 0")
        .boolean("active").default_true()
        .boolean("verified").default_false()
        .build();
    
    let sql = constrained_table.to_sql();
    println!("Constraints SQL: {}", sql);
    
    // Verify constraints
    assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
    assert!(sql.contains("username TEXT NOT NULL UNIQUE"));
    assert!(sql.contains("score INTEGER DEFAULT 100 CHECK (score >= 0)"));
    assert!(sql.contains("active INTEGER DEFAULT 1"));  // true = 1
    assert!(sql.contains("verified INTEGER DEFAULT 0")); // false = 0
    
    // Verify both boolean fields are detected
    let boolean_fields = constrained_table.boolean_columns();
    assert_eq!(boolean_fields, vec!["active", "verified"]);
}

#[tokio::test]
async fn test_boolean_default_values() {
    // Test boolean default value handling
    let table_true = TableDefinition::new("test_true")
        .boolean("flag").default_true()
        .build();
    
    let table_false = TableDefinition::new("test_false")
        .boolean("flag").default_false()
        .build();
    
    let sql_true = table_true.to_sql();
    let sql_false = table_false.to_sql();
    
    assert!(sql_true.contains("flag INTEGER DEFAULT 1"));  // true = 1
    assert!(sql_false.contains("flag INTEGER DEFAULT 0")); // false = 0
}

#[tokio::test]
async fn test_schema_fluent_api() {
    // Test the fluent API chaining
    let complex_table = TableDefinition::new("complex_example")
        .integer("id").primary_key().auto_increment()
        .text("title").not_null()
        .text("slug").unique()
        .boolean("published").default_false()
        .boolean("featured").default_false()
        .integer("views").default(DefaultValue::Integer(0))
        .text("tags").check("length(tags) > 0")
        .datetime("created_at").default(DefaultValue::CurrentTimestamp)
        .datetime("updated_at").default(DefaultValue::CurrentTimestamp)
        .json("metadata")
        .build();
    
    let sql = complex_table.to_sql();
    println!("Complex table SQL: {}", sql);
    
    // Verify complex chaining works
    assert!(sql.contains("CREATE TABLE complex_example"));
    assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
    assert!(sql.contains("published INTEGER DEFAULT 0"));
    assert!(sql.contains("featured INTEGER DEFAULT 0"));
    assert!(sql.contains("views INTEGER DEFAULT 0"));
    assert!(sql.contains("metadata TEXT"));
    
    // Verify boolean metadata
    let boolean_fields = complex_table.boolean_columns();
    assert_eq!(boolean_fields, vec!["published", "featured"]);
}

#[tokio::test] 
async fn test_schema_vs_old_api_comparison() {
    // Show the difference between old string-based API and new type-safe API
    
    // NEW API (type-safe, boolean-aware)
    let new_table = TableDefinition::new("modern_posts")
        .integer("id").primary_key().auto_increment()
        .text("title").not_null()
        .boolean("is_published").default_false()  // Type-safe boolean!
        .boolean("is_featured").default_true()    // Another boolean!
        .datetime("created_at").default(DefaultValue::CurrentTimestamp)
        .build();
    
    println!("NEW API SQL: {}", new_table.to_sql());
    
    // The new API automatically:
    // 1. Creates INTEGER columns for booleans
    // 2. Converts boolean defaults to 0/1
    // 3. Provides metadata for boolean field detection
    assert!(new_table.to_sql().contains("is_published INTEGER DEFAULT 0"));
    assert!(new_table.to_sql().contains("is_featured INTEGER DEFAULT 1"));
    assert_eq!(new_table.boolean_columns(), vec!["is_published", "is_featured"]);
    
    // OLD API would require manual work:
    // .column("is_published", "INTEGER").default("0")  // Error-prone!
    // .column("is_featured", "INTEGER").default("1")   // No type safety!
}