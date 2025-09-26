/// Test entities for fixture system
/// 
/// These entities are used for testing and provide known types and relationships
/// for cross-database compatibility testing.

use serde::{Deserialize, Serialize};

/// User entity for relationship testing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestUser {
    pub id: Option<i64>,
    pub name: String,
    pub email: String,
    pub created_at: String, // String format for cross-database compatibility
}

impl TestUser {
    #[allow(dead_code)]
    pub fn new(name: String, email: String) -> Self {
        Self {
            id: None,
            name,
            email,
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
    
    pub fn john_doe() -> Self {
        Self {
            id: Some(1),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            created_at: "2024-01-01 00:00:00".to_string(),
        }
    }
    
    pub fn jane_smith() -> Self {
        Self {
            id: Some(2),
            name: "Jane Smith".to_string(),
            email: "jane@example.com".to_string(),
            created_at: "2024-01-02 00:00:00".to_string(),
        }
    }
}

/// Post entity for relationship testing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestPost {
    pub id: Option<i64>,
    pub title: String,
    pub content: Option<String>,
    pub user_id: i64,
    pub created_at: String,
}

impl TestPost {
    #[allow(dead_code)]
    pub fn new(title: String, content: Option<String>, user_id: i64) -> Self {
        Self {
            id: None,
            title,
            content,
            user_id,
            created_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
    
    pub fn first_post() -> Self {
        Self {
            id: Some(1),
            title: "First Post".to_string(),
            content: Some("Hello World".to_string()),
            user_id: 1,
            created_at: "2024-01-03 00:00:00".to_string(),
        }
    }
}

/// Type test entity for cross-database type compatibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestTypeEntity {
    pub id: Option<i64>,
    pub text_col: Option<String>,
    pub integer_col: Option<i64>,
    pub real_col: Option<f64>,
    pub boolean_col: Option<bool>,
    pub date_col: Option<String>,
    pub datetime_col: Option<String>,
    // Database-specific fields
    pub json_col: Option<serde_json::Value>,
    pub uuid_col: Option<String>,
}

impl TestTypeEntity {
    pub fn sample() -> Self {
        Self {
            id: Some(1),
            text_col: Some("test text".to_string()),
            integer_col: Some(42),
            real_col: Some(3.14),
            boolean_col: Some(true),
            date_col: Some("2024-01-01".to_string()),
            datetime_col: Some("2024-01-01 12:00:00".to_string()),
            json_col: Some(serde_json::json!({"key": "value"})),
            uuid_col: Some("123e4567-e89b-12d3-a456-426614174000".to_string()),
        }
    }
    
    pub fn null_values() -> Self {
        Self {
            id: Some(2),
            text_col: None,
            integer_col: None,
            real_col: None,
            boolean_col: None,
            date_col: None,
            datetime_col: None,
            json_col: None,
            uuid_col: None,
        }
    }
}

/// Performance test entity for large dataset testing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestPerformanceEntity {
    pub id: Option<i64>,
    pub data: String,
    pub value: i64,
    pub created_at: Option<String>,
}

impl TestPerformanceEntity {
    pub fn new(data: String, value: i64) -> Self {
        Self {
            id: None,
            data,
            value,
            created_at: Some(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()),
        }
    }
    
    pub fn generate_batch(count: usize) -> Vec<Self> {
        (0..count).map(|i| {
            Self::new(
                format!("performance data {}", i),
                (i % 1000) as i64
            )
        }).collect()
    }
}

/// Entity relationship helpers
impl TestUser {
    /// Get all posts for this user (simulated)
    #[allow(dead_code)]
    pub fn sample_posts(&self) -> Vec<TestPost> {
        if let Some(user_id) = self.id {
            match user_id {
                1 => vec![TestPost::first_post()],
                _ => vec![],
            }
        } else {
            vec![]
        }
    }
}

impl TestPost {
    /// Get the user who created this post (simulated)
    #[allow(dead_code)]
    pub fn sample_user(&self) -> Option<TestUser> {
        match self.user_id {
            1 => Some(TestUser::john_doe()),
            2 => Some(TestUser::jane_smith()),
            _ => None,
        }
    }
}

/// Validation helpers for test entities
pub trait TestEntityValidation {
    fn validate(&self) -> Result<(), String>;
}

impl TestEntityValidation for TestUser {
    fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }
        if !self.email.contains('@') {
            return Err("Email must contain @".to_string());
        }
        Ok(())
    }
}

impl TestEntityValidation for TestPost {
    fn validate(&self) -> Result<(), String> {
        if self.title.is_empty() {
            return Err("Title cannot be empty".to_string());
        }
        if self.user_id <= 0 {
            return Err("User ID must be positive".to_string());
        }
        Ok(())
    }
}

impl TestEntityValidation for TestPerformanceEntity {
    fn validate(&self) -> Result<(), String> {
        if self.data.is_empty() {
            return Err("Data cannot be empty".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_creation() {
        let user = TestUser::john_doe();
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");
        assert_eq!(user.id, Some(1));
        assert!(user.validate().is_ok());
    }
    
    #[test]
    fn test_post_creation() {
        let post = TestPost::first_post();
        assert_eq!(post.title, "First Post");
        assert_eq!(post.user_id, 1);
        assert_eq!(post.content, Some("Hello World".to_string()));
        assert!(post.validate().is_ok());
    }
    
    #[test]
    fn test_type_entity() {
        let entity = TestTypeEntity::sample();
        assert_eq!(entity.text_col, Some("test text".to_string()));
        assert_eq!(entity.integer_col, Some(42));
        assert_eq!(entity.boolean_col, Some(true));
    }
    
    #[test]
    fn test_performance_entity_batch() {
        let batch = TestPerformanceEntity::generate_batch(100);
        assert_eq!(batch.len(), 100);
        assert_eq!(batch[0].data, "performance data 0");
        assert_eq!(batch[99].data, "performance data 99");
    }
    
    #[test]
    fn test_user_validation() {
        let mut user = TestUser::john_doe();
        assert!(user.validate().is_ok());
        
        user.name = "".to_string();
        assert!(user.validate().is_err());
        
        user.name = "Test".to_string();
        user.email = "invalid".to_string();
        assert!(user.validate().is_err());
    }
    
    #[test]
    fn test_post_validation() {
        let mut post = TestPost::first_post();
        assert!(post.validate().is_ok());
        
        post.title = "".to_string();
        assert!(post.validate().is_err());
        
        post.title = "Test".to_string();
        post.user_id = -1;
        assert!(post.validate().is_err());
    }
}