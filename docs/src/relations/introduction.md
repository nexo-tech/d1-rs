# Introduction to Relations

d1-rs provides a powerful, type-safe relations system inspired by [ent-go](https://entgo.io/) that lets you model complex data relationships with ease. Our graph-based approach supports eager loading, cycle prevention, and intuitive traversal patterns.

## Relation Types

d1-rs supports all standard database relationship patterns:

### One-to-One Relations
A user has one profile, a profile belongs to one user.

```rust
User 1 ←→ 1 Profile
```

### One-to-Many Relations  
A user can have many posts, each post belongs to one user.

```rust
User 1 ←→ ∞ Post
```

### Many-to-Many Relations
Posts can have many categories, categories can belong to many posts.

```rust
Post ∞ ←→ ∞ Category
```

## Key Features

### 🎯 **Type-Safe Traversal**
Relations are validated at compile-time, preventing runtime errors:

```rust
// This compiles and is type-safe
let user_posts = user.traverse(&db, "posts").await?;

// This would cause a compile error
let invalid = user.traverse(&db, "invalid_relation").await?; // ❌
```

### 🚀 **Eager Loading**
Load related data efficiently with a single query:

```rust
let users = User::query()
    .with(vec!["profile", "posts"])
    .all(&db)
    .await?;
```

### 🔄 **Graph Traversal**
Navigate complex data relationships easily:

```rust
let user = User::find(&db, user_id).await?.unwrap();
let related_categories = user
    .traverse(&db, "posts")  // Get user's posts
    .traverse(&db, "categories")  // Get categories from posts
    .await?;
```

### ⚡ **Cycle Prevention**
Built-in cycle detection prevents infinite loops in complex graphs.

## Setting Up Relations

Relations are defined in your schema migrations using the fluent API:

```rust
let migration = SchemaMigration::new("create_relations".to_string())
    // Create tables first...
    .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().build()
    .build()
    
    .create_table("profiles")
        .integer("id").primary_key().auto_increment().build()
        .integer("user_id").not_null().build()
        .text("bio").build()
    .build()
    
    // Then define relations
    .create_relation("user_profile", "users", "profiles")
        .one_to_one("user_id", "id")
    .build();

migration.execute(&db).await?;
```

## Working with Relations

### Basic Traversal

```rust
#[derive(Entity, RelationalEntity)]
pub struct User {
    #[primary_key] 
    pub id: i64,
    pub name: String,
}

#[derive(Entity, RelationalEntity)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
}

// Find a user and get their posts
let user = User::find(&db, 1).await?.unwrap();
let posts = user.traverse(&db, "posts").await?;
```

### Eager Loading

```rust
// Load users with their profiles in a single query
let users_with_profiles = User::query()
    .with(vec!["profile"])
    .all(&db)
    .await?;
```

### Junction Tables

For many-to-many relations, d1-rs automatically handles junction tables:

```rust
#[derive(Entity)]
#[table(name = "post_categories")]
pub struct PostCategory {
    #[primary_key]
    pub id: i64,
    pub post_id: i64,
    pub category_id: i64,
}

// The relation handles the junction table automatically
let migration = SchemaMigration::new("many_to_many".to_string())
    .create_relation("post_categories_junction", "posts", "categories")
        .many_to_many("post_categories", "post_id", "id", "category_id", "id")
    .build();
```

## Performance Considerations

### Lazy vs Eager Loading

```rust
// Lazy loading - executes N+1 queries
for user in users {
    let posts = user.traverse(&db, "posts").await?; // Separate query for each user
}

// Eager loading - single query
let users_with_posts = User::query()
    .with(vec!["posts"])
    .all(&db)
    .await?; // Only one query
```

### Depth Control

Prevent expensive deep traversals:

```rust
let context = TraversalContext::new()
    .with_max_depth(3)
    .with_includes(vec!["posts", "categories"]);

let data = user.load_graph(&db, vec!["posts.categories"]).await?;
```

## Next Steps

Ready to implement relations in your application? Check out specific guides:

- [One-to-One Relations](./one-to-one.md) - User profiles, settings
- [One-to-Many Relations](./one-to-many.md) - Users and posts, categories and items  
- [Many-to-Many Relations](./many-to-many.md) - Posts and tags, users and roles
- [Graph Traversal](./graph-traversal.md) - Advanced navigation patterns