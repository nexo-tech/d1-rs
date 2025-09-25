/// Pre-defined data sets for comprehensive testing
/// 
/// This module provides curated data sets for various testing scenarios,
/// including edge cases, performance testing, and cross-database compatibility.

use serde_json::{json, Value};
use std::collections::HashMap;
use super::entities::{TestUser, TestPost, TestTypeEntity, TestPerformanceEntity};

/// Data set collection for different testing scenarios
pub struct TestDataSets;

impl TestDataSets {
    /// Basic user and post data for relationship testing
    pub fn users_posts_basic() -> (Vec<TestUser>, Vec<TestPost>) {
        let users = vec![
            TestUser::john_doe(),
            TestUser::jane_smith(),
            TestUser {
                id: Some(3),
                name: "Alice Johnson".to_string(),
                email: "alice@example.com".to_string(),
                created_at: "2024-01-03 00:00:00".to_string(),
            },
        ];
        
        let posts = vec![
            TestPost::first_post(),
            TestPost {
                id: Some(2),
                title: "Second Post".to_string(),
                content: Some("More content here".to_string()),
                user_id: 1,
                created_at: "2024-01-04 00:00:00".to_string(),
            },
            TestPost {
                id: Some(3),
                title: "Jane's Post".to_string(),
                content: Some("Jane's thoughts".to_string()),
                user_id: 2,
                created_at: "2024-01-05 00:00:00".to_string(),
            },
        ];
        
        (users, posts)
    }
    
    /// Extended data set with edge cases
    pub fn users_posts_extended() -> (Vec<TestUser>, Vec<TestPost>) {
        let users = vec![
            TestUser::john_doe(),
            TestUser::jane_smith(),
            TestUser {
                id: Some(3),
                name: "O'Connor, Patrick".to_string(), // Special characters
                email: "patrick.oconnor+test@example.co.uk".to_string(), // Complex email
                created_at: "2024-01-03 12:30:45".to_string(),
            },
            TestUser {
                id: Some(4),
                name: "李小明".to_string(), // Unicode characters
                email: "xiaoming@example.cn".to_string(),
                created_at: "2024-01-04 00:00:00".to_string(),
            },
            TestUser {
                id: Some(5),
                name: "José María González-Martínez".to_string(), // Accented characters
                email: "jose.maria@ejemplo.es".to_string(),
                created_at: "2024-01-05 00:00:00".to_string(),
            },
        ];
        
        let posts = vec![
            TestPost::first_post(),
            TestPost {
                id: Some(2),
                title: "Post with emoji 🚀".to_string(),
                content: Some("Content with emoji 🌟 and symbols ∞ ™".to_string()),
                user_id: 1,
                created_at: "2024-01-04 00:00:00".to_string(),
            },
            TestPost {
                id: Some(3),
                title: "Very Long Title That Exceeds Normal Expectations And Tests Database Column Length Limits For Title Fields".to_string(),
                content: Some("A".repeat(10000)), // Large content
                user_id: 2,
                created_at: "2024-01-05 00:00:00".to_string(),
            },
            TestPost {
                id: Some(4),
                title: "Empty Content Post".to_string(),
                content: None, // NULL content
                user_id: 3,
                created_at: "2024-01-06 00:00:00".to_string(),
            },
            TestPost {
                id: Some(5),
                title: "中文标题".to_string(), // Chinese title
                content: Some("中文内容测试".to_string()), // Chinese content
                user_id: 4,
                created_at: "2024-01-07 00:00:00".to_string(),
            },
        ];
        
        (users, posts)
    }
    
    /// Comprehensive type testing data
    pub fn type_testing_comprehensive() -> Vec<TestTypeEntity> {
        vec![
            TestTypeEntity::sample(),
            TestTypeEntity::null_values(),
            TestTypeEntity {
                id: Some(3),
                text_col: Some("".to_string()), // Empty string
                integer_col: Some(0), // Zero value
                real_col: Some(0.0), // Zero float
                boolean_col: Some(false), // False boolean
                date_col: Some("1970-01-01".to_string()), // Epoch date
                datetime_col: Some("1970-01-01 00:00:00".to_string()), // Epoch datetime
                json_col: Some(json!({})), // Empty JSON object
                uuid_col: Some("00000000-0000-0000-0000-000000000000".to_string()), // Null UUID
            },
            TestTypeEntity {
                id: Some(4),
                text_col: Some("Special chars: 'quotes' \"double quotes\" \n\t\r".to_string()),
                integer_col: Some(i64::MAX), // Maximum integer
                real_col: Some(f64::MAX), // Maximum float
                boolean_col: Some(true),
                date_col: Some("9999-12-31".to_string()), // Far future date
                datetime_col: Some("9999-12-31 23:59:59".to_string()),
                json_col: Some(json!({
                    "nested": {
                        "array": [1, 2, 3],
                        "string": "value",
                        "boolean": true,
                        "null": null
                    }
                })), // Complex JSON
                uuid_col: Some("ffffffff-ffff-ffff-ffff-ffffffffffff".to_string()), // Max UUID
            },
            TestTypeEntity {
                id: Some(5),
                text_col: Some("Unicode: 🌟 🚀 ❤️ 中文 العربية Русский".to_string()),
                integer_col: Some(i64::MIN), // Minimum integer
                real_col: Some(-f64::MAX), // Negative maximum float
                boolean_col: Some(false),
                date_col: Some("0001-01-01".to_string()), // Very old date
                datetime_col: Some("0001-01-01 00:00:00".to_string()),
                json_col: Some(json!(["array", "with", "multiple", "types", 42, true, null])), // JSON array
                uuid_col: Some("123e4567-e89b-12d3-a456-426614174000".to_string()),
            },
        ]
    }
    
    /// Performance testing data generation
    pub fn performance_data_small() -> Vec<TestPerformanceEntity> {
        TestPerformanceEntity::generate_batch(1000)
    }
    
    pub fn performance_data_medium() -> Vec<TestPerformanceEntity> {
        TestPerformanceEntity::generate_batch(10000)
    }
    
    pub fn performance_data_large() -> Vec<TestPerformanceEntity> {
        TestPerformanceEntity::generate_batch(100000)
    }
    
    /// Generate performance data with specific patterns for testing
    pub fn performance_data_patterns() -> Vec<TestPerformanceEntity> {
        let mut data = Vec::new();
        
        // Sequential pattern
        for i in 0..1000 {
            data.push(TestPerformanceEntity {
                id: Some((i + 1) as i64),
                data: format!("sequential_{:04}", i),
                value: i as i64,
                created_at: Some(format!("2024-01-{:02} {:02}:00:00", 
                    (i % 30) + 1, 
                    i % 24
                )),
            });
        }
        
        // Random pattern
        for i in 1000..2000 {
            data.push(TestPerformanceEntity {
                id: Some((i + 1) as i64),
                data: format!("random_{}", i * 7 % 9999),
                value: (i * 17 % 1000) as i64,
                created_at: Some(format!("2024-02-{:02} {:02}:{:02}:00", 
                    (i * 3 % 28) + 1,
                    i % 24,
                    (i * 7) % 60
                )),
            });
        }
        
        // Clustered pattern
        for i in 2000..3000 {
            let cluster = i / 100;
            data.push(TestPerformanceEntity {
                id: Some((i + 1) as i64),
                data: format!("cluster_{}_{:03}", cluster, i % 100),
                value: cluster as i64,
                created_at: Some(format!("2024-03-{:02} 12:00:00", 
                    (cluster % 30) + 1
                )),
            });
        }
        
        data
    }
    
    /// Edge case data for stress testing
    pub fn edge_cases_data() -> HashMap<String, Vec<Value>> {
        let mut cases = HashMap::new();
        
        // Empty strings
        cases.insert("empty_strings".to_string(), vec![
            json!(""),
            json!("   "), // Whitespace only
            json!("\n\t\r"), // Control characters
        ]);
        
        // Extreme numbers
        cases.insert("extreme_numbers".to_string(), vec![
            json!(i64::MAX),
            json!(i64::MIN),
            json!(0),
            json!(-0),
            json!(f64::MAX),
            json!(f64::MIN),
            json!(f64::INFINITY),
            json!(-f64::INFINITY),
            // Note: NaN is not supported in JSON
        ]);
        
        // Special characters and encoding
        cases.insert("special_characters".to_string(), vec![
            json!("'single quotes'"),
            json!("\"double quotes\""),
            json!("backslash\\test"),
            json!("forward/slash"),
            json!("percent%test"),
            json!("underscore_test"),
            json!("dash-test"),
            json!("dot.test"),
            json!("at@test"),
            json!("hash#test"),
            json!("dollar$test"),
            json!("pipe|test"),
            json!("question?test"),
            json!("exclamation!test"),
        ]);
        
        // Unicode and international characters
        cases.insert("unicode_characters".to_string(), vec![
            json!("🚀🌟❤️🎉"), // Emoji
            json!("中文测试"), // Chinese
            json!("العربية"), // Arabic
            json!("Русский"), // Russian
            json!("Français"), // French with accents
            json!("Español"), // Spanish with accents
            json!("Deutsch"), // German
            json!("日本語"), // Japanese
            json!("한국어"), // Korean
            json!("हिन्दी"), // Hindi
        ]);
        
        // Date and time edge cases
        cases.insert("date_edge_cases".to_string(), vec![
            json!("1970-01-01"), // Unix epoch
            json!("2000-01-01"), // Y2K
            json!("2038-01-19"), // 32-bit timestamp limit
            json!("0001-01-01"), // Very old date
            json!("9999-12-31"), // Far future date
            json!("2024-02-29"), // Leap year
            json!("2024-12-31 23:59:59"), // End of year
            json!("2024-01-01 00:00:00"), // Start of year
        ]);
        
        // JSON data variations
        cases.insert("json_variations".to_string(), vec![
            json!({}), // Empty object
            json!([]), // Empty array
            json!(null), // Null value
            json!({ "key": null }), // Object with null
            json!([null, null]), // Array with nulls
            json!({ "nested": { "deep": { "very": "deep" } } }), // Deep nesting
            json!([1, "two", true, null, { "five": 5 }]), // Mixed array
        ]);
        
        cases
    }
    
    /// SQL injection test cases (should be safely handled)
    pub fn sql_injection_tests() -> Vec<String> {
        vec![
            "'; DROP TABLE users; --".to_string(),
            "' OR '1'='1".to_string(),
            "'; DELETE FROM posts WHERE 1=1; --".to_string(),
            "' UNION SELECT * FROM users --".to_string(),
            "'; INSERT INTO users (name, email) VALUES ('hacker', 'hack@evil.com'); --".to_string(),
            "admin'--".to_string(),
            "admin'/*".to_string(),
            "' OR 1=1#".to_string(),
            "' OR 'a'='a".to_string(),
            "') OR ('1'='1".to_string(),
            "'; EXEC xp_cmdshell('dir'); --".to_string(),
            "' AND (SELECT COUNT(*) FROM users) > 0 --".to_string(),
        ]
    }
    
    /// Performance benchmark datasets with known characteristics
    pub fn benchmark_datasets() -> HashMap<String, Vec<TestPerformanceEntity>> {
        let mut datasets = HashMap::new();
        
        // Sorted data
        let mut sorted_data = Vec::new();
        for i in 0..5000 {
            sorted_data.push(TestPerformanceEntity {
                id: Some(i as i64),
                data: format!("sorted_{:05}", i),
                value: i as i64,
                created_at: Some(format!("2024-01-01 {:02}:00:00", i % 24)),
            });
        }
        datasets.insert("sorted".to_string(), sorted_data);
        
        // Reverse sorted data
        let mut reverse_data = Vec::new();
        for i in 0..5000 {
            reverse_data.push(TestPerformanceEntity {
                id: Some(i as i64),
                data: format!("reverse_{:05}", 5000 - i),
                value: (5000 - i) as i64,
                created_at: Some(format!("2024-01-01 {:02}:00:00", (24 - (i % 24)) % 24)),
            });
        }
        datasets.insert("reverse".to_string(), reverse_data);
        
        // Random data (pseudo-random for reproducibility)
        let mut random_data = Vec::new();
        for i in 0..5000 {
            let pseudo_random = (i * 17 + 7) % 9973; // Simple pseudo-random
            random_data.push(TestPerformanceEntity {
                id: Some(i as i64),
                data: format!("random_{:05}", pseudo_random),
                value: pseudo_random as i64,
                created_at: Some(format!("2024-{:02}-{:02} {:02}:{:02}:{:02}", 
                    (pseudo_random % 12) + 1,
                    (pseudo_random % 28) + 1,
                    pseudo_random % 24,
                    (pseudo_random / 24) % 60,
                    (pseudo_random / 1440) % 60
                )),
            });
        }
        datasets.insert("random".to_string(), random_data);
        
        datasets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_users_posts_basic() {
        let (users, posts) = TestDataSets::users_posts_basic();
        assert_eq!(users.len(), 3);
        assert_eq!(posts.len(), 3);
        
        // Verify referential integrity
        let user_ids: Vec<i64> = users.iter().filter_map(|u| u.id).collect();
        for post in &posts {
            assert!(user_ids.contains(&post.user_id));
        }
    }
    
    #[test]
    fn test_users_posts_extended() {
        let (users, posts) = TestDataSets::users_posts_extended();
        assert_eq!(users.len(), 5);
        assert_eq!(posts.len(), 5);
        
        // Test special characters
        let special_user = users.iter().find(|u| u.name.contains("O'Connor")).unwrap();
        assert!(special_user.email.contains("+test"));
        
        // Test unicode
        let unicode_user = users.iter().find(|u| u.name.contains("李")).unwrap();
        assert!(unicode_user.email.contains(".cn"));
    }
    
    #[test]
    fn test_type_testing_comprehensive() {
        let entities = TestDataSets::type_testing_comprehensive();
        assert_eq!(entities.len(), 5);
        
        // Test null values
        let null_entity = entities.iter().find(|e| e.id == Some(2)).unwrap();
        assert!(null_entity.text_col.is_none());
        
        // Test extreme values
        let extreme_entity = entities.iter().find(|e| e.id == Some(4)).unwrap();
        assert_eq!(extreme_entity.integer_col, Some(i64::MAX));
        assert_eq!(extreme_entity.real_col, Some(f64::MAX));
    }
    
    #[test]
    fn test_performance_data_generation() {
        let small = TestDataSets::performance_data_small();
        let medium = TestDataSets::performance_data_medium();
        let large = TestDataSets::performance_data_large();
        
        assert_eq!(small.len(), 1000);
        assert_eq!(medium.len(), 10000);
        assert_eq!(large.len(), 100000);
    }
    
    #[test]
    fn test_performance_data_patterns() {
        let patterns = TestDataSets::performance_data_patterns();
        assert_eq!(patterns.len(), 3000);
        
        // Test sequential pattern
        let sequential = &patterns[0..1000];
        for (i, entity) in sequential.iter().enumerate() {
            assert_eq!(entity.value, i as i64);
            assert!(entity.data.starts_with("sequential_"));
        }
        
        // Test clustered pattern
        let clustered = &patterns[2000..2100];
        for entity in clustered {
            assert!(entity.data.starts_with("cluster_20_"));
        }
    }
    
    #[test]
    fn test_edge_cases_data() {
        let cases = TestDataSets::edge_cases_data();
        
        assert!(cases.contains_key("empty_strings"));
        assert!(cases.contains_key("extreme_numbers"));
        assert!(cases.contains_key("special_characters"));
        assert!(cases.contains_key("unicode_characters"));
        assert!(cases.contains_key("date_edge_cases"));
        assert!(cases.contains_key("json_variations"));
        
        let unicode = &cases["unicode_characters"];
        assert!(unicode.iter().any(|v| v.as_str().unwrap().contains("🚀")));
    }
    
    #[test]
    fn test_sql_injection_tests() {
        let injections = TestDataSets::sql_injection_tests();
        assert!(!injections.is_empty());
        
        // Should contain common injection patterns
        assert!(injections.iter().any(|s| s.contains("DROP TABLE")));
        assert!(injections.iter().any(|s| s.contains("OR '1'='1")));
        assert!(injections.iter().any(|s| s.contains("UNION SELECT")));
    }
    
    #[test]
    fn test_benchmark_datasets() {
        let datasets = TestDataSets::benchmark_datasets();
        
        assert!(datasets.contains_key("sorted"));
        assert!(datasets.contains_key("reverse"));
        assert!(datasets.contains_key("random"));
        
        let sorted = &datasets["sorted"];
        let reverse = &datasets["reverse"];
        
        // Verify sorting
        for i in 1..sorted.len() {
            assert!(sorted[i-1].value <= sorted[i].value);
        }
        
        for i in 1..reverse.len() {
            assert!(reverse[i-1].value >= reverse[i].value);
        }
    }
}