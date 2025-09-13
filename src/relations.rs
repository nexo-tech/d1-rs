use crate::{D1Client, Result, Entity};
use serde_json::Value;
use std::collections::HashMap;
use std::marker::PhantomData;
use async_trait::async_trait;

/// Relation types supported by the ORM
#[derive(Debug, Clone, PartialEq)]
pub enum RelationType {
    /// One-to-one relation (user has one profile)
    OneToOne {
        foreign_key: String,
        references: String,
    },
    /// One-to-many relation (user has many posts)
    OneToMany {
        foreign_key: String,
        references: String,
    },
    /// Many-to-many relation through junction table (users have many roles)
    ManyToMany {
        junction_table: String,
        foreign_key: String,
        references: String,
        target_foreign_key: String,
        target_references: String,
    },
}

/// Edge in the entity graph representing a relation
#[derive(Debug)]
pub struct Edge<From, To> 
where 
    From: Entity,
    To: Entity,
{
    pub relation_type: RelationType,
    pub name: String,
    _phantom: PhantomData<(From, To)>,
}

impl<From, To> Edge<From, To> 
where 
    From: Entity,
    To: Entity,
{
    pub fn new(name: String, relation_type: RelationType) -> Self {
        Self {
            relation_type,
            name,
            _phantom: PhantomData,
        }
    }
}

/// Graph traversal context for eager loading
#[derive(Debug, Default)]
pub struct TraversalContext {
    /// Entities that should be eagerly loaded
    pub includes: Vec<String>,
    /// Maximum depth to prevent infinite recursion
    pub max_depth: usize,
    /// Current depth in traversal
    pub current_depth: usize,
    /// Already loaded entities to prevent cycles
    pub loaded: HashMap<String, Vec<Value>>,
}

impl TraversalContext {
    pub fn new() -> Self {
        Self {
            max_depth: 10, // Reasonable default
            ..Default::default()
        }
    }
    
    pub fn with_includes(mut self, includes: Vec<String>) -> Self {
        self.includes = includes;
        self
    }
    
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }
}

/// Trait for entities that participate in relations
#[async_trait]
pub trait RelationalEntity: Entity {
    /// Get all edges (relations) for this entity type
    fn edges() -> Vec<Box<dyn RelationEdge>>;
    
    /// Load related entities based on traversal context
    async fn load_relations(
        &mut self,
        db: &D1Client,
        context: &mut TraversalContext,
    ) -> Result<()>;
}

/// Dynamic relation edge that can be used at runtime
#[async_trait]
pub trait RelationEdge: Send + Sync {
    /// Name of the relation
    fn name(&self) -> &str;
    
    /// Type of relation
    fn relation_type(&self) -> &RelationType;
    
    /// Load related entities for a given primary key
    async fn load_related(
        &self,
        db: &D1Client,
        primary_key: &Value,
        context: &mut TraversalContext,
    ) -> Result<Vec<Value>>;
    
    /// Get the target entity table name
    fn target_table(&self) -> &str;
}

/// Concrete implementation of RelationEdge
pub struct ConcreteRelationEdge {
    pub name: String,
    pub relation_type: RelationType,
    pub target_table: String,
}

#[async_trait]
impl RelationEdge for ConcreteRelationEdge {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn relation_type(&self) -> &RelationType {
        &self.relation_type
    }
    
    fn target_table(&self) -> &str {
        &self.target_table
    }
    
    async fn load_related(
        &self,
        db: &D1Client,
        primary_key: &Value,
        context: &mut TraversalContext,
    ) -> Result<Vec<Value>> {
        if context.current_depth >= context.max_depth {
            return Ok(vec![]);
        }
        
        // Check if already loaded to prevent cycles
        let cache_key = format!("{}_{}", self.target_table, primary_key.to_string());
        if let Some(cached) = context.loaded.get(&cache_key) {
            return Ok(cached.clone());
        }
        
        let sql = match &self.relation_type {
            RelationType::OneToOne { foreign_key, .. } |
            RelationType::OneToMany { foreign_key, .. } => {
                format!("SELECT * FROM {} WHERE {} = ?", self.target_table, foreign_key)
            }
            RelationType::ManyToMany {
                junction_table,
                foreign_key,
                target_foreign_key,
                target_references,
                ..
            } => {
                format!(
                    "SELECT t.* FROM {} t INNER JOIN {} j ON t.{} = j.{} WHERE j.{} = ?",
                    self.target_table,
                    junction_table,
                    target_references,
                    target_foreign_key,
                    foreign_key
                )
            }
        };
        
        let result = db.execute(&sql, &[primary_key.clone()]).await?;
        
        // Cache the result
        context.loaded.insert(cache_key, result.rows.clone());
        
        Ok(result.rows)
    }
}

/// Builder for creating relations in migrations
pub struct RelationBuilder {
    pub from_table: String,
    pub to_table: String,
    pub relation_type: RelationType,
    pub name: String,
}

impl RelationBuilder {
    pub fn new(name: String, from_table: String, to_table: String) -> Self {
        Self {
            from_table,
            to_table,
            name,
            relation_type: RelationType::OneToMany {
                foreign_key: format!("{}_id", from_table),
                references: "id".to_string(),
            },
        }
    }
    
    /// Create a one-to-one relation
    pub fn one_to_one(mut self, foreign_key: String, references: String) -> Self {
        self.relation_type = RelationType::OneToOne { foreign_key, references };
        self
    }
    
    /// Create a one-to-many relation
    pub fn one_to_many(mut self, foreign_key: String, references: String) -> Self {
        self.relation_type = RelationType::OneToMany { foreign_key, references };
        self
    }
    
    /// Create a many-to-many relation
    pub fn many_to_many(
        mut self,
        junction_table: String,
        foreign_key: String,
        references: String,
        target_foreign_key: String,
        target_references: String,
    ) -> Self {
        self.relation_type = RelationType::ManyToMany {
            junction_table,
            foreign_key,
            references,
            target_foreign_key,
            target_references,
        };
        self
    }
    
    /// Generate SQL for creating the relation (foreign key constraints)
    pub fn to_sql(&self) -> Vec<String> {
        let mut statements = Vec::new();
        
        match &self.relation_type {
            RelationType::OneToOne { foreign_key, references } |
            RelationType::OneToMany { foreign_key, references } => {
                // Add foreign key constraint
                statements.push(format!(
                    "ALTER TABLE {} ADD CONSTRAINT fk_{}_{} FOREIGN KEY ({}) REFERENCES {}({})",
                    self.to_table,
                    self.to_table,
                    foreign_key,
                    foreign_key,
                    self.from_table,
                    references
                ));
            }
            RelationType::ManyToMany {
                junction_table,
                foreign_key,
                references,
                target_foreign_key,
                target_references,
            } => {
                // Create junction table if it doesn't exist
                statements.push(format!(
                    "CREATE TABLE IF NOT EXISTS {} ({} INTEGER, {} INTEGER, PRIMARY KEY ({}, {}))",
                    junction_table, foreign_key, target_foreign_key, foreign_key, target_foreign_key
                ));
                
                // Add foreign key constraints for junction table
                statements.push(format!(
                    "ALTER TABLE {} ADD CONSTRAINT fk_{}_{}_{} FOREIGN KEY ({}) REFERENCES {}({})",
                    junction_table,
                    junction_table,
                    self.from_table,
                    foreign_key,
                    foreign_key,
                    self.from_table,
                    references
                ));
                
                statements.push(format!(
                    "ALTER TABLE {} ADD CONSTRAINT fk_{}_{}_{} FOREIGN KEY ({}) REFERENCES {}({})",
                    junction_table,
                    junction_table,
                    self.to_table,
                    target_foreign_key,
                    target_foreign_key,
                    self.to_table,
                    target_references
                ));
            }
        }
        
        statements
    }
}

/// Query builder extension for including relations
pub trait WithRelations<T: RelationalEntity> {
    /// Include related entities in the query result
    fn with(self, relation_names: Vec<&str>) -> Self;
    
    /// Load with custom traversal context
    fn with_context(self, context: TraversalContext) -> Self;
}

/// Extension methods for entity results to enable graph traversal
pub trait GraphTraversal<T: RelationalEntity> {
    /// Traverse to related entities
    async fn traverse(&mut self, db: &D1Client, path: &str) -> Result<Vec<Value>>;
    
    /// Eagerly load all specified relations
    async fn load_graph(&mut self, db: &D1Client, includes: Vec<&str>) -> Result<()>;
}

#[async_trait]
impl<T: RelationalEntity> GraphTraversal<T> for T {
    async fn traverse(&mut self, db: &D1Client, path: &str) -> Result<Vec<Value>> {
        let mut context = TraversalContext::new().with_includes(vec![path.to_string()]);
        
        // Find the edge for this path
        let edges = T::edges();
        let edge = edges.iter().find(|e| e.name() == path);
        
        if let Some(edge) = edge {
            let pk_value = crate::types::SqlType::to_sql_value(self.primary_key());
            edge.load_related(db, &pk_value, &mut context).await
        } else {
            Ok(vec![])
        }
    }
    
    async fn load_graph(&mut self, db: &D1Client, includes: Vec<&str>) -> Result<()> {
        let mut context = TraversalContext::new()
            .with_includes(includes.iter().map(|s| s.to_string()).collect());
        
        self.load_relations(db, &mut context).await
    }
}