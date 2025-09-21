# One-to-One Relations

One-to-one relations represent a direct 1:1 relationship between two entities, where each record in one table corresponds to exactly one record in another table.

## Schema Setup

### Database Migration

```rust
use d1_rs::*;

async fn create_user_profile_tables(db: &D1Client) -> Result<()> {
    // Create users table
    let users_migration = SchemaMigration::new("create_users".to_string())
        .create_table("users")
            .integer("id").primary_key().auto_increment().build()
            .text("name").not_null().build()
            .text("email").not_null().unique().build()
        .build();
    
    users_migration.execute(db).await?;
    
    // Create profiles table with unique foreign key
    let profiles_migration = SchemaMigration::new("create_profiles".to_string())
        .create_table("profiles")
            .integer("id").primary_key().auto_increment().build()
            .integer("user_id").not_null().unique().build() // UNIQUE ensures 1:1
            .text("bio").build()
            .text("avatar_url").build()
            .datetime("created_at").default_value(DefaultValue::CurrentTimestamp).build()
        .build();
    
    profiles_migration.execute(db).await?;
    
    Ok(())
}
```

### Entity Definitions

```rust
use d1_rs::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct User {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct Profile {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

### Relation Definition

Define the one-to-one relationship using the `relations!` macro:

```rust
relations! {
    User {
        has_one profile: Profile via user_id,
    }
    
    Profile {
        belongs_to user: User via user_id,
    }
}
```

## Working with One-to-One Relations

### Creating Related Records

```rust
async fn create_user_with_profile(db: &D1Client) -> Result<(User, Profile)> {
    // Create user first
    let user = User::create()
        .set_name("Alice Johnson".to_string())
        .set_email("alice@example.com".to_string())
        .save(db)
        .await?;

    // Create associated profile
    let profile = Profile::create()
        .set_user_id(user.id)
        .set_bio(Some("Software engineer and coffee enthusiast".to_string()))
        .set_avatar_url(Some("https://example.com/avatar.jpg".to_string()))
        .set_created_at(Utc::now())
        .save(db)
        .await?;

    Ok((user, profile))
}
```

### Accessing Related Records

```rust
async fn get_user_profile(db: &D1Client, user_id: i64) -> Result<()> {
    // Get the user
    if let Some(user) = User::find(db, user_id).await? {
        // Get the user's profile using the association method
        let profile = user.profile().first(db).await?;
        
        match profile {
            Some(p) => println!("User {} has profile: {:?}", user.name, p.bio),
            None => println!("User {} has no profile", user.name),
        }
    }
    
    Ok(())
}

async fn get_profile_user(db: &D1Client, profile_id: i64) -> Result<()> {
    // Get the profile
    if let Some(profile) = Profile::find(db, profile_id).await? {
        // Get the profile's user using the association method
        let user = profile.user().first(db).await?;
        
        match user {
            Some(u) => println!("Profile belongs to user: {}", u.name),
            None => println!("Profile has no associated user"),
        }
    }
    
    Ok(())
}
```

## Helper Methods

Add convenience methods to your entities for easier one-to-one access:

### User Methods

```rust
impl User {
    /// Get user's profile (convenience method)
    pub async fn get_profile(&self, db: &D1Client) -> Result<Option<Profile>> {
        self.profile().first(db).await
    }
    
    /// Create a profile for this user
    pub async fn create_profile(
        &self,
        db: &D1Client,
        bio: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<Profile> {
        // Check if profile already exists
        if self.get_profile(db).await?.is_some() {
            return Err(d1_rs::D1RsError::Database("User already has a profile".to_string()));
        }
        
        Profile::create()
            .set_user_id(self.id)
            .set_bio(bio)
            .set_avatar_url(avatar_url)
            .set_created_at(Utc::now())
            .save(db)
            .await
    }
    
    /// Get or create profile for this user
    pub async fn get_or_create_profile(&self, db: &D1Client) -> Result<Profile> {
        if let Some(profile) = self.get_profile(db).await? {
            Ok(profile)
        } else {
            self.create_profile(db, None, None).await
        }
    }
}
```

### Profile Methods

```rust
impl Profile {
    /// Get the user who owns this profile (convenience method)
    pub async fn get_user(&self, db: &D1Client) -> Result<User> {
        self.user().first(db).await?
            .ok_or(d1_rs::D1RsError::Database("Profile has no associated user".to_string()))
    }
    
    /// Update profile bio
    pub async fn update_bio(&self, db: &D1Client, new_bio: String) -> Result<Profile> {
        Profile::update(self.id)
            .set_bio(Some(new_bio))
            .save(db)
            .await
    }
    
    /// Update profile avatar
    pub async fn update_avatar(&self, db: &D1Client, avatar_url: String) -> Result<Profile> {
        Profile::update(self.id)
            .set_avatar_url(Some(avatar_url))
            .save(db)
            .await
    }
    
    /// Clear profile avatar
    pub async fn clear_avatar(&self, db: &D1Client) -> Result<Profile> {
        Profile::update(self.id)
            .set_avatar_url(None)
            .save(db)
            .await
    }
}
```

## Common One-to-One Patterns

### User Settings

A common pattern is user settings as a one-to-one relationship:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct UserSettings {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub theme: String,
    pub notifications_enabled: bool,
    pub language: String,
    pub timezone: String,
}

relations! {
    User {
        has_one settings: UserSettings via user_id,
    }
    
    UserSettings {
        belongs_to user: User via user_id,
    }
}

impl User {
    pub async fn get_or_create_settings(&self, db: &D1Client) -> Result<UserSettings> {
        if let Some(settings) = self.settings().first(db).await? {
            Ok(settings)
        } else {
            // Create default settings
            UserSettings::create()
                .set_user_id(self.id)
                .set_theme("light".to_string())
                .set_notifications_enabled(true)
                .set_language("en".to_string())
                .set_timezone("UTC".to_string())
                .save(db)
                .await
        }
    }
}
```

### Account Verification

Another common pattern is verification data:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Entity, PartialEq)]
pub struct EmailVerification {
    #[primary_key]
    pub id: i64,
    pub user_id: i64,
    pub verification_token: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

relations! {
    User {
        has_one email_verification: EmailVerification via user_id,
    }
    
    EmailVerification {
        belongs_to user: User via user_id,
    }
}

impl User {
    pub async fn create_verification(&self, db: &D1Client) -> Result<EmailVerification> {
        use rand::Rng;
        
        let token = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        
        let expires_at = Utc::now() + chrono::Duration::hours(24);
        
        EmailVerification::create()
            .set_user_id(self.id)
            .set_verification_token(token)
            .set_expires_at(expires_at)
            .save(db)
            .await
    }
    
    pub async fn verify_email(&self, db: &D1Client, token: &str) -> Result<bool> {
        if let Some(verification) = self.email_verification().first(db).await? {
            if verification.verification_token == token && 
               verification.expires_at > Utc::now() &&
               verification.verified_at.is_none() {
                
                EmailVerification::update(verification.id)
                    .set_verified_at(Some(Utc::now()))
                    .save(db)
                    .await?;
                
                return Ok(true);
            }
        }
        Ok(false)
    }
}
```

## Performance Considerations

### Indexing

Always index foreign key columns for optimal performance:

```rust
async fn add_indexes(db: &D1Client) -> Result<()> {
    let migration = SchemaMigration::new("add_one_to_one_indexes".to_string())
        .alter_table("profiles")
            .add_unique_index("idx_profiles_user_id", vec!["user_id"])
        .build()
        .alter_table("user_settings")
            .add_unique_index("idx_user_settings_user_id", vec!["user_id"])
        .build();
    
    migration.execute(db).await
}
```

### Selective Loading

Only load related data when needed:

```rust
async fn efficient_user_loading(db: &D1Client) -> Result<()> {
    // Good - only load profiles when needed
    let users = User::query()
        .where_email_like("%@company.com")
        .all(db)
        .await?;
    
    for user in users {
        // Only load profile if we need to display it
        if needs_profile_display(&user) {
            let profile = user.get_profile(db).await?;
            display_user_with_profile(&user, profile);
        } else {
            display_user_basic(&user);
        }
    }
    
    Ok(())
}

fn needs_profile_display(user: &User) -> bool {
    // Your business logic here
    true
}

fn display_user_with_profile(user: &User, profile: Option<Profile>) {
    println!("User: {} - Bio: {:?}", user.name, profile.and_then(|p| p.bio));
}

fn display_user_basic(user: &User) {
    println!("User: {}", user.name);
}
```

## Testing One-to-One Relations

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_one_to_one_relations() {
        let db = D1Client::new_in_memory().await.unwrap();
        
        // Set up schema
        create_user_profile_tables(&db).await.unwrap();
        
        // Create user
        let user = User::create()
            .set_name("Test User".to_string())
            .set_email("test@example.com".to_string())
            .save(&db)
            .await
            .unwrap();
        
        // Initially no profile
        assert!(user.get_profile(&db).await.unwrap().is_none());
        
        // Create profile
        let profile = user.create_profile(
            &db,
            Some("Test bio".to_string()),
            Some("https://example.com/avatar.jpg".to_string()),
        ).await.unwrap();
        
        // Now user has profile
        let loaded_profile = user.get_profile(&db).await.unwrap().unwrap();
        assert_eq!(loaded_profile.id, profile.id);
        assert_eq!(loaded_profile.bio, Some("Test bio".to_string()));
        
        // Profile points back to user
        let profile_user = profile.get_user(&db).await.unwrap();
        assert_eq!(profile_user.id, user.id);
        
        // Cannot create second profile
        assert!(user.create_profile(&db, None, None).await.is_err());
    }
    
    #[tokio::test]
    async fn test_profile_updates() {
        let db = D1Client::new_in_memory().await.unwrap();
        create_user_profile_tables(&db).await.unwrap();
        
        let user = User::create()
            .set_name("Test User".to_string())
            .set_email("test@example.com".to_string())
            .save(&db)
            .await
            .unwrap();
        
        let profile = user.create_profile(&db, None, None).await.unwrap();
        
        // Update bio
        let updated = profile.update_bio(&db, "New bio".to_string()).await.unwrap();
        assert_eq!(updated.bio, Some("New bio".to_string()));
        
        // Update avatar
        let updated = profile.update_avatar(&db, "https://example.com/new.jpg".to_string()).await.unwrap();
        assert_eq!(updated.avatar_url, Some("https://example.com/new.jpg".to_string()));
        
        // Clear avatar
        let updated = profile.clear_avatar(&db).await.unwrap();
        assert_eq!(updated.avatar_url, None);
    }
}
```

## Next Steps

- Learn about [One-to-Many Relations](./one-to-many.md) for parent-child relationships
- Explore [Many-to-Many Relations](./many-to-many.md) for complex associations  
- Check out [Advanced Relations](./advanced.md) for complex scenarios
- Review [Performance Tips](../advanced/performance.md) for optimizing relation queries