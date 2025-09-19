// Data seeding integration for automatic data population during migrations
use crate::{D1Client, Result, Entity};
use serde_json::Value;
use std::collections::HashMap;

/// Data seeding coordinator for automatic data population
/// Provides type-safe data seeding with compile-time entity validation
#[derive(Debug)]
pub struct DataSeeder {
    /// Seed data registry by table name
    seed_registry: HashMap<String, Vec<SeedDefinition>>,
    /// Whether to update existing records or skip them
    update_existing: bool,
    /// Conflict resolution strategy
    conflict_resolution: ConflictResolution,
}

/// Individual seed data definition
#[derive(Debug, Clone)]
pub struct SeedDefinition {
    /// Unique identifier for this seed
    pub id: String,
    /// Human-readable description
    pub description: String,
    /// Table to insert data into
    pub table_name: String,
    /// Seed data records
    pub data: Vec<SeedRecord>,
    /// Dependencies - other seeds that must run first
    pub dependencies: Vec<String>,
    /// Whether this seed is required for the application to function
    pub required: bool,
    /// Environment where this seed should run
    pub environment: SeedEnvironment,
}

/// Individual seed data record
#[derive(Debug, Clone)]
pub struct SeedRecord {
    /// Field values for this record
    pub fields: HashMap<String, Value>,
    /// Unique identifier fields for conflict detection
    pub unique_keys: Vec<String>,
    /// Whether to update if record exists
    pub update_if_exists: bool,
}

/// Seed environment configuration
#[derive(Debug, Clone, PartialEq)]
pub enum SeedEnvironment {
    /// Run in all environments
    All,
    /// Run only in development
    Development,
    /// Run only in testing
    Testing,
    /// Run only in production
    Production,
    /// Custom environment specification
    Custom(Vec<String>),
}

/// Conflict resolution strategy for existing data
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolution {
    /// Skip if record exists
    Skip,
    /// Update existing record with new data
    Update,
    /// Replace existing record completely
    Replace,
    /// Fail if conflict detected
    Fail,
}

/// Result of data seeding operation
#[derive(Debug)]
pub struct SeedResult {
    /// Seeds that were successfully executed
    pub completed_seeds: Vec<String>,
    /// Seeds that failed with error messages
    pub failed_seeds: Vec<(String, String)>,
    /// Total records inserted
    pub records_inserted: usize,
    /// Total records updated
    pub records_updated: usize,
    /// Total records skipped due to conflicts
    pub records_skipped: usize,
    /// Total execution time
    pub execution_time: std::time::Duration,
}

/// Comprehensive seeding plan with dependency resolution
#[derive(Debug, Clone)]
pub struct SeedingPlan {
    /// Ordered seed operations (dependency-resolved)
    pub seed_operations: Vec<SeedDefinition>,
    /// Estimated total execution time
    pub estimated_duration: std::time::Duration,
    /// Validation checks to run after seeding
    pub validation_checks: Vec<SeedValidation>,
}

/// Validation check for seed data integrity
#[derive(Debug, Clone)]
pub struct SeedValidation {
    /// Unique identifier for this validation
    pub id: String,
    /// Description of what is being validated
    pub description: String,
    /// SQL query to validate seed success
    pub validation_query: String,
    /// Expected result for successful validation
    pub expected_result: SeedValidationExpectation,
}

/// Expected result for seed validation
#[derive(Debug, Clone)]
pub enum SeedValidationExpectation {
    /// Expect minimum number of records
    MinRecordCount(usize),
    /// Expect exact number of records
    ExactRecordCount(usize),
    /// Expect specific value in result
    SpecificValue(Value),
    /// Custom validation logic
    Custom(String),
}

impl DataSeeder {
    /// Create a new data seeder with default configuration
    pub fn new() -> Self {
        Self {
            seed_registry: HashMap::new(),
            update_existing: false,
            conflict_resolution: ConflictResolution::Skip,
        }
    }

    /// Create data seeder with custom configuration
    pub fn with_config(update_existing: bool, conflict_resolution: ConflictResolution) -> Self {
        Self {
            seed_registry: HashMap::new(),
            update_existing,
            conflict_resolution,
        }
    }

    /// Register seed data for an entity type
    /// This provides compile-time type safety for seed data
    pub fn register_entity_seeds<T: Entity + 'static>(
        &mut self,
        seed_id: &str,
        description: &str,
        data: Vec<T>,
        environment: SeedEnvironment,
        required: bool,
    ) -> Result<()> {
        let table_name = T::TABLE_NAME.to_string();
        
        // Convert entity instances to seed records
        let mut seed_records = Vec::new();
        for entity in data {
            let serialized = serde_json::to_value(&entity).map_err(|e| {
                crate::D1RsError::SerializationError(format!("Failed to serialize seed data: {}", e))
            })?;
            
            if let Value::Object(fields) = serialized {
                let field_map: HashMap<String, Value> = fields.into_iter().collect();
                
                seed_records.push(SeedRecord {
                    fields: field_map,
                    unique_keys: vec!["id".to_string()], // Default to id field
                    update_if_exists: self.update_existing,
                });
            }
        }

        let seed_definition = SeedDefinition {
            id: seed_id.to_string(),
            description: description.to_string(),
            table_name: table_name.clone(),
            data: seed_records,
            dependencies: vec![],
            required,
            environment,
        };

        self.seed_registry
            .entry(table_name)
            .or_insert_with(Vec::new)
            .push(seed_definition);

        Ok(())
    }

    /// Register raw seed data with SQL-like interface
    pub fn register_raw_seeds(
        &mut self,
        seed_id: &str,
        description: &str,
        table_name: &str,
        data: Vec<HashMap<String, Value>>,
        unique_keys: Vec<String>,
        environment: SeedEnvironment,
        required: bool,
    ) {
        let seed_records: Vec<SeedRecord> = data
            .into_iter()
            .map(|fields| SeedRecord {
                fields,
                unique_keys: unique_keys.clone(),
                update_if_exists: self.update_existing,
            })
            .collect();

        let seed_definition = SeedDefinition {
            id: seed_id.to_string(),
            description: description.to_string(),
            table_name: table_name.to_string(),
            data: seed_records,
            dependencies: vec![],
            required,
            environment,
        };

        self.seed_registry
            .entry(table_name.to_string())
            .or_insert_with(Vec::new)
            .push(seed_definition);
    }

    /// Add dependency between seeds
    pub fn add_seed_dependency(&mut self, seed_id: &str, depends_on: &str) -> Result<()> {
        for table_seeds in self.seed_registry.values_mut() {
            for seed in table_seeds.iter_mut() {
                if seed.id == seed_id {
                    if !seed.dependencies.contains(&depends_on.to_string()) {
                        seed.dependencies.push(depends_on.to_string());
                    }
                    return Ok(());
                }
            }
        }
        
        Err(crate::D1RsError::ValidationError(format!(
            "Seed '{}' not found when adding dependency",
            seed_id
        )))
    }

    /// Create comprehensive seeding plan with dependency resolution
    pub fn create_seeding_plan(&self, environment: &str) -> Result<SeedingPlan> {
        let mut applicable_seeds = Vec::new();
        
        // Collect all seeds applicable to this environment
        for table_seeds in self.seed_registry.values() {
            for seed in table_seeds {
                if self.seed_applies_to_environment(seed, environment) {
                    applicable_seeds.push(seed.clone());
                }
            }
        }

        // Resolve dependencies using topological sort
        let ordered_seeds = self.resolve_dependencies(applicable_seeds)?;
        
        // Estimate execution time
        let estimated_duration = std::time::Duration::from_millis(
            ordered_seeds.len() as u64 * 100 + // Base time per seed
            ordered_seeds.iter().map(|s| s.data.len() as u64 * 10).sum::<u64>() // Time per record
        );

        // Create validation checks
        let validation_checks = self.create_validation_checks(&ordered_seeds);

        Ok(SeedingPlan {
            seed_operations: ordered_seeds,
            estimated_duration,
            validation_checks,
        })
    }

    /// Execute seeding plan
    pub async fn execute_seeding(&self, db: &D1Client, plan: &SeedingPlan) -> Result<SeedResult> {
        let start_time = std::time::Instant::now();
        let mut completed_seeds = Vec::new();
        let mut failed_seeds = Vec::new();
        let mut records_inserted = 0;
        let mut records_updated = 0;
        let mut records_skipped = 0;

        for seed in &plan.seed_operations {
            match self.execute_seed(db, seed).await {
                Ok((inserted, updated, skipped)) => {
                    completed_seeds.push(seed.id.clone());
                    records_inserted += inserted;
                    records_updated += updated;
                    records_skipped += skipped;
                }
                Err(e) => {
                    failed_seeds.push((seed.id.clone(), e.to_string()));
                    if seed.required {
                        // Stop execution if a required seed fails
                        break;
                    }
                }
            }
        }

        // Run validation checks
        for validation in &plan.validation_checks {
            if let Err(e) = self.run_seed_validation(db, validation).await {
                failed_seeds.push((
                    validation.id.clone(),
                    format!("Validation failed: {}", e),
                ));
            }
        }

        Ok(SeedResult {
            completed_seeds,
            failed_seeds,
            records_inserted,
            records_updated,
            records_skipped,
            execution_time: start_time.elapsed(),
        })
    }

    /// Execute individual seed
    async fn execute_seed(&self, db: &D1Client, seed: &SeedDefinition) -> Result<(usize, usize, usize)> {
        let mut inserted = 0;
        let mut updated = 0;
        let mut skipped = 0;

        for record in &seed.data {
            match self.insert_or_update_record(db, &seed.table_name, record).await? {
                RecordAction::Inserted => inserted += 1,
                RecordAction::Updated => updated += 1,
                RecordAction::Skipped => skipped += 1,
            }
        }

        Ok((inserted, updated, skipped))
    }

    /// Insert or update individual record based on conflict resolution
    async fn insert_or_update_record(
        &self,
        db: &D1Client,
        table_name: &str,
        record: &SeedRecord,
    ) -> Result<RecordAction> {
        // Check if record exists
        if let Some(existing_record) = self.find_existing_record(db, table_name, record).await? {
            match self.conflict_resolution {
                ConflictResolution::Skip => Ok(RecordAction::Skipped),
                ConflictResolution::Update => {
                    self.update_record(db, table_name, record, &existing_record).await?;
                    Ok(RecordAction::Updated)
                }
                ConflictResolution::Replace => {
                    self.replace_record(db, table_name, record, &existing_record).await?;
                    Ok(RecordAction::Updated)
                }
                ConflictResolution::Fail => {
                    Err(crate::D1RsError::ValidationError(format!(
                        "Conflict detected for record in table '{}' and conflict resolution is set to Fail",
                        table_name
                    )))
                }
            }
        } else {
            self.insert_record(db, table_name, record).await?;
            Ok(RecordAction::Inserted)
        }
    }

    /// Find existing record based on unique keys
    async fn find_existing_record(
        &self,
        _db: &D1Client,
        table_name: &str,
        record: &SeedRecord,
    ) -> Result<Option<HashMap<String, Value>>> {
        if record.unique_keys.is_empty() {
            return Ok(None);
        }

        // Build WHERE clause for unique keys
        let conditions: Vec<String> = record.unique_keys
            .iter()
            .map(|key| format!("{} = ?", key))
            .collect();
        let where_clause = conditions.join(" AND ");

        let _query = format!("SELECT * FROM {} WHERE {}", table_name, where_clause);
        
        // Extract values for unique keys
        let mut params = Vec::new();
        for key in &record.unique_keys {
            if let Some(value) = record.fields.get(key) {
                params.push(value.clone());
            }
        }

        // Execute query (this is a placeholder - actual implementation would depend on D1Client API)
        // For now, return None to indicate no existing record found
        Ok(None)
    }

    /// Insert new record
    async fn insert_record(&self, db: &D1Client, table_name: &str, record: &SeedRecord) -> Result<()> {
        let columns: Vec<String> = record.fields.keys().cloned().collect();
        let placeholders: Vec<String> = (0..columns.len()).map(|_| "?".to_string()).collect();
        
        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            placeholders.join(", ")
        );

        // Convert values to parameters (placeholder implementation)
        let _params: Vec<Value> = columns
            .iter()
            .map(|col| record.fields.get(col).cloned().unwrap_or(Value::Null))
            .collect();

        // Execute insert (placeholder)
        let _result = db.execute(&query, &[]).await?;
        Ok(())
    }

    /// Update existing record
    async fn update_record(
        &self,
        db: &D1Client,
        table_name: &str,
        record: &SeedRecord,
        _existing: &HashMap<String, Value>,
    ) -> Result<()> {
        let set_clauses: Vec<String> = record.fields
            .keys()
            .filter(|key| !record.unique_keys.contains(key)) // Don't update unique keys
            .map(|key| format!("{} = ?", key))
            .collect();

        if set_clauses.is_empty() {
            return Ok(()); // Nothing to update
        }

        let where_clauses: Vec<String> = record.unique_keys
            .iter()
            .map(|key| format!("{} = ?", key))
            .collect();

        let query = format!(
            "UPDATE {} SET {} WHERE {}",
            table_name,
            set_clauses.join(", "),
            where_clauses.join(" AND ")
        );

        // Execute update (placeholder)
        let _result = db.execute(&query, &[]).await?;
        Ok(())
    }

    /// Replace existing record (delete + insert)
    async fn replace_record(
        &self,
        db: &D1Client,
        table_name: &str,
        record: &SeedRecord,
        _existing: &HashMap<String, Value>,
    ) -> Result<()> {
        // Delete existing record
        let where_clauses: Vec<String> = record.unique_keys
            .iter()
            .map(|key| format!("{} = ?", key))
            .collect();

        let delete_query = format!(
            "DELETE FROM {} WHERE {}",
            table_name,
            where_clauses.join(" AND ")
        );

        let _result = db.execute(&delete_query, &[]).await?;

        // Insert new record
        self.insert_record(db, table_name, record).await
    }

    /// Check if seed applies to given environment
    fn seed_applies_to_environment(&self, seed: &SeedDefinition, environment: &str) -> bool {
        match &seed.environment {
            SeedEnvironment::All => true,
            SeedEnvironment::Development => environment == "development",
            SeedEnvironment::Testing => environment == "testing",
            SeedEnvironment::Production => environment == "production",
            SeedEnvironment::Custom(envs) => envs.contains(&environment.to_string()),
        }
    }

    /// Resolve seed dependencies using topological sort
    fn resolve_dependencies(&self, seeds: Vec<SeedDefinition>) -> Result<Vec<SeedDefinition>> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();

        fn visit(
            seed_id: &str,
            seeds: &[SeedDefinition],
            visited: &mut std::collections::HashSet<String>,
            temp_visited: &mut std::collections::HashSet<String>,
            result: &mut Vec<SeedDefinition>,
        ) -> Result<()> {
            if temp_visited.contains(seed_id) {
                return Err(crate::D1RsError::ValidationError(format!(
                    "Circular dependency detected involving seed '{}'",
                    seed_id
                )));
            }

            if visited.contains(seed_id) {
                return Ok(());
            }

            temp_visited.insert(seed_id.to_string());

            if let Some(seed) = seeds.iter().find(|s| s.id == seed_id) {
                for dep in &seed.dependencies {
                    visit(dep, seeds, visited, temp_visited, result)?;
                }
                visited.insert(seed_id.to_string());
                result.push(seed.clone());
            }

            temp_visited.remove(seed_id);
            Ok(())
        }

        for seed in &seeds {
            if !visited.contains(&seed.id) {
                visit(&seed.id, &seeds, &mut visited, &mut temp_visited, &mut result)?;
            }
        }

        Ok(result)
    }

    /// Create validation checks for seeding plan
    fn create_validation_checks(&self, seeds: &[SeedDefinition]) -> Vec<SeedValidation> {
        let mut validations = Vec::new();

        for seed in seeds {
            if !seed.data.is_empty() {
                validations.push(SeedValidation {
                    id: format!("{}_validation", seed.id),
                    description: format!("Verify seed '{}' was applied correctly", seed.id),
                    validation_query: format!(
                        "SELECT COUNT(*) FROM {}",
                        seed.table_name
                    ),
                    expected_result: SeedValidationExpectation::MinRecordCount(seed.data.len()),
                });
            }
        }

        validations
    }

    /// Run seed validation check
    async fn run_seed_validation(&self, db: &D1Client, validation: &SeedValidation) -> Result<()> {
        // Execute validation query (placeholder implementation)
        let _result = db.execute(&validation.validation_query, &[]).await?;
        
        // Check result against expectation (placeholder)
        match &validation.expected_result {
            SeedValidationExpectation::MinRecordCount(_count) => {
                // Verify minimum record count
            }
            SeedValidationExpectation::ExactRecordCount(_count) => {
                // Verify exact record count
            }
            SeedValidationExpectation::SpecificValue(_value) => {
                // Verify specific value
            }
            SeedValidationExpectation::Custom(_logic) => {
                // Custom validation logic
            }
        }

        Ok(())
    }
}

/// Record action taken during seeding
#[derive(Debug, Clone, PartialEq)]
enum RecordAction {
    Inserted,
    Updated,
    Skipped,
}

impl Default for DataSeeder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct TestUser {
        id: i64,
        name: String,
        email: String,
    }

    // Mock builder types for testing
    struct TestUserQueryBuilder;
    struct TestUserCreateBuilder;
    struct TestUserUpdateBuilder;

    impl crate::QueryBuilder<TestUser> for TestUserQueryBuilder {
        async fn all(self, _db: &crate::D1Client) -> crate::Result<Vec<TestUser>> {
            unimplemented!()
        }
        async fn first(self, _db: &crate::D1Client) -> crate::Result<Option<TestUser>> {
            unimplemented!()
        }
        async fn count(self, _db: &crate::D1Client) -> crate::Result<i64> {
            unimplemented!()
        }
        fn apply_relation_constraint(self, _field: &str, _value: serde_json::Value) -> Self {
            self
        }
    }

    impl crate::CreateBuilder<TestUser> for TestUserCreateBuilder {
        async fn save(self, _db: &crate::D1Client) -> crate::Result<TestUser> {
            unimplemented!()
        }
    }

    impl crate::UpdateBuilder<TestUser> for TestUserUpdateBuilder {
        async fn save(self, _db: &crate::D1Client) -> crate::Result<TestUser> {
            unimplemented!()
        }
    }

    // Mock Entity implementation for testing
    impl crate::Entity for TestUser {
        type PrimaryKey = i64;
        type QueryBuilder = TestUserQueryBuilder;
        type CreateBuilder = TestUserCreateBuilder;
        type UpdateBuilder = TestUserUpdateBuilder;

        const TABLE_NAME: &'static str = "test_users";

        fn primary_key(&self) -> &Self::PrimaryKey { &self.id }
        fn query() -> Self::QueryBuilder { TestUserQueryBuilder }
        fn create() -> Self::CreateBuilder { TestUserCreateBuilder }
        fn update(_key: Self::PrimaryKey) -> Self::UpdateBuilder { TestUserUpdateBuilder }
        fn boolean_fields() -> &'static [&'static str] { &[] }

        async fn find(_db: &crate::D1Client, _key: Self::PrimaryKey) -> crate::Result<Option<Self>> {
            unimplemented!()
        }
        async fn delete(_db: &crate::D1Client, _key: Self::PrimaryKey) -> crate::Result<()> {
            unimplemented!()
        }
    }

    #[test]
    fn test_data_seeder_creation() {
        let seeder = DataSeeder::new();
        assert_eq!(seeder.conflict_resolution, ConflictResolution::Skip);
        assert!(!seeder.update_existing);

        let custom_seeder = DataSeeder::with_config(true, ConflictResolution::Update);
        assert_eq!(custom_seeder.conflict_resolution, ConflictResolution::Update);
        assert!(custom_seeder.update_existing);
    }

    #[test]
    fn test_seed_environment_matching() {
        let seeder = DataSeeder::new();

        let seed = SeedDefinition {
            id: "test_seed".to_string(),
            description: "Test seed".to_string(),
            table_name: "test_table".to_string(),
            data: vec![],
            dependencies: vec![],
            required: false,
            environment: SeedEnvironment::Development,
        };

        assert!(seeder.seed_applies_to_environment(&seed, "development"));
        assert!(!seeder.seed_applies_to_environment(&seed, "production"));

        let all_env_seed = SeedDefinition {
            environment: SeedEnvironment::All,
            ..seed
        };

        assert!(seeder.seed_applies_to_environment(&all_env_seed, "development"));
        assert!(seeder.seed_applies_to_environment(&all_env_seed, "production"));
    }

    #[test]
    fn test_dependency_resolution() {
        let seeder = DataSeeder::new();

        let seed1 = SeedDefinition {
            id: "seed1".to_string(),
            description: "First seed".to_string(),
            table_name: "table1".to_string(),
            data: vec![],
            dependencies: vec![],
            required: false,
            environment: SeedEnvironment::All,
        };

        let seed2 = SeedDefinition {
            id: "seed2".to_string(),
            description: "Second seed".to_string(),
            table_name: "table2".to_string(),
            data: vec![],
            dependencies: vec!["seed1".to_string()],
            required: false,
            environment: SeedEnvironment::All,
        };

        let seeds = vec![seed2.clone(), seed1.clone()]; // Deliberately out of order
        let resolved = seeder.resolve_dependencies(seeds).unwrap();

        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].id, "seed1");
        assert_eq!(resolved[1].id, "seed2");
    }

    #[test]
    fn test_circular_dependency_detection() {
        let seeder = DataSeeder::new();

        let seed1 = SeedDefinition {
            id: "seed1".to_string(),
            description: "First seed".to_string(),
            table_name: "table1".to_string(),
            data: vec![],
            dependencies: vec!["seed2".to_string()],
            required: false,
            environment: SeedEnvironment::All,
        };

        let seed2 = SeedDefinition {
            id: "seed2".to_string(),
            description: "Second seed".to_string(),
            table_name: "table2".to_string(),
            data: vec![],
            dependencies: vec!["seed1".to_string()],
            required: false,
            environment: SeedEnvironment::All,
        };

        let seeds = vec![seed1, seed2];
        let result = seeder.resolve_dependencies(seeds);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular dependency"));
    }

    #[test]
    fn test_conflict_resolution_modes() {
        let modes = vec![
            ConflictResolution::Skip,
            ConflictResolution::Update,
            ConflictResolution::Replace,
            ConflictResolution::Fail,
        ];

        for mode in modes {
            let seeder = DataSeeder::with_config(false, mode.clone());
            assert_eq!(seeder.conflict_resolution, mode);
        }
    }

    #[test]
    fn test_validation_expectation_types() {
        let expectations = vec![
            SeedValidationExpectation::MinRecordCount(10),
            SeedValidationExpectation::ExactRecordCount(5),
            SeedValidationExpectation::SpecificValue(Value::String("test".to_string())),
            SeedValidationExpectation::Custom("custom logic".to_string()),
        ];

        for expectation in expectations {
            // Test that each expectation type can be created and matched
            match expectation {
                SeedValidationExpectation::MinRecordCount(_) => {},
                SeedValidationExpectation::ExactRecordCount(_) => {},
                SeedValidationExpectation::SpecificValue(_) => {},
                SeedValidationExpectation::Custom(_) => {},
            }
        }
    }
}