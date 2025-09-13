/// New edge/relations system inspired by ent-go
/// This provides a much cleaner API for defining and querying relationships

use crate::{D1Client, D1RsError, Entity, Result, QueryBuilder};
use std::marker::PhantomData;
use serde_json;

/// Represents a relationship edge between two entities
pub trait Edge<Parent: Entity, Child: Entity> {
    /// Get the foreign key field name
    fn foreign_key() -> &'static str;
    
    /// Get the reference field name  
    fn references() -> &'static str;
    
    /// Get the edge name for queries
    fn edge_name() -> &'static str;
    
    /// Create a query builder for this edge
    fn query_builder(parent_id: serde_json::Value) -> EdgeQueryBuilder<Parent, Child>;
}

/// One-to-many relationship (Parent has many Children)
pub struct OneToMany<Parent: Entity, Child: Entity> {
    _phantom: PhantomData<(Parent, Child)>,
}

impl<Parent: Entity, Child: Entity> OneToMany<Parent, Child> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

/// Many-to-one relationship (Child belongs to Parent)
pub struct ManyToOne<Parent: Entity, Child: Entity> {
    _phantom: PhantomData<(Parent, Child)>,
}

impl<Parent: Entity, Child: Entity> ManyToOne<Parent, Child> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

/// One-to-one relationship
pub struct OneToOne<Parent: Entity, Child: Entity> {
    _phantom: PhantomData<(Parent, Child)>,
}

impl<Parent: Entity, Child: Entity> OneToOne<Parent, Child> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

/// Many-to-many relationship through junction table
pub struct ManyToMany<Parent: Entity, Child: Entity> {
    _phantom: PhantomData<(Parent, Child)>,
}

impl<Parent: Entity, Child: Entity> ManyToMany<Parent, Child> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

/// Association represents an instance-level relationship - FULLY TYPE-SAFE!
pub struct Association<Parent: Entity + HasEdges, Child: Entity + Clone> {
    parent_id: serde_json::Value,
    edge_name: String,
    loaded_data: Option<Vec<Child>>,
    _phantom: PhantomData<(Parent, Child)>,
}

impl<Parent: Entity + HasEdges, Child: Entity + Clone> Association<Parent, Child> {
    pub fn new(parent_id: serde_json::Value, edge_name: String) -> Self {
        Self {
            parent_id,
            edge_name,
            loaded_data: None,
            _phantom: PhantomData,
        }
    }
    
    /// Get eagerly loaded data if available
    pub fn loaded(&self) -> Option<&Vec<Child>> {
        self.loaded_data.as_ref()
    }
    
    /// Load all related entities - FULLY TYPE-SAFE, NO STRING LITERALS!
    pub async fn all(&self, db: &D1Client) -> Result<Vec<Child>> {
        if let Some(loaded) = &self.loaded_data {
            Ok(loaded.to_vec())
        } else {
            self.execute_relation_query(db).await
        }
    }
    
    /// Load first related entity
    pub async fn first(&self, db: &D1Client) -> Result<Option<Child>> {
        if let Some(loaded) = &self.loaded_data {
            Ok(loaded.first().cloned())
        } else {
            let results = self.execute_relation_query(db).await?;
            Ok(results.into_iter().next())
        }
    }
    
    /// Count related entities
    pub async fn count(&self, db: &D1Client) -> Result<i64> {
        if let Some(loaded) = &self.loaded_data {
            Ok(loaded.len() as i64)
        } else {
            let results = self.execute_relation_query(db).await?;
            Ok(results.len() as i64)
        }
    }
    
    /// Execute the actual database query based on relation type
    async fn execute_relation_query(&self, db: &D1Client) -> Result<Vec<Child>> {
        // Get edge definition from Parent entity
        let edges = Parent::edges();
        let edge = edges.iter()
            .find(|e| e.name == self.edge_name)
            .ok_or_else(|| D1RsError::ValidationError(format!("No relation '{}' found", self.edge_name)))?;
        
        match edge.edge_type {
            EdgeType::OneToMany => self.query_one_to_many(db, edge).await,
            EdgeType::ManyToOne => self.query_many_to_one(db, edge).await,
            EdgeType::OneToOne => self.query_one_to_one(db, edge).await,
            EdgeType::ManyToMany => self.query_many_to_many(db, edge).await,
        }
    }
    
    /// Handle one-to-many relations (Parent has many Children)
    async fn query_one_to_many(&self, db: &D1Client, edge: &EdgeDefinition) -> Result<Vec<Child>> {
        let sql = format!(
            "SELECT * FROM {} WHERE {} = ?",
            Child::TABLE_NAME,
            edge.foreign_key
        );
        
        let params = vec![self.parent_id.clone()];
        let result = db.execute(&sql, &params).await?;
        
        self.convert_rows_to_entities(result.rows).await
    }
    
    /// Handle many-to-one relations (Child belongs to Parent)
    async fn query_many_to_one(&self, db: &D1Client, edge: &EdgeDefinition) -> Result<Vec<Child>> {
        let sql = format!(
            "SELECT * FROM {} WHERE {} = ?",
            Child::TABLE_NAME,
            edge.references
        );
        
        let params = vec![self.parent_id.clone()];
        let result = db.execute(&sql, &params).await?;
        
        self.convert_rows_to_entities(result.rows).await
    }
    
    /// Handle one-to-one relations
    async fn query_one_to_one(&self, db: &D1Client, edge: &EdgeDefinition) -> Result<Vec<Child>> {
        let results = self.query_one_to_many(db, edge).await?;
        Ok(results.into_iter().take(1).collect())
    }
    
    /// Handle many-to-many relations through junction table
    async fn query_many_to_many(&self, db: &D1Client, edge: &EdgeDefinition) -> Result<Vec<Child>> {
        let through_table = edge.through_table.as_ref()
            .ok_or_else(|| D1RsError::ValidationError("Many-to-many relation requires through_table".to_string()))?;
        
        // Generate proper column names based on entity types
        // Parent foreign key is what's specified in the edge (usually like "post_id")
        let parent_foreign_key = &edge.foreign_key;
        
        // Child foreign key: derive from target entity name
        let child_foreign_key = format!("{}_id", 
            edge.target_entity.to_lowercase()
                .chars()
                .enumerate()
                .map(|(i, c)| if i > 0 && c.is_uppercase() { format!("_{}", c.to_lowercase()) } else { c.to_string() })
                .collect::<String>()
        );
        
        let sql = format!(
            "SELECT c.* FROM {} c \
             INNER JOIN {} j ON c.{} = j.{} \
             WHERE j.{} = ?",
            Child::TABLE_NAME,
            through_table,
            edge.references,
            child_foreign_key,
            parent_foreign_key
        );
        
        let params = vec![self.parent_id.clone()];
        let result = db.execute(&sql, &params).await?;
        
        self.convert_rows_to_entities(result.rows).await
    }
    
    /// Convert database rows to entities
    async fn convert_rows_to_entities(&self, rows: Vec<serde_json::Value>) -> Result<Vec<Child>> {
        let mut results = Vec::new();
        
        for row in rows {
            let converted = Child::convert_from_sqlite(row);
            let entity: Child = serde_json::from_value(converted)
                .map_err(|e| D1RsError::SerializationError(e.to_string()))?;
            results.push(entity);
        }
        
        Ok(results)
    }
    
    /// Attach a related entity (for many-to-many relationships)
    pub async fn attach(&self, db: &D1Client, target_id: i64) -> Result<()> {
        // This is a simplified implementation - in a real system this would use the edge metadata
        // to determine the junction table and column names
        let junction_table = format!("{}_{}", 
            std::any::type_name::<Parent>().split("::").last().unwrap_or("parent").to_lowercase(),
            std::any::type_name::<Child>().split("::").last().unwrap_or("child").to_lowercase()
        );
        
        let parent_col = format!("{}_id", std::any::type_name::<Parent>().split("::").last().unwrap_or("parent").to_lowercase());
        let child_col = format!("{}_id", std::any::type_name::<Child>().split("::").last().unwrap_or("child").to_lowercase());
        
        let sql = format!(
            "INSERT INTO {} ({}, {}) VALUES (?, ?) ON CONFLICT DO NOTHING",
            junction_table, parent_col, child_col
        );
        
        let params = vec![self.parent_id.clone(), serde_json::Value::Number(target_id.into())];
        db.execute(&sql, &params).await?;
        Ok(())
    }
    
    /// Detach a related entity (for many-to-many relationships)
    pub async fn detach(&self, db: &D1Client, target_id: i64) -> Result<()> {
        let junction_table = format!("{}_{}", 
            std::any::type_name::<Parent>().split("::").last().unwrap_or("parent").to_lowercase(),
            std::any::type_name::<Child>().split("::").last().unwrap_or("child").to_lowercase()
        );
        
        let parent_col = format!("{}_id", std::any::type_name::<Parent>().split("::").last().unwrap_or("parent").to_lowercase());
        let child_col = format!("{}_id", std::any::type_name::<Child>().split("::").last().unwrap_or("child").to_lowercase());
        
        let sql = format!(
            "DELETE FROM {} WHERE {} = ? AND {} = ?",
            junction_table, parent_col, child_col
        );
        
        let params = vec![self.parent_id.clone(), serde_json::Value::Number(target_id.into())];
        db.execute(&sql, &params).await?;
        Ok(())
    }
}

/// Query builder for edges
pub struct EdgeQueryBuilder<Parent: Entity, Child: Entity> {
    parent_id: serde_json::Value,
    child_query: Child::QueryBuilder,
    _phantom: PhantomData<Parent>,
}

impl<Parent: Entity, Child: Entity> EdgeQueryBuilder<Parent, Child> {
    pub fn new(parent_id: serde_json::Value, child_query: Child::QueryBuilder) -> Self {
        Self {
            parent_id,
            child_query,
            _phantom: PhantomData,
        }
    }
    
    /// Execute the query and return all results
    pub async fn all(self, db: &D1Client) -> Result<Vec<Child>> {
        self.child_query.all(db).await
    }
    
    /// Execute the query and return first result
    pub async fn first(self, db: &D1Client) -> Result<Option<Child>> {
        self.child_query.first(db).await
    }
    
    /// Count the results
    pub async fn count(self, db: &D1Client) -> Result<i64> {
        self.child_query.count(db).await
    }
}

/// Predicate system for complex queries
#[derive(Debug, Clone)]
pub enum Predicate {
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    Field(FieldPredicate),
    Relation(RelationPredicate),
}

#[derive(Debug, Clone)]
pub struct FieldPredicate {
    pub field: String,
    pub operator: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct RelationPredicate {
    pub relation: String,
    pub predicate: Option<Box<Predicate>>,
    pub exists: bool, // true for HAS, false for NOT HAS
}

impl Predicate {
    /// Create an AND predicate
    pub fn and(predicates: Vec<Predicate>) -> Self {
        Predicate::And(predicates)
    }
    
    /// Create an OR predicate
    pub fn or(predicates: Vec<Predicate>) -> Self {
        Predicate::Or(predicates)
    }
    
    /// Create a field predicate
    pub fn field<T: Into<serde_json::Value>>(field: &str, operator: &str, value: T) -> Self {
        Predicate::Field(FieldPredicate {
            field: field.to_string(),
            operator: operator.to_string(),
            value: value.into(),
        })
    }
    
    /// Create a relation exists predicate
    pub fn has(relation: &str) -> Self {
        Predicate::Relation(RelationPredicate {
            relation: relation.to_string(),
            predicate: None,
            exists: true,
        })
    }
    
    /// Create a relation exists with condition predicate
    pub fn has_with(relation: &str, predicate: Predicate) -> Self {
        Predicate::Relation(RelationPredicate {
            relation: relation.to_string(),
            predicate: Some(Box::new(predicate)),
            exists: true,
        })
    }
    
    /// Generate SQL WHERE clause from predicate
    pub fn to_sql(&self, table_alias: &str) -> (String, Vec<serde_json::Value>) {
        match self {
            Predicate::And(predicates) => {
                let mut sql_parts = Vec::new();
                let mut params = Vec::new();
                
                for predicate in predicates {
                    let (sql, mut pred_params) = predicate.to_sql(table_alias);
                    sql_parts.push(format!("({})", sql));
                    params.append(&mut pred_params);
                }
                
                (sql_parts.join(" AND "), params)
            }
            Predicate::Or(predicates) => {
                let mut sql_parts = Vec::new();
                let mut params = Vec::new();
                
                for predicate in predicates {
                    let (sql, mut pred_params) = predicate.to_sql(table_alias);
                    sql_parts.push(format!("({})", sql));
                    params.append(&mut pred_params);
                }
                
                (sql_parts.join(" OR "), params)
            }
            Predicate::Field(field_pred) => {
                let sql = format!("{}.{} {} ?", table_alias, field_pred.field, field_pred.operator);
                (sql, vec![field_pred.value.clone()])
            }
            Predicate::Relation(rel_pred) => {
                // This would need to generate EXISTS subqueries
                // For now, return a placeholder
                ("1=1".to_string(), vec![])
            }
        }
    }
}

/// Trait for entities that have relationships
pub trait HasEdges: Entity {
    /// Get all edge definitions for this entity
    fn edges() -> Vec<EdgeDefinition>;
}

/// Definition of an edge/relationship
#[derive(Debug, Clone)]
pub struct EdgeDefinition {
    pub name: String,
    pub target_entity: String,
    pub edge_type: EdgeType,
    pub foreign_key: String,
    pub references: String,
    pub through_table: Option<String>, // For many-to-many
}

#[derive(Debug, Clone)]
pub enum EdgeType {
    OneToMany,
    ManyToOne,
    OneToOne,
    ManyToMany,
}