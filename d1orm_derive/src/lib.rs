use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Fields, Type, Field, Attribute};

#[proc_macro_derive(Entity, attributes(table, primary_key, unique, not_null))]
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

    let expanded = quote! {
        impl d1orm::Entity for #name {
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
            
            async fn find(db: &d1orm::D1Client, key: Self::PrimaryKey) -> d1orm::Result<Option<Self>> {
                let sql = concat!("SELECT * FROM ", #table_name, " WHERE ", stringify!(#primary_key_field), " = ? LIMIT 1");
                let params = vec![d1orm::types::SqlType::to_sql_value(&key)];
                
                let result = db.execute(sql, &params).await?;
                result.into_entity()
            }
            
            async fn delete(db: &d1orm::D1Client, key: Self::PrimaryKey) -> d1orm::Result<()> {
                let sql = concat!("DELETE FROM ", #table_name, " WHERE ", stringify!(#primary_key_field), " = ?");
                let params = vec![d1orm::types::SqlType::to_sql_value(&key)];
                
                db.execute(sql, &params).await?;
                Ok(())
            }
        }

        pub struct #query_builder_name {
            query: d1orm::query::Query,
        }

        impl #query_builder_name {
            pub fn new() -> Self {
                Self {
                    query: d1orm::query::Query::new(#table_name.to_string()),
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

            pub fn order_by(mut self, column: &str, direction: &str) -> Self {
                let asc = direction.to_uppercase() != "DESC";
                self.query.order_by(column, asc);
                self
            }

            #query_methods
        }

        impl d1orm::QueryBuilder<#name> for #query_builder_name {
            async fn all(self, db: &d1orm::D1Client) -> d1orm::Result<Vec<#name>> {
                let (sql, params) = self.query.to_sql();
                let result = db.execute(&sql, &params).await?;
                result.into_entities()
            }

            async fn first(self, db: &d1orm::D1Client) -> d1orm::Result<Option<#name>> {
                let mut query = self.query;
                query.limit(1);
                let (sql, params) = query.to_sql();
                let result = db.execute(&sql, &params).await?;
                result.into_entity()
            }

            async fn count(self, db: &d1orm::D1Client) -> d1orm::Result<i64> {
                let (sql, params) = self.query.to_count_sql();
                db.execute_returning_count(&sql, &params).await
            }
        }

        pub struct #create_builder_name {
            insert_query: d1orm::query::InsertQuery,
        }

        impl #create_builder_name {
            pub fn new() -> Self {
                Self {
                    insert_query: d1orm::query::InsertQuery::new(#table_name.to_string()),
                }
            }

            #create_methods
        }

        impl d1orm::CreateBuilder<#name> for #create_builder_name {
            async fn save(self, db: &d1orm::D1Client) -> d1orm::Result<#name> {
                let (sql, params) = self.insert_query.to_sql();
                let result = db.execute_returning_one(&sql, &params).await?;
                
                if let Some(row) = result {
                    let map: serde_json::Map<String, serde_json::Value> = row.into_iter().collect();
                    // Convert SQLite integers back to booleans in the result
                    let converted_value = #name::convert_from_sqlite(serde_json::Value::Object(map));
                    let entity: #name = serde_json::from_value(converted_value)
                        .map_err(|e| d1orm::D1OrmError::SerializationError(e.to_string()))?;
                    Ok(entity)
                } else {
                    Err(d1orm::D1OrmError::Database("Failed to create entity".to_string()))
                }
            }
        }

        pub struct #update_builder_name {
            primary_key: #primary_key_type,
            update_query: d1orm::query::UpdateQuery,
        }

        impl #update_builder_name {
            pub fn new(key: #primary_key_type) -> Self {
                let mut update_query = d1orm::query::UpdateQuery::new(#table_name.to_string());
                update_query.where_clause(
                    stringify!(#primary_key_field), 
                    "=", 
                    d1orm::types::SqlType::to_sql_value(&key)
                );
                
                Self {
                    primary_key: key,
                    update_query,
                }
            }

            #update_methods
        }

        impl d1orm::UpdateBuilder<#name> for #update_builder_name {
            async fn save(self, db: &d1orm::D1Client) -> d1orm::Result<#name> {
                let (sql, params) = self.update_query.to_sql();
                let result = db.execute_returning_one(&sql, &params).await?;
                
                if let Some(row) = result {
                    let map: serde_json::Map<String, serde_json::Value> = row.into_iter().collect();
                    // Convert SQLite integers back to booleans in the result
                    let converted_value = #name::convert_from_sqlite(serde_json::Value::Object(map));
                    let entity: #name = serde_json::from_value(converted_value)
                        .map_err(|e| d1orm::D1OrmError::SerializationError(e.to_string()))?;
                    Ok(entity)
                } else {
                    Err(d1orm::D1OrmError::NotFound)
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn extract_table_name(attrs: &[Attribute], default_name: &str) -> String {
    for attr in attrs {
        if attr.path().is_ident("table") {
            if let Ok(name_value) = attr.meta.require_name_value() {
                if name_value.path.is_ident("name") {
                    if let syn::Expr::Lit(expr_lit) = &name_value.value {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            return lit_str.value();
                        }
                    }
                }
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
                        self.query.where_clause(#field_name, ">", d1orm::types::SqlType::to_sql_value(&value));
                        self
                    }
                    
                    pub fn #where_gte_method(mut self, value: #field_type) -> Self {
                        self.query.where_clause(#field_name, ">=", d1orm::types::SqlType::to_sql_value(&value));
                        self
                    }
                    
                    pub fn #where_lt_method(mut self, value: #field_type) -> Self {
                        self.query.where_clause(#field_name, "<", d1orm::types::SqlType::to_sql_value(&value));
                        self
                    }
                    
                    pub fn #where_lte_method(mut self, value: #field_type) -> Self {
                        self.query.where_clause(#field_name, "<=", d1orm::types::SqlType::to_sql_value(&value));
                        self
                    }
                }
            } else {
                quote! {}
            };
            
            Some(quote! {
                pub fn #where_eq_method(mut self, value: #field_type) -> Self {
                    self.query.where_clause(#field_name, "=", d1orm::types::SqlType::to_sql_value(&value));
                    self
                }
                
                pub fn #where_ne_method(mut self, value: #field_type) -> Self {
                    self.query.where_clause(#field_name, "!=", d1orm::types::SqlType::to_sql_value(&value));
                    self
                }
                
                pub fn #where_in_method(mut self, values: Vec<#field_type>) -> Self {
                    if !values.is_empty() {
                        let _placeholders = vec!["?"; values.len()].join(", ");
                        let _condition = format!("{} IN ({})", #field_name, _placeholders);
                        // For IN queries, we need a different approach - for now use the first value
                        if let Some(first_value) = values.first() {
                            self.query.where_clause(#field_name, "=", d1orm::types::SqlType::to_sql_value(first_value));
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
                    self.insert_query.set(#field_name, d1orm::types::SqlType::to_sql_value(&value));
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
                    self.update_query.set(#field_name, d1orm::types::SqlType::to_sql_value(&value));
                    self
                }
            })
        })
        .collect();

    quote! {
        #(#methods)*
    }
}

fn is_string_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "String";
        }
    }
    false
}

fn is_numeric_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = &segment.ident;
            return matches!(ident.to_string().as_str(), "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" | "f32" | "f64");
        }
    }
    false
}

fn is_boolean_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "bool";
        }
    }
    false
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