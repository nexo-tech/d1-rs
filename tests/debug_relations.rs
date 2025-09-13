// Debug relations macro to isolate the issue
use chrono::{DateTime, Utc};
use d1_rs::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
}

// Try creating a simple version without optional parameters
macro_rules! simple_relations {
    (
        $(
            $entity:ident {
                $(
                    $relation_type:ident $relation_name:ident : $target:ident via $foreign_key:ident,
                )*
            }
        )*
    ) => {
        $(
            impl $crate::edges::HasEdges for $entity {
                fn edges() -> Vec<$crate::edges::EdgeDefinition> {
                    vec![
                        $(
                            $crate::edges::EdgeDefinition {
                                name: stringify!($relation_name).to_string(),
                                target_entity: stringify!($target).to_string(),
                                edge_type: $crate::edges::EdgeType::OneToMany,
                                foreign_key: stringify!($foreign_key).to_string(),
                                references: "id".to_string(),
                                through_table: None,
                                config: $crate::edges::EdgeConfig::default(),
                            }
                        ),*
                    ]
                }
            }
            
            impl $entity {
                $(
                    pub fn $relation_name(&self) -> $crate::edges::Association<Self, $target> where Self: $crate::edges::HasEdges {
                        $crate::edges::Association::new(
                            ::serde_json::to_value(self.primary_key()).unwrap(),
                            stringify!($relation_name).to_string()
                        )
                    }
                )*
            }
        )*
    };
}

// Try the simple version first
simple_relations! {
    User {
        has_many posts: Post via user_id,
    }
    
    Post {
        belongs_to user: User via user_id,
    }
}

#[tokio::test]
async fn test_basic_relations_macro() {
    // Test if the basic macro generates the HasEdges trait
    let user_edges = User::edges();
    assert_eq!(user_edges.len(), 1);
    assert_eq!(user_edges[0].name, "posts");
    
    println!("✅ HasEdges trait is working!");
    
    // Test if association methods are generated
    let user = User {
        id: 1,
        name: "test".to_string(),
        email: "test@example.com".to_string(),
        created_at: chrono::Utc::now(),
    };
    
    // This should work if the macro is generating properly
    let _association = user.posts();
    println!("✅ Association method is working!");
}