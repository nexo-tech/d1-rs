# Performance Optimization

This guide covers strategies for optimizing d1-rs applications for high performance, both in Cloudflare Workers and local development.

## Database Design

### Indexing Strategy

Proper indexing is crucial for query performance:

```rust
// Migration with strategic indexes
let migration = SchemaMigration::new("optimize_performance".to_string())
    .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("email").not_null().unique().build()
        .text("name").not_null().build()
        .boolean("is_active").default_value(DefaultValue::Boolean(true)).build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .datetime("last_login").build()
    .build()
    
    // Add strategic indexes
    .alter_table("users")
        // Single column indexes
        .add_index("idx_users_email", vec!["email"])
        .add_index("idx_users_active", vec!["is_active"])
        .add_index("idx_users_created", vec!["created_at"])
        .add_index("idx_users_last_login", vec!["last_login"])
        
        // Composite indexes for common query patterns
        .add_index("idx_users_active_created", vec!["is_active", "created_at"])
        .add_index("idx_users_active_name", vec!["is_active", "name"])
    .build()
    
    .create_table("posts")
        .integer("id").primary_key().auto_increment().build()
        .integer("user_id").not_null().build()
        .text("title").not_null().build()
        .text("content").not_null().build()
        .boolean("is_published").default_value(DefaultValue::Boolean(false)).build()
        .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .datetime("published_at").build()
    .build()
    
    .alter_table("posts")
        // Foreign key index
        .add_index("idx_posts_user_id", vec!["user_id"])
        
        // Query-specific indexes
        .add_index("idx_posts_published", vec!["is_published"])
        .add_index("idx_posts_published_at", vec!["published_at"])
        
        // Composite indexes for common queries
        .add_index("idx_posts_user_published", vec!["user_id", "is_published"])
        .add_index("idx_posts_published_created", vec!["is_published", "created_at"])
    .build();
```

### Denormalization Strategies

Strategic denormalization can improve read performance:

```rust
#[derive(Entity, RelationalEntity)]
pub struct OptimizedUser {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_active: bool,
    
    // Denormalized counters
    pub post_count: i64,
    pub published_post_count: i64,
    pub last_post_date: Option<DateTime<Utc>>,
    
    // Cached derived data
    pub activity_score: f64, // Calculated based on posts, comments, etc.
    pub stats_updated_at: DateTime<Utc>,
}

impl OptimizedUser {
    // Update denormalized data
    pub async fn refresh_stats(&self, db: &D1Client) -> Result<OptimizedUser> {
        let posts = self.traverse::<Post>(db, "posts").await?;
        let published_posts: Vec<_> = posts.iter()
            .filter(|p| p.is_published)
            .collect();
        
        let last_post_date = posts.iter()
            .map(|p| p.created_at)
            .max();
        
        // Calculate activity score (example algorithm)
        let activity_score = self.calculate_activity_score(&posts).await;
        
        OptimizedUser::update(self.id)
            .set_post_count(posts.len() as i64)
            .set_published_post_count(published_posts.len() as i64)
            .set_last_post_date(last_post_date)
            .set_activity_score(activity_score)
            .set_stats_updated_at(Utc::now())
            .save(db)
            .await
    }
    
    async fn calculate_activity_score(&self, posts: &[Post]) -> f64 {
        // Example: Score based on recency and volume
        let now = Utc::now();
        let mut score = 0.0;
        
        for post in posts {
            let days_old = (now - post.created_at).num_days() as f64;
            let recency_factor = 1.0 / (1.0 + days_old / 30.0); // Decay over 30 days
            
            if post.is_published {
                score += 2.0 * recency_factor;
            } else {
                score += 1.0 * recency_factor;
            }
        }
        
        score
    }
    
    // Fast queries using denormalized data
    pub async fn most_active_users(db: &D1Client, limit: i64) -> Result<Vec<OptimizedUser>> {
        OptimizedUser::query()
            .where_is_active_eq(true)
            .order_by_activity_score_desc()
            .limit(limit)
            .all(db)
            .await
    }
    
    pub async fn recently_active_users(db: &D1Client, days: i64) -> Result<Vec<OptimizedUser>> {
        let cutoff = Utc::now() - Duration::days(days);
        
        OptimizedUser::query()
            .where_last_post_date_gte(cutoff)
            .order_by_last_post_date_desc()
            .all(db)
            .await
    }
}
```

## Query Optimization

### Efficient Query Patterns

Write queries that minimize database work:

```rust
// Good: Specific queries
impl Post {
    pub async fn recent_published(db: &D1Client, days: i64, limit: i64) -> Result<Vec<Post>> {
        let cutoff = Utc::now() - Duration::days(days);
        
        Post::query()
            .where_is_published_eq(true)
            .where_created_at_gte(cutoff)
            .order_by_created_at_desc()
            .limit(limit)
            .all(db)
            .await
    }
    
    // Avoid: Loading unnecessary data
    // pub async fn all_posts_then_filter() -> Result<Vec<Post>> {
    //     let all_posts = Post::query().all(db).await?;
    //     let filtered: Vec<_> = all_posts.into_iter()
    //         .filter(|p| p.is_published)
    //         .take(10)
    //         .collect();
    //     Ok(filtered)
    // }
}
```

### Pagination Best Practices

Implement efficient pagination:

```rust
#[derive(Debug, Serialize)]
pub struct PaginatedResult<T> {
    pub data: Vec<T>,
    pub page: i64,
    pub per_page: i64,
    pub total_count: Option<i64>,
    pub has_next: bool,
}

impl<T> PaginatedResult<T> {
    pub fn new(data: Vec<T>, page: i64, per_page: i64, has_next: bool) -> Self {
        Self {
            data,
            page,
            per_page,
            total_count: None,
            has_next,
        }
    }
    
    pub fn with_total_count(mut self, total_count: i64) -> Self {
        self.total_count = Some(total_count);
        self
    }
}

impl Post {
    pub async fn paginated(
        db: &D1Client,
        page: i64,
        per_page: i64,
        include_total: bool,
    ) -> Result<PaginatedResult<Post>> {
        let offset = page * per_page;
        
        // Load one extra to check if there's a next page
        let posts = Post::query()
            .where_is_published_eq(true)
            .order_by_created_at_desc()
            .limit(per_page + 1)
            .offset(offset)
            .all(db)
            .await?;
        
        let has_next = posts.len() > per_page as usize;
        let mut data = posts;
        if has_next {
            data.pop(); // Remove the extra item
        }
        
        let mut result = PaginatedResult::new(data, page, per_page, has_next);
        
        if include_total {
            let total_count = Post::query()
                .where_is_published_eq(true)
                .count(db)
                .await?;
            result = result.with_total_count(total_count);
        }
        
        Ok(result)
    }
    
    // Cursor-based pagination (more efficient for large datasets)
    pub async fn paginated_cursor(
        db: &D1Client,
        cursor: Option<i64>, // Last seen ID
        limit: i64,
    ) -> Result<PaginatedResult<Post>> {
        let mut query = Post::query()
            .where_is_published_eq(true)
            .order_by_id_desc()
            .limit(limit + 1);
        
        if let Some(cursor_id) = cursor {
            query = query.where_id_lt(cursor_id);
        }
        
        let posts = query.all(db).await?;
        
        let has_next = posts.len() > limit as usize;
        let mut data = posts;
        if has_next {
            data.pop();
        }
        
        Ok(PaginatedResult::new(data, 0, limit, has_next))
    }
}
```

### Batch Operations

Optimize bulk operations:

```rust
impl User {
    // Efficient bulk user creation
    pub async fn create_bulk(db: &D1Client, user_data: Vec<CreateUserData>) -> Result<Vec<User>> {
        let mut created_users = Vec::new();
        
        // In a real implementation with transactions:
        // db.transaction(|tx| async {
        //     for data in user_data {
        //         let user = User::create()
        //             .set_name(data.name)
        //             .set_email(data.email)
        //             .save(&tx)
        //             .await?;
        //         created_users.push(user);
        //     }
        //     Ok(created_users)
        // }).await
        
        // Current implementation
        for data in user_data {
            let user = User::create()
                .set_name(data.name)
                .set_email(data.email)
                .set_is_active(true)
                .save(db)
                .await?;
            created_users.push(user);
        }
        
        Ok(created_users)
    }
    
    // Efficient bulk updates
    pub async fn activate_users_bulk(db: &D1Client, user_ids: Vec<i64>) -> Result<Vec<User>> {
        let mut updated_users = Vec::new();
        
        for user_id in user_ids {
            let updated = User::update(user_id)
                .set_is_active(true)
                .save(db)
                .await?;
            updated_users.push(updated);
        }
        
        Ok(updated_users)
    }
}

#[derive(Debug)]
pub struct CreateUserData {
    pub name: String,
    pub email: String,
}
```

## Memory Management

### Lazy Loading

Avoid loading unnecessary data:

```rust
impl User {
    // Lazy-loaded relationships
    pub async fn recent_posts_lazy(&self, db: &D1Client, limit: i64) -> Result<Vec<Post>> {
        // Only load what we need
        Post::query()
            .where_user_id_eq(self.id)
            .order_by_created_at_desc()
            .limit(limit)
            .all(db)
            .await
    }
    
    // Avoid: Eager loading everything
    // pub async fn all_data_eager(&self, db: &D1Client) -> Result<UserWithEverything> {
    //     let posts = self.traverse::<Post>(db, "posts").await?;
    //     let profile = self.traverse::<Profile>(db, "profile").await?;
    //     let comments = self.traverse::<Comment>(db, "comments").await?;
    //     // This loads potentially massive amounts of data
    // }
}
```

### Selective Field Loading

Load only required fields (future feature):

```rust
// This pattern will be supported in future versions
// #[derive(Debug, Serialize, Deserialize)]
// pub struct UserSummary {
//     pub id: i64,
//     pub name: String,
//     pub is_active: bool,
// }
// 
// impl User {
//     pub async fn summaries(db: &D1Client, limit: i64) -> Result<Vec<UserSummary>> {
//         // Future: SELECT id, name, is_active FROM users LIMIT ?
//         // For now: Load full entities and transform
//         let users = User::query().limit(limit).all(db).await?;
//         let summaries = users.into_iter().map(|u| UserSummary {
//             id: u.id,
//             name: u.name,
//             is_active: u.is_active,
//         }).collect();
//         Ok(summaries)
//     }
// }
```

## Cloudflare Workers Optimization

### Edge-Specific Patterns

Optimize for the Workers runtime:

```rust
use worker::*;

// Efficient handler pattern
#[event(fetch)]
async fn main(req: Request, env: Env, ctx: worker::Context) -> Result<Response> {
    let db = D1Client::new(env.d1("DB")?);
    
    match req.method() {
        Method::Get => {
            match req.path().as_str() {
                "/users" => handle_get_users(&db, &req).await,
                "/posts" => handle_get_posts(&db, &req).await,
                path if path.starts_with("/users/") => handle_get_user(&db, path).await,
                _ => Response::error("Not found", 404),
            }
        }
        Method::Post => {
            match req.path().as_str() {
                "/users" => handle_create_user(&db, req).await,
                "/posts" => handle_create_post(&db, req).await,
                _ => Response::error("Not found", 404),
            }
        }
        _ => Response::error("Method not allowed", 405),
    }
}

async fn handle_get_users(db: &D1Client, req: &Request) -> Result<Response> {
    // Parse query parameters for pagination
    let url = req.url()?;
    let page = url.search_params()
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(0);
    let per_page = url.search_params()
        .get("per_page")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(10)
        .min(100); // Limit max page size
    
    // Use efficient pagination
    let result = User::paginated(db, page, per_page, false).await
        .map_err(|e| worker::Error::RustError(format!("Database error: {}", e)))?;
    
    Response::from_json(&result)
}

async fn handle_get_posts(db: &D1Client, req: &Request) -> Result<Response> {
    let url = req.url()?;
    
    // Check for cursor-based pagination
    if let Some(cursor) = url.search_params().get("cursor") {
        let cursor_id = cursor.parse::<i64>()
            .map_err(|_| worker::Error::BadRequest)?;
        
        let limit = url.search_params()
            .get("limit")
            .and_then(|l| l.parse::<i64>().ok())
            .unwrap_or(20)
            .min(100);
        
        let result = Post::paginated_cursor(db, Some(cursor_id), limit).await
            .map_err(|e| worker::Error::RustError(format!("Database error: {}", e)))?;
        
        Response::from_json(&result)
    } else {
        // Regular pagination
        let page = url.search_params()
            .get("page")
            .and_then(|p| p.parse::<i64>().ok())
            .unwrap_or(0);
        
        let result = Post::paginated(db, page, 20, false).await
            .map_err(|e| worker::Error::RustError(format!("Database error: {}", e)))?;
        
        Response::from_json(&result)
    }
}
```

### Response Caching

Cache responses at the edge:

```rust
async fn handle_popular_posts(db: &D1Client, ctx: &worker::Context) -> Result<Response> {
    let cache = ctx.env.cf().unwrap().cache();
    let cache_key = "popular_posts_v1";
    
    // Try cache first
    if let Some(cached_response) = cache.get(cache_key, CacheOptions::default()).await {
        return Ok(cached_response);
    }
    
    // Load from database
    let posts = Post::query()
        .where_is_published_eq(true)
        .order_by_created_at_desc()
        .limit(10)
        .all(db)
        .await
        .map_err(|e| worker::Error::RustError(format!("Database error: {}", e)))?;
    
    let response = Response::from_json(&posts)?;
    
    // Cache for 5 minutes
    cache.put(cache_key, response.clone(), CacheTtl::Minutes(5)).await;
    
    Ok(response)
}
```

## Monitoring and Profiling

### Performance Metrics

Track key performance indicators:

```rust
use std::time::Instant;

pub struct QueryMetrics {
    pub query_time: std::time::Duration,
    pub result_count: usize,
    pub query_type: String,
}

impl User {
    pub async fn find_with_metrics(db: &D1Client, id: i64) -> Result<(Option<User>, QueryMetrics)> {
        let start = Instant::now();
        
        let user = User::find(db, id).await?;
        
        let metrics = QueryMetrics {
            query_time: start.elapsed(),
            result_count: if user.is_some() { 1 } else { 0 },
            query_type: "find_user".to_string(),
        };
        
        // Log metrics (in Workers, use console.log)
        console_log!(
            "Query: {} took {:?} and returned {} results",
            metrics.query_type,
            metrics.query_time,
            metrics.result_count
        );
        
        Ok((user, metrics))
    }
}

// Middleware pattern for automatic metrics
pub async fn with_metrics<F, T>(
    operation_name: &str,
    operation: F,
) -> Result<(T, QueryMetrics)>
where
    F: std::future::Future<Output = Result<T>>,
    T: std::fmt::Debug,
{
    let start = Instant::now();
    let result = operation.await?;
    
    let metrics = QueryMetrics {
        query_time: start.elapsed(),
        result_count: 1, // Would need to be calculated based on T
        query_type: operation_name.to_string(),
    };
    
    console_log!("Operation {} completed in {:?}", operation_name, metrics.query_time);
    
    Ok((result, metrics))
}
```

### Query Analysis

Analyze and optimize slow queries:

```rust
pub struct SlowQueryLogger;

impl SlowQueryLogger {
    pub fn log_if_slow(query_name: &str, duration: std::time::Duration, threshold_ms: u64) {
        if duration.as_millis() > threshold_ms as u128 {
            console_log!(
                "SLOW QUERY: {} took {}ms (threshold: {}ms)",
                query_name,
                duration.as_millis(),
                threshold_ms
            );
        }
    }
}

impl User {
    pub async fn complex_query_with_logging(db: &D1Client) -> Result<Vec<User>> {
        let start = Instant::now();
        
        let users = User::query()
            .where_is_active_eq(true)
            .order_by_name_asc()
            .all(db)
            .await?;
        
        SlowQueryLogger::log_if_slow("complex_user_query", start.elapsed(), 100);
        
        Ok(users)
    }
}
```

## Best Practices Summary

### Do's

1. **Index frequently queried columns**
2. **Use pagination for large result sets**
3. **Load only necessary data**
4. **Batch operations when possible**
5. **Cache expensive computations**
6. **Monitor query performance**

### Don'ts

1. **Don't load all data then filter in Rust**
2. **Don't ignore database indexes**
3. **Don't perform N+1 queries**
4. **Don't load unnecessary relationship data**
5. **Don't forget to limit query results**

### Cloudflare Workers Specific

1. **Use edge caching for static data**
2. **Implement request coalescing for similar queries**
3. **Monitor cold start times**
4. **Optimize bundle size**

## Next Steps

- Learn about [Testing Strategies](./testing.md) for performance testing
- Explore [Boolean Handling](./boolean-handling.md) for type conversion optimization
- Check out [Migrations](../migrations.md) for evolving schemas efficiently