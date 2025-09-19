// Phase 2.2 - Revolutionary Smart Migration Strategies
use crate::Result;
use super::{
    SchemaDiff, MigrationPlan, MigrationOperation, SafetyWarning, SafetyWarningType,
    TableChange, ColumnChange, ChangeType, ColumnChanges
};
use super::introspector::{ColumnSchema, TableSchema};
use std::collections::{HashMap, HashSet};

/// Revolutionary Smart Migration Strategies Engine
/// Features:
/// - Intelligent column rename detection
/// - Advanced data migration support
/// - SQLite-aware table restructuring
/// - Junction table evolution
/// - Relationship cascade planning
pub struct SmartMigrationStrategies {
    /// Minimum similarity threshold for rename detection (0.0 to 1.0)
    rename_similarity_threshold: f32,
    /// Enable complex table restructuring for SQLite limitations
    enable_table_restructuring: bool,
    /// Enable automatic data migration suggestions
    enable_data_migration: bool,
}

impl SmartMigrationStrategies {
    pub fn new() -> Self {
        Self {
            rename_similarity_threshold: 0.7,
            enable_table_restructuring: true,
            enable_data_migration: true,
        }
    }

    pub fn with_rename_threshold(mut self, threshold: f32) -> Self {
        self.rename_similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_table_restructuring(mut self, enabled: bool) -> Self {
        self.enable_table_restructuring = enabled;
        self
    }

    pub fn with_data_migration(mut self, enabled: bool) -> Self {
        self.enable_data_migration = enabled;
        self
    }

    /// Apply smart migration strategies to enhance a migration plan
    pub fn enhance_migration_plan(&self, plan: MigrationPlan, diff: &SchemaDiff) -> Result<EnhancedMigrationPlan> {
        let mut enhanced_operations = Vec::new();
        let mut data_migrations = Vec::new();
        let mut safety_warnings = plan.safety_warnings;

        // Apply smart strategies to each table change
        for table_change in &diff.table_changes {
            let enhanced_ops = self.apply_smart_strategies_to_table(
                table_change, 
                &plan.operations,
                &mut data_migrations,
                &mut safety_warnings
            )?;
            enhanced_operations.extend(enhanced_ops);
        }

        // Apply relationship cascade planning
        let cascade_operations = self.plan_relationship_cascades(&diff.table_changes)?;
        enhanced_operations.extend(cascade_operations);

        // Generate enhanced rollback plan
        let rollback_plan = self.generate_enhanced_rollback(&enhanced_operations, diff)?;

        // Calculate estimated duration before moving operations
        let estimated_duration = self.estimate_enhanced_duration(&enhanced_operations);

        Ok(EnhancedMigrationPlan {
            operations: enhanced_operations,
            data_migrations,
            estimated_duration,
            safety_warnings,
            rollback_plan,
            restructuring_operations: Vec::new(), // Will be populated if needed
            junction_table_evolutions: Vec::new(), // Will be populated if needed
        })
    }

    /// Apply smart strategies to a single table change
    fn apply_smart_strategies_to_table(
        &self,
        table_change: &TableChange,
        original_operations: &[MigrationOperation],
        data_migrations: &mut Vec<DataMigration>,
        safety_warnings: &mut Vec<SafetyWarning>,
    ) -> Result<Vec<MigrationOperation>> {
        let mut operations = Vec::new();

        match table_change.change_type {
            ChangeType::Modify => {
                // 1. Apply column rename detection
                let rename_operations = self.detect_column_renames(&table_change.column_changes)?;
                operations.extend(rename_operations);

                // 2. Apply complex table restructuring if needed
                if self.requires_table_restructuring(table_change) {
                    let restructuring_ops = self.plan_table_restructuring(table_change, data_migrations)?;
                    operations.extend(restructuring_ops);
                } else {
                    // Apply standard column operations
                    let column_ops = self.plan_enhanced_column_operations(
                        table_change, 
                        data_migrations,
                        safety_warnings
                    )?;
                    operations.extend(column_ops);
                }

                // 3. Apply junction table evolution
                let junction_ops = self.plan_junction_table_evolution(table_change)?;
                operations.extend(junction_ops);
            }
            _ => {
                // For non-modify operations, use standard planning
                operations.extend(
                    original_operations
                        .iter()
                        .filter(|op| self.operation_affects_table(op, &table_change.table_name))
                        .cloned()
                );
            }
        }

        Ok(operations)
    }

    /// Revolutionary column rename detection using advanced similarity analysis
    pub fn detect_column_renames(&self, column_changes: &[ColumnChange]) -> Result<Vec<MigrationOperation>> {
        let mut operations = Vec::new();
        let mut removed_columns: Vec<&ColumnChange> = Vec::new();
        let mut added_columns: Vec<&ColumnChange> = Vec::new();

        // Separate removed and added columns
        for change in column_changes {
            match change.change_type {
                ChangeType::Remove => removed_columns.push(change),
                ChangeType::Add => added_columns.push(change),
                _ => {}
            }
        }

        // Find rename candidates using similarity analysis
        let mut used_additions = HashSet::new();
        
        for removed in &removed_columns {
            let mut best_match = None;
            let mut best_similarity = 0.0;

            for (idx, added) in added_columns.iter().enumerate() {
                if used_additions.contains(&idx) {
                    continue;
                }

                let similarity = self.calculate_column_similarity(removed, added)?;
                if similarity > best_similarity && similarity >= self.rename_similarity_threshold {
                    best_similarity = similarity;
                    best_match = Some((idx, added));
                }
            }

            // If we found a good match, create a rename operation
            if let Some((idx, added)) = best_match {
                if let (Some(old_def), Some(new_def)) = (&removed.old_definition, &added.new_definition) {
                    operations.push(MigrationOperation::RenameColumn {
                        table: "PLACEHOLDER_TABLE".to_string(), // Will be set by caller
                        old_name: old_def.name.clone(),
                        new_name: new_def.name.clone(),
                    });
                    used_additions.insert(idx);
                }
            }
        }

        Ok(operations)
    }

    /// Calculate similarity between two columns for rename detection
    fn calculate_column_similarity(&self, removed: &ColumnChange, added: &ColumnChange) -> Result<f32> {
        let old_def = removed.old_definition.as_ref()
            .ok_or_else(|| crate::D1RsError::ValidationError("Missing old definition".to_string()))?;
        let new_def = added.new_definition.as_ref()
            .ok_or_else(|| crate::D1RsError::ValidationError("Missing new definition".to_string()))?;

        let mut similarity_score = 0.0;
        let mut total_weight = 0.0;

        // Name similarity (weight: 0.4)
        let name_similarity = self.calculate_string_similarity(&old_def.name, &new_def.name);
        similarity_score += name_similarity * 0.4;
        total_weight += 0.4;

        // Type compatibility (weight: 0.3)
        let type_compatibility = self.calculate_type_compatibility(&old_def.column_type, &new_def.column_type);
        similarity_score += type_compatibility * 0.3;
        total_weight += 0.3;

        // Constraint similarity (weight: 0.2)
        let constraint_similarity = self.calculate_constraint_similarity(old_def, new_def);
        similarity_score += constraint_similarity * 0.2;
        total_weight += 0.2;

        // Nullable compatibility (weight: 0.1)
        let nullable_compatibility = if old_def.nullable == new_def.nullable { 1.0 } else { 0.0 };
        similarity_score += nullable_compatibility * 0.1;
        total_weight += 0.1;

        Ok(similarity_score / total_weight)
    }

    /// Calculate string similarity using Levenshtein distance
    pub fn calculate_string_similarity(&self, s1: &str, s2: &str) -> f32 {
        if s1 == s2 {
            return 1.0;
        }
        
        let len1 = s1.len();
        let len2 = s2.len();
        
        if len1 == 0 || len2 == 0 {
            return 0.0;
        }

        let distance = levenshtein_distance(s1, s2);
        let max_len = len1.max(len2) as f32;
        1.0 - (distance as f32 / max_len)
    }

    /// Calculate type compatibility for column types
    pub fn calculate_type_compatibility(&self, old_type: &str, new_type: &str) -> f32 {
        if old_type == new_type {
            return 1.0;
        }

        // Normalize types for comparison
        let old_normalized = self.normalize_column_type(old_type);
        let new_normalized = self.normalize_column_type(new_type);

        if old_normalized == new_normalized {
            return 0.9; // High compatibility for equivalent types
        }

        // Check for compatible type families
        let old_family = self.get_type_family(&old_normalized);
        let new_family = self.get_type_family(&new_normalized);

        if old_family == new_family {
            0.7 // Moderate compatibility within same family
        } else if self.are_compatible_families(&old_family, &new_family) {
            0.5 // Some compatibility between related families
        } else {
            0.0 // No compatibility
        }
    }

    /// Calculate constraint similarity between columns
    pub fn calculate_constraint_similarity(&self, old_col: &ColumnSchema, new_col: &ColumnSchema) -> f32 {
        let mut matches = 0;
        let mut total = 0;

        // Primary key
        total += 1;
        if old_col.primary_key == new_col.primary_key {
            matches += 1;
        }

        // Auto increment
        total += 1;
        if old_col.auto_increment == new_col.auto_increment {
            matches += 1;
        }

        // Unique
        total += 1;
        if old_col.unique == new_col.unique {
            matches += 1;
        }

        matches as f32 / total as f32
    }

    /// Normalize column type for comparison
    pub fn normalize_column_type(&self, column_type: &str) -> String {
        column_type
            .to_uppercase()
            .replace("VARCHAR", "TEXT")
            .replace("CHAR", "TEXT")
            .replace("BOOLEAN", "INTEGER")
            .replace("BOOL", "INTEGER")
            .split('(')
            .next()
            .unwrap_or(column_type)
            .trim()
            .to_string()
    }

    /// Get type family for compatibility checking
    fn get_type_family(&self, column_type: &str) -> TypeFamily {
        match column_type {
            "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" => TypeFamily::Integer,
            "REAL" | "FLOAT" | "DOUBLE" | "DECIMAL" | "NUMERIC" => TypeFamily::Numeric,
            "TEXT" | "CLOB" => TypeFamily::Text,
            "BLOB" => TypeFamily::Binary,
            _ => TypeFamily::Other,
        }
    }

    /// Check if type families are compatible
    fn are_compatible_families(&self, family1: &TypeFamily, family2: &TypeFamily) -> bool {
        match (family1, family2) {
            (TypeFamily::Integer, TypeFamily::Numeric) | 
            (TypeFamily::Numeric, TypeFamily::Integer) => true,
            _ => false,
        }
    }

    /// Check if table restructuring is required for SQLite limitations
    pub fn requires_table_restructuring(&self, table_change: &TableChange) -> bool {
        if !self.enable_table_restructuring {
            return false;
        }

        // Check for operations that require table restructuring in SQLite
        for column_change in &table_change.column_changes {
            match column_change.change_type {
                ChangeType::Remove => return true, // SQLite doesn't support DROP COLUMN
                ChangeType::Modify => {
                    // Complex column modifications may require restructuring
                    if let (Some(old_def), Some(new_def)) = (&column_change.old_definition, &column_change.new_definition) {
                        // Type changes often require restructuring
                        if old_def.column_type != new_def.column_type {
                            return true;
                        }
                        // Primary key changes require restructuring
                        if old_def.primary_key != new_def.primary_key {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }

        false
    }

    /// Plan complex table restructuring for SQLite limitations
    fn plan_table_restructuring(
        &self,
        table_change: &TableChange,
        data_migrations: &mut Vec<DataMigration>,
    ) -> Result<Vec<MigrationOperation>> {
        let mut operations = Vec::new();

        // Only restructure if we have the necessary schema information
        if let Some(old_schema) = &table_change.old_schema {
            // Generate temporary table name
            let temp_table_name = format!("{}_temp_migration", table_change.table_name);

            // 1. Create new table with desired schema
            let new_schema = self.build_target_schema(table_change)?;
            operations.push(MigrationOperation::CreateTable {
                definition: new_schema.clone(),
            });

            // 2. Plan data migration from old to new table
            let data_migration = self.plan_table_restructuring_data_migration(
                &table_change.table_name,
                &temp_table_name,
                old_schema,
                &new_schema,
                &table_change.column_changes,
            )?;
            data_migrations.push(data_migration);

            // 3. Drop old table
            operations.push(MigrationOperation::DropTable {
                name: table_change.table_name.clone(),
            });

            // 4. Rename new table to original name
            operations.push(MigrationOperation::RenameTable {
                old_name: temp_table_name,
                new_name: table_change.table_name.clone(),
            });
        }

        Ok(operations)
    }

    /// Build target schema for table restructuring
    pub fn build_target_schema(&self, table_change: &TableChange) -> Result<TableSchema> {
        let mut target_schema = table_change.old_schema.as_ref()
            .ok_or_else(|| crate::D1RsError::ValidationError("Old schema required for restructuring".to_string()))?
            .clone();

        // Apply column changes to build target schema
        for column_change in &table_change.column_changes {
            match column_change.change_type {
                ChangeType::Add => {
                    if let Some(new_def) = &column_change.new_definition {
                        target_schema.columns.push(new_def.clone());
                    }
                }
                ChangeType::Remove => {
                    target_schema.columns.retain(|col| col.name != column_change.column_name);
                }
                ChangeType::Modify => {
                    if let Some(new_def) = &column_change.new_definition {
                        if let Some(col) = target_schema.columns.iter_mut()
                            .find(|col| col.name == column_change.column_name) {
                            *col = new_def.clone();
                        }
                    }
                }
                ChangeType::Rename => {
                    if let (Some(old_def), Some(new_def)) = (&column_change.old_definition, &column_change.new_definition) {
                        if let Some(col) = target_schema.columns.iter_mut()
                            .find(|col| col.name == old_def.name) {
                            col.name = new_def.name.clone();
                        }
                    }
                }
            }
        }

        // Update table name to temporary name for creation
        target_schema.name = format!("{}_temp_migration", target_schema.name);

        Ok(target_schema)
    }

    /// Plan enhanced column operations with data migration support
    fn plan_enhanced_column_operations(
        &self,
        table_change: &TableChange,
        data_migrations: &mut Vec<DataMigration>,
        safety_warnings: &mut Vec<SafetyWarning>,
    ) -> Result<Vec<MigrationOperation>> {
        let mut operations = Vec::new();

        for column_change in &table_change.column_changes {
            match column_change.change_type {
                ChangeType::Add => {
                    if let Some(new_def) = &column_change.new_definition {
                        operations.push(MigrationOperation::AddColumn {
                            table: table_change.table_name.clone(),
                            column: new_def.clone(),
                        });

                        // Plan data population if needed
                        if self.enable_data_migration && new_def.default_value.is_some() {
                            let data_migration = DataMigration {
                                migration_type: DataMigrationType::PopulateColumn,
                                table_name: table_change.table_name.clone(),
                                source_columns: Vec::new(),
                                target_columns: vec![new_def.name.clone()],
                                transformation: TransformationStrategy::SetDefault {
                                    value: new_def.default_value.clone().unwrap_or_default(),
                                },
                                conditions: Vec::new(),
                            };
                            data_migrations.push(data_migration);
                        }
                    }
                }
                ChangeType::Modify => {
                    if let (Some(old_def), Some(new_def)) = (&column_change.old_definition, &column_change.new_definition) {
                        // Plan data migration for type changes
                        if self.enable_data_migration && old_def.column_type != new_def.column_type {
                            let data_migration = self.plan_column_type_migration(
                                &table_change.table_name,
                                old_def,
                                new_def,
                            )?;
                            data_migrations.push(data_migration);

                            // Add safety warning for type conversion
                            safety_warnings.push(SafetyWarning {
                                operation: format!("ALTER COLUMN {}.{}", table_change.table_name, new_def.name),
                                warning_type: SafetyWarningType::DataLoss,
                                message: format!("Type conversion from {} to {} may cause data loss", old_def.column_type, new_def.column_type),
                                recommendation: "Verify data compatibility before applying migration".to_string(),
                            });
                        }

                        let changes = self.build_column_changes(old_def, new_def);
                        operations.push(MigrationOperation::ModifyColumn {
                            table: table_change.table_name.clone(),
                            column: new_def.name.clone(),
                            changes,
                        });
                    }
                }
                _ => {
                    // Handle other change types normally
                }
            }
        }

        Ok(operations)
    }

    /// Plan junction table evolution for M2M relationships
    fn plan_junction_table_evolution(&self, _table_change: &TableChange) -> Result<Vec<MigrationOperation>> {
        // TODO: Implement junction table evolution logic
        // This would analyze foreign key changes to detect M2M relationship modifications
        Ok(Vec::new())
    }

    /// Plan relationship cascade operations
    fn plan_relationship_cascades(&self, _table_changes: &[TableChange]) -> Result<Vec<MigrationOperation>> {
        // TODO: Implement relationship cascade planning
        // This would analyze FK dependencies and plan proper ordering
        Ok(Vec::new())
    }

    /// Plan data migration for table restructuring
    fn plan_table_restructuring_data_migration(
        &self,
        old_table: &str,
        new_table: &str,
        old_schema: &TableSchema,
        new_schema: &TableSchema,
        column_changes: &[ColumnChange],
    ) -> Result<DataMigration> {
        let mut source_columns = Vec::new();
        let mut target_columns = Vec::new();
        let _transformations: Vec<TransformationStrategy> = Vec::new();

        // Map columns between old and new schema
        for new_col in &new_schema.columns {
            if let Some(old_col) = old_schema.columns.iter().find(|col| col.name == new_col.name) {
                // Direct mapping for unchanged columns
                source_columns.push(old_col.name.clone());
                target_columns.push(new_col.name.clone());
            } else {
                // Check for renamed columns
                for column_change in column_changes {
                    if column_change.change_type == ChangeType::Rename {
                        if let (Some(old_def), Some(new_def)) = (&column_change.old_definition, &column_change.new_definition) {
                            if new_def.name == new_col.name {
                                source_columns.push(old_def.name.clone());
                                target_columns.push(new_col.name.clone());
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(DataMigration {
            migration_type: DataMigrationType::TableRestructuring,
            table_name: new_table.to_string(),
            source_columns,
            target_columns,
            transformation: TransformationStrategy::CopyWithConversion {
                source_table: old_table.to_string(),
                target_table: new_table.to_string(),
                column_mappings: HashMap::new(), // TODO: Build proper mappings
            },
            conditions: Vec::new(),
        })
    }

    /// Plan data migration for column type changes
    pub fn plan_column_type_migration(
        &self,
        table_name: &str,
        old_def: &ColumnSchema,
        new_def: &ColumnSchema,
    ) -> Result<DataMigration> {
        let transformation = self.determine_type_conversion(&old_def.column_type, &new_def.column_type)?;

        Ok(DataMigration {
            migration_type: DataMigrationType::TypeConversion,
            table_name: table_name.to_string(),
            source_columns: vec![old_def.name.clone()],
            target_columns: vec![new_def.name.clone()],
            transformation,
            conditions: Vec::new(),
        })
    }

    /// Determine appropriate data conversion for type changes
    pub fn determine_type_conversion(&self, old_type: &str, new_type: &str) -> Result<TransformationStrategy> {
        let old_family = self.get_type_family(&self.normalize_column_type(old_type));
        let new_family = self.get_type_family(&self.normalize_column_type(new_type));

        match (old_family, new_family) {
            (TypeFamily::Integer, TypeFamily::Text) => {
                Ok(TransformationStrategy::TypeConversion {
                    conversion_type: ConversionType::IntegerToText,
                    validation_rules: vec!["CAST(column AS TEXT)".to_string()],
                })
            }
            (TypeFamily::Text, TypeFamily::Integer) => {
                Ok(TransformationStrategy::TypeConversion {
                    conversion_type: ConversionType::TextToInteger,
                    validation_rules: vec!["column IS NOT NULL AND column REGEXP '^[0-9]+$'".to_string()],
                })
            }
            _ => {
                Ok(TransformationStrategy::TypeConversion {
                    conversion_type: ConversionType::Generic,
                    validation_rules: Vec::new(),
                })
            }
        }
    }

    /// Build column changes for modification operations
    fn build_column_changes(&self, old_def: &ColumnSchema, new_def: &ColumnSchema) -> ColumnChanges {
        let type_change = if old_def.column_type != new_def.column_type {
            Some((old_def.column_type.clone(), new_def.column_type.clone()))
        } else {
            None
        };

        let null_change = if old_def.nullable != new_def.nullable {
            Some((old_def.nullable, new_def.nullable))
        } else {
            None
        };

        let default_change = if old_def.default_value != new_def.default_value {
            Some((old_def.default_value.clone(), new_def.default_value.clone()))
        } else {
            None
        };

        ColumnChanges {
            type_change,
            null_change,
            default_change,
            constraint_changes: Vec::new(),
        }
    }

    /// Generate enhanced rollback plan
    fn generate_enhanced_rollback(
        &self,
        _operations: &[MigrationOperation],
        _diff: &SchemaDiff,
    ) -> Result<Vec<MigrationOperation>> {
        // TODO: Implement enhanced rollback generation
        Ok(Vec::new())
    }

    /// Estimate duration for enhanced operations
    fn estimate_enhanced_duration(&self, operations: &[MigrationOperation]) -> std::time::Duration {
        let base_seconds = operations.len() * 2; // Base estimation
        let restructuring_penalty = operations.iter()
            .filter(|op| matches!(op, MigrationOperation::CreateTable { .. }))
            .count() * 10; // Extra time for table restructuring

        std::time::Duration::from_secs((base_seconds + restructuring_penalty) as u64)
    }

    /// Check if operation affects a specific table
    fn operation_affects_table(&self, operation: &MigrationOperation, table_name: &str) -> bool {
        match operation {
            MigrationOperation::CreateTable { definition } => definition.name == table_name,
            MigrationOperation::DropTable { name } => name == table_name,
            MigrationOperation::AddColumn { table, .. } => table == table_name,
            MigrationOperation::DropColumn { table, .. } => table == table_name,
            MigrationOperation::ModifyColumn { table, .. } => table == table_name,
            MigrationOperation::CreateIndex { table, .. } => table == table_name,
            MigrationOperation::RenameTable { old_name, .. } => old_name == table_name,
            MigrationOperation::RenameColumn { table, .. } => table == table_name,
            _ => false,
        }
    }
}

/// Enhanced migration plan with smart strategies applied
#[derive(Debug)]
pub struct EnhancedMigrationPlan {
    pub operations: Vec<MigrationOperation>,
    pub data_migrations: Vec<DataMigration>,
    pub estimated_duration: std::time::Duration,
    pub safety_warnings: Vec<SafetyWarning>,
    pub rollback_plan: Vec<MigrationOperation>,
    pub restructuring_operations: Vec<TableRestructuringOperation>,
    pub junction_table_evolutions: Vec<JunctionTableEvolution>,
}

/// Data migration for handling data transformations
#[derive(Debug, Clone)]
pub struct DataMigration {
    pub migration_type: DataMigrationType,
    pub table_name: String,
    pub source_columns: Vec<String>,
    pub target_columns: Vec<String>,
    pub transformation: TransformationStrategy,
    pub conditions: Vec<String>,
}

/// Types of data migrations
#[derive(Debug, Clone, PartialEq)]
pub enum DataMigrationType {
    TypeConversion,
    PopulateColumn,
    TableRestructuring,
    JunctionTableMigration,
}

/// Data transformation strategies for smart migration planning
#[derive(Debug, Clone, PartialEq)]
pub enum TransformationStrategy {
    SetDefault { value: String },
    TypeConversion { conversion_type: ConversionType, validation_rules: Vec<String> },
    CopyWithConversion { source_table: String, target_table: String, column_mappings: HashMap<String, String> },
    CustomScript { script: String },
}

/// Type conversion strategies
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionType {
    IntegerToText,
    TextToInteger,
    IntegerToReal,
    RealToInteger,
    Generic,
}

/// Table restructuring operation
#[derive(Debug, Clone)]
pub struct TableRestructuringOperation {
    pub table_name: String,
    pub old_schema: TableSchema,
    pub new_schema: TableSchema,
    pub data_preservation_strategy: DataPreservationStrategy,
}

/// Data preservation strategy during restructuring
#[derive(Debug, Clone)]
pub enum DataPreservationStrategy {
    CopyAll,
    CopyWithTransformation,
    SelectivePreservation { preserved_columns: Vec<String> },
}

/// Junction table evolution for M2M relationships
#[derive(Debug, Clone)]
pub struct JunctionTableEvolution {
    pub junction_table: String,
    pub evolution_type: JunctionEvolutionType,
    pub affected_relationships: Vec<String>,
}

/// Types of junction table evolutions
#[derive(Debug, Clone)]
pub enum JunctionEvolutionType {
    Created,
    Modified,
    Removed,
    Restructured,
}

/// Type families for compatibility checking
#[derive(Debug, Clone, PartialEq)]
enum TypeFamily {
    Integer,
    Numeric,
    Text,
    Binary,
    Other,
}

/// Calculate Levenshtein distance between two strings
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();
    
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    // Initialize first row and column
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len1][len2]
}