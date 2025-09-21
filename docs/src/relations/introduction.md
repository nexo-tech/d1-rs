# Relations System

d1-rs provides a type-safe relations system that allows you to define and work with entity relationships using compile-time validation. The system supports one-to-one, one-to-many, many-to-many, and recursive relationships.

## Relation Types

### One-to-One Relations
A single entity is associated with exactly one other entity.
```rust
User 1 ←→ 1 Profile  // Each user has one profile
```

### One-to-Many Relations
A single entity can be associated with multiple other entities.
```rust
User 1 ←→ ∞ Post     // A user can have many posts
```

### Many-to-Many Relations
Multiple entities can be associated with multiple other entities through a junction table.
```rust
Post ∞ ←→ ∞ Category  // Posts can have multiple categories
```

### Recursive Self-Referential Relations
Entities can reference other entities of the same type.
```rust
Category ∞ ←→ ∞ Category  // Categories can have parent/child relationships
User 1 ←→ ∞ User          // Users can have manager/employee relationships
Comment 1 ←→ ∞ Comment    // Comments can have parent/reply relationships
```

## The relations! Macro

The `relations!` macro is the core of d1-rs's relationship system. It allows you to define type-safe relationships between entities without using string literals.

### Basic Usage

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct Post {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub content: String,
}

// Define relationships using the relations! macro
relations! {
    User {
        has_many posts: Post via user_id,
    }
    
    Post {
        belongs_to user: User via user_id,
    }
}
```

### Generated Association Methods

Once you define relations, d1-rs automatically generates type-safe association methods:

```rust
// Get all posts for a user
let user_posts = user.posts().all(&db).await?;

// Count user's posts without loading them
let post_count = user.posts().count(&db).await?;

// Get the first post
let first_post = user.posts().first(&db).await?;

// Get the user who wrote a post
let post_author = post.user().first(&db).await?;
```

## Relation Keywords

### has_many
Defines a one-to-many relationship where the current entity can have multiple related entities.

```rust
relations! {
    User {
        has_many posts: Post via user_id,
        has_many comments: Comment via user_id,
    }
}
```

### belongs_to
Defines the inverse of a one-to-many relationship, where the current entity belongs to another entity.

```rust
relations! {
    Post {
        belongs_to user: User via user_id,
        belongs_to category: Category via category_id,
    }
}
```

### has_one
Defines a one-to-one relationship where the current entity has exactly one related entity.

```rust
relations! {
    User {
        has_one profile: Profile via user_id,
    }
}
```

### has_many_through (Many-to-Many)
Defines a many-to-many relationship through a junction table.

```rust
relations! {
    Post {
        has_many categories: Category through post_categories,
    }
    
    Category {
        has_many posts: Post through post_categories,
    }
}
```

## Recursive Relationships

d1-rs supports self-referential relationships where entities reference other entities of the same type.

### Category Hierarchy Example

```rust
#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct Category {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,  // Self-referential foreign key
}

relations! {
    Category {
        belongs_to parent: Category via parent_id,
        has_many children: Category via parent_id,
    }
}

// Usage
let parent_category = child_category.parent().first(&db).await?;
let subcategories = root_category.children().all(&db).await?;
```

### User Management Hierarchy

```rust
#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub manager_id: Option<i64>,  // Self-referential foreign key
}

relations! {
    User {
        belongs_to manager: User via manager_id,
        has_many employees: User via manager_id,
    }
}

// Usage
let manager = employee.manager().first(&db).await?;
let team_members = manager.employees().all(&db).await?;
```

## Junction Tables and Many-to-Many Relations

For many-to-many relationships, you can define junction tables as first-class entities with additional fields.

### Basic Many-to-Many

```rust
#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct PostCategory {
    #[primary_key]
    pub id: i64,
    pub post_id: i64,
    pub category_id: i64,
}

relations! {
    Post {
        has_many post_categories: PostCategory via post_id,
    }
    
    Category {
        has_many post_categories: PostCategory via category_id,
    }
    
    PostCategory {
        belongs_to post: Post via post_id,
        belongs_to category: Category via category_id,
    }
}
```

### Rich Junction Tables

Junction tables can have additional fields beyond the foreign keys:

```rust
#[derive(Debug, Serialize, Deserialize, Entity)]
pub struct UserRole {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub role_id: i64,
    pub granted_at: DateTime<Utc>,
    pub granted_by: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

relations! {
    User {
        has_many user_roles: UserRole via user_id,
    }
    
    Role {
        has_many user_roles: UserRole via role_id,
    }
    
    UserRole {
        belongs_to user: User via user_id,
        belongs_to role: Role via role_id,
    }
}

// Query junction table directly
let active_roles = UserRole::query()
    .where_is_active_eq(true)
    .where_expires_at_gt(current_time)
    .all(&db).await?;

// Navigate through junction
let user_from_role = user_role.user().first(&db).await?;
let role_from_user = user_role.role().first(&db).await?;
```

## Association Methods

The relations macro generates several methods for each relationship:

### For has_many relationships:
- `.posts()` - Returns a query builder for related posts
- `.posts().all(&db)` - Get all related posts
- `.posts().count(&db)` - Count related posts
- `.posts().first(&db)` - Get first related post

### For belongs_to relationships:
- `.user()` - Returns a query builder for the related user
- `.user().first(&db)` - Get the related user

## Type Safety Benefits

### Compile-Time Validation
All relation names and entity types are validated at compile time:

```rust
// This works - posts is a valid relation on User
let posts = user.posts().all(&db).await?;

// This would cause a compile error - invalid_relation doesn't exist
// let invalid = user.invalid_relation(); // ❌ Compile error
```

### IDE Support
Your IDE will provide auto-completion for all available relations and methods, making development faster and preventing typos.

### No String Literals
Unlike other ORMs, d1-rs relations never use string literals, eliminating a common source of runtime errors.

## Performance Considerations

### Efficient Queries
Association methods generate efficient SQL queries:

```rust
// This generates: SELECT COUNT(*) FROM posts WHERE user_id = ?
let count = user.posts().count(&db).await?;

// This generates: SELECT * FROM posts WHERE user_id = ? LIMIT 1
let first = user.posts().first(&db).await?;
```

### Avoiding N+1 Queries
Be mindful of N+1 query patterns when working with relations:

```rust
// ❌ This creates N+1 queries (inefficient)
let users = User::query().all(&db).await?;
for user in users {
    let posts = user.posts().all(&db).await?; // N queries
}

// ✅ Better approach - minimize queries
let users = User::query().all(&db).await?;
let all_user_ids: Vec<i64> = users.iter().map(|u| u.id).collect();
let all_posts = Post::query()
    .where_user_id_in(all_user_ids)
    .all(&db).await?;
```

## Next Steps

- Learn about specific relation types in detail:
  - [One-to-One Relations](./one-to-one.md)
  - [One-to-Many Relations](./one-to-many.md)  
  - [Many-to-Many Relations](./many-to-many.md)
- Explore [Advanced Relations](./advanced.md) for complex scenarios
- Check out [Performance Tips](../advanced/performance.md) for optimizing relation queries