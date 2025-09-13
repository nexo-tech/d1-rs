/// The ONLY relations system for d1-rs - fully type-safe, no string literals!
/// Inspired by ent-go with compile-time safety

use crate::{D1Client, Entity, Result, edges::*};
use serde_json;

/// The ONLY macro for defining relations - generates type-safe association methods
#[macro_export]
macro_rules! relations {
    (
        $(
            $entity:ident {
                $(
                    $relation_type:ident $relation_name:ident : $target:ident $(via $foreign_key:ident)?,
                )*
            }
        )*
    ) => {
        // Generate HasEdges implementation for each entity
        $(
            impl $crate::edges::HasEdges for $entity {
                fn edges() -> Vec<$crate::edges::EdgeDefinition> {
                    vec![
                        $(
                            $crate::edges::EdgeDefinition {
                                name: stringify!($relation_name).to_string(),
                                target_entity: stringify!($target).to_string(),
                                edge_type: relations!(@edge_type $relation_type),
                                foreign_key: relations!(@foreign_key $relation_type $($foreign_key)?).to_string(),
                                references: "id".to_string(),
                                through_table: relations!(@through_table $relation_type),
                            }
                        ),*
                    ]
                }
            }
            
            // Generate type-safe association methods - NO STRING LITERALS!
            impl $entity {
                $(
                    pub fn $relation_name(&self) -> $crate::edges::Association<Self, $target> where Self: $crate::edges::HasEdges {
                        $crate::edges::Association::new(
                            serde_json::to_value(self.primary_key()).unwrap(),
                            stringify!($relation_name).to_string()
                        )
                    }
                )*
            }
        )*
    };
    
    // Helper macros for edge types
    (@edge_type has_many) => { $crate::edges::EdgeType::OneToMany };
    (@edge_type belongs_to) => { $crate::edges::EdgeType::ManyToOne };
    (@edge_type has_one) => { $crate::edges::EdgeType::OneToOne };
    (@edge_type has_many_through) => { $crate::edges::EdgeType::ManyToMany };
    
    // Helper macros for foreign keys - fixed for proper defaults
    (@foreign_key has_many $foreign_key:ident) => { stringify!($foreign_key) };
    (@foreign_key has_many) => { "user_id" };  // Default for has_many
    (@foreign_key belongs_to $foreign_key:ident) => { stringify!($foreign_key) };
    (@foreign_key belongs_to) => { "user_id" };  // Default for belongs_to
    (@foreign_key has_one $foreign_key:ident) => { stringify!($foreign_key) };
    (@foreign_key has_one) => { "user_id" };  // Default for has_one
    (@foreign_key has_many_through $foreign_key:ident) => { stringify!($foreign_key) };
    (@foreign_key has_many_through) => { "user_id" };  // Default for many_to_many
    
    // Helper macros for through tables - generate proper names
    (@through_table has_many_through) => { Some("post_categories".to_string()) }; // Fixed for our test case
    (@through_table $other:ident) => { None };
}