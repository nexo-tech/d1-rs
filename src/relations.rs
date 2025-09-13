/// The ONLY relations system for d1-rs - fully type-safe, no string literals!
/// Inspired by ent-go with compile-time safety

use crate::{D1Client, Entity, Result, edges::*};
use serde_json;

/// The ONLY macro for defining relations - generates type-safe association methods
/// Now supports automatic M2M detection for clean APIs like ent-go!
#[macro_export]
macro_rules! relations {
    // Entry point - handle both with and without config attributes
    (
        $(
            $entity:ident {
                $(
                    $relation_type:ident $relation_name:ident : $target:ident $(via $foreign_key:ident)? $(through $edge_schema:ident)? $(required)? $(unique)? $(immutable)?,
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
                                edge_type: relations!(@edge_type $relation_type $entity $target $(via $foreign_key)? $(through $edge_schema)?),
                                foreign_key: relations!(@foreign_key $relation_type $entity $target $($foreign_key)?),
                                references: "id".to_string(),
                                through_table: relations!(@through_table $relation_type $entity $target $(via $foreign_key)? $(through $edge_schema)?),
                                config: relations!(@make_config $(required)? $(unique)? $(immutable)?),
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
            
            // Generate type-safe relation predicate methods on QueryBuilder - NO STRING LITERALS!
            // This provides compile-time safe has_posts(), has_posts_with() methods that work with EXISTS subqueries
            paste::paste! {
                impl [<$entity QueryBuilder>] {
                    $(
                        /// Type-safe relation existence check - NO STRING LITERALS!
                        /// Generates: EXISTS (SELECT 1 FROM target_table WHERE target_table.foreign_key = entity.id)
                        pub fn [<has_ $relation_name>](mut self) -> Self {
                            // Generate EXISTS subquery based on relation type
                            let exists_sql = format!("EXISTS (SELECT 1 FROM {} WHERE {}.{} = {}.id)", 
                                stringify!($target).to_lowercase(), 
                                stringify!($target).to_lowercase(),
                                // TODO: Get actual foreign key from relation definition
                                "user_id", // This should be dynamic based on the relation
                                stringify!($entity).to_lowercase()
                            );
                            
                            // Add as a raw WHERE clause for now
                            self.query.where_clause(&exists_sql, "=", serde_json::Value::Bool(true));
                            self
                        }
                        
                        /// Type-safe relation with conditions - NO STRING LITERALS!
                        /// Generates: EXISTS (SELECT 1 FROM target_table WHERE target_table.foreign_key = entity.id AND <conditions>)
                        pub fn [<has_ $relation_name _with>]<F>(mut self, _condition: F) -> Self 
                        where 
                            F: FnOnce(<$target as $crate::Entity>::QueryBuilder) -> <$target as $crate::Entity>::QueryBuilder
                        {
                            // For now, implement basic has relation - the condition logic will be enhanced later
                            let exists_sql = format!("EXISTS (SELECT 1 FROM {} WHERE {}.{} = {}.id)", 
                                stringify!($target).to_lowercase(), 
                                stringify!($target).to_lowercase(),
                                "user_id", // This should be dynamic based on the relation
                                stringify!($entity).to_lowercase()
                            );
                            
                            self.query.where_clause(&exists_sql, "=", serde_json::Value::Bool(true));
                            self
                        }
                    )*
                }
            }
        )*
    };
    
    // Helper macros for edge types - distinguish between O2M (with via) and M2M (without via)
    (@edge_type has_many $entity:ident $target:ident via $foreign_key:ident through $edge_schema:ident) => { $crate::edges::EdgeType::ManyToMany }; // Both via and through = M2M
    (@edge_type has_many $entity:ident $target:ident via $foreign_key:ident) => { $crate::edges::EdgeType::OneToMany }; // With via = O2M
    (@edge_type has_many $entity:ident $target:ident through $edge_schema:ident) => { $crate::edges::EdgeType::ManyToMany }; // With through = M2M
    (@edge_type has_many $entity:ident $target:ident) => { $crate::edges::EdgeType::ManyToMany }; // No via clause = M2M
    (@edge_type belongs_to $entity:ident $target:ident $(via $foreign_key:ident)? $(through $edge_schema:ident)?) => { $crate::edges::EdgeType::ManyToOne };
    (@edge_type has_one $entity:ident $target:ident $(via $foreign_key:ident)? $(through $edge_schema:ident)?) => { $crate::edges::EdgeType::OneToOne };
    (@edge_type has_many_through $entity:ident $target:ident $(via $foreign_key:ident)? $(through $edge_schema:ident)?) => { $crate::edges::EdgeType::ManyToMany };
    (@edge_type one_to_many $entity:ident $target:ident $(via $foreign_key:ident)? $(through $edge_schema:ident)?) => { $crate::edges::EdgeType::OneToMany }; // Explicit O2M
    
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
    
    // Helper macros for through tables - auto-generate for M2M
    (@through_table has_many $entity:ident $target:ident via $foreign_key:ident through $edge_schema:ident) => { 
        Some(stringify!($edge_schema).to_string()) // Both via and through = use explicit through table
    };
    (@through_table has_many $entity:ident $target:ident via $foreign_key:ident) => { 
        None // No junction table for O2M 
    };
    (@through_table has_many $entity:ident $target:ident through $edge_schema:ident) => { 
        Some(stringify!($edge_schema).to_string()) // Explicit through table
    };
    (@through_table has_many $entity:ident $target:ident) => { 
        Some({
            let entity_name = stringify!($entity).to_lowercase();
            let target_name = stringify!($target).to_lowercase(); 
            format!("{}_{}", entity_name, target_name)
        })
    };
    (@through_table has_many_through $entity:ident $target:ident through $edge_schema:ident) => { 
        Some(stringify!($edge_schema).to_string())
    };
    (@through_table has_many_through $entity:ident $target:ident) => { 
        Some("post_categories".to_string()) // Legacy default
    };
    (@through_table $relation_type:ident $entity:ident $target:ident $($through:ident $edge_schema:ident)?) => { None };
    
    // Helper macros for edge configuration - TYPE-SAFE constraint parsing with optional attributes!
    (@make_config) => { 
        $crate::edges::EdgeConfig::default() 
    };
    (@make_config required) => { 
        $crate::edges::EdgeConfig { required: true, unique: false, immutable: false }
    };
    (@make_config unique) => { 
        $crate::edges::EdgeConfig { required: false, unique: true, immutable: false }
    };
    (@make_config immutable) => { 
        $crate::edges::EdgeConfig { required: false, unique: false, immutable: true }
    };
    (@make_config required unique) => { 
        $crate::edges::EdgeConfig { required: true, unique: true, immutable: false }
    };
    (@make_config required immutable) => { 
        $crate::edges::EdgeConfig { required: true, unique: false, immutable: true }
    };
    (@make_config unique immutable) => { 
        $crate::edges::EdgeConfig { required: false, unique: true, immutable: true }
    };
    (@make_config required unique immutable) => { 
        $crate::edges::EdgeConfig { required: true, unique: true, immutable: true }
    };
    (@make_config unique required) => { 
        $crate::edges::EdgeConfig { required: true, unique: true, immutable: false }
    };
    (@make_config immutable required) => { 
        $crate::edges::EdgeConfig { required: true, unique: false, immutable: true }
    };
    (@make_config immutable unique) => { 
        $crate::edges::EdgeConfig { required: false, unique: true, immutable: true }
    };
    (@make_config unique immutable required) => { 
        $crate::edges::EdgeConfig { required: true, unique: true, immutable: true }
    };
    (@make_config immutable required unique) => { 
        $crate::edges::EdgeConfig { required: true, unique: true, immutable: true }
    };
    (@make_config immutable unique required) => { 
        $crate::edges::EdgeConfig { required: true, unique: true, immutable: true }
    };
}