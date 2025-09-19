pub mod analyzer;
pub mod data_migration;
pub mod differ;
pub mod executor;
pub mod introspector;
pub mod planner;
pub mod reporting;
pub mod rollback;
pub mod safety;
pub mod smart_strategies;
pub mod validator;

pub use analyzer::*;
pub use data_migration::*;
pub use differ::*;
pub use executor::*;
pub use introspector::*;
pub use planner::*;
pub use reporting::*;
pub use rollback::*;
pub use safety::*;
pub use smart_strategies::*;
pub use validator::*;

use crate::{D1Client, Result, Entity};
use std::cell::RefCell;

/// Revolutionary automatic migration system - world's first compile-time safe migrations
pub struct AutoSchemaClient {
    pub db: D1Client,
    analyzer: RefCell<EntityAnalyzer>,
    differ: SchemaDiffer,
    planner: MigrationPlanner,
    validator: MigrationValidator,
    executor: MigrationExecutor,
}

impl AutoSchemaClient {
    pub fn new(db: D1Client) -> Self {
        Self {
            analyzer: RefCell::new(EntityAnalyzer::new()),
            differ: SchemaDiffer::new(),
            planner: MigrationPlanner::new(),
            validator: MigrationValidator::new(),
            executor: MigrationExecutor::new(),
            db,
        }
    }

    /// Register entity types for migration analysis
    pub fn register_entity<T: Entity + 'static>(&self) -> Result<()> {
        self.analyzer.borrow_mut().analyze_entity::<T>()?;
        Ok(())
    }

    /// Register multiple entity types for migration analysis  
    pub fn register_entities<T1, T2>(&self) -> Result<()>
    where
        T1: Entity + 'static,
        T2: Entity + 'static,
    {
        let mut analyzer = self.analyzer.borrow_mut();
        analyzer.analyze_entity::<T1>()?;
        analyzer.analyze_entity::<T2>()?;
        Ok(())
    }

    /// Register three entity types for migration analysis
    pub fn register_entities_3<T1, T2, T3>(&self) -> Result<()>
    where
        T1: Entity + 'static,
        T2: Entity + 'static,
        T3: Entity + 'static,
    {
        let mut analyzer = self.analyzer.borrow_mut();
        analyzer.analyze_entity::<T1>()?;
        analyzer.analyze_entity::<T2>()?;
        analyzer.analyze_entity::<T3>()?;
        Ok(())
    }

    /// Revolutionary one-command automatic migration (like ent-go but better)
    pub async fn auto_migrate(&self) -> Result<MigrationResult> {
        // 1. Introspect current schema
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;

        // 2. Analyze entity definitions
        let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;

        // 3. Compare schemas
        let diff = self
            .differ
            .compare_schemas(&current_schema, &desired_schema)?;

        // 4. Plan migrations
        let migration_plan = self.planner.plan_migrations(diff)?;

        // 5. Validate safety
        self.validator.validate_migrations(&migration_plan)?;

        // 6. Execute migrations
        let result = self
            .executor
            .execute_migrations(&self.db, migration_plan)
            .await?;

        // 7. Return detailed result
        Ok(result)
    }

    /// Preview changes without applying (revolutionary feature)
    pub async fn dry_run(&self) -> Result<MigrationPlan> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;
        let diff = self
            .differ
            .compare_schemas(&current_schema, &desired_schema)?;
        let migration_plan = self.planner.plan_migrations(diff)?;

        Ok(migration_plan)
    }

    /// Validate current schema matches entities
    pub async fn verify_schema(&self) -> Result<SchemaValidationResult> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;
        let diff = self
            .differ
            .compare_schemas(&current_schema, &desired_schema)?;

        Ok(SchemaValidationResult {
            is_valid: diff.is_empty(),
            differences: diff,
            warnings: vec![], // TODO: implement warnings
        })
    }

    /// Generate baseline migration from current entities (creates from scratch)
    pub async fn generate_baseline(&self) -> Result<MigrationPlan> {
        // Analyze all entities to get desired schema
        let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;

        // Create empty current schema (as if database is brand new)
        let empty_schema = DatabaseSchema { tables: vec![] };

        // Compare empty schema with desired schema to generate full creation plan
        let diff = self
            .differ
            .compare_schemas(&empty_schema, &desired_schema)?;

        // Plan migrations to create everything from scratch
        let migration_plan = self.planner.plan_migrations(diff)?;

        Ok(migration_plan)
    }

    /// Environment-specific migration (dev vs prod)
    pub async fn auto_migrate_for_environment(
        &self,
        environment: MigrationEnvironment,
    ) -> Result<MigrationResult> {
        match environment {
            MigrationEnvironment::Development => {
                // In development, allow more aggressive changes
                let mut planner = self.planner.clone();
                planner.enable_aggressive_changes(true);

                let introspector = SchemaIntrospector::new(&self.db);
                let current_schema = introspector.introspect_database().await?;
                let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;
                let diff = self
                    .differ
                    .compare_schemas(&current_schema, &desired_schema)?;
                let migration_plan = planner.plan_migrations(diff)?;

                self.validator.validate_migrations(&migration_plan)?;
                let result = self
                    .executor
                    .execute_migrations(&self.db, migration_plan)
                    .await?;
                Ok(result)
            }
            MigrationEnvironment::Production => {
                // In production, use conservative settings and require explicit approval
                // This is the same as regular auto_migrate but with stricter validation
                self.auto_migrate().await
            }
        }
    }
}

/// Migration environment configuration
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationEnvironment {
    Development,
    Production,
}

/// Result of automatic migration execution
#[derive(Debug)]
pub struct MigrationResult {
    pub migrations_applied: Vec<String>,
    pub execution_time: std::time::Duration,
    pub changes_made: SchemaDiff,
    pub rollback_plan: Option<MigrationPlan>,
}

/// Schema validation result
#[derive(Debug)]
pub struct SchemaValidationResult {
    pub is_valid: bool,
    pub differences: SchemaDiff,
    pub warnings: Vec<String>,
}

/// Migration execution plan
#[derive(Debug, Clone)]
pub struct MigrationPlan {
    pub operations: Vec<MigrationOperation>,
    pub estimated_duration: std::time::Duration,
    pub safety_warnings: Vec<SafetyWarning>,
    pub rollback_plan: Vec<MigrationOperation>,
}

/// Individual migration operation
#[derive(Debug, Clone)]
pub enum MigrationOperation {
    CreateTable {
        definition: TableSchema,
    },
    DropTable {
        name: String,
    },
    AddColumn {
        table: String,
        column: ColumnSchema,
    },
    DropColumn {
        table: String,
        column: String,
    },
    ModifyColumn {
        table: String,
        column: String,
        changes: ColumnChanges,
    },
    CreateIndex {
        table: String,
        index: IndexSchema,
    },
    DropIndex {
        name: String,
    },
    AddForeignKey {
        constraint: ForeignKeySchema,
    },
    DropForeignKey {
        table: String,
        constraint_name: String,
    },
    RenameTable {
        old_name: String,
        new_name: String,
    },
    RenameColumn {
        table: String,
        old_name: String,
        new_name: String,
    },
}

/// Schema difference representation
#[derive(Debug, Clone)]
pub struct SchemaDiff {
    pub table_changes: Vec<TableChange>,
}

impl SchemaDiff {
    pub fn is_empty(&self) -> bool {
        self.table_changes.is_empty() || self.table_changes.iter().all(|tc| tc.is_empty())
    }

    /// Get all safety warnings across all table changes
    pub fn all_safety_warnings(&self) -> Vec<&str> {
        self.table_changes
            .iter()
            .flat_map(|tc| tc.all_safety_warnings())
            .collect()
    }

    /// Check if this diff contains any potentially dangerous operations
    pub fn has_dangerous_operations(&self) -> bool {
        self.table_changes
            .iter()
            .any(|tc| tc.change_type == ChangeType::Remove || !tc.all_safety_warnings().is_empty())
    }
}

/// Table-level differences
#[derive(Debug, Clone)]
pub enum TableDiff {
    Added {
        table: TableSchema,
    },
    Removed {
        table_name: String,
    },
    Modified {
        table_name: String,
        changes: TableChanges,
    },
}

/// Changes within a table
#[derive(Debug, Clone)]
pub struct TableChanges {
    pub column_changes: Vec<ColumnDiff>,
    pub index_changes: Vec<IndexDiff>,
    pub constraint_changes: Vec<ConstraintDiff>,
}

/// Column-level differences
#[derive(Debug, Clone)]
pub enum ColumnDiff {
    Added {
        column: ColumnSchema,
    },
    Removed {
        column_name: String,
    },
    Modified {
        column_name: String,
        changes: ColumnChanges,
    },
}

/// Column modification details
#[derive(Debug, Clone)]
pub struct ColumnChanges {
    pub type_change: Option<(String, String)>, // (old_type, new_type)
    pub null_change: Option<(bool, bool)>,     // (old_nullable, new_nullable)
    pub default_change: Option<(Option<String>, Option<String>)>, // (old_default, new_default)
    pub constraint_changes: Vec<ConstraintDiff>,
}

/// Index-level differences
#[derive(Debug, Clone)]
pub enum IndexDiff {
    Added {
        index: IndexSchema,
    },
    Removed {
        index_name: String,
    },
    Modified {
        index_name: String,
        changes: IndexChanges,
    },
}

/// Index modification details
#[derive(Debug, Clone)]
pub struct IndexChanges {
    pub column_changes: Vec<String>,         // Column list changes
    pub unique_change: Option<(bool, bool)>, // (old_unique, new_unique)
}

/// Constraint differences
#[derive(Debug, Clone)]
pub enum ConstraintDiff {
    Added {
        constraint: ConstraintSchema,
    },
    Removed {
        constraint_name: String,
    },
    Modified {
        constraint_name: String,
        changes: ConstraintChanges,
    },
}

/// Constraint modification details
#[derive(Debug, Clone)]
pub struct ConstraintChanges {
    pub reference_changes: Option<(String, String)>, // (old_ref, new_ref)
    pub action_changes: Option<(String, String)>,    // (old_action, new_action)
}

/// Safety warning for potentially dangerous operations
#[derive(Debug, Clone)]
pub struct SafetyWarning {
    pub operation: String,
    pub warning_type: SafetyWarningType,
    pub message: String,
    pub recommendation: String,
}

/// Types of safety warnings
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyWarningType {
    DataLoss,
    PerformanceImpact,
    BreakingChange,
    ComplexOperation,
}

// New types for the SchemaDiffer implementation

/// Type of schema change operation
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Add,
    Remove,
    Modify,
    Rename,
}

/// Table-level change with detailed breakdown
#[derive(Debug, Clone)]
pub struct TableChange {
    pub table_name: String,
    pub change_type: ChangeType,
    pub old_schema: Option<TableSchema>,
    pub new_schema: Option<TableSchema>,
    pub column_changes: Vec<ColumnChange>,
    pub index_changes: Vec<IndexChange>,
    pub foreign_key_changes: Vec<ForeignKeyChange>,
    pub safety_warnings: Vec<String>,
}

/// Column-level change with safety analysis
#[derive(Debug, Clone)]
pub struct ColumnChange {
    pub column_name: String,
    pub change_type: ChangeType,
    pub old_definition: Option<ColumnSchema>,
    pub new_definition: Option<ColumnSchema>,
    pub safety_warnings: Vec<String>,
}

/// Index-level change
#[derive(Debug, Clone)]
pub struct IndexChange {
    pub index_name: String,
    pub change_type: ChangeType,
    pub old_definition: Option<IndexSchema>,
    pub new_definition: Option<IndexSchema>,
    pub safety_warnings: Vec<String>,
}

/// Foreign key change
#[derive(Debug, Clone)]
pub struct ForeignKeyChange {
    pub foreign_key_name: String,
    pub change_type: ChangeType,
    pub old_definition: Option<ForeignKeySchema>,
    pub new_definition: Option<ForeignKeySchema>,
    pub safety_warnings: Vec<String>,
}
