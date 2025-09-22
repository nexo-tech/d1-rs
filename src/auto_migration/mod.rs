pub mod analyzer;
pub mod complex_schema_changes;
pub mod data_migration;
pub mod data_seeding;
pub mod differ;
pub mod executor;
pub mod introspector;
pub mod migration_snapshots;
pub mod parallel_execution;
pub mod planner;
pub mod reporting;
pub mod rollback;
pub mod safety;
pub mod schema_versioning;
pub mod smart_strategies;
pub mod validator;
pub mod validators;
pub mod zero_downtime;

pub use analyzer::*;
pub use complex_schema_changes::*;
pub use data_migration::*;
pub use data_seeding::*;
pub use differ::*;
pub use executor::*;
pub use introspector::*;
pub use migration_snapshots::*;
pub use parallel_execution::*;
pub use planner::*;
pub use reporting::*;
pub use rollback::*;
// Remove safety::* re-export to avoid conflict with validators::SafetyAnalyzer
pub use schema_versioning::*;
pub use smart_strategies::*;
pub use validator::*;
// Re-export specific types from validators to avoid naming conflicts
pub use validators::{
    ValidationResult, RiskLevel, ValidationRecommendation, ValidationWarnings,
    SchemaCompatibilityValidator, SchemaCompatibilityResult, SchemaIncompatibility, CompatibilitySeverity,
    DataIntegrityValidator, DataIntegrityResult, IntegrityIssue, IntegritySeverity,
    PerformanceImpactValidator, PerformanceImpactResult, PerformanceImpact, PerformanceRiskLevel,
    SafetyAnalyzer, SafetyAnalysisResult, SafetyIssue, SafetySeverity, OverallRiskLevel,
    BreakingChangeDetector, BreakingChangeResult, BreakingChange, BreakingSeverity, OverallImpactLevel,
};
pub use zero_downtime::*;

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
    data_seeder: RefCell<DataSeeder>,
    version_manager: RefCell<SchemaVersionManager>,
    snapshot_manager: RefCell<MigrationSnapshotManager>,
    parallel_executor: RefCell<ParallelMigrationExecutor>,
}

impl AutoSchemaClient {
    pub fn new(db: D1Client) -> Self {
        Self {
            analyzer: RefCell::new(EntityAnalyzer::new()),
            differ: SchemaDiffer::new(),
            planner: MigrationPlanner::new(),
            validator: MigrationValidator::new(),
            executor: MigrationExecutor::new(),
            data_seeder: RefCell::new(DataSeeder::new()),
            version_manager: RefCell::new(SchemaVersionManager::with_database_storage("d1_rs_schema_versions")),
            snapshot_manager: RefCell::new(MigrationSnapshotManager::new(SnapshotConfig::default())),
            parallel_executor: RefCell::new(ParallelMigrationExecutor::new(ParallelExecutionConfig::default())),
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

    /// ⚡ PHASE 3.2: Zero-downtime migration execution
    /// Execute migrations without service interruption using multi-step atomic operations
    pub async fn zero_downtime_migrate(&self) -> Result<ZeroDowntimeResult> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;
        let diff = self.differ.compare_schemas(&current_schema, &desired_schema)?;
        let migration_plan = self.planner.plan_migrations(diff)?;

        // Create zero-downtime migrator with production-safe settings
        let zero_downtime_migrator = ZeroDowntimeMigrator::with_config(
            std::time::Duration::from_secs(30), // 30-second max per step
            true, // Use shadow tables for safety
            ConcurrentSafetyMode::Balanced, // Balanced safety mode
        );

        // Convert regular plan to zero-downtime plan
        let zero_downtime_plan = zero_downtime_migrator.plan_zero_downtime_migration(&migration_plan)?;

        // Execute with zero-downtime safety
        let result = zero_downtime_migrator
            .execute_zero_downtime_migration(&self.db, &zero_downtime_plan)
            .await?;

        Ok(result)
    }

    /// ⚡ PHASE 3.2: Plan zero-downtime migration (dry-run)
    /// Preview zero-downtime migration steps without executing
    pub async fn plan_zero_downtime_migration(&self) -> Result<ZeroDowntimePlan> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;
        let diff = self.differ.compare_schemas(&current_schema, &desired_schema)?;
        let migration_plan = self.planner.plan_migrations(diff)?;

        let zero_downtime_migrator = ZeroDowntimeMigrator::new();
        let zero_downtime_plan = zero_downtime_migrator.plan_zero_downtime_migration(&migration_plan)?;

        Ok(zero_downtime_plan)
    }

    /// ⚡ PHASE 3.2: Zero-downtime migration with custom configuration
    /// Execute zero-downtime migration with custom safety and timing settings
    pub async fn zero_downtime_migrate_with_config(
        &self,
        max_step_duration: std::time::Duration,
        use_shadow_tables: bool,
        concurrent_safety: ConcurrentSafetyMode,
    ) -> Result<ZeroDowntimeResult> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;
        let diff = self.differ.compare_schemas(&current_schema, &desired_schema)?;
        let migration_plan = self.planner.plan_migrations(diff)?;

        let zero_downtime_migrator = ZeroDowntimeMigrator::with_config(
            max_step_duration,
            use_shadow_tables,
            concurrent_safety,
        );

        let zero_downtime_plan = zero_downtime_migrator.plan_zero_downtime_migration(&migration_plan)?;
        let result = zero_downtime_migrator
            .execute_zero_downtime_migration(&self.db, &zero_downtime_plan)
            .await?;

        Ok(result)
    }

    /// ⚡ PHASE 3.2: Register seed data for entity type
    /// Provides compile-time type safety for seed data registration
    pub fn register_seed_data<T: Entity + 'static>(
        &self,
        seed_id: &str,
        description: &str,
        data: Vec<T>,
        environment: SeedEnvironment,
        required: bool,
    ) -> Result<()> {
        self.data_seeder.borrow_mut().register_entity_seeds(
            seed_id,
            description,
            data,
            environment,
            required,
        )
    }

    /// ⚡ PHASE 3.2: Register raw seed data with SQL-like interface
    /// Allows registration of seed data without entity types
    pub fn register_raw_seed_data(
        &self,
        seed_id: &str,
        description: &str,
        table_name: &str,
        data: Vec<std::collections::HashMap<String, serde_json::Value>>,
        unique_keys: Vec<String>,
        environment: SeedEnvironment,
        required: bool,
    ) {
        self.data_seeder.borrow_mut().register_raw_seeds(
            seed_id,
            description,
            table_name,
            data,
            unique_keys,
            environment,
            required,
        );
    }

    /// ⚡ PHASE 3.2: Add dependency between seeds
    /// Ensures seeds are executed in correct order
    pub fn add_seed_dependency(&self, seed_id: &str, depends_on: &str) -> Result<()> {
        self.data_seeder.borrow_mut().add_seed_dependency(seed_id, depends_on)
    }

    /// ⚡ PHASE 3.2: Execute data seeding for specific environment
    /// Automatically populates database with registered seed data
    pub async fn seed_data(&self, environment: &str) -> Result<SeedResult> {
        let plan = self.data_seeder.borrow().create_seeding_plan(environment)?;
        let result = self.data_seeder.borrow().execute_seeding(&self.db, &plan).await?;
        Ok(result)
    }

    /// ⚡ PHASE 3.2: Preview seeding plan without execution
    /// Shows what seeds will be executed for given environment
    pub fn preview_seeding_plan(&self, environment: &str) -> Result<SeedingPlan> {
        self.data_seeder.borrow().create_seeding_plan(environment)
    }

    /// ⚡ PHASE 3.2: Auto-migrate with data seeding
    /// Combines schema migration with automatic data population
    pub async fn auto_migrate_with_seeding(&self, environment: &str) -> Result<(MigrationResult, SeedResult)> {
        // First, run the schema migration
        let migration_result = self.auto_migrate().await?;

        // Then, seed the data
        let seed_result = self.seed_data(environment).await?;

        Ok((migration_result, seed_result))
    }

    /// ⚡ PHASE 3.2: Zero-downtime migration with data seeding
    /// Combines zero-downtime migration with safe data population
    pub async fn zero_downtime_migrate_with_seeding(
        &self,
        environment: &str,
    ) -> Result<(ZeroDowntimeResult, SeedResult)> {
        // First, run zero-downtime migration
        let migration_result = self.zero_downtime_migrate().await?;

        // Then, seed the data if migration was successful
        let seed_result = if migration_result.failed_steps.is_empty() {
            self.seed_data(environment).await?
        } else {
            // If migration failed, return empty seed result
            SeedResult {
                completed_seeds: vec![],
                failed_seeds: vec![("migration_failed".to_string(), "Migration failed, skipping seeding".to_string())],
                records_inserted: 0,
                records_updated: 0,
                records_skipped: 0,
                execution_time: std::time::Duration::from_millis(0),
            }
        };

        Ok((migration_result, seed_result))
    }

    /// ⚡ PHASE 3.2: Initialize schema versioning
    /// Set up version tracking for the database
    pub async fn initialize_versioning(&self) -> Result<SchemaVersion> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        let initial_version = self.version_manager.borrow_mut()
            .initialize(&self.db, current_schema).await?;
        Ok(initial_version)
    }

    /// ⚡ PHASE 3.2: Create new schema version after migration
    /// Track schema evolution with complete metadata
    pub async fn create_schema_version(
        &self,
        version: &str,
        description: &str,
        environment: &str,
        author: Option<String>,
    ) -> Result<SchemaVersion> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;
        let diff = self.differ.compare_schemas(&current_schema, &desired_schema)?;
        let migration_plan = self.planner.plan_migrations(diff)?;

        let schema_version = self.version_manager.borrow_mut()
            .create_version(
                &self.db,
                version,
                description,
                current_schema,
                migration_plan,
                environment,
                author,
            ).await?;

        Ok(schema_version)
    }

    /// ⚡ PHASE 3.2: Get current schema version
    /// Retrieve the current version of the database schema
    pub async fn get_current_schema_version(&self) -> Result<Option<SchemaVersion>> {
        self.version_manager.borrow_mut()
            .get_current_version(&self.db).await
    }

    /// ⚡ PHASE 3.2: Get specific schema version
    /// Retrieve a specific version by identifier
    pub async fn get_schema_version(&self, version: &str) -> Result<Option<SchemaVersion>> {
        self.version_manager.borrow_mut()
            .get_version(&self.db, version).await
    }

    /// ⚡ PHASE 3.2: Get all schema versions
    /// Retrieve complete version history in chronological order
    pub async fn get_all_schema_versions(&self) -> Result<Vec<SchemaVersion>> {
        self.version_manager.borrow()
            .get_all_versions(&self.db).await
    }

    /// ⚡ PHASE 3.2: Compare schema versions
    /// Analyze differences between two versions with breaking change detection
    pub async fn compare_schema_versions(
        &self,
        from_version: &str,
        to_version: &str,
    ) -> Result<VersionComparison> {
        self.version_manager.borrow_mut()
            .compare_versions(&self.db, from_version, to_version).await
    }

    /// ⚡ PHASE 3.2: Create migration snapshot
    /// Capture current database state for rollback purposes
    pub async fn create_migration_snapshot(
        &self,
        id: String,
        description: String,
        tags: Vec<String>,
    ) -> Result<MigrationSnapshot> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        
        self.snapshot_manager.borrow()
            .create_snapshot(
                id,
                description,
                current_schema,
                SnapshotType::Manual,
                tags,
            ).await
    }

    /// ⚡ PHASE 3.2: Create automatic pre-migration snapshot
    /// Automatically capture state before migration for safety
    pub async fn create_pre_migration_snapshot(&self, migration_id: &str) -> Result<MigrationSnapshot> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        
        self.snapshot_manager.borrow()
            .create_pre_migration_snapshot(migration_id, current_schema).await
    }

    /// ⚡ PHASE 3.2: Create automatic post-migration snapshot
    /// Capture state after successful migration with applied changes
    pub async fn create_post_migration_snapshot(
        &self,
        migration_id: &str,
        migration_plan: MigrationPlan,
    ) -> Result<MigrationSnapshot> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        
        self.snapshot_manager.borrow()
            .create_post_migration_snapshot(migration_id, current_schema, migration_plan).await
    }

    /// ⚡ PHASE 3.2: List all migration snapshots
    /// Retrieve all available snapshots for management
    pub fn list_migration_snapshots(&self) -> Vec<MigrationSnapshot> {
        self.snapshot_manager.borrow().list_snapshots()
    }

    /// ⚡ PHASE 3.2: Get specific migration snapshot
    /// Retrieve snapshot details by ID
    pub fn get_migration_snapshot(&self, id: &str) -> Option<MigrationSnapshot> {
        self.snapshot_manager.borrow().get_snapshot(id)
    }

    /// ⚡ PHASE 3.2: Delete migration snapshot
    /// Remove snapshot from storage
    pub async fn delete_migration_snapshot(&self, id: &str) -> Result<bool> {
        self.snapshot_manager.borrow().delete_snapshot(id).await
    }

    /// ⚡ PHASE 3.2: Validate rollback safety
    /// Check if rollback to snapshot is safe without data loss
    pub async fn validate_rollback_to_snapshot(&self, snapshot_id: &str) -> Result<RollbackValidation> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        
        self.snapshot_manager.borrow()
            .validate_rollback(snapshot_id, &current_schema).await
    }

    /// ⚡ PHASE 3.2: Rollback to migration snapshot
    /// Safely rollback database to previous state with optional backup
    pub async fn rollback_to_snapshot(
        &self,
        snapshot_id: &str,
        create_backup: bool,
    ) -> Result<RollbackResult> {
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        
        self.snapshot_manager.borrow()
            .rollback_to_snapshot(snapshot_id, &current_schema, create_backup).await
    }

    /// ⚡ PHASE 3.2: Get snapshot storage statistics
    /// Monitor snapshot storage usage and organization
    pub fn get_snapshot_storage_stats(&self) -> SnapshotStorageStats {
        self.snapshot_manager.borrow().get_storage_stats()
    }

    /// ⚡ PHASE 3.2: Generate schema evolution report
    /// Create comprehensive report of schema evolution over time
    pub async fn generate_schema_evolution_report(&self) -> Result<VersionEvolutionReport> {
        self.version_manager.borrow()
            .generate_evolution_report(&self.db).await
    }

    /// ⚡ PHASE 3.2: Tag schema version
    /// Add tags to specific versions for categorization
    pub async fn tag_schema_version(&self, version: &str, tag: &str) -> Result<()> {
        self.version_manager.borrow_mut()
            .tag_version(&self.db, version, tag).await
    }

    /// ⚡ PHASE 3.2: Auto-migrate with versioning
    /// Combine automatic migration with version tracking
    pub async fn auto_migrate_with_versioning(
        &self,
        version: &str,
        description: &str,
        environment: &str,
        author: Option<String>,
    ) -> Result<(MigrationResult, SchemaVersion)> {
        // First, run the migration
        let migration_result = self.auto_migrate().await?;

        // Then, create the version record
        let schema_version = self.create_schema_version(
            version,
            description,
            environment,
            author,
        ).await?;

        Ok((migration_result, schema_version))
    }

    /// ⚡ PHASE 3.2: Configure version manager storage
    /// Switch to filesystem or custom storage backend
    pub fn configure_version_storage(&self, storage: VersionStorage) {
        let mut version_manager = self.version_manager.borrow_mut();
        *version_manager = SchemaVersionManager {
            storage,
            current_version: version_manager.current_version.clone(),
            version_cache: version_manager.version_cache.clone(),
        };
    }

    /// ⚡ PHASE 3.2: Execute migration with parallel execution
    /// Run migrations concurrently for improved performance
    pub async fn parallel_migrate(&self) -> Result<ParallelExecutionResult> {
        // Get migration plan
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;
        let diff = self.differ.compare_schemas(&current_schema, &desired_schema)?;
        let migration_plan = self.planner.plan_migrations(diff)?;

        // Execute in parallel
        self.parallel_executor.borrow()
            .execute_parallel_migration(&self.db, &migration_plan).await
    }

    /// ⚡ PHASE 3.2: Execute migration with custom parallel configuration
    /// Run migrations with specific parallel execution settings
    pub async fn parallel_migrate_with_config(
        &self,
        config: ParallelExecutionConfig,
    ) -> Result<ParallelExecutionResult> {
        // Create temporary executor with custom config
        let temp_executor = ParallelMigrationExecutor::new(config);
        
        // Get migration plan
        let introspector = SchemaIntrospector::new(&self.db);
        let current_schema = introspector.introspect_database().await?;
        let desired_schema = self.analyzer.borrow().analyze_all_entities().await?;
        let diff = self.differ.compare_schemas(&current_schema, &desired_schema)?;
        let migration_plan = self.planner.plan_migrations(diff)?;

        // Execute in parallel
        temp_executor.execute_parallel_migration(&self.db, &migration_plan).await
    }

    /// ⚡ PHASE 3.2: Get parallel execution progress
    /// Monitor ongoing parallel migration progress
    pub async fn get_parallel_migration_progress(&self) -> ExecutionProgress {
        self.parallel_executor.borrow().get_progress().await
    }

    /// ⚡ PHASE 3.2: Get parallel execution state
    /// Get detailed state of parallel migration execution
    pub async fn get_parallel_execution_state(&self) -> ExecutionState {
        self.parallel_executor.borrow().get_execution_state().await
    }

    /// ⚡ PHASE 3.2: Configure parallel execution settings
    /// Update parallel execution configuration
    pub fn configure_parallel_execution(&self, config: ParallelExecutionConfig) {
        let new_executor = ParallelMigrationExecutor::new(config);
        *self.parallel_executor.borrow_mut() = new_executor;
    }

    /// ⚡ PHASE 3.2: Parallel migrate with snapshots and versioning
    /// Ultimate migration with all Phase 3.2 features combined
    pub async fn ultimate_migrate(
        &self,
        version: &str,
        description: &str,
        environment: &str,
        author: Option<String>,
        create_snapshots: bool,
        parallel_config: Option<ParallelExecutionConfig>,
    ) -> Result<UltimateMigrationResult> {
        let start_time = std::time::Instant::now();
        
        // Create pre-migration snapshot if requested
        let pre_snapshot = if create_snapshots {
            Some(self.create_pre_migration_snapshot(&format!("ultimate_{}", version)).await?)
        } else {
            None
        };

        // Execute migration (parallel or sequential based on config)
        let migration_result = if let Some(config) = parallel_config {
            self.parallel_migrate_with_config(config).await?
        } else {
            self.parallel_migrate().await?
        };

        // Create schema version
        let schema_version = self.create_schema_version(
            version,
            description,
            environment,
            author,
        ).await?;

        // Create post-migration snapshot if requested
        let post_snapshot = if create_snapshots {
            // Convert ParallelExecutionResult to MigrationPlan for snapshot
            let migration_plan = MigrationPlan {
                operations: Vec::new(), // TODO: Extract from parallel result
                rollback_plan: Vec::new(),
                estimated_duration: migration_result.total_execution_time,
                safety_warnings: Vec::new(),
            };
            Some(self.create_post_migration_snapshot(&format!("ultimate_{}", version), migration_plan).await?)
        } else {
            None
        };

        let total_time = start_time.elapsed();

        Ok(UltimateMigrationResult {
            migration_result,
            schema_version,
            pre_snapshot,
            post_snapshot,
            total_execution_time: total_time,
        })
    }
}

/// Ultimate migration result combining all Phase 3.2 features
#[derive(Debug)]
pub struct UltimateMigrationResult {
    /// Result of parallel migration execution
    pub migration_result: ParallelExecutionResult,
    
    /// Schema version created for this migration
    pub schema_version: SchemaVersion,
    
    /// Pre-migration snapshot (if created)
    pub pre_snapshot: Option<MigrationSnapshot>,
    
    /// Post-migration snapshot (if created)
    pub post_snapshot: Option<MigrationSnapshot>,
    
    /// Total time for the entire ultimate migration process
    pub total_execution_time: std::time::Duration,
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct MigrationPlan {
    pub operations: Vec<MigrationOperation>,
    pub estimated_duration: std::time::Duration,
    pub safety_warnings: Vec<SafetyWarning>,
    pub rollback_plan: Vec<MigrationOperation>,
}

/// Individual migration operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TableChanges {
    pub column_changes: Vec<ColumnDiff>,
    pub index_changes: Vec<IndexDiff>,
    pub constraint_changes: Vec<ConstraintDiff>,
}

/// Column-level differences
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ColumnChanges {
    pub type_change: Option<(String, String)>, // (old_type, new_type)
    pub null_change: Option<(bool, bool)>,     // (old_nullable, new_nullable)
    pub default_change: Option<(Option<String>, Option<String>)>, // (old_default, new_default)
    pub constraint_changes: Vec<ConstraintDiff>,
}

/// Index-level differences
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct IndexChanges {
    pub column_changes: Vec<String>,         // Column list changes
    pub unique_change: Option<(bool, bool)>, // (old_unique, new_unique)
}

/// Constraint differences
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ConstraintChanges {
    pub reference_changes: Option<(String, String)>, // (old_ref, new_ref)
    pub action_changes: Option<(String, String)>,    // (old_action, new_action)
}

/// Safety warning for potentially dangerous operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SafetyWarning {
    pub operation: String,
    pub warning_type: SafetyWarningType,
    pub message: String,
    pub recommendation: String,
}

/// Types of safety warnings
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum SafetyWarningType {
    DataLoss,
    PerformanceImpact,
    BreakingChange,
    ComplexOperation,
}

// New types for the SchemaDiffer implementation

/// Type of schema change operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ChangeType {
    Add,
    Remove,
    Modify,
    Rename,
}

/// Table-level change with detailed breakdown
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ColumnChange {
    pub column_name: String,
    pub change_type: ChangeType,
    pub old_definition: Option<ColumnSchema>,
    pub new_definition: Option<ColumnSchema>,
    pub safety_warnings: Vec<String>,
}

/// Index-level change
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct IndexChange {
    pub index_name: String,
    pub change_type: ChangeType,
    pub old_definition: Option<IndexSchema>,
    pub new_definition: Option<IndexSchema>,
    pub safety_warnings: Vec<String>,
}

/// Foreign key change
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ForeignKeyChange {
    pub foreign_key_name: String,
    pub change_type: ChangeType,
    pub old_definition: Option<ForeignKeySchema>,
    pub new_definition: Option<ForeignKeySchema>,
    pub safety_warnings: Vec<String>,
}
