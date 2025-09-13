# Advanced Relations

This section covers advanced relationship patterns, graph traversal, performance optimization, and complex scenarios in d1-rs.

## Graph Traversal

d1-rs relations system is inspired by ent-go's graph-based approach, allowing you to traverse complex relationship networks with type safety and cycle prevention.

### Multi-Level Traversal

Navigate through multiple levels of relationships:

```rust
// Get categories of posts written by a user
let user = User::find(&db, 1).await?.unwrap();

// Traverse: User -> Posts -> Categories
let user_post_categories = user
    .traverse::<Post>(&db, "posts").await?
    .into_iter()
    .map(|post| async move {
        let categories = post.traverse::<Category>(&db, "categories").await?;
        Ok::<Vec<Category>, D1RsError>(categories)
    })
    .collect::<Vec<_>>();

// Flatten results (in real use, you might want to collect unique categories)
let mut all_categories = Vec::new();
for future in user_post_categories {
    let categories = future.await?;
    all_categories.extend(categories);
}

// Remove duplicates
all_categories.sort_by_key(|c| c.id);
all_categories.dedup_by_key(|c| c.id);
```

### Cycle Prevention

The traversal system automatically prevents infinite loops:

```rust
#[derive(Entity, RelationalEntity)]
pub struct User {
    #[primary_key] pub id: i64,
    pub name: String,
}

#[derive(Entity, RelationalEntity)]
pub struct Friendship {
    #[primary_key] pub id: i64,
    pub user_id: i64,
    pub friend_id: i64,
    pub status: String, // "pending", "accepted", "blocked"
}

// Self-referencing relationship that could cause cycles
// The traversal system tracks visited nodes to prevent infinite loops
impl User {
    pub async fn friends(&self, db: &D1Client) -> Result<Vec<User>> {
        let friendships = Friendship::query()
            .where_user_id_eq(self.id)
            .where_status_eq("accepted".to_string())
            .all(db)
            .await?;
        
        let mut friends = Vec::new();
        for friendship in friendships {
            if let Some(friend) = User::find(db, friendship.friend_id).await? {
                friends.push(friend);
            }
        }
        
        Ok(friends)
    }
    
    // Find friends of friends (with cycle prevention)
    pub async fn friends_of_friends(&self, db: &D1Client) -> Result<Vec<User>> {
        let direct_friends = self.friends(db).await?;
        let mut visited = std::collections::HashSet::new();
        visited.insert(self.id);
        
        let mut friends_of_friends = Vec::new();
        
        for friend in direct_friends {
            visited.insert(friend.id);
            let friend_friends = friend.friends(db).await?;
            
            for friend_friend in friend_friends {
                if !visited.contains(&friend_friend.id) {
                    visited.insert(friend_friend.id);
                    friends_of_friends.push(friend_friend);
                }
            }
        }
        
        Ok(friends_of_friends)
    }
}
```

## Complex Query Patterns

### Conditional Traversal

Traverse relationships based on conditions:

```rust
impl User {
    // Get categories of only published posts
    pub async fn published_post_categories(&self, db: &D1Client) -> Result<Vec<Category>> {
        let published_posts = Post::query()
            .where_user_id_eq(self.id)
            .where_is_published_eq(true)
            .all(db)
            .await?;
        
        let mut categories = Vec::new();
        for post in published_posts {
            let post_categories = post.traverse::<Category>(db, "categories").await?;
            categories.extend(post_categories);
        }
        
        // Remove duplicates
        categories.sort_by_key(|c| c.id);
        categories.dedup_by_key(|c| c.id);
        
        Ok(categories)
    }
    
    // Get posts in specific categories only
    pub async fn posts_in_categories(
        &self, 
        db: &D1Client, 
        category_names: Vec<String>
    ) -> Result<Vec<Post>> {
        let user_posts = self.traverse::<Post>(db, "posts").await?;
        let mut filtered_posts = Vec::new();
        
        for post in user_posts {
            let post_categories = post.traverse::<Category>(db, "categories").await?;
            
            let has_target_category = post_categories.iter()
                .any(|cat| category_names.contains(&cat.name));
            
            if has_target_category {
                filtered_posts.push(post);
            }
        }
        
        Ok(filtered_posts)
    }
}
```

### Aggregation Through Relations

Perform calculations across relationships:

```rust
impl User {
    // Get total posts across all categories
    pub async fn category_post_stats(&self, db: &D1Client) -> Result<Vec<(String, i64)>> {
        let categories = self.published_post_categories(db).await?;
        let mut stats = Vec::new();
        
        for category in categories {
            // Count this user's posts in this category
            let user_posts_in_category = Post::query()
                .where_user_id_eq(self.id)
                .where_is_published_eq(true)
                .all(db)
                .await?;
            
            let mut count = 0;
            for post in user_posts_in_category {
                let post_categories = post.traverse::<Category>(db, "categories").await?;
                if post_categories.iter().any(|c| c.id == category.id) {
                    count += 1;
                }
            }
            
            if count > 0 {
                stats.push((category.name.clone(), count));
            }
        }
        
        // Sort by post count descending
        stats.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(stats)
    }
    
    // Get activity metrics
    pub async fn activity_metrics(&self, db: &D1Client) -> Result<UserActivityMetrics> {
        let posts = self.traverse::<Post>(db, "posts").await?;
        let published_posts: Vec<_> = posts.iter()
            .filter(|p| p.is_published)
            .collect();
        
        // Get unique categories
        let mut all_categories = Vec::new();
        for post in &published_posts {
            let categories = post.traverse::<Category>(db, "categories").await?;
            all_categories.extend(categories);
        }
        all_categories.sort_by_key(|c| c.id);
        all_categories.dedup_by_key(|c| c.id);
        
        Ok(UserActivityMetrics {
            total_posts: posts.len() as i64,
            published_posts: published_posts.len() as i64,
            draft_posts: (posts.len() - published_posts.len()) as i64,
            unique_categories: all_categories.len() as i64,
            most_used_category: self.most_used_category(db).await?.map(|c| c.name),
        })
    }
    
    async fn most_used_category(&self, db: &D1Client) -> Result<Option<Category>> {
        let stats = self.category_post_stats(db).await?;
        if let Some((category_name, _count)) = stats.first() {
            Category::query()
                .where_name_eq(category_name.clone())
                .first(db)
                .await
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserActivityMetrics {
    pub total_posts: i64,
    pub published_posts: i64,
    pub draft_posts: i64,
    pub unique_categories: i64,
    pub most_used_category: Option<String>,
}
```

## Advanced Schema Patterns

### Polymorphic Relations

Handle relationships where one entity can relate to multiple types:

```rust
#[derive(Entity, RelationalEntity)]
pub struct Comment {
    #[primary_key]
    pub id: i64,
    pub content: String,
    pub author_id: i64,
    
    // Polymorphic fields
    pub commentable_type: String, // "Post", "User", "Category", etc.
    pub commentable_id: i64,
    
    pub created_at: DateTime<Utc>,
}

impl Comment {
    // Create comment on a post
    pub async fn create_on_post(
        db: &D1Client,
        post_id: i64,
        author_id: i64,
        content: String,
    ) -> Result<Comment> {
        Comment::create()
            .set_content(content)
            .set_author_id(author_id)
            .set_commentable_type("Post".to_string())
            .set_commentable_id(post_id)
            .set_created_at(Utc::now())
            .save(db)
            .await
    }
    
    // Create comment on a user (like a wall post)
    pub async fn create_on_user(
        db: &D1Client,
        user_id: i64,
        author_id: i64,
        content: String,
    ) -> Result<Comment> {
        Comment::create()
            .set_content(content)
            .set_author_id(author_id)
            .set_commentable_type("User".to_string())
            .set_commentable_id(user_id)
            .set_created_at(Utc::now())
            .save(db)
            .await
    }
    
    // Get the related entity (requires manual handling)
    pub async fn get_commentable_entity(&self, db: &D1Client) -> Result<CommentableEntity> {
        match self.commentable_type.as_str() {
            "Post" => {
                let post = Post::find(db, self.commentable_id).await?
                    .ok_or(D1RsError::NotFound)?;
                Ok(CommentableEntity::Post(post))
            },
            "User" => {
                let user = User::find(db, self.commentable_id).await?
                    .ok_or(D1RsError::NotFound)?;
                Ok(CommentableEntity::User(user))
            },
            _ => Err(D1RsError::Database(format!("Unknown commentable type: {}", self.commentable_type))),
        }
    }
}

#[derive(Debug)]
pub enum CommentableEntity {
    Post(Post),
    User(User),
    Category(Category),
}

impl Post {
    pub async fn comments(&self, db: &D1Client) -> Result<Vec<Comment>> {
        Comment::query()
            .where_commentable_type_eq("Post".to_string())
            .where_commentable_id_eq(self.id)
            .order_by_created_at_asc()
            .all(db)
            .await
    }
}

impl User {
    pub async fn wall_comments(&self, db: &D1Client) -> Result<Vec<Comment>> {
        Comment::query()
            .where_commentable_type_eq("User".to_string())
            .where_commentable_id_eq(self.id)
            .order_by_created_at_desc()
            .all(db)
            .await
    }
}
```

### Hierarchical Data

Manage tree-like structures efficiently:

```rust
#[derive(Entity, RelationalEntity)]
pub struct MenuNode {
    #[primary_key]
    pub id: i64,
    pub parent_id: Option<i64>,
    pub title: String,
    pub url: Option<String>,
    pub sort_order: i32,
    pub depth: i32, // Denormalized for performance
    pub path: String, // Materialized path: "1.3.7"
}

impl MenuNode {
    // Create root node
    pub async fn create_root(db: &D1Client, title: String, url: Option<String>) -> Result<MenuNode> {
        let node = MenuNode::create()
            .set_title(title)
            .set_url(url)
            .set_sort_order(0)
            .set_depth(0)
            .set_path("".to_string())
            .save(db)
            .await?;
        
        // Update path to include own ID
        MenuNode::update(node.id)
            .set_path(node.id.to_string())
            .save(db)
            .await
    }
    
    // Add child node
    pub async fn add_child(&self, db: &D1Client, title: String, url: Option<String>) -> Result<MenuNode> {
        let next_order = self.max_child_order(db).await? + 1;
        
        let child = MenuNode::create()
            .set_parent_id(Some(self.id))
            .set_title(title)
            .set_url(url)
            .set_sort_order(next_order)
            .set_depth(self.depth + 1)
            .set_path("".to_string())
            .save(db)
            .await?;
        
        // Update path
        let child_path = if self.path.is_empty() {
            format!("{}.{}", self.id, child.id)
        } else {
            format!("{}.{}", self.path, child.id)
        };
        
        MenuNode::update(child.id)
            .set_path(child_path)
            .save(db)
            .await
    }
    
    // Get all children
    pub async fn children(&self, db: &D1Client) -> Result<Vec<MenuNode>> {
        MenuNode::query()
            .where_parent_id_eq(self.id)
            .order_by_sort_order_asc()
            .all(db)
            .await
    }
    
    // Get all descendants
    pub async fn descendants(&self, db: &D1Client) -> Result<Vec<MenuNode>> {
        let pattern = format!("{}%", self.path);
        MenuNode::query()
            .where_path_like(&pattern)
            .order_by_path_asc()
            .all(db)
            .await
    }
    
    // Get ancestors
    pub async fn ancestors(&self, db: &D1Client) -> Result<Vec<MenuNode>> {
        let path_parts: Vec<&str> = self.path.split('.').collect();
        let mut ancestors = Vec::new();
        
        for i in 1..path_parts.len() {
            if let Ok(ancestor_id) = path_parts[i-1].parse::<i64>() {
                if let Some(ancestor) = MenuNode::find(db, ancestor_id).await? {
                    ancestors.push(ancestor);
                }
            }
        }
        
        Ok(ancestors)
    }
    
    async fn max_child_order(&self, db: &D1Client) -> Result<i32> {
        let children = self.children(db).await?;
        Ok(children.iter().map(|c| c.sort_order).max().unwrap_or(0))
    }
    
    // Move node to different parent
    pub async fn move_to_parent(&self, db: &D1Client, new_parent_id: Option<i64>) -> Result<MenuNode> {
        let (new_depth, new_path_prefix) = if let Some(parent_id) = new_parent_id {
            let parent = MenuNode::find(db, parent_id).await?
                .ok_or(D1RsError::NotFound)?;
            (parent.depth + 1, parent.path.clone())
        } else {
            (0, String::new())
        };
        
        let new_path = if new_path_prefix.is_empty() {
            self.id.to_string()
        } else {
            format!("{}.{}", new_path_prefix, self.id)
        };
        
        // Update this node
        let updated = MenuNode::update(self.id)
            .set_parent_id(new_parent_id)
            .set_depth(new_depth)
            .set_path(new_path.clone())
            .save(db)
            .await?;
        
        // Update all descendants
        self.update_descendant_paths(db, &new_path).await?;
        
        Ok(updated)
    }
    
    async fn update_descendant_paths(&self, db: &D1Client, new_base_path: &str) -> Result<()> {
        let descendants = self.descendants(db).await?;
        
        for descendant in descendants {
            // Calculate new path based on the new base
            let relative_path = descendant.path.strip_prefix(&self.path)
                .unwrap_or(&descendant.path);
            let new_path = format!("{}{}", new_base_path, relative_path);
            
            MenuNode::update(descendant.id)
                .set_path(new_path)
                .set_depth(new_base_path.split('.').count() as i32 - 1 + 
                          (descendant.depth - self.depth))
                .save(db)
                .await?;
        }
        
        Ok(())
    }
}
```

## Performance Optimization

### Eager Loading Strategies

Optimize data loading patterns:

```rust
// Custom eager loading implementation
impl User {
    pub async fn with_posts_and_categories(db: &D1Client, user_ids: Vec<i64>) -> Result<Vec<UserWithData>> {
        // Load users
        let users = User::query()
            .where_id_in(user_ids.clone()) // Note: IN query limitation exists
            .all(db)
            .await?;
        
        // Load all posts for these users in batch
        let mut all_posts = Vec::new();
        for user_id in &user_ids {
            let posts = Post::query()
                .where_user_id_eq(*user_id)
                .all(db)
                .await?;
            all_posts.extend(posts);
        }
        
        // Load all categories for these posts in batch
        let post_ids: Vec<i64> = all_posts.iter().map(|p| p.id).collect();
        let mut post_categories_map: std::collections::HashMap<i64, Vec<Category>> = std::collections::HashMap::new();
        
        for post_id in post_ids {
            let categories = Post::find(db, post_id).await?
                .unwrap()
                .traverse::<Category>(db, "categories")
                .await?;
            post_categories_map.insert(post_id, categories);
        }
        
        // Organize data by user
        let mut result = Vec::new();
        for user in users {
            let user_posts: Vec<_> = all_posts.iter()
                .filter(|p| p.user_id == user.id)
                .cloned()
                .collect();
            
            let posts_with_categories: Vec<PostWithCategories> = user_posts.into_iter()
                .map(|post| PostWithCategories {
                    post: post.clone(),
                    categories: post_categories_map.get(&post.id).cloned().unwrap_or_default(),
                })
                .collect();
            
            result.push(UserWithData {
                user,
                posts: posts_with_categories,
            });
        }
        
        Ok(result)
    }
}

#[derive(Debug)]
pub struct UserWithData {
    pub user: User,
    pub posts: Vec<PostWithCategories>,
}

#[derive(Debug)]
pub struct PostWithCategories {
    pub post: Post,
    pub categories: Vec<Category>,
}
```

### Caching Strategies

Implement caching for frequently accessed relations:

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Simple in-memory cache (in production, use Redis or similar)
pub struct RelationCache {
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl RelationCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let cache = self.cache.read().await;
        if let Some(data) = cache.get(key) {
            serde_json::from_slice(data).ok()
        } else {
            None
        }
    }
    
    pub async fn set<T: serde::Serialize>(&self, key: &str, value: &T) {
        if let Ok(data) = serde_json::to_vec(value) {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), data);
        }
    }
}

impl User {
    pub async fn cached_posts(&self, db: &D1Client, cache: &RelationCache) -> Result<Vec<Post>> {
        let cache_key = format!("user_posts:{}", self.id);
        
        // Try cache first
        if let Some(posts) = cache.get::<Vec<Post>>(&cache_key).await {
            return Ok(posts);
        }
        
        // Load from database
        let posts = self.traverse::<Post>(db, "posts").await?;
        
        // Cache result
        cache.set(&cache_key, &posts).await;
        
        Ok(posts)
    }
}
```

## Testing Advanced Relations

Create comprehensive tests for complex relationship scenarios:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_complex_graph_traversal() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Set up complex relationship network
        // User -> Posts -> Categories
        // User -> Profile
        // Categories -> Posts (reverse)
        
        // Create test data...
        let user = create_test_user(&db).await;
        let posts = create_test_posts(&db, &user).await;
        let categories = create_test_categories(&db).await;
        associate_posts_categories(&db, &posts, &categories).await;
        
        // Test multi-level traversal
        let user_categories = get_user_categories_through_posts(&db, &user).await.unwrap();
        assert!(!user_categories.is_empty());
        
        // Test cycle prevention
        let result = test_cycle_prevention(&db, &user).await;
        assert!(result.is_ok());
    }
    
    async fn create_test_user(db: &D1Client) -> User {
        // Implementation...
    }
    
    // ... other test helpers
}
```

## Next Steps

- Learn about [Performance Optimization](../advanced/performance.md) for scaling complex relationships
- Explore [Testing Strategies](../advanced/testing.md) for comprehensive relation testing
- Check out [Migrations](../migrations.md) for evolving your relationship schema