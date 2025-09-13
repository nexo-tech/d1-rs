# One-to-One Relations

One-to-one relations represent a direct 1:1 relationship between two entities, where each record in one table corresponds to exactly one record in another table.

## Basic One-to-One

### Schema Definition

```rust
// User has one Profile
let migration = SchemaMigration::new("create_user_profile_relation".to_string())
    .create_table("users")
        .integer("id").primary_key().auto_increment().build()
        .text("name").not_null().build()
        .text("email").not_null().unique().build()
    .build()
    
    .create_table("profiles")
        .integer("id").primary_key().auto_increment().build()
        .integer("user_id").not_null().unique().build() // UNIQUE constraint ensures 1:1
        .text("bio").build()
        .text("avatar_url").build()
        .datetime("updated_at").default_value(DefaultValue::CurrentTimestamp).build()
    .build()
    
    // Define the relation
    .create_relation("user_profile", "users", "profiles")
        .one_to_one("user_id", "id")
    .build();

migration.execute(&db).await?;
```

### Entity Definitions

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, RelationalEntity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, RelationalEntity, PartialEq)]
pub struct Profile {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}
```

## Traversing One-to-One Relations

### From Parent to Child

```rust
// Get user's profile
let user = User::find(&db, 1).await?.unwrap();
let profile = user.traverse::<Profile>(&db, "profile").await?;

match profile.first() {
    Some(profile) => println!("User bio: {:?}", profile.bio),
    None => println!("User has no profile"),
}
```

### From Child to Parent

```rust
// Get profile's user
let profile = Profile::find(&db, 1).await?.unwrap();
let users = profile.traverse::<User>(&db, "user").await?;

if let Some(user) = users.first() {
    println!("Profile belongs to: {}", user.name);
}
```

## Eager Loading

Load users with their profiles in a single operation:

```rust
// Load users with profiles
let users_with_profiles = User::query()
    .with(vec!["profile"])
    .all(&db)
    .await?;

for user in users_with_profiles {
    println!("User: {}", user.name);
    
    // Access loaded profile data
    let profiles = user.traverse::<Profile>(&db, "profile").await?;
    if let Some(profile) = profiles.first() {
        println!("  Bio: {:?}", profile.bio);
    }
}
```

## Creating Related Records

### Creating User with Profile

```rust
// Create user first
let user = User::create()
    .set_name("Alice Johnson".to_string())
    .set_email("alice@example.com".to_string())
    .save(&db)
    .await?;

// Create associated profile
let profile = Profile::create()
    .set_user_id(user.id)
    .set_bio(Some("Software engineer and coffee enthusiast".to_string()))
    .set_avatar_url(Some("https://example.com/avatar.jpg".to_string()))
    .set_updated_at(Utc::now())
    .save(&db)
    .await?;

println!("Created user {} with profile {}", user.id, profile.id);
```

## Advanced Patterns

### Profile with Required Relationship

Ensure every profile has a valid user:

```rust
impl Profile {
    pub async fn create_for_user(
        db: &D1Client,
        user_id: i64,
        bio: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<Profile> {
        // Verify user exists
        let user = User::find(db, user_id).await?
            .ok_or(D1RsError::NotFound)?;
        
        Profile::create()
            .set_user_id(user.id)
            .set_bio(bio)
            .set_avatar_url(avatar_url)
            .set_updated_at(Utc::now())
            .save(db)
            .await
    }
    
    pub async fn get_user(&self, db: &D1Client) -> Result<User> {
        User::find(db, self.user_id).await?
            .ok_or(D1RsError::NotFound)
    }
}
```

### User with Profile Helper

```rust
impl User {
    pub async fn get_profile(&self, db: &D1Client) -> Result<Option<Profile>> {
        Profile::query()
            .where_user_id_eq(self.id)
            .first(db)
            .await
    }
    
    pub async fn create_profile(
        &self,
        db: &D1Client,
        bio: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<Profile> {
        // Check if profile already exists
        if let Some(_) = self.get_profile(db).await? {
            return Err(D1RsError::Database("User already has a profile".to_string()));
        }
        
        Profile::create()
            .set_user_id(self.id)
            .set_bio(bio)
            .set_avatar_url(avatar_url)
            .set_updated_at(Utc::now())
            .save(db)
            .await
    }
}
```

## Optional vs Required Relations

### Optional Profile (Current Implementation)

```rust
// User may or may not have a profile
let user = User::find(&db, 1).await?.unwrap();
match user.get_profile(&db).await? {
    Some(profile) => println!("Has profile: {:?}", profile.bio),
    None => println!("No profile found"),
}
```

### Required Profile Pattern

For cases where the relationship should always exist:

```rust
#[derive(Entity, RelationalEntity)]
pub struct Account {
    #[primary_key]
    pub id: i64,
    pub username: String,
    // profile_id as required foreign key
    pub profile_id: i64,
}

#[derive(Entity, RelationalEntity)]
pub struct AccountProfile {
    #[primary_key]
    pub id: i64,
    pub display_name: String,
    pub settings: String, // JSON settings
}

impl Account {
    pub async fn create_with_profile(
        db: &D1Client,
        username: String,
        display_name: String,
    ) -> Result<(Account, AccountProfile)> {
        // Create profile first
        let profile = AccountProfile::create()
            .set_display_name(display_name)
            .set_settings("{}".to_string())
            .save(db)
            .await?;
        
        // Create account with profile reference
        let account = Account::create()
            .set_username(username)
            .set_profile_id(profile.id)
            .save(db)
            .await?;
        
        Ok((account, profile))
    }
}
```

## Performance Considerations

### Indexing Foreign Keys

Always index foreign key columns for better performance:

```rust
let migration = SchemaMigration::new("add_profile_indexes".to_string())
    .alter_table("profiles")
        .add_unique_index("idx_profiles_user_id", vec!["user_id"])
    .build();
```

### Selective Loading

Only load profiles when needed:

```rust
// Good - only load when you need the profile data
let users = User::query()
    .where_is_active_eq(true)
    .all(&db)
    .await?;

for user in users {
    if should_show_profile(&user) {
        let profile = user.get_profile(&db).await?;
        // Use profile data
    }
}

// Less efficient - always loads profile data
let users_with_profiles = User::query()
    .with(vec!["profile"])
    .all(&db)
    .await?;
```

## Common Patterns

### User Settings

```rust
#[derive(Entity, RelationalEntity)]
pub struct UserSettings {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub theme: String,
    pub notifications_enabled: bool,
    pub language: String,
}

impl User {
    pub async fn get_or_create_settings(&self, db: &D1Client) -> Result<UserSettings> {
        if let Some(settings) = UserSettings::query()
            .where_user_id_eq(self.id)
            .first(db)
            .await? 
        {
            Ok(settings)
        } else {
            // Create default settings
            UserSettings::create()
                .set_user_id(self.id)
                .set_theme("light".to_string())
                .set_notifications_enabled(true)
                .set_language("en".to_string())
                .save(db)
                .await
        }
    }
}
```

### Profile Updates

```rust
impl Profile {
    pub async fn update_bio(&self, db: &D1Client, new_bio: String) -> Result<Profile> {
        Profile::update(self.id)
            .set_bio(Some(new_bio))
            .set_updated_at(Utc::now())
            .save(db)
            .await
    }
    
    pub async fn update_avatar(&self, db: &D1Client, avatar_url: String) -> Result<Profile> {
        Profile::update(self.id)
            .set_avatar_url(Some(avatar_url))
            .set_updated_at(Utc::now())
            .save(db)
            .await
    }
}
```

## Next Steps

- Learn about [One-to-Many Relations](./one-to-many.md) for parent-child relationships
- Explore [Many-to-Many Relations](./many-to-many.md) for complex associations
- Check out [Advanced Relations](./advanced.md) for complex traversal patterns