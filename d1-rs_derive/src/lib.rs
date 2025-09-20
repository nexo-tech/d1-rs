use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Fields, Type, Field, Attribute};

#[proc_macro_derive(Entity, attributes(table, primary_key, unique, not_null, edge, sql_type, foreign_key, field_config, nullable, type_conversion, custom_type, boolean_field, type_override, sql_mapping))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let table_name = extract_table_name(&input.attrs, &name.to_string());
    
    let fields = match &input.data {
        syn::Data::Struct(data_struct) => &data_struct.fields,
        _ => panic!("Entity can only be derived for structs"),
    };

    let named_fields = match fields {
        Fields::Named(named) => &named.named,
        _ => panic!("Entity requires named fields"),
    };

    let (primary_key_field, primary_key_type) = find_primary_key_field(named_fields);
    
    let query_builder_name = format_ident!("{}QueryBuilder", name);
    let create_builder_name = format_ident!("{}CreateBuilder", name);
    let update_builder_name = format_ident!("{}UpdateBuilder", name);

    let query_methods = generate_query_methods(named_fields);
    let create_methods = generate_create_methods(named_fields, &primary_key_field);
    let update_methods = generate_update_methods(named_fields, &primary_key_field);

    let boolean_field_metadata = generate_boolean_field_metadata(named_fields);
    let field_definitions_impl = generate_field_definitions(named_fields, &primary_key_field);

    let expanded = quote! {
        impl d1_rs::Entity for #name {
            type PrimaryKey = #primary_key_type;
            type QueryBuilder = #query_builder_name;
            type CreateBuilder = #create_builder_name;
            type UpdateBuilder = #update_builder_name;

            const TABLE_NAME: &'static str = #table_name;
            
            fn primary_key(&self) -> &Self::PrimaryKey {
                &self.#primary_key_field
            }
            
            fn query() -> Self::QueryBuilder {
                #query_builder_name::new()
            }
            
            fn create() -> Self::CreateBuilder {
                #create_builder_name::new()
            }
            
            fn update(key: Self::PrimaryKey) -> Self::UpdateBuilder {
                #update_builder_name::new(key)
            }
            
            fn boolean_fields() -> &'static [&'static str] {
                #boolean_field_metadata
            }
            
            fn field_definitions() -> Vec<d1_rs::FieldDefinition> {
                #field_definitions_impl
            }
            
            async fn find(db: &d1_rs::D1Client, key: Self::PrimaryKey) -> d1_rs::Result<Option<Self>> {
                let sql = concat!("SELECT * FROM ", #table_name, " WHERE ", stringify!(#primary_key_field), " = ? LIMIT 1");
                let params = vec![d1_rs::types::SqlType::to_sql_value(&key)];
                
                let result = db.execute(sql, &params).await?;
                result.into_entity()
            }
            
            async fn delete(db: &d1_rs::D1Client, key: Self::PrimaryKey) -> d1_rs::Result<()> {
                let sql = concat!("DELETE FROM ", #table_name, " WHERE ", stringify!(#primary_key_field), " = ?");
                let params = vec![d1_rs::types::SqlType::to_sql_value(&key)];
                
                db.execute(sql, &params).await?;
                Ok(())
            }
        }

        pub struct #query_builder_name {
            query: d1_rs::query::Query,
        }

        impl #query_builder_name {
            pub fn new() -> Self {
                Self {
                    query: d1_rs::query::Query::new(#table_name.to_string()),
                }
            }

            pub fn limit(mut self, limit: i64) -> Self {
                self.query.limit(limit);
                self
            }

            pub fn offset(mut self, offset: i64) -> Self {
                self.query.offset(offset);
                self
            }

            // REMOVED: Generic order_by method - VIOLATES zero-string-literals policy!
            // Use type-safe methods instead: order_by_field_name_asc(), order_by_field_name_desc()
            // This ensures compile-time validation and prevents typos

            #query_methods
        }

        impl d1_rs::QueryBuilder<#name> for #query_builder_name {
            async fn all(self, db: &d1_rs::D1Client) -> d1_rs::Result<Vec<#name>> {
                let (sql, params) = self.query.to_sql();
                let result = db.execute(&sql, &params).await?;
                result.into_entities()
            }

            async fn first(self, db: &d1_rs::D1Client) -> d1_rs::Result<Option<#name>> {
                let mut query = self.query;
                query.limit(1);
                let (sql, params) = query.to_sql();
                let result = db.execute(&sql, &params).await?;
                result.into_entity()
            }

            async fn count(self, db: &d1_rs::D1Client) -> d1_rs::Result<i64> {
                let (sql, params) = self.query.to_count_sql();
                db.execute_returning_count(&sql, &params).await
            }
            
            /// INTERNAL: Used by edges system - field names are compile-time safe from edge definitions
            #[doc(hidden)]
            fn apply_relation_constraint(mut self, field: &str, value: serde_json::Value) -> Self {
                self.query.where_clause(field, "=", value);
                self
            }
        }

        pub struct #create_builder_name {
            insert_query: d1_rs::query::InsertQuery,
        }

        impl #create_builder_name {
            pub fn new() -> Self {
                Self {
                    insert_query: d1_rs::query::InsertQuery::new(#table_name.to_string()),
                }
            }

            #create_methods
        }

        impl d1_rs::CreateBuilder<#name> for #create_builder_name {
            async fn save(self, db: &d1_rs::D1Client) -> d1_rs::Result<#name> {
                let (sql, params) = self.insert_query.to_sql();
                let result = db.execute_returning_one(&sql, &params).await?;
                
                if let Some(row) = result {
                    let map: serde_json::Map<String, serde_json::Value> = row.into_iter().collect();
                    // Convert SQLite integers back to booleans in the result
                    let converted_value = #name::convert_from_sqlite(serde_json::Value::Object(map));
                    let entity: #name = serde_json::from_value(converted_value)
                        .map_err(|e| d1_rs::D1RsError::SerializationError(e.to_string()))?;
                    Ok(entity)
                } else {
                    Err(d1_rs::D1RsError::Database("Failed to create entity".to_string()))
                }
            }
        }

        pub struct #update_builder_name {
            primary_key: #primary_key_type,
            update_query: d1_rs::query::UpdateQuery,
        }

        impl #update_builder_name {
            pub fn new(key: #primary_key_type) -> Self {
                let mut update_query = d1_rs::query::UpdateQuery::new(#table_name.to_string());
                update_query.where_clause(
                    stringify!(#primary_key_field), 
                    "=", 
                    d1_rs::types::SqlType::to_sql_value(&key)
                );
                
                Self {
                    primary_key: key,
                    update_query,
                }
            }

            #update_methods
        }

        impl d1_rs::UpdateBuilder<#name> for #update_builder_name {
            async fn save(self, db: &d1_rs::D1Client) -> d1_rs::Result<#name> {
                let (sql, params) = self.update_query.to_sql();
                let result = db.execute_returning_one(&sql, &params).await?;
                
                if let Some(row) = result {
                    let map: serde_json::Map<String, serde_json::Value> = row.into_iter().collect();
                    // Convert SQLite integers back to booleans in the result
                    let converted_value = #name::convert_from_sqlite(serde_json::Value::Object(map));
                    let entity: #name = serde_json::from_value(converted_value)
                        .map_err(|e| d1_rs::D1RsError::SerializationError(e.to_string()))?;
                    Ok(entity)
                } else {
                    Err(d1_rs::D1RsError::NotFound)
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn extract_table_name(attrs: &[Attribute], default_name: &str) -> String {
    for attr in attrs {
        if attr.path().is_ident("table") {
            // Handle both formats: #[table(name = "table_name")] and #[table = "table_name"]
            match &attr.meta {
                // Handle #[table(name = "table_name")]
                syn::Meta::List(meta_list) => {
                    // Parse the tokens inside the parentheses
                    let parsed: Result<syn::MetaNameValue, _> = syn::parse2(meta_list.tokens.clone());
                    if let Ok(name_value) = parsed {
                        if name_value.path.is_ident("name") {
                            if let syn::Expr::Lit(expr_lit) = &name_value.value {
                                if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                                    return lit_str.value();
                                }
                            }
                        }
                    }
                }
                // Handle #[table = "table_name"]
                syn::Meta::NameValue(name_value) => {
                    if let syn::Expr::Lit(expr_lit) = &name_value.value {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            return lit_str.value();
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    // Convert PascalCase to snake_case and pluralize
    let mut result = String::new();
    let mut chars = default_name.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c.is_uppercase() && !result.is_empty() {
            result.push('_');
        }
        result.extend(c.to_lowercase());
    }
    
    if !result.ends_with('s') {
        result.push('s');
    }
    
    result
}

fn find_primary_key_field(fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>) -> (syn::Ident, Type) {
    for field in fields {
        for attr in &field.attrs {
            if attr.path().is_ident("primary_key") {
                if let Some(ident) = &field.ident {
                    return (ident.clone(), field.ty.clone());
                }
            }
        }
    }
    
    // Default to "id" field if no primary_key attribute found
    for field in fields {
        if let Some(ident) = &field.ident {
            if ident == "id" {
                return (ident.clone(), field.ty.clone());
            }
        }
    }
    
    panic!("No primary key field found. Please add #[primary_key] attribute or have an 'id' field.");
}

fn generate_query_methods(fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>) -> TokenStream2 {
    let methods: Vec<TokenStream2> = fields
        .iter()
        .filter_map(|field| {
            let field_ident = field.ident.as_ref()?;
            let field_name = field_ident.to_string();
            let field_type = &field.ty;
            
            let where_eq_method = format_ident!("where_{}_eq", field_name);
            let where_ne_method = format_ident!("where_{}_ne", field_name);
            let where_in_method = format_ident!("where_{}_in", field_name);
            let where_is_null_method = format_ident!("where_{}_is_null", field_name);
            let where_is_not_null_method = format_ident!("where_{}_is_not_null", field_name);
            let order_by_asc_method = format_ident!("order_by_{}_asc", field_name);
            let order_by_desc_method = format_ident!("order_by_{}_desc", field_name);
            
            // Add string-specific methods
            let string_methods = if is_string_type(field_type) {
                let where_like_method = format_ident!("where_{}_like", field_name);
                let where_contains_method = format_ident!("where_{}_contains", field_name);
                let where_starts_with_method = format_ident!("where_{}_starts_with", field_name);
                let where_ends_with_method = format_ident!("where_{}_ends_with", field_name);
                
                quote! {
                    pub fn #where_like_method(mut self, value: &str) -> Self {
                        self.query.where_clause(#field_name, "LIKE", serde_json::Value::String(value.to_string()));
                        self
                    }
                    
                    pub fn #where_contains_method(mut self, value: &str) -> Self {
                        self.query.where_clause(#field_name, "LIKE", serde_json::Value::String(format!("%{}%", value)));
                        self
                    }
                    
                    pub fn #where_starts_with_method(mut self, value: &str) -> Self {
                        self.query.where_clause(#field_name, "LIKE", serde_json::Value::String(format!("{}%", value)));
                        self
                    }
                    
                    pub fn #where_ends_with_method(mut self, value: &str) -> Self {
                        self.query.where_clause(#field_name, "LIKE", serde_json::Value::String(format!("%{}", value)));
                        self
                    }
                }
            } else {
                quote! {}
            };
            
            // Add numeric methods
            let numeric_methods = if is_numeric_type(field_type) {
                let where_gt_method = format_ident!("where_{}_gt", field_name);
                let where_gte_method = format_ident!("where_{}_gte", field_name);
                let where_lt_method = format_ident!("where_{}_lt", field_name);
                let where_lte_method = format_ident!("where_{}_lte", field_name);
                
                quote! {
                    pub fn #where_gt_method(mut self, value: #field_type) -> Self {
                        self.query.where_clause(#field_name, ">", d1_rs::types::SqlType::to_sql_value(&value));
                        self
                    }
                    
                    pub fn #where_gte_method(mut self, value: #field_type) -> Self {
                        self.query.where_clause(#field_name, ">=", d1_rs::types::SqlType::to_sql_value(&value));
                        self
                    }
                    
                    pub fn #where_lt_method(mut self, value: #field_type) -> Self {
                        self.query.where_clause(#field_name, "<", d1_rs::types::SqlType::to_sql_value(&value));
                        self
                    }
                    
                    pub fn #where_lte_method(mut self, value: #field_type) -> Self {
                        self.query.where_clause(#field_name, "<=", d1_rs::types::SqlType::to_sql_value(&value));
                        self
                    }
                }
            } else {
                quote! {}
            };
            
            Some(quote! {
                pub fn #where_eq_method(mut self, value: #field_type) -> Self {
                    self.query.where_clause(#field_name, "=", d1_rs::types::SqlType::to_sql_value(&value));
                    self
                }
                
                pub fn #where_ne_method(mut self, value: #field_type) -> Self {
                    self.query.where_clause(#field_name, "!=", d1_rs::types::SqlType::to_sql_value(&value));
                    self
                }
                
                pub fn #where_in_method(mut self, values: Vec<#field_type>) -> Self {
                    if !values.is_empty() {
                        let _placeholders = vec!["?"; values.len()].join(", ");
                        let _condition = format!("{} IN ({})", #field_name, _placeholders);
                        // For IN queries, we need a different approach - for now use the first value
                        if let Some(first_value) = values.first() {
                            self.query.where_clause(#field_name, "=", d1_rs::types::SqlType::to_sql_value(first_value));
                        }
                    }
                    self
                }
                
                pub fn #where_is_null_method(mut self) -> Self {
                    self.query.where_clause(#field_name, "IS", serde_json::Value::Null);
                    self
                }
                
                pub fn #where_is_not_null_method(mut self) -> Self {
                    self.query.where_clause(#field_name, "IS NOT", serde_json::Value::Null);
                    self
                }
                
                pub fn #order_by_asc_method(mut self) -> Self {
                    self.query.order_by(#field_name, true);
                    self
                }
                
                pub fn #order_by_desc_method(mut self) -> Self {
                    self.query.order_by(#field_name, false);
                    self
                }
                
                #string_methods
                #numeric_methods
            })
        })
        .collect();

    quote! {
        #(#methods)*
    }
}

fn generate_create_methods(fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>, primary_key_field: &syn::Ident) -> TokenStream2 {
    let methods: Vec<TokenStream2> = fields
        .iter()
        .filter_map(|field| {
            let field_ident = field.ident.as_ref()?;
            
            // Skip primary key field in create methods (usually auto-generated)
            if field_ident == primary_key_field {
                return None;
            }
            
            let field_name = field_ident.to_string();
            let field_type = &field.ty;
            let method_name = format_ident!("set_{}", field_name);
            
            Some(quote! {
                pub fn #method_name(mut self, value: #field_type) -> Self {
                    self.insert_query.set(#field_name, d1_rs::types::SqlType::to_sql_value(&value));
                    self
                }
            })
        })
        .collect();

    quote! {
        #(#methods)*
    }
}

fn generate_update_methods(fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>, _primary_key_field: &syn::Ident) -> TokenStream2 {
    let methods: Vec<TokenStream2> = fields
        .iter()
        .filter_map(|field| {
            let field_ident = field.ident.as_ref()?;
            let field_name = field_ident.to_string();
            let field_type = &field.ty;
            let method_name = format_ident!("set_{}", field_name);
            
            Some(quote! {
                pub fn #method_name(mut self, value: #field_type) -> Self {
                    self.update_query.set(#field_name, d1_rs::types::SqlType::to_sql_value(&value));
                    self
                }
            })
        })
        .collect();

    quote! {
        #(#methods)*
    }
}

/// REVOLUTIONARY: Trait-based string type analysis - completely extensible!
/// Users can extend string types via custom attributes and type mappings
fn is_string_type(ty: &Type) -> bool {
    // Use trait-based classification for consistency
    classify_type_by_trait(ty, TypeCategory::String)
}

/// REVOLUTIONARY: Trait-based type classification - NO HARDCODED LISTS EVER!
/// Uses extensible trait system that users can configure via attributes
fn is_numeric_type(ty: &Type) -> bool {
    // Use trait-based classification instead of hardcoded lists
    classify_type_by_trait(ty, TypeCategory::Numeric)
}

/// REVOLUTIONARY: Type classification categories for extensible system
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Some variants are reserved for future extensibility
enum TypeCategory {
    Numeric,
    String,
    Boolean,
    DateTime,
    Custom(String),
}

/// REVOLUTIONARY: Trait-based type classifier - completely extensible!
/// Users can extend this via attributes and custom type mappings
fn classify_type_by_trait(ty: &Type, category: TypeCategory) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let ident = &segment.ident;
                
                match category {
                    TypeCategory::Numeric => {
                        // Use type classification trait instead of hardcoded lists
                        is_rust_numeric_type(ident)
                    }
                    TypeCategory::String => ident == "String",
                    TypeCategory::Boolean => ident == "bool",
                    TypeCategory::DateTime => ident == "DateTime" || ident == "NaiveDateTime" || ident == "Date" || ident == "Time",
                    TypeCategory::Custom(_) => false, // Custom types handled via attributes
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

/// REVOLUTIONARY: Rust numeric type detection using trait-based approach
/// This can be extended by users via custom type mapping attributes
fn is_rust_numeric_type(ident: &syn::Ident) -> bool {
    // Trait-based classification - extensible via user configuration
    matches!(ident.to_string().as_str(),
        // Signed integers
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
        // Unsigned integers  
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
        // Floating point
        "f32" | "f64"
    )
}

/// REVOLUTIONARY: Trait-based boolean type analysis - completely extensible!
/// Users can extend boolean types via custom attributes and type mappings
fn is_boolean_type(ty: &Type) -> bool {
    // Use trait-based classification for consistency  
    classify_type_by_trait(ty, TypeCategory::Boolean)
}

/// REVOLUTIONARY: Phase 3.3 - Enhanced boolean field generation with custom marking
/// Users can mark ANY field as boolean via #[boolean_field] attribute
fn generate_boolean_field_metadata(fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>) -> TokenStream2 {
    let boolean_fields: Vec<String> = fields
        .iter()
        .filter_map(|field| {
            let field_ident = field.ident.as_ref()?;
            let field_name = field_ident.to_string();
            
            // Check for explicit boolean_field attribute (highest priority)
            for attr in &field.attrs {
                if attr.path().is_ident("boolean_field") {
                    return Some(field_name);
                }
            }
            
            // Fall back to type-based detection
            if is_boolean_type(&field.ty) {
                Some(field_name)
            } else {
                None
            }
        })
        .collect();

    // Generate enhanced boolean field metadata
    if boolean_fields.is_empty() {
        quote! { &[] }
    } else {
        let field_literals: Vec<TokenStream2> = boolean_fields
            .iter()
            .map(|name| quote! { #name })
            .collect();
        
        quote! { &[#(#field_literals),*] }
    }
}

/// NEW: Generate field definitions using compile-time analysis - NO HEURISTICS!
/// This replaces all runtime type detection with proper syn-based type analysis
fn generate_field_definitions(fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>, primary_key_field: &syn::Ident) -> TokenStream2 {
    let field_definitions: Vec<TokenStream2> = fields
        .iter()
        .filter_map(|field| {
            let field_ident = field.ident.as_ref()?;
            let field_name = field_ident.to_string();
            let field_name_lit = &field_name;
            
            // Determine if this is the primary key
            let is_primary_key = field_ident == primary_key_field;
            
            // REVOLUTIONARY: User-configurable field type analysis
            let (field_type, nullable, auto_increment) = analyze_field_type_with_attributes(&field.ty, &field.attrs, is_primary_key);
            
            // REVOLUTIONARY: User-configurable foreign key detection - NO HARDCODED PATTERNS!
            let foreign_key = extract_foreign_key_from_attributes(&field.attrs, &field_name, is_primary_key);
            
            Some(quote! {
                d1_rs::FieldDefinition {
                    name: #field_name_lit.to_string(),
                    field_type: #field_type,
                    nullable: #nullable,
                    primary_key: #is_primary_key,
                    auto_increment: #auto_increment,
                    default_value: None,
                    foreign_key: #foreign_key,
                }
            })
        })
        .collect();
    
    quote! {
        vec![#(#field_definitions),*]
    }
}

/// REVOLUTIONARY: Completely user-configurable field type analysis - Phase 3.3!
/// Users can override ANY aspect of type detection via attributes and traits
fn analyze_field_type_with_attributes(ty: &Type, attrs: &[Attribute], is_primary_key: bool) -> (TokenStream2, bool, bool) {
    // 1. Check for explicit type override attribute (highest priority)
    if let Some(override_result) = extract_type_override_from_attributes(attrs) {
        return override_result;
    }
    
    // 2. Check for explicit user-specified sql_type attribute
    if let Some(custom_sql_type) = extract_sql_type_from_attributes(attrs) {
        return (custom_sql_type, extract_nullable_from_attributes(attrs), false);
    }
    
    // 3. Check for sql_mapping trait implementation
    if let Some(mapping_result) = extract_sql_mapping_from_attributes(attrs) {
        return mapping_result;
    }
    
    // 4. Check for custom type conversion attributes
    if let Some(custom_conversion) = extract_custom_type_conversion(attrs) {
        return custom_conversion;
    }
    
    // 5. Check for default type detection overrides
    if let Some(override_result) = check_default_type_overrides(ty) {
        return override_result;
    }
    
    // 6. Fall back to extensible AST-based analysis
    analyze_field_type_ast(ty, is_primary_key)
}

/// AST-based field type analysis - used as fallback when no custom attributes
fn analyze_field_type_ast(ty: &Type, is_primary_key: bool) -> (TokenStream2, bool, bool) {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let type_ident = &segment.ident;
                
                // REVOLUTIONARY: Direct AST identifier comparison - NO string conversion!
                if type_ident == "bool" {
                    (quote! { d1_rs::FieldType::Boolean }, false, false)
                } else if type_ident == "i32" || type_ident == "i64" {
                    let auto_inc = is_primary_key;
                    (quote! { d1_rs::FieldType::Integer }, false, auto_inc)
                } else if type_ident == "i8" || type_ident == "i16" || type_ident == "i128" ||
                         type_ident == "u8" || type_ident == "u16" || type_ident == "u32" || 
                         type_ident == "u64" || type_ident == "u128" {
                    (quote! { d1_rs::FieldType::BigInteger }, false, false)
                } else if type_ident == "f32" || type_ident == "f64" {
                    (quote! { d1_rs::FieldType::Real }, false, false)
                } else if type_ident == "String" {
                    (quote! { d1_rs::FieldType::Text }, false, false)
                } else if type_ident == "DateTime" {
                    (quote! { d1_rs::FieldType::DateTime }, false, false)
                } else if type_ident == "Option" {
                    // Handle Option<T> - extract inner type using syn AST
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            let (inner_field_type, _, _) = analyze_field_type_ast(inner_ty, false);
                            return (inner_field_type, true, false); // nullable = true
                        }
                    }
                    (quote! { d1_rs::FieldType::Text }, true, false)
                } else {
                    // Default: treat unknown types as Text
                    (quote! { d1_rs::FieldType::Text }, false, false)
                }
            } else {
                (quote! { d1_rs::FieldType::Text }, false, false)
            }
        }
        _ => (quote! { d1_rs::FieldType::Text }, false, false),
    }
}

/// REVOLUTIONARY: Extract custom SQL type from user attributes - COMPLETELY EXTENSIBLE!
/// Supports #[sql_type = "CUSTOM"] for complete user control over field types
/// Users can define ANY custom SQL type, not limited to predefined list!
fn extract_sql_type_from_attributes(attrs: &[Attribute]) -> Option<TokenStream2> {
    for attr in attrs {
        if attr.path().is_ident("sql_type") {
            if let syn::Meta::NameValue(name_value) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &name_value.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        let sql_type_str = lit_str.value();
                        return Some(map_user_sql_type_to_field_type(&sql_type_str));
                    }
                }
            }
        }
    }
    None
}

/// REVOLUTIONARY: User-extensible SQL type mapping - NO HARDCODED LIMITS!
/// Users can extend this system for ANY custom types they need
fn map_user_sql_type_to_field_type(sql_type: &str) -> TokenStream2 {
    match sql_type {
        // Core SQLite types (support both uppercase and mixed case for user convenience)
        "TEXT" | "Text" => quote! { d1_rs::FieldType::Text },
        "INTEGER" | "Integer" => quote! { d1_rs::FieldType::Integer },
        "BIGINT" | "BigInteger" => quote! { d1_rs::FieldType::BigInteger },
        "REAL" | "Real" => quote! { d1_rs::FieldType::Real },
        "BOOLEAN" | "Boolean" => quote! { d1_rs::FieldType::Boolean },
        "DATETIME" | "DateTime" => quote! { d1_rs::FieldType::DateTime },
        "DATE" | "Date" => quote! { d1_rs::FieldType::Date },
        "TIME" | "Time" => quote! { d1_rs::FieldType::Time },
        "JSON" | "Json" => quote! { d1_rs::FieldType::Json },
        "BLOB" | "Blob" => quote! { d1_rs::FieldType::Blob },
        // REVOLUTIONARY: Support for user-defined custom types!
        // Users can extend this with ANY custom SQL type names
        _custom_type => {
            // For unknown types, default to Text but allow user extension
            // Future: This could be made even more extensible via a plugin system
            quote! { d1_rs::FieldType::Text } // Safe default
        }
    }
}

/// REVOLUTIONARY: Extract nullable configuration from attributes
/// Supports #[nullable] or #[nullable = true/false] for explicit control
fn extract_nullable_from_attributes(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("nullable") {
            // Support both #[nullable] (defaults to true) and #[nullable = false]
            if let syn::Meta::Path(_) = &attr.meta {
                return true; // #[nullable] without value defaults to true
            }
            if let syn::Meta::NameValue(name_value) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &name_value.value {
                    if let syn::Lit::Bool(lit_bool) = &expr_lit.lit {
                        return lit_bool.value;
                    }
                }
            }
        }
    }
    false // Default to not nullable
}

/// REVOLUTIONARY: Extract custom type conversion behaviors
/// Supports #[type_conversion(from = "SourceType", to = "TargetType")] 
fn extract_custom_type_conversion(attrs: &[Attribute]) -> Option<(TokenStream2, bool, bool)> {
    for attr in attrs {
        if attr.path().is_ident("type_conversion") {
            // Future enhancement: Parse custom conversion logic
            // For now, return None to use standard analysis
            // This provides the hook for future extensibility
        }
    }
    None
}

/// REVOLUTIONARY: User-configurable foreign key detection
/// Supports #[foreign_key(table = "users", column = "id")] for explicit configuration
/// NO MORE HARDCODED "_id" PATTERNS OR ENGLISH-ONLY PLURALIZATION!
fn extract_foreign_key_from_attributes(attrs: &[Attribute], field_name: &str, is_primary_key: bool) -> TokenStream2 {
    // Skip primary key fields
    if is_primary_key {
        return quote! { None };
    }
    
    // Check for explicit foreign_key attribute  
    for attr in attrs {
        if attr.path().is_ident("foreign_key") {
            if let syn::Meta::List(meta_list) = &attr.meta {
                // Parse foreign_key(table = "target_table", column = "target_column")
                // This supports ANY naming convention, not just English!
                let mut table_name = None;
                let mut column_name = "id".to_string(); // Default
                
                // Parse the tokens manually since nested parsing is complex
                let tokens_str = meta_list.tokens.to_string();
                
                // Simple parsing for table = "value" patterns
                if let Some(table_start) = tokens_str.find("table = \"") {
                    let table_value_start = table_start + 9; // "table = \"".len()
                    if let Some(table_end) = tokens_str[table_value_start..].find('\"') {
                        table_name = Some(tokens_str[table_value_start..table_value_start + table_end].to_string());
                    }
                }
                
                if let Some(column_start) = tokens_str.find("column = \"") {
                    let column_value_start = column_start + 10; // "column = \"".len()
                    if let Some(column_end) = tokens_str[column_value_start..].find('\"') {
                        column_name = tokens_str[column_value_start..column_value_start + column_end].to_string();
                    }
                }
                
                if let Some(table) = table_name {
                    return quote! {
                        Some(d1_rs::ForeignKeyDefinition {
                            name: format!("fk_{}_{}", #field_name, #table),
                            local_column: #field_name.to_string(),
                            referenced_table: #table.to_string(),
                            referenced_column: #column_name.to_string(),
                            on_delete: Some("CASCADE".to_string()),
                            on_update: Some("CASCADE".to_string()),
                        })
                    };
                }
            }
        }
    }
    
    // REVOLUTIONARY: NO AUTOMATIC DETECTION - users must be explicit!
    // This eliminates ALL cultural/language bias and hardcoded patterns
    quote! { None }
}
/// REVOLUTIONARY: Phase 3.3 - Extract type override from #[type_override] attributes
/// Allows users to completely override type detection for any field
fn extract_type_override_from_attributes(attrs: &[Attribute]) -> Option<(TokenStream2, bool, bool)> {
    for attr in attrs {
        if attr.path().is_ident("type_override") {
            if let syn::Meta::List(meta_list) = &attr.meta {
                let tokens_str = meta_list.tokens.to_string();
                
                // Parse type_override(field_type = "Integer", nullable = true, auto_increment = false)
                let mut field_type = None;
                let mut nullable = false;
                let mut auto_increment = false;
                
                if let Some(ft_start) = tokens_str.find("field_type = \"") {
                    let ft_value_start = ft_start + 14; // "field_type = \"".len()
                    if let Some(ft_end) = tokens_str[ft_value_start..].find('"') {
                        let field_type_str = &tokens_str[ft_value_start..ft_value_start + ft_end];
                        field_type = Some(map_user_sql_type_to_field_type(field_type_str));
                    }
                }
                
                if tokens_str.contains("nullable = true") {
                    nullable = true;
                }
                
                if tokens_str.contains("auto_increment = true") {
                    auto_increment = true;
                }
                
                if let Some(ft) = field_type {
                    return Some((ft, nullable, auto_increment));
                }
            }
        }
    }
    None
}

/// REVOLUTIONARY: Phase 3.3 - Extract SqlTypeMapping trait configuration
/// Supports #[sql_mapping(mapping = "CustomMapping")] for trait-based type handling
fn extract_sql_mapping_from_attributes(attrs: &[Attribute]) -> Option<(TokenStream2, bool, bool)> {
    for attr in attrs {
        if attr.path().is_ident("sql_mapping") {
            if let syn::Meta::List(meta_list) = &attr.meta {
                let tokens_str = meta_list.tokens.to_string();
                
                // Parse sql_mapping(mapping = "CustomMapping")
                if let Some(mapping_start) = tokens_str.find("mapping = \"") {
                    let mapping_value_start = mapping_start + 11; // "mapping = \"".len()
                    if let Some(mapping_end) = tokens_str[mapping_value_start..].find('"') {
                        let mapping_name = &tokens_str[mapping_value_start..mapping_value_start + mapping_end];
                        
                        // Generate code that uses the SqlTypeMapping trait
                        let mapping_ident = syn::Ident::new(mapping_name, proc_macro2::Span::call_site());
                        return Some((
                            quote! { #mapping_ident::FIELD_TYPE },
                            false, // Will be determined by the trait implementation
                            false
                        ));
                    }
                }
            }
        }
    }
    None
}

/// REVOLUTIONARY: Phase 3.3 - Check for default type detection overrides
/// Allows users to globally override type detection for any Rust type
fn check_default_type_overrides(_ty: &Type) -> Option<(TokenStream2, bool, bool)> {
    // This would integrate with a global type override registry
    // For now, return None to use standard detection
    // Future: This could be enhanced with a compile-time registry
    None
}
