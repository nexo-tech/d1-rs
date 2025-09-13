# Introduction to Relations

d1-rs provides a powerful, **completely type-safe** relations system inspired by [ent-go](https://entgo.io/) that eliminates string literals and provides compile-time safety. Define relationships once using our `relations!` macro and get generated association methods with zero runtime overhead.

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

### 🎯 **Zero String Literals**
All relationships are type-safe at compile time with generated methods:

```rust
// Type-safe association methods - NO STRING LITERALS!
let user_posts = user.posts().all(&db).await?;
let post_count = user.posts().count(&db).await?;
let first_post = user.posts().first(&db).await?;

// This would cause a compile error - method doesn't exist
let invalid = user.invalid_relation(); // ❌ Compile-time error
```

### 🚀 **Auto-Generated Methods**
Every relation generates appropriate methods automatically:

```rust
// One-to-many: User has many Posts
let user_posts = user.posts().all(&db).await?;
let post_count = user.posts().count(&db).await?;
let first_post = user.posts().first(&db).await?;

// Many-to-one: Post belongs to User  
let post_author = post.user().first(&db).await?;

// Many-to-many: Post has many Categories
let post_categories = post.categories().all(&db).await?;
```

### ⚡ **Migration Auto-Generation**
Junction tables and relations are created automatically from your entity definitions:

```rust
let migration = SchemaMigration::new("blog".to_string())
    // Create your entity tables normally...
    .auto_generate_for::<User>()     // Generates all User relations
    .auto_generate_for::<Post>()     // Generates all Post relations  
    .auto_generate_for::<Category>(); // Generates all Category relations
```

## Setting Up Relations

Relations are defined using the simple `relations!` macro - no complex migration code needed:

```rust
use d1_rs::*;

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key] pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Post {
    #[primary_key] pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
}

// Define relations with one simple macro - NO STRING LITERALS!
relations! {
    User {
        has_many posts: Post via user_id,
    }
    
    Post {
        belongs_to user: User via user_id,
    }
}

// That's it! Now you have type-safe association methods
let user_posts = user.posts().all(&db).await?;
let post_author = post.user().first(&db).await?;
```

## Working with Relations

### Association Methods

The `relations!` macro generates type-safe methods for each relation:

```rust
// Get all related records
let all_posts = user.posts().all(&db).await?;

// Count related records
let post_count = user.posts().count(&db).await?;

// Get first related record
let latest_post = user.posts().first(&db).await?;

// Many-to-one relations
let post_author = post.user().first(&db).await?;

// Many-to-many relations  
let post_categories = post.categories().all(&db).await?;
```

### Loaded Data Access

If relations are pre-loaded, access them without database queries:

```rust
// Check if data is already loaded
if let Some(posts) = user.posts().loaded() {
    println!("Already have {} posts loaded", posts.len());
} else {
    // This will query the database
    let posts = user.posts().all(&db).await?;
}
```

### Many-to-Many Relations

For many-to-many relations, simply define them in the macro and let d1-rs handle everything:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
#[table(name = "post_categories")]  // Specify junction table name
pub struct PostCategory {
    #[primary_key] pub id: i64,
    pub post_id: i64,
    pub category_id: i64,
}

relations! {
    Post {
        has_many_through categories: Category via post_id,
    }
    
    Category {
        has_many_through posts: Post via category_id,
    }
}

// Junction table is created automatically with migration auto-generation
let migration = SchemaMigration::new("blog".to_string())
    .auto_generate_for::<Post>()
    .auto_generate_for::<Category>();
```

## Managing Relations

### Attach/Detach (Many-to-Many)

```rust
// Attach a category to a post
post.categories().attach(&db, category.id).await?;

// Detach a category from a post
post.categories().detach(&db, category.id).await?;
```

## Performance Considerations

### Lazy Loading

Each association method call executes a separate query:

```rust
// This executes one query per user - N+1 problem
for user in users {
    let posts = user.posts().all(&db).await?; 
}
```

Future versions will include eager loading capabilities to solve N+1 queries efficiently.

## Next Steps

Ready to implement relations in your application? Check out specific guides:

- [One-to-One Relations](./one-to-one.md) - User profiles, account settings
- [One-to-Many Relations](./one-to-many.md) - Users and posts, categories and items  
- [Many-to-Many Relations](./many-to-many.md) - Posts and tags, users and roles
- [Advanced Relations](./advanced.md) - Complex patterns and performance optimization

The new type-safe API eliminates the need for string literals and provides compile-time safety that wasn't possible with the old traversal system.