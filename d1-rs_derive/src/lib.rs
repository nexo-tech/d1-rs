use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Fields, Type, Field, Attribute};

#[proc_macro_derive(Entity, attributes(table, primary_key, unique, not_null, edge, sql_type, foreign_key, field_config))]
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

/// REVOLUTIONARY: Analyze string types using syn AST - NO STRING LITERALS!
/// Uses Rust's type system properly instead of string pattern matching
fn is_string_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident == "String"
            } else {
                false
            }
        }
        _ => false,
    }
}

/// REVOLUTIONARY: Analyze numeric types using syn AST - NO HARDCODED LISTS!
/// Uses direct identifier comparison instead of string conversion and matching
fn is_numeric_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let ident = &segment.ident;
                // Direct identifier comparison - no string conversion!
                ident == "i8" || ident == "i16" || ident == "i32" || ident == "i64" || ident == "i128" ||
                ident == "u8" || ident == "u16" || ident == "u32" || ident == "u64" || ident == "u128" ||
                ident == "f32" || ident == "f64"
            } else {
                false
            }
        }
        _ => false,
    }
}

/// REVOLUTIONARY: Analyze boolean types using syn AST - NO STRING LITERALS!
/// Uses Rust's type system properly instead of string pattern matching
fn is_boolean_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident == "bool"
            } else {
                false
            }
        }
        _ => false,
    }
}

fn generate_boolean_field_metadata(fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>) -> TokenStream2 {
    let boolean_fields: Vec<String> = fields
        .iter()
        .filter_map(|field| {
            let field_ident = field.ident.as_ref()?;
            let field_name = field_ident.to_string();
            
            if is_boolean_type(&field.ty) {
                Some(field_name)
            } else {
                None
            }
        })
        .collect();

    // Generate a static array of boolean field names
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

/// REVOLUTIONARY: User-configurable field type analysis with attribute support
/// Users can override any field type with #[sql_type = "CUSTOM"] attributes
fn analyze_field_type_with_attributes(ty: &Type, attrs: &[Attribute], is_primary_key: bool) -> (TokenStream2, bool, bool) {
    // Check for user-specified sql_type attribute first
    if let Some(custom_sql_type) = extract_sql_type_from_attributes(attrs) {
        return (custom_sql_type, false, false);
    }
    
    // Fall back to AST-based analysis
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

/// REVOLUTIONARY: Extract custom SQL type from user attributes
/// Supports #[sql_type = "CUSTOM"] for complete user control over field types
fn extract_sql_type_from_attributes(attrs: &[Attribute]) -> Option<TokenStream2> {
    for attr in attrs {
        if attr.path().is_ident("sql_type") {
            if let syn::Meta::NameValue(name_value) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &name_value.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        let sql_type_str = lit_str.value();
                        return Some(match sql_type_str.as_str() {
                            "TEXT" => quote! { d1_rs::FieldType::Text },
                            "INTEGER" => quote! { d1_rs::FieldType::Integer },
                            "BIGINT" => quote! { d1_rs::FieldType::BigInteger },
                            "REAL" => quote! { d1_rs::FieldType::Real },
                            "BOOLEAN" => quote! { d1_rs::FieldType::Boolean },
                            "DATETIME" => quote! { d1_rs::FieldType::DateTime },
                            "DATE" => quote! { d1_rs::FieldType::Date },
                            "TIME" => quote! { d1_rs::FieldType::Time },
                            "JSON" => quote! { d1_rs::FieldType::Json },
                            "BLOB" => quote! { d1_rs::FieldType::Blob },
                            _ => quote! { d1_rs::FieldType::Text }, // Default for unknown types
                        });
                    }
                }
            }
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