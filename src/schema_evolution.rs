use crate::{D1Client, Result, D1RsError};
use crate::schema::{ColumnType, DefaultValue, TableDefinition};
use crate::relations::{RelationBuilder, RelationType};
use serde_json::Value;

/// Schema evolution operations for migrations
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
    /// Modify an existing column
    ModifyColumn {
        table: String,
        column: String,
        new_definition: ColumnDefinition,
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
    /// Add a foreign key constraint
    AddForeignKey {
        table: String,
        constraint_name: String,
        column: String,
        references_table: String,
        references_column: String,
        on_delete: Option<ForeignKeyAction>,
        on_update: Option<ForeignKeyAction>,
    },
    /// Drop a foreign key constraint
    DropForeignKey {
        table: String,
        constraint_name: String,
    },
    /// Create a relation between tables
    CreateRelation {
        relation: RelationBuilder,
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
    Restrict,
    NoAction,
}

impl ForeignKeyAction {
    fn to_sql(&self) -> &'static str {
        match self {
            ForeignKeyAction::Cascade => "CASCADE",
            ForeignKeyAction::SetNull => "SET NULL",
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
            
            SchemaOperation::DropColumn { table, column } => {
                // SQLite doesn't support DROP COLUMN directly, need to recreate table
                Err(D1RsError::Database(
                    "SQLite doesn't support DROP COLUMN. Use a custom migration instead.".to_string()
                ))
            }
            
            SchemaOperation::ModifyColumn { table, column, new_definition } => {
                // SQLite doesn't support ALTER COLUMN directly, need to recreate table
                Err(D1RsError::Database(
                    "SQLite doesn't support ALTER COLUMN. Use a custom migration instead.".to_string()
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
            
            SchemaOperation::AddForeignKey {
                table,
                constraint_name,
                column,
                references_table,
                references_column,
                on_delete,
                on_update,
            } => {
                let mut sql = format!(
                    "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({})",
                    table, constraint_name, column, references_table, references_column
                );
                
                if let Some(on_delete) = on_delete {
                    sql.push_str(&format!(" ON DELETE {}", on_delete.to_sql()));
                }
                
                if let Some(on_update) = on_update {
                    sql.push_str(&format!(" ON UPDATE {}", on_update.to_sql()));
                }
                
                Ok(vec![sql])
            }
            
            SchemaOperation::DropForeignKey { table, constraint_name } => {
                // SQLite doesn't support dropping foreign key constraints directly
                Err(D1RsError::Database(
                    "SQLite doesn't support DROP CONSTRAINT. Use a custom migration instead.".to_string()
                ))
            }
            
            SchemaOperation::CreateRelation { relation } => {
                Ok(relation.to_sql())
            }
            
            SchemaOperation::RawSql { sql } => {
                Ok(vec![sql.clone()])
            }
        }
    }
}

/// Schema migration builder with fluent API
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
    pub fn create_table(mut self, name: &str) -> TableMigrationBuilder {
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
    pub fn alter_table(mut self, name: &str) -> AlterTableBuilder {
        AlterTableBuilder::new(self, name.to_string())
    }
    
    /// Create a relation between tables
    pub fn create_relation(mut self, name: &str, from_table: &str, to_table: &str) -> RelationMigrationBuilder {
        RelationMigrationBuilder::new(self, name.to_string(), from_table.to_string(), to_table.to_string())
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
    pub fn integer(mut self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::Integer)
    }
    
    /// Add a text column
    pub fn text(mut self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::Text)
    }
    
    /// Add a boolean column
    pub fn boolean(mut self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::Boolean)
    }
    
    /// Add a datetime column
    pub fn datetime(mut self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::DateTime)
    }
    
    /// Add a real/float column
    pub fn real(mut self, name: &str) -> ColumnMigrationBuilder {
        ColumnMigrationBuilder::new(self, name.to_string(), ColumnType::Real)
    }
    
    /// Add a JSON column
    pub fn json(mut self, name: &str) -> ColumnMigrationBuilder {
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
    pub fn add_column(mut self, name: &str, column_type: ColumnType) -> ColumnMigrationBuilder {
        let table_builder = TableMigrationBuilder {
            migration: self.migration,
            table_name: self.table_name,
            columns: Vec::new(),
        };
        ColumnMigrationBuilder::new(table_builder, name.to_string(), column_type)
    }
    
    /// Rename a column
    pub fn rename_column(mut self, old_name: &str, new_name: &str) -> Self {
        self.migration.operations.push(SchemaOperation::RenameColumn {
            table: self.table_name.clone(),
            old_name: old_name.to_string(),
            new_name: new_name.to_string(),
        });
        self.migration
    }
    
    /// Add an index
    pub fn add_index(mut self, name: &str, columns: Vec<&str>) -> Self {
        self.migration.operations.push(SchemaOperation::AddIndex {
            table: self.table_name.clone(),
            name: name.to_string(),
            columns: columns.iter().map(|s| s.to_string()).collect(),
            unique: false,
        });
        self.migration
    }
    
    /// Add a unique index
    pub fn add_unique_index(mut self, name: &str, columns: Vec<&str>) -> Self {
        self.migration.operations.push(SchemaOperation::AddIndex {
            table: self.table_name.clone(),
            name: name.to_string(),
            columns: columns.iter().map(|s| s.to_string()).collect(),
            unique: true,
        });
        self.migration
    }
}

/// Builder for creating relations in migrations
pub struct RelationMigrationBuilder {
    migration: SchemaMigration,
    relation_builder: RelationBuilder,
}

impl RelationMigrationBuilder {
    fn new(migration: SchemaMigration, name: String, from_table: String, to_table: String) -> Self {
        Self {
            migration,
            relation_builder: RelationBuilder::new(name, from_table, to_table),
        }
    }
    
    /// Set as one-to-one relation
    pub fn one_to_one(mut self, foreign_key: &str, references: &str) -> Self {
        self.relation_builder = self.relation_builder.one_to_one(
            foreign_key.to_string(),
            references.to_string(),
        );
        self
    }
    
    /// Set as one-to-many relation
    pub fn one_to_many(mut self, foreign_key: &str, references: &str) -> Self {
        self.relation_builder = self.relation_builder.one_to_many(
            foreign_key.to_string(),
            references.to_string(),
        );
        self
    }
    
    /// Set as many-to-many relation
    pub fn many_to_many(
        mut self,
        junction_table: &str,
        foreign_key: &str,
        references: &str,
        target_foreign_key: &str,
        target_references: &str,
    ) -> Self {
        self.relation_builder = self.relation_builder.many_to_many(
            junction_table.to_string(),
            foreign_key.to_string(),
            references.to_string(),
            target_foreign_key.to_string(),
            target_references.to_string(),
        );
        self
    }
    
    /// Finish building the relation
    pub fn build(mut self) -> SchemaMigration {
        self.migration.operations.push(SchemaOperation::CreateRelation {
            relation: self.relation_builder,
        });
        self.migration
    }
}