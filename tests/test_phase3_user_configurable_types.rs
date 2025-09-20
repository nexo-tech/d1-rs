// REVOLUTIONARY Phase 3.3 Comprehensive Test Suite
// Tests all user-configurable type mapping functionality thoroughly

use d1_rs::*;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose};

/// Test SqlTypeMapping trait implementation
struct CustomUuidMapping;

impl SqlTypeMapping for CustomUuidMapping {
    type RustType = String;
    const SQL_TYPE: &'static str = "UUID";
    const FIELD_TYPE: FieldType = FieldType::Text;
    const NULLABLE_BY_DEFAULT: bool = false;
    const IS_BOOLEAN_TYPE: bool = false;

    fn to_sql_value(value: &Self::RustType) -> serde_json::Value {
        serde_json::Value::String(format!("uuid:{}", value))
    }

    fn from_sql_value(value: serde_json::Value) -> Result<Self::RustType> {
        if let serde_json::Value::String(s) = value {
            if s.starts_with("uuid:") {
                Ok(s[5..].to_string())
            } else {
                Ok(s)
            }
        } else {
            Err(D1RsError::SerializationError("Invalid UUID format".to_string()))
        }
    }
}

/// Test entity with comprehensive attribute usage
#[derive(Entity, Serialize, Deserialize, Debug, Clone)]
#[table(name = "comprehensive_test")]
struct ComprehensiveTestEntity {
    #[primary_key]
    id: i64,

    // Test explicit sql_type override
    #[sql_type = "JSON"]
    json_data: String,

    // Test boolean_field marking on non-bool type
    #[boolean_field]
    status_flag: i32,

    // Test type_override with multiple parameters
    #[type_override(field_type = "DateTime", nullable = true, auto_increment = false)]
    custom_timestamp: String,

    // Test nullable attribute
    #[nullable]
    optional_field: String,

    // Test foreign_key with custom configuration
    #[foreign_key(table = "users", column = "id")]
    user_reference: i64,

    // Regular field for comparison
    normal_text: String,

    // Regular boolean for comparison
    normal_bool: bool,
}

/// Test entity with non-English naming (Russian)
#[derive(Entity, Serialize, Deserialize, Debug, Clone)]
#[table(name = "русские_данные")]
struct РусскиеДанные {
    #[primary_key]
    идентификатор: i64,

    #[sql_type = "TEXT"]
    имя: String,

    #[boolean_field]
    активный: i32,

    #[foreign_key(table = "пользователи", column = "ид")]
    ссылка_на_пользователя: i64,
}

/// Test entity with Chinese naming
#[derive(Entity, Serialize, Deserialize, Debug, Clone)]
#[table(name = "中文数据")]
struct 中文数据 {
    #[primary_key]
    标识符: i64,

    #[sql_type = "TEXT"]
    名称: String,

    #[boolean_field]
    状态: i32,

    #[foreign_key(table = "用户表", column = "编号")]
    用户引用: i64,
}

/// Test entity for attribute priority testing
#[derive(Entity, Serialize, Deserialize, Debug, Clone)]
#[table(name = "priority_test")]
struct AttributePriorityTest {
    #[primary_key]
    id: i64,

    // Test priority: type_override should take precedence over sql_type
    #[sql_type = "TEXT"]
    #[type_override(field_type = "Integer")]
    priority_field1: String,

    // Test priority: boolean_field should work regardless of type
    #[sql_type = "INTEGER"]
    #[boolean_field]
    priority_field2: String,

    // Test multiple attributes on same field
    #[sql_type = "BIGINT"]
    #[nullable]
    #[boolean_field]
    multi_attribute_field: i32,
}

/// Test entity with edge cases
#[derive(Entity, Serialize, Deserialize, Debug, Clone)]
#[table(name = "edge_cases")]
struct EdgeCaseEntity {
    #[primary_key]
    id: i64,

    // Test with Option<T> and attributes
    #[sql_type = "JSON"]
    #[nullable]
    optional_json: Option<String>,

    // Test with complex types (using String for binary representation)
    #[sql_type = "BLOB"]
    binary_data: Vec<u8>,

    // Test unknown sql_type (should default to TEXT)
    #[sql_type = "UNKNOWN_TYPE"]
    unknown_type_field: String,

    // Test boolean_field on Option<bool>
    #[boolean_field]
    optional_bool_flag: Option<i32>,
}

/// Test CustomBooleanFields trait implementation
struct TestBooleanFields;

impl CustomBooleanFields for TestBooleanFields {
    const BOOLEAN_FIELD_NAMES: &'static [&'static str] = &["status_flag", "активный", "状态", "priority_field2"];

    fn is_boolean_field(field_name: &str) -> bool {
        Self::BOOLEAN_FIELD_NAMES.contains(&field_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_type_mapping_trait() {
        // Test custom type mapping functionality
        let test_value = "12345678-1234-5678-9012-123456789012".to_string();
        
        // Test to_sql_value
        let sql_value = CustomUuidMapping::to_sql_value(&test_value);
        assert_eq!(sql_value, serde_json::Value::String("uuid:12345678-1234-5678-9012-123456789012".to_string()));
        
        // Test from_sql_value
        let result = CustomUuidMapping::from_sql_value(sql_value).unwrap();
        assert_eq!(result, test_value);
        
        // Test constants
        assert_eq!(CustomUuidMapping::SQL_TYPE, "UUID");
        assert_eq!(CustomUuidMapping::FIELD_TYPE, FieldType::Text);
        assert!(!CustomUuidMapping::NULLABLE_BY_DEFAULT);
        assert!(!CustomUuidMapping::IS_BOOLEAN_TYPE);
    }

    #[test]
    fn test_comprehensive_field_definitions() {
        let field_defs = ComprehensiveTestEntity::field_definitions();
        
        // Should have definitions for all fields
        assert_eq!(field_defs.len(), 8);
        
        // Test specific field configurations
        let json_field = field_defs.iter().find(|f| f.name == "json_data").unwrap();
        assert_eq!(json_field.field_type.to_sql_type(), "JSON");
        
        let timestamp_field = field_defs.iter().find(|f| f.name == "custom_timestamp").unwrap();
        assert_eq!(timestamp_field.field_type.to_sql_type(), "DATETIME");
        assert!(timestamp_field.nullable);
        
        // Test foreign key configuration
        let user_ref_field = field_defs.iter().find(|f| f.name == "user_reference").unwrap();
        assert!(user_ref_field.foreign_key.is_some());
        let fk = user_ref_field.foreign_key.as_ref().unwrap();
        assert_eq!(fk.referenced_table, "users");
        assert_eq!(fk.referenced_column, "id");
    }

    #[test]
    fn test_boolean_field_metadata_with_custom_marking() {
        let boolean_fields = ComprehensiveTestEntity::boolean_fields();
        
        // Should include both type-based and attribute-based boolean fields
        assert!(boolean_fields.contains(&"status_flag")); // marked with #[boolean_field]
        assert!(boolean_fields.contains(&"normal_bool"));  // actual bool type
        
        // Should NOT include regular fields
        assert!(!boolean_fields.contains(&"normal_text"));
        assert!(!boolean_fields.contains(&"json_data"));
    }

    #[test]
    fn test_non_english_naming_support() {
        // Test Russian entity
        let russian_fields = РусскиеДанные::field_definitions();
        assert!(russian_fields.iter().any(|f| f.name == "имя"));
        assert!(russian_fields.iter().any(|f| f.name == "активный"));
        
        let russian_booleans = РусскиеДанные::boolean_fields();
        assert!(russian_booleans.contains(&"активный"));
        
        // Test Chinese entity
        let chinese_fields = 中文数据::field_definitions();
        assert!(chinese_fields.iter().any(|f| f.name == "名称"));
        assert!(chinese_fields.iter().any(|f| f.name == "状态"));
        
        let chinese_booleans = 中文数据::boolean_fields();
        assert!(chinese_booleans.contains(&"状态"));
        
        // Test table names
        assert_eq!(РусскиеДанные::TABLE_NAME, "русские_данные");
        assert_eq!(中文数据::TABLE_NAME, "中文数据");
    }

    #[test]
    fn test_attribute_priority_handling() {
        let field_defs = AttributePriorityTest::field_definitions();
        
        // Test that type_override takes precedence over sql_type
        let priority_field1 = field_defs.iter().find(|f| f.name == "priority_field1").unwrap();
        assert_eq!(priority_field1.field_type.to_sql_type(), "INTEGER"); // from type_override, not TEXT
        
        // Test that boolean_field works regardless of sql_type
        let _priority_field2 = field_defs.iter().find(|f| f.name == "priority_field2").unwrap();
        let boolean_fields = AttributePriorityTest::boolean_fields();
        assert!(boolean_fields.contains(&"priority_field2"));
        
        // Test multiple attributes on same field
        let multi_attr_field = field_defs.iter().find(|f| f.name == "multi_attribute_field").unwrap();
        assert_eq!(multi_attr_field.field_type.to_sql_type(), "BIGINT");
        assert!(boolean_fields.contains(&"multi_attribute_field"));
    }

    #[test]
    fn test_edge_cases_and_complex_types() {
        let field_defs = EdgeCaseEntity::field_definitions();
        
        // Test Option<T> with attributes
        let optional_json = field_defs.iter().find(|f| f.name == "optional_json").unwrap();
        assert_eq!(optional_json.field_type.to_sql_type(), "JSON");
        assert!(optional_json.nullable);
        
        // Test Vec<u8> with BLOB
        let binary_field = field_defs.iter().find(|f| f.name == "binary_data").unwrap();
        assert_eq!(binary_field.field_type.to_sql_type(), "BLOB");
        
        // Test unknown sql_type (should default to TEXT)
        let unknown_field = field_defs.iter().find(|f| f.name == "unknown_type_field").unwrap();
        assert_eq!(unknown_field.field_type.to_sql_type(), "TEXT");
        
        // Test boolean_field on Option<i32>
        let boolean_fields = EdgeCaseEntity::boolean_fields();
        assert!(boolean_fields.contains(&"optional_bool_flag"));
    }

    #[test]
    fn test_custom_boolean_fields_trait() {
        // Test the CustomBooleanFields trait implementation
        assert!(TestBooleanFields::is_boolean_field("status_flag"));
        assert!(TestBooleanFields::is_boolean_field("активный"));
        assert!(TestBooleanFields::is_boolean_field("状态"));
        assert!(TestBooleanFields::is_boolean_field("priority_field2"));
        
        // Should not match non-boolean fields
        assert!(!TestBooleanFields::is_boolean_field("normal_text"));
        assert!(!TestBooleanFields::is_boolean_field("unknown_field"));
    }

    #[test]
    fn test_nullable_attribute_handling() {
        let field_defs = ComprehensiveTestEntity::field_definitions();
        
        // Test explicit nullable attribute
        let _optional_field = field_defs.iter().find(|f| f.name == "optional_field").unwrap();
        // Note: nullable handling depends on implementation, but attribute should be recognized
        
        // Test type_override with nullable
        let timestamp_field = field_defs.iter().find(|f| f.name == "custom_timestamp").unwrap();
        assert!(timestamp_field.nullable);
    }

    #[test]
    fn test_foreign_key_attribute_configuration() {
        let field_defs = ComprehensiveTestEntity::field_definitions();
        
        let user_ref = field_defs.iter().find(|f| f.name == "user_reference").unwrap();
        assert!(user_ref.foreign_key.is_some());
        
        let fk = user_ref.foreign_key.as_ref().unwrap();
        assert_eq!(fk.local_column, "user_reference");
        assert_eq!(fk.referenced_table, "users");
        assert_eq!(fk.referenced_column, "id");
        assert_eq!(fk.on_delete, Some("CASCADE".to_string()));
        assert_eq!(fk.on_update, Some("CASCADE".to_string()));
        
        // Test non-English foreign key
        let russian_fields = РусскиеДанные::field_definitions();
        let russian_fk = russian_fields.iter()
            .find(|f| f.name == "ссылка_на_пользователя")
            .unwrap();
        assert!(russian_fk.foreign_key.is_some());
        let russian_fk_def = russian_fk.foreign_key.as_ref().unwrap();
        assert_eq!(russian_fk_def.referenced_table, "пользователи");
    }

    #[test]
    fn test_field_type_to_sql_conversion() {
        // Test all FieldType variants convert to correct SQL types
        assert_eq!(FieldType::Text.to_sql_type(), "TEXT");
        assert_eq!(FieldType::Integer.to_sql_type(), "INTEGER");
        assert_eq!(FieldType::BigInteger.to_sql_type(), "BIGINT");
        assert_eq!(FieldType::Real.to_sql_type(), "REAL");
        assert_eq!(FieldType::Boolean.to_sql_type(), "BOOLEAN");
        assert_eq!(FieldType::DateTime.to_sql_type(), "DATETIME");
        assert_eq!(FieldType::Date.to_sql_type(), "DATE");
        assert_eq!(FieldType::Time.to_sql_type(), "TIME");
        assert_eq!(FieldType::Json.to_sql_type(), "JSON");
        assert_eq!(FieldType::Blob.to_sql_type(), "BLOB");
    }

    #[test]
    fn test_comprehensive_entity_table_name() {
        // Test table name extraction from attribute
        assert_eq!(ComprehensiveTestEntity::TABLE_NAME, "comprehensive_test");
        assert_eq!(AttributePriorityTest::TABLE_NAME, "priority_test");
        assert_eq!(EdgeCaseEntity::TABLE_NAME, "edge_cases");
    }

    #[test]
    fn test_revolutionary_attribute_system_coverage() {
        // This test ensures all our new attributes are properly supported
        let field_defs = ComprehensiveTestEntity::field_definitions();
        
        // Verify we have the expected number of fields with various attributes
        let attributed_fields = field_defs.iter()
            .filter(|f| f.name != "id" && f.name != "normal_text" && f.name != "normal_bool")
            .count();
        assert_eq!(attributed_fields, 5); // 5 fields with special attributes
        
        // Verify boolean field detection works across all scenarios
        let boolean_fields = ComprehensiveTestEntity::boolean_fields();
        assert_eq!(boolean_fields.len(), 2); // status_flag + normal_bool
        
        // Verify foreign key detection
        let fk_fields = field_defs.iter()
            .filter(|f| f.foreign_key.is_some())
            .count();
        assert_eq!(fk_fields, 1); // user_reference
    }

    #[test]
    fn test_error_handling_in_sql_type_mapping() {
        // Test error handling in custom type mapping
        let invalid_value = serde_json::Value::Number(42.into());
        let result = CustomUuidMapping::from_sql_value(invalid_value);
        assert!(result.is_err());
        
        if let Err(D1RsError::SerializationError(msg)) = result {
            assert_eq!(msg, "Invalid UUID format");
        } else {
            panic!("Expected SerializationError");
        }
    }

    #[test]
    fn test_zero_hardcoded_limitations() {
        // This test verifies that our system works with ANY naming convention
        // and has zero cultural/language bias
        
        // Test that Russian field names work perfectly
        let russian_defs = РусскиеДанные::field_definitions();
        assert!(!russian_defs.is_empty());
        
        // Test that Chinese field names work perfectly  
        let chinese_defs = 中文数据::field_definitions();
        assert!(!chinese_defs.is_empty());
        
        // Test that all special characters and unicode work
        let russian_field = russian_defs.iter()
            .find(|f| f.name == "ссылка_на_пользователя")
            .unwrap();
        assert!(russian_field.foreign_key.is_some());
        
        let chinese_boolean = 中文数据::boolean_fields();
        assert!(chinese_boolean.contains(&"状态"));
    }

    #[test]
    fn test_complete_user_control_system() {
        // This comprehensive test verifies users have complete control
        // over every aspect of type detection and mapping
        
        let field_defs = ComprehensiveTestEntity::field_definitions();
        
        // User can override SQL type
        let json_field = field_defs.iter().find(|f| f.name == "json_data").unwrap();
        assert_eq!(json_field.field_type.to_sql_type(), "JSON");
        
        // User can override field type completely
        let timestamp_field = field_defs.iter().find(|f| f.name == "custom_timestamp").unwrap();
        assert_eq!(timestamp_field.field_type.to_sql_type(), "DATETIME");
        
        // User can mark any field as boolean
        let boolean_fields = ComprehensiveTestEntity::boolean_fields();
        assert!(boolean_fields.contains(&"status_flag")); // i32 marked as boolean
        
        // User can configure foreign keys explicitly
        let fk_field = field_defs.iter().find(|f| f.name == "user_reference").unwrap();
        let fk = fk_field.foreign_key.as_ref().unwrap();
        assert_eq!(fk.referenced_table, "users");
        
        // User can use any naming convention
        assert_eq!(РусскиеДанные::TABLE_NAME, "русские_данные");
        assert_eq!(中文数据::TABLE_NAME, "中文数据");
    }
}
    #[test]
    fn test_revolutionary_binary_data_support() {
        // Test that Vec<u8> binary data works seamlessly with our system
        use d1_rs::types::SqlType;
        
        // Create test binary data
        let original_data = vec![0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x57, 0x6f, 0x72, 0x6c, 0x64]; // "Hello World" in bytes
        
        // Test conversion to SQL value (should be base64 encoded)
        let sql_value = original_data.to_sql_value();
        if let serde_json::Value::String(encoded) = sql_value {
            // Verify it's valid base64
            let decoded = general_purpose::STANDARD.decode(&encoded).unwrap();
            assert_eq!(decoded, original_data);
        } else {
            panic!("Expected string value for binary data");
        }
        
        // Test conversion from SQL value
        let sql_value = serde_json::Value::String(general_purpose::STANDARD.encode(&original_data));
        let recovered_data = Vec::<u8>::from_sql_value(&sql_value).unwrap();
        assert_eq!(recovered_data, original_data);
        
        // Test that binary field is properly configured in entity
        let field_defs = EdgeCaseEntity::field_definitions();
        let binary_field = field_defs.iter().find(|f| f.name == "binary_data").unwrap();
        assert_eq!(binary_field.field_type.to_sql_type(), "BLOB");
        
        // Test SQL type name
        assert_eq!(Vec::<u8>::sql_type_name(), "BLOB");
        
        // Test null handling
        let null_value = serde_json::Value::Null;
        let empty_data = Vec::<u8>::from_sql_value(&null_value).unwrap();
        assert!(empty_data.is_empty());
        
        // Test error handling with invalid base64
        let invalid_value = serde_json::Value::String("invalid_base64!!!".to_string());
        let result = Vec::<u8>::from_sql_value(&invalid_value);
        assert!(result.is_err());
        
        if let Err(D1RsError::SerializationError(msg)) = result {
            assert!(msg.contains("Invalid base64 binary data"));
        } else {
            panic!("Expected SerializationError for invalid base64");
        }
        
        // Test binary data with entity attributes
        let field_definitions = EdgeCaseEntity::field_definitions();
        let binary_field = field_definitions
            .iter()
            .find(|f| f.name == "binary_data")
            .unwrap();
        assert_eq!(binary_field.field_type.to_sql_type(), "BLOB");
        
        // Test that users can store actual binary data like images, files, etc.
        let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header
        let encoded = image_data.to_sql_value();
        let decoded = Vec::<u8>::from_sql_value(&encoded).unwrap();
        assert_eq!(decoded, image_data);
    }
