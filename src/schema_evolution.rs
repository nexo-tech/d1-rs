use crate::{D1Client, Result, D1RsError};
use crate::schema::{ColumnType, DefaultValue, TableDefinition};
use crate::edges::{EdgeConfig, EdgeDefinition, EdgeType, HasEdges};
use crate::Entity;

/// Enhanced schema evolution with better relation support
#[derive(Debug, Clone)]
pub enum SchemaOperation {
    /// Create a new table
    CreateTable {
        definition: TableDefinition,
    },
    /// Drop an existing table
    DropTable {
        name: String,
        if_exists: bool,
    },
    /// Add a column to an existing table
    AddColumn {
        table: String,
        column: ColumnDefinition,
    },
    /// Drop a column from an existing table
    DropColumn {
        table: String,
        column: String,
    },
    /// Rename a column
    RenameColumn {
        table: String,
        old_name: String,
        new_name: String,
    },
    /// Add an index
    AddIndex {
        table: String,
        name: String,
        columns: Vec<String>,
        unique: bool,
    },
    /// Drop an index
    DropIndex {
        name: String,
        if_exists: bool,
    },
    /// Create edge/relation (automatically handles junction tables)
    CreateEdge {
        edge: EdgeDefinition,
    },
    /// Create foreign key constraint
    AddForeignKey {
        table: String,
        column: String,
        references_table: String,
        references_column: String,
        on_delete: Option<ForeignKeyAction>,
        on_update: Option<ForeignKeyAction>,
    },
    /// Execute raw SQL
    RawSql {
        sql: String,
    },
}

#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    pub name: String,
    pub column_type: ColumnType,
    pub nullable: bool,
    pub default: Option<DefaultValue>,
    pub unique: bool,
    pub primary_key: bool,
    pub auto_increment: bool,
}

#[derive(Debug, Clone)]
pub enum ForeignKeyAction {
    Cascade,
    SetNull,
    SetDefault,
    Restrict,
    NoAction,
}

impl ForeignKeyAction {
    fn to_sql(&self) -> &'static str {
        match self {
            ForeignKeyAction::Cascade => "CASCADE",
            ForeignKeyAction::SetNull => "SET NULL", 
            ForeignKeyAction::SetDefault => "SET DEFAULT",
            ForeignKeyAction::Restrict => "RESTRICT",
            ForeignKeyAction::NoAction => "NO ACTION",
        }
    }
}

impl ColumnDefinition {
    pub fn new(name: String, column_type: ColumnType) -> Self {
        Self {
            name,
            column_type,
            nullable: true,
            default: None,
            unique: false,
            primary_key: false,
            auto_increment: false,
        }
    }
    
    pub fn not_null(mut self) -> Self {
        self.nullable = false;
        self
    }
    
    pub fn default_value(mut self, default: DefaultValue) -> Self {
        self.default = Some(default);
        self
    }
    
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }
    
    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self.nullable = false;
        self
    }
    
    pub fn auto_increment(mut self) -> Self {
        self.auto_increment = true;
        self
    }
    
    /// Convert to SQL column definition
    pub fn to_sql(&self) -> String {
        let mut sql = format!("{} {}", self.name, self.column_type.to_sql());
        
        if self.primary_key {
            sql.push_str(" PRIMARY KEY");
        }
        
        if self.auto_increment && matches!(self.column_type, ColumnType::Integer) {
            sql.push_str(" AUTOINCREMENT");
        }
        
        if !self.nullable && !self.primary_key {
            sql.push_str(" NOT NULL");
        }
        
        if self.unique && !self.primary_key {
            sql.push_str(" UNIQUE");
        }
        
        if let Some(ref default) = self.default {
            sql.push_str(&format!(" DEFAULT {}", default.to_sql()));
        }
        
        sql
    }
}

impl SchemaOperation {
    /// Convert the operation to SQL statements
    pub fn to_sql(&self) -> Result<Vec<String>> {
        match self {
            SchemaOperation::CreateTable { definition } => {
                Ok(vec![definition.to_sql()])
            }
            
            SchemaOperation::DropTable { name, if_exists } => {
                let if_exists_clause = if *if_exists { " IF EXISTS" } else { "" };
                Ok(vec![format!("DROP TABLE{} {}", if_exists_clause, name)])
            }
            
            SchemaOperation::AddColumn { table, column } => {
                Ok(vec![format!("ALTER TABLE {} ADD COLUMN {}", table, column.to_sql())])
            }
            
            SchemaOperation::DropColumn { table: _, column: _ } => {
                Err(D1RsError::Database(
                    "SQLite doesn't support DROP COLUMN directly. Use raw SQL migration.".to_string()
                ))
            }
            
            SchemaOperation::RenameColumn { table, old_name, new_name } => {
                Ok(vec![format!("ALTER TABLE {} RENAME COLUMN {} TO {}", table, old_name, new_name)])
            }
            
            SchemaOperation::AddIndex { table, name, columns, unique } => {
                let unique_clause = if *unique { "UNIQUE " } else { "" };
                let columns_str = columns.join(", ");
                Ok(vec![format!("CREATE {}INDEX {} ON {} ({})", unique_clause, name, table, columns_str)])
            }
            
            SchemaOperation::DropIndex { name, if_exists } => {
                let if_exists_clause = if *if_exists { " IF EXISTS" } else { "" };
                Ok(vec![format!("DROP INDEX{} {}", if_exists_clause, name)])
            }
            
            SchemaOperation::CreateEdge { edge } => {
                Ok(self.generate_edge_sql(edge))
            }
            
            SchemaOperation::AddForeignKey {
                table,
                column,
                references_table,
                references_column,
                on_delete,
                on_update,
            } => {
                let mut sql = format!(
                    "ALTER TABLE {} ADD CONSTRAINT fk_{}_{} FOREIGN KEY ({}) REFERENCES {}({})",
                    table, table, column, column, references_table, references_column
                );
                
                if let Some(on_delete) = on_delete {
                    sql.push_str(&format!(" ON DELETE {}", on_delete.to_sql()));
                }
                
                if let Some(on_update) = on_update {
                    sql.push_str(&format!(" ON UPDATE {}", on_update.to_sql()));
                }
                
                Ok(vec![sql])
            }
            
            SchemaOperation::RawSql { sql } => {
                Ok(vec![sql.clone()])
            }
        }
    }
    
    /// Generate SQL for edges/relations (handles junction tables automatically)
    fn generate_edge_sql(&self, edge: &EdgeDefinition) -> Vec<String> {
        let mut statements = Vec::new();
        
        match edge.edge_type {
            EdgeType::OneToMany | EdgeType::ManyToOne | EdgeType::OneToOne => {
                // For basic relations, we assume tables already have proper foreign keys
                // The edge metadata is used for querying, not for creating constraints in SQLite
                // SQLite foreign keys should be defined during table creation
            }
            EdgeType::ManyToMany => {
                if let Some(ref junction_table) = edge.through_table {
                    // Create junction table for many-to-many relations
                    statements.push(format!(
                        "CREATE TABLE IF NOT EXISTS {} ({} INTEGER, {} INTEGER, PRIMARY KEY ({}, {}))",
                        junction_table,
                        edge.foreign_key,
                        format!("{}_id", edge.target_entity.to_lowercase()),
                        edge.foreign_key,
                        format!("{}_id", edge.target_entity.to_lowercase())
                    ));
                }
            }
        }
        
        statements
    }
}

/// Enhanced schema migration builder with relation support
pub struct SchemaMigration {
    operations: Vec<SchemaOperation>,
    name: String,
}

impl SchemaMigration {
    pub fn new(name: String) -> Self {
        Self {
            operations: Vec::new(),
            name,
        }
    }
    
    /// Create a new table
    pub fn create_table(self, name: &str) -> TableMigrationBuilder {
        TableMigrationBuilder::new(self, name.to_string())
    }
    
    /// Drop a table
    pub fn drop_table(mut self, name: &str) -> Self {
        self.operations.push(SchemaOperation::DropTable {
            name: name.to_string(),
            if_exists: false,
        });
        self
    }
    
    /// Drop a table if it exists
    pub fn drop_table_if_exists(mut self, name: &str) -> Self {
        self.operations.push(SchemaOperation::DropTable {
            name: name.to_string(),
            if_exists: true,
        });
        self
    }
    
    /// Modify an existing table
    pub fn alter_table(self, name: &str) -> AlterTableBuilder {
        AlterTableBuilder::new(self, name.to_string())
    }
    
    /// Create edge between entities (much simpler API)
    pub fn add_edge<Parent: Entity + HasEdges, Child: Entity>(self) -> EdgeMigrationBuilder<Parent, Child> {
        EdgeMigrationBuilder::new(self)
    }
    
    /// Create custom edge with manual configuration
    pub fn create_edge(mut self, edge: EdgeDefinition) -> Self {
        self.operations.push(SchemaOperation::CreateEdge { edge });
        self
    }
    
    /// Auto-generate all migrations for an entity with edges
    pub fn auto_generate_for<T: Entity + HasEdges>(mut self) -> Self {
        let edges = T::edges();
        for edge in edges {
            self.operations.push(SchemaOperation::CreateEdge { edge });
        }
        self
    }
    
    /// Execute raw SQL
    pub fn raw_sql(mut self, sql: &str) -> Self {
        self.operations.push(SchemaOperation::RawSql {
            sql: sql.to_string(),
        });
        self
    }
    
    /// Execute the migration
    pub async fn execute(self, db: &D1Client) -> Result<()> {
        for operation in self.operations {
            let statements = operation.to_sql()?;
            for statement in statements {
                db.execute(&statement, &[]).await?;
            }
        }
        Ok(())
    }
    
    /// Get all SQL statements for this migration
    pub fn to_sql_statements(self) -> Result<Vec<String>> {
        let mut all_statements = Vec::new();
        for operation in self.operations {
            let statements = operation.to_sql()?;
            all_statements.extend(statements);
        }
        Ok(all_statements)
    }
}

/// Builder for table creation in migrations
pub struct TableMigrationBuilder {
    migration: SchemaMigration,
    table_name: String,
    columns: Vec<ColumnDefinition>,
}

impl TableMigrationBuilder {
    fn new(migration: SchemaMigration, table_name: String) -> Self {
        Self {
            migration,
            table_name,
            columns: Vec::new(),
        }
    }
    
    /// Add an integer column
    pub fn integer(self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::Integer)
    }
    
    /// Add a text column
    pub fn text(self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::Text)
    }
    
    /// Add a boolean column
    pub fn boolean(self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::Boolean)
    }
    
    /// Add a datetime column
    pub fn datetime(self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::DateTime)
    }
    
    /// Add a real/float column
    pub fn real(self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::Real)
    }
    
    /// Add a JSON column
    pub fn json(self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::Json)
    }
    
    /// Finish building the table and return to migration
    pub fn build(mut self) -> SchemaMigration {
        let table_def = TableDefinition::from_columns(self.table_name.clone(), self.columns);
        self.migration.operations.push(SchemaOperation::CreateTable {
            definition: table_def,
        });
        self.migration
    }
}

/// Builder for column definitions in table creation
pub struct ColumnMigrationBuilder {
    table_builder: TableMigrationBuilder,
    column: ColumnDefinition,
}

impl ColumnMigrationBuilder {
    fn new(table_builder: TableMigrationBuilder, name: String, column_type: ColumnType) -> Self {
        Self {
            table_builder,
            column: ColumnDefinition::new(name, column_type),
        }
    }
    
    /// Make column not null
    pub fn not_null(mut self) -> Self {
        self.column = self.column.not_null();
        self
    }
    
    /// Add default value
    pub fn default_value(mut self, default: DefaultValue) -> Self {
        self.column = self.column.default_value(default);
        self
    }
    
    /// Make column unique
    pub fn unique(mut self) -> Self {
        self.column = self.column.unique();
        self
    }
    
    /// Make column primary key
    pub fn primary_key(mut self) -> Self {
        self.column = self.column.primary_key();
        self
    }
    
    /// Make column auto increment (integers only)
    pub fn auto_increment(mut self) -> Self {
        self.column = self.column.auto_increment();
        self
    }
    
    /// Add foreign key reference (simpler than manual foreign key)
    pub fn references(mut self, table: &str, column: &str) -> Self {
        // This would be stored and used to generate foreign key constraint
        self
    }
    
    /// Finish building column and return to table builder
    pub fn build(mut self) -> TableMigrationBuilder {
        self.table_builder.columns.push(self.column);
        self.table_builder
    }
}

/// Builder for altering existing tables
pub struct AlterTableBuilder {
    migration: SchemaMigration,
    table_name: String,
}

impl AlterTableBuilder {
    fn new(migration: SchemaMigration, table_name: String) -> Self {
        Self {
            migration,
            table_name,
        }
    }
    
    /// Add a column to the table
    pub fn add_column(self, name: &str, column_type: ColumnType) -> ColumnMigrationBuilder {
        let table_builder = TableMigrationBuilder {
            migration: self.migration,
            table_name: self.table_name,
            columns: Vec::new(),
        };
        ColumnMigrationBuilder::new(table_builder, name.to_string(), column_type)
    }
    
    /// Rename a column
    pub fn rename_column(mut self, old_name: &str, new_name: &str) -> SchemaMigration {
        self.migration.operations.push(SchemaOperation::RenameColumn {
            table: self.table_name.clone(),
            old_name: old_name.to_string(),
            new_name: new_name.to_string(),
        });
        self.migration
    }
    
    /// Add an index
    pub fn add_index(mut self, name: &str, columns: Vec<&str>) -> SchemaMigration {
        self.migration.operations.push(SchemaOperation::AddIndex {
            table: self.table_name.clone(),
            name: name.to_string(),
            columns: columns.iter().map(|s| s.to_string()).collect(),
            unique: false,
        });
        self.migration
    }
    
    /// Add a unique index
    pub fn add_unique_index(mut self, name: &str, columns: Vec<&str>) -> SchemaMigration {
        self.migration.operations.push(SchemaOperation::AddIndex {
            table: self.table_name.clone(),
            name: name.to_string(),
            columns: columns.iter().map(|s| s.to_string()).collect(),
            unique: true,
        });
        self.migration
    }
    
    /// Build and return migration
    pub fn build(self) -> SchemaMigration {
        self.migration
    }
}

/// Type-safe edge builder
pub struct EdgeMigrationBuilder<Parent: Entity, Child: Entity> {
    migration: SchemaMigration,
    _phantom: std::marker::PhantomData<(Parent, Child)>,
}

impl<Parent: Entity + HasEdges, Child: Entity> EdgeMigrationBuilder<Parent, Child> {
    fn new(migration: SchemaMigration) -> Self {
        Self {
            migration,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Create a one-to-many relationship (Parent has many Children)
    pub fn one_to_many(mut self) -> Self {
        let edge = EdgeDefinition {
            name: format!("{}_to_{}", Parent::TABLE_NAME, Child::TABLE_NAME),
            target_entity: Child::TABLE_NAME.to_string(),
            edge_type: EdgeType::OneToMany,
            foreign_key: format!("{}_id", Parent::TABLE_NAME.trim_end_matches('s')),
            references: "id".to_string(),
            through_table: None,
            config: EdgeConfig::default(),
        };
        
        self.migration.operations.push(SchemaOperation::CreateEdge { edge });
        self
    }
    
    /// Create a many-to-many relationship (automatically creates junction table)
    pub fn many_to_many(mut self) -> Self {
        let junction_table = format!("{}_{}", 
            Parent::TABLE_NAME.trim_end_matches('s'),
            Child::TABLE_NAME
        );
        
        let edge = EdgeDefinition {
            name: format!("{}_to_{}", Parent::TABLE_NAME, Child::TABLE_NAME),
            target_entity: Child::TABLE_NAME.to_string(),
            edge_type: EdgeType::ManyToMany,
            foreign_key: format!("{}_id", Parent::TABLE_NAME.trim_end_matches('s')),
            references: "id".to_string(),
            through_table: Some(junction_table),
            config: EdgeConfig::default(),
        };
        
        self.migration.operations.push(SchemaOperation::CreateEdge { edge });
        self
    }
    
    /// Create a one-to-one relationship
    pub fn one_to_one(mut self) -> Self {
        let edge = EdgeDefinition {
            name: format!("{}_to_{}", Parent::TABLE_NAME, Child::TABLE_NAME),
            target_entity: Child::TABLE_NAME.to_string(),
            edge_type: EdgeType::OneToOne,
            foreign_key: format!("{}_id", Parent::TABLE_NAME.trim_end_matches('s')),
            references: "id".to_string(),
            through_table: None,
            config: EdgeConfig::default(),
        };
        
        self.migration.operations.push(SchemaOperation::CreateEdge { edge });
        self
    }
    
    /// Finish and return migration
    pub fn build(self) -> SchemaMigration {
        self.migration
    }
}