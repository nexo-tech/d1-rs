/// The ONLY relations system for d1-rs - fully type-safe, no string literals!
/// Inspired by ent-go with compile-time safety

use crate::{D1Client, Entity, Result, edges::*};
use serde_json;

/// The ONLY macro for defining relations - generates type-safe association methods
/// Now supports automatic M2M detection for clean APIs like ent-go!
#[macro_export]
macro_rules! relations {
    (
        $(
            $entity:ident {
                $(
                    $relation_type:ident $relation_name:ident : $target:ident $(via $foreign_key:ident)? $(through $edge_schema:ident)?,
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
                                edge_type: relations!(@edge_type $relation_type $entity $target $(through $edge_schema)?),
                                foreign_key: relations!(@foreign_key $relation_type $entity $target $($foreign_key)?),
                                references: "id".to_string(),
                                through_table: relations!(@through_table $relation_type $entity $target $(through $edge_schema)?),
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
    
    // Helper macros for edge types - with proper backward compatibility!
    (@edge_type has_many $entity:ident $target:ident through $edge_schema:ident) => { $crate::edges::EdgeType::ManyToMany };
    (@edge_type has_many $entity:ident $target:ident) => { $crate::edges::EdgeType::OneToMany }; // Default to O2M for backward compatibility
    (@edge_type belongs_to $entity:ident $target:ident $($through:ident $edge_schema:ident)?) => { $crate::edges::EdgeType::ManyToOne };
    (@edge_type has_one $entity:ident $target:ident $($through:ident $edge_schema:ident)?) => { $crate::edges::EdgeType::OneToOne };
    (@edge_type has_many_through $entity:ident $target:ident $($through:ident $edge_schema:ident)?) => { $crate::edges::EdgeType::ManyToMany };
    (@edge_type one_to_many $entity:ident $target:ident $($through:ident $edge_schema:ident)?) => { $crate::edges::EdgeType::OneToMany }; // Explicit O2M
    
    // Helper macros for foreign keys - handle explicit via clauses properly!
    (@foreign_key has_many $entity:ident $target:ident through $edge_schema:ident $foreign_key:ident) => { stringify!($foreign_key).to_string() };
    (@foreign_key has_many $entity:ident $target:ident through $edge_schema:ident) => { "post_id".to_string() }; // Default for M2M
    (@foreign_key has_many $entity:ident $target:ident $foreign_key:ident) => { stringify!($foreign_key).to_string() };
    (@foreign_key has_many $entity:ident $target:ident) => { "user_id".to_string() }; // Default for O2M (backward compatibility)
    (@foreign_key belongs_to $entity:ident $target:ident $foreign_key:ident) => { stringify!($foreign_key).to_string() };
    (@foreign_key belongs_to $entity:ident $target:ident) => { "user_id".to_string() }; // Default
    (@foreign_key has_one $entity:ident $target:ident $foreign_key:ident) => { stringify!($foreign_key).to_string() };
    (@foreign_key has_one $entity:ident $target:ident) => { "user_id".to_string() }; // Default
    (@foreign_key one_to_many $entity:ident $target:ident $foreign_key:ident) => { stringify!($foreign_key).to_string() };
    (@foreign_key one_to_many $entity:ident $target:ident) => { "user_id".to_string() }; // Default
    (@foreign_key has_many_through $entity:ident $target:ident $foreign_key:ident) => { stringify!($foreign_key).to_string() };
    (@foreign_key has_many_through $entity:ident $target:ident) => { "post_id".to_string() }; // Default
    
    // Helper macros for through tables - simplified approach
    (@through_table has_many $entity:ident $target:ident through $edge_schema:ident) => { 
        Some(stringify!($edge_schema).to_string())
    };
    (@through_table has_many $entity:ident $target:ident) => { 
        None // No junction table for O2M by default
    };
    (@through_table has_many_through $entity:ident $target:ident through $edge_schema:ident) => { 
        Some(stringify!($edge_schema).to_string())
    };
    (@through_table has_many_through $entity:ident $target:ident) => { 
        Some("post_categories".to_string()) // Legacy default
    };
    (@through_table $relation_type:ident $entity:ident $target:ident $($through:ident $edge_schema:ident)?) => { None };
}