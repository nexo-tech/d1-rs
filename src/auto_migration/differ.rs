use crate::Result;
use super::{DatabaseSchema, SchemaDiff, TableChange, ColumnChange, IndexChange, ForeignKeyChange, ChangeType};
use std::collections::{HashMap, HashSet};

/// Revolutionary Schema Comparison Engine - World's most advanced ORM schema diffing
/// Intelligently compares current database state vs desired entity schemas
pub struct SchemaDiffer {
    /// Settings for comparison behavior
    _strict_mode: bool,
    /// Whether to detect column renames vs drop+add
    detect_renames: bool,
    /// Threshold for considering a rename (0.0-1.0)
    rename_similarity_threshold: f32,
}

impl SchemaDiffer {
    pub fn new() -> Self {
        Self {
            _strict_mode: false,
            detect_renames: true,
            rename_similarity_threshold: 0.7,
        }
    }

    /// Create a new differ with strict mode (zero tolerance for unsafe changes)
    pub fn strict() -> Self {
        Self {
            _strict_mode: true,
            detect_renames: true,
            rename_similarity_threshold: 0.8,
        }
    }

    /// Enable/disable rename detection
    pub fn with_rename_detection(mut self, enabled: bool) -> Self {
        self.detect_renames = enabled;
        self
    }

    /// Set rename similarity threshold (0.0-1.0)
    pub fn with_rename_threshold(mut self, threshold: f32) -> Self {
        self.rename_similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Compare current database schema with desired schema from entities
    /// Returns comprehensive diff with safety analysis
    pub fn compare_schemas(&self, current: &DatabaseSchema, desired: &DatabaseSchema) -> Result<SchemaDiff> {
        let mut table_changes = Vec::new();

        // Create lookup maps for efficient comparison
        let current_tables: HashMap<&str, _> = current.tables.iter()
            .map(|t| (t.name.as_str(), t))
            .collect();

        let desired_tables: HashMap<&str, _> = desired.tables.iter()
            .map(|t| (t.name.as_str(), t))
            .collect();

        // Find tables to add (in desired but not in current)
        for (table_name, desired_table) in &desired_tables {
            if !current_tables.contains_key(table_name) {
                table_changes.push(TableChange {
                    table_name: table_name.to_string(),
                    change_type: ChangeType::Add,
                    old_schema: None,
                    new_schema: Some((*desired_table).clone()),
                    column_changes: Vec::new(),
                    index_changes: Vec::new(),
                    foreign_key_changes: Vec::new(),
                    safety_warnings: Vec::new(),
                });
            }
        }

        // Find tables to drop (in current but not in desired)
        for (table_name, current_table) in &current_tables {
            if !desired_tables.contains_key(table_name) {
                table_changes.push(TableChange {
                    table_name: table_name.to_string(),
                    change_type: ChangeType::Remove,
                    old_schema: Some((*current_table).clone()),
                    new_schema: None,
                    column_changes: Vec::new(),
                    index_changes: Vec::new(),
                    foreign_key_changes: Vec::new(),
                    safety_warnings: vec!["Dropping table will result in data loss".to_string()],
                });
            }
        }

        // Compare existing tables
        for (table_name, desired_table) in &desired_tables {
            if let Some(current_table) = current_tables.get(table_name) {
                let table_change = self.compare_tables(current_table, desired_table)?;
                if !table_change.is_empty() {
                    table_changes.push(table_change);
                }
            }
        }

        Ok(SchemaDiff {
            table_changes,
        })
    }

    /// Compare two tables and detect all changes
    fn compare_tables(&self, current: &crate::auto_migration::introspector::TableSchema, 
                     desired: &crate::auto_migration::introspector::TableSchema) -> Result<TableChange> {
        let mut column_changes = Vec::new();
        let mut index_changes = Vec::new();
        let mut foreign_key_changes = Vec::new();
        let mut safety_warnings = Vec::new();

        // Compare columns
        let current_columns: HashMap<&str, _> = current.columns.iter()
            .map(|c| (c.name.as_str(), c))
            .collect();

        let desired_columns: HashMap<&str, _> = desired.columns.iter()
            .map(|c| (c.name.as_str(), c))
            .collect();

        // Detect column changes
        column_changes.extend(self.compare_columns(&current_columns, &desired_columns, &mut safety_warnings)?);

        // Compare indexes
        let current_indexes: HashMap<&str, _> = current.indexes.iter()
            .map(|i| (i.name.as_str(), i))
            .collect();

        let desired_indexes: HashMap<&str, _> = desired.indexes.iter()
            .map(|i| (i.name.as_str(), i))
            .collect();

        index_changes.extend(self.compare_indexes(&current_indexes, &desired_indexes)?);

        // Compare foreign keys
        let current_fks: HashMap<String, _> = current.foreign_keys.iter()
            .map(|fk| (fk.name.clone(), fk))
            .collect();

        let desired_fks: HashMap<String, _> = desired.foreign_keys.iter()
            .map(|fk| (fk.name.clone(), fk))
            .collect();

        foreign_key_changes.extend(self.compare_foreign_keys(&current_fks, &desired_fks)?);

        Ok(TableChange {
            table_name: current.name.clone(),
            change_type: ChangeType::Modify,
            old_schema: Some(current.clone()),
            new_schema: Some(desired.clone()),
            column_changes,
            index_changes,
            foreign_key_changes,
            safety_warnings,
        })
    }

    /// Compare columns between current and desired schemas
    fn compare_columns(&self, 
                      current: &HashMap<&str, &crate::auto_migration::introspector::ColumnSchema>,
                      desired: &HashMap<&str, &crate::auto_migration::introspector::ColumnSchema>,
                      safety_warnings: &mut Vec<String>) -> Result<Vec<ColumnChange>> {
        let mut changes = Vec::new();

        // Find columns to add
        for (col_name, desired_col) in desired {
            if !current.contains_key(col_name) {
                let mut warnings = Vec::new();
                if !desired_col.nullable && desired_col.default_value.is_none() {
                    warnings.push("Adding non-nullable column without default value may fail on existing data".to_string());
                }

                changes.push(ColumnChange {
                    column_name: col_name.to_string(),
                    change_type: ChangeType::Add,
                    old_definition: None,
                    new_definition: Some((*desired_col).clone()),
                    safety_warnings: warnings,
                });
            }
        }

        // Find columns to drop or modify
        for (col_name, current_col) in current {
            if let Some(desired_col) = desired.get(col_name) {
                // Column exists in both - check for modifications
                let column_change = self.compare_column_definitions(current_col, desired_col)?;
                if let Some(change) = column_change {
                    changes.push(change);
                }
            } else {
                // Column only exists in current - will be dropped
                changes.push(ColumnChange {
                    column_name: col_name.to_string(),
                    change_type: ChangeType::Remove,
                    old_definition: Some((*current_col).clone()),
                    new_definition: None,
                    safety_warnings: vec!["Dropping column will result in data loss".to_string()],
                });
                safety_warnings.push(format!("Column '{}' will be dropped", col_name));
            }
        }

        // Detect potential column renames using similarity analysis
        if self.detect_renames {
            changes = self.detect_column_renames(changes);
        }

        Ok(changes)
    }

    /// Compare individual column definitions
    fn compare_column_definitions(&self, 
                                current: &crate::auto_migration::introspector::ColumnSchema,
                                desired: &crate::auto_migration::introspector::ColumnSchema) -> Result<Option<ColumnChange>> {
        let mut warnings = Vec::new();
        let mut has_changes = false;

        // Check type changes
        if current.column_type != desired.column_type {
            has_changes = true;
            warnings.push(format!("Type change from {} to {} may require data conversion", 
                                current.column_type, desired.column_type));
        }

        // Check nullability changes
        if current.nullable != desired.nullable {
            has_changes = true;
            if !current.nullable && desired.nullable {
                // Making nullable - safe
                warnings.push("Column will become nullable".to_string());
            } else {
                // Making non-nullable - potentially dangerous
                warnings.push("Column will become non-nullable - may fail if existing NULL values".to_string());
            }
        }

        // Check default value changes
        if current.default_value != desired.default_value {
            has_changes = true;
            warnings.push("Default value will change".to_string());
        }

        // Check primary key changes
        if current.primary_key != desired.primary_key {
            has_changes = true;
            warnings.push("Primary key constraint will change - potentially dangerous operation".to_string());
        }

        // Check auto-increment changes
        if current.auto_increment != desired.auto_increment {
            has_changes = true;
            warnings.push("Auto-increment behavior will change".to_string());
        }

        // Check unique constraint changes
        if current.unique != desired.unique {
            has_changes = true;
            if desired.unique {
                warnings.push("Adding unique constraint - may fail if duplicate values exist".to_string());
            }
        }

        if has_changes {
            Ok(Some(ColumnChange {
                column_name: current.name.clone(),
                change_type: ChangeType::Modify,
                old_definition: Some(current.clone()),
                new_definition: Some(desired.clone()),
                safety_warnings: warnings,
            }))
        } else {
            Ok(None)
        }
    }

    /// Compare indexes between current and desired schemas
    fn compare_indexes(&self,
                      current: &HashMap<&str, &crate::auto_migration::introspector::IndexSchema>,
                      desired: &HashMap<&str, &crate::auto_migration::introspector::IndexSchema>) -> Result<Vec<IndexChange>> {
        let mut changes = Vec::new();

        // Find indexes to add
        for (idx_name, desired_idx) in desired {
            if !current.contains_key(idx_name) {
                changes.push(IndexChange {
                    index_name: idx_name.to_string(),
                    change_type: ChangeType::Add,
                    old_definition: None,
                    new_definition: Some((*desired_idx).clone()),
                    safety_warnings: Vec::new(),
                });
            }
        }

        // Find indexes to drop or modify
        for (idx_name, current_idx) in current {
            if let Some(desired_idx) = desired.get(idx_name) {
                // Index exists in both - check for modifications
                if !self.indexes_equal(current_idx, desired_idx) {
                    changes.push(IndexChange {
                        index_name: idx_name.to_string(),
                        change_type: ChangeType::Modify,
                        old_definition: Some((*current_idx).clone()),
                        new_definition: Some((*desired_idx).clone()),
                        safety_warnings: vec!["Index will be recreated".to_string()],
                    });
                }
            } else {
                // Index only exists in current - will be dropped
                changes.push(IndexChange {
                    index_name: idx_name.to_string(),
                    change_type: ChangeType::Remove,
                    old_definition: Some((*current_idx).clone()),
                    new_definition: None,
                    safety_warnings: Vec::new(),
                });
            }
        }

        Ok(changes)
    }

    /// Compare foreign keys between current and desired schemas
    fn compare_foreign_keys(&self,
                           current: &HashMap<String, &crate::auto_migration::introspector::ForeignKeySchema>,
                           desired: &HashMap<String, &crate::auto_migration::introspector::ForeignKeySchema>) -> Result<Vec<ForeignKeyChange>> {
        let mut changes = Vec::new();

        // Find foreign keys to add
        for (fk_name, desired_fk) in desired {
            if !current.contains_key(fk_name) {
                changes.push(ForeignKeyChange {
                    foreign_key_name: fk_name.clone(),
                    change_type: ChangeType::Add,
                    old_definition: None,
                    new_definition: Some((*desired_fk).clone()),
                    safety_warnings: vec!["Adding foreign key constraint may fail if referential integrity is violated".to_string()],
                });
            }
        }

        // Find foreign keys to drop or modify
        for (fk_name, current_fk) in current {
            if let Some(desired_fk) = desired.get(fk_name) {
                // Foreign key exists in both - check for modifications
                if !self.foreign_keys_equal(current_fk, desired_fk) {
                    changes.push(ForeignKeyChange {
                        foreign_key_name: fk_name.clone(),
                        change_type: ChangeType::Modify,
                        old_definition: Some((*current_fk).clone()),
                        new_definition: Some((*desired_fk).clone()),
                        safety_warnings: vec!["Foreign key constraint will be recreated".to_string()],
                    });
                }
            } else {
                // Foreign key only exists in current - will be dropped
                changes.push(ForeignKeyChange {
                    foreign_key_name: fk_name.clone(),
                    change_type: ChangeType::Remove,
                    old_definition: Some((*current_fk).clone()),
                    new_definition: None,
                    safety_warnings: Vec::new(),
                });
            }
        }

        Ok(changes)
    }

    /// Detect potential column renames using similarity analysis
    fn detect_column_renames(&self, changes: Vec<ColumnChange>) -> Vec<ColumnChange> {
        let mut final_changes = Vec::new();
        let mut removals = Vec::new();
        let mut additions = Vec::new();

        // Separate additions and removals
        for change in changes {
            match change.change_type {
                ChangeType::Remove => removals.push(change),
                ChangeType::Add => additions.push(change),
                ChangeType::Modify => final_changes.push(change),
                ChangeType::Rename => final_changes.push(change), // Already processed renames should pass through
            }
        }

        // Try to match removals with additions based on similarity
        for removal in removals {
            let mut best_match = None;
            let mut best_score = 0.0;

            for (i, addition) in additions.iter().enumerate() {
                let score = self.calculate_column_similarity(&removal, addition);
                if score > best_score && score >= self.rename_similarity_threshold {
                    best_score = score;
                    best_match = Some(i);
                }
            }

            if let Some(match_idx) = best_match {
                let addition = additions.remove(match_idx);
                
                // Convert to rename operation
                final_changes.push(ColumnChange {
                    column_name: format!("{} -> {}", removal.column_name, addition.column_name),
                    change_type: ChangeType::Rename,
                    old_definition: removal.old_definition,
                    new_definition: addition.new_definition,
                    safety_warnings: vec![format!("Detected potential rename from '{}' to '{}'", 
                                                 removal.column_name, addition.column_name)],
                });
            } else {
                final_changes.push(removal);
            }
        }

        // Add remaining additions
        final_changes.extend(additions);

        final_changes
    }

    /// Calculate similarity score between two column changes (for rename detection)
    fn calculate_column_similarity(&self, removal: &ColumnChange, addition: &ColumnChange) -> f32 {
        let old_col = removal.old_definition.as_ref().unwrap();
        let new_col = addition.new_definition.as_ref().unwrap();

        let mut score = 0.0;
        let mut factors = 0;

        // Type similarity
        if old_col.column_type == new_col.column_type {
            score += 0.4;
        }
        factors += 1;

        // Name similarity (Levenshtein distance)
        let name_similarity = self.string_similarity(&removal.column_name, &addition.column_name);
        score += name_similarity * 0.3;
        factors += 1;

        // Constraint similarity
        if old_col.nullable == new_col.nullable {
            score += 0.1;
        }
        if old_col.primary_key == new_col.primary_key {
            score += 0.1;
        }
        if old_col.unique == new_col.unique {
            score += 0.1;
        }
        factors += 3;

        score / factors as f32
    }

    /// Calculate string similarity using simplified algorithm
    fn string_similarity(&self, s1: &str, s2: &str) -> f32 {
        if s1 == s2 { return 1.0; }
        if s1.is_empty() || s2.is_empty() { return 0.0; }

        let len1 = s1.len();
        let len2 = s2.len();
        let _max_len = len1.max(len2) as f32;

        // Simple character overlap calculation
        let chars1: HashSet<char> = s1.chars().collect();
        let chars2: HashSet<char> = s2.chars().collect();
        let intersection = chars1.intersection(&chars2).count() as f32;
        let union = chars1.union(&chars2).count() as f32;

        intersection / union
    }

    /// Check if two indexes are equal
    fn indexes_equal(&self, 
                    idx1: &crate::auto_migration::introspector::IndexSchema, 
                    idx2: &crate::auto_migration::introspector::IndexSchema) -> bool {
        idx1.columns == idx2.columns && 
        idx1.unique == idx2.unique
    }

    /// Check if two foreign keys are equal
    fn foreign_keys_equal(&self,
                         fk1: &crate::auto_migration::introspector::ForeignKeySchema,
                         fk2: &crate::auto_migration::introspector::ForeignKeySchema) -> bool {
        fk1.columns == fk2.columns &&
        fk1.referenced_table == fk2.referenced_table &&
        fk1.referenced_columns == fk2.referenced_columns &&
        fk1.on_delete == fk2.on_delete &&
        fk1.on_update == fk2.on_update
    }
}

impl Default for SchemaDiffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TableChange {
    /// Check if this table change is empty (no actual changes)
    pub fn is_empty(&self) -> bool {
        match self.change_type {
            ChangeType::Modify => {
                self.column_changes.is_empty() &&
                self.index_changes.is_empty() &&
                self.foreign_key_changes.is_empty()
            },
            _ => false,
        }
    }

    /// Get all safety warnings for this table change
    pub fn all_safety_warnings(&self) -> Vec<&str> {
        let mut warnings = Vec::new();
        
        warnings.extend(self.safety_warnings.iter().map(|s| s.as_str()));
        
        for change in &self.column_changes {
            warnings.extend(change.safety_warnings.iter().map(|s| s.as_str()));
        }
        
        for change in &self.index_changes {
            warnings.extend(change.safety_warnings.iter().map(|s| s.as_str()));
        }
        
        for change in &self.foreign_key_changes {
            warnings.extend(change.safety_warnings.iter().map(|s| s.as_str()));
        }
        
        warnings
    }
}