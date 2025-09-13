use d1orm::*;
use std::sync::atomic::{AtomicBool, Ordering};
use worker::d1::D1Database;
use worker::{console_error, console_log, Result as WorkerResult};

static MIGRATIONS_INITIALIZED: AtomicBool = AtomicBool::new(false);

// Global migration runner for this application
pub fn get_migration_runner() -> MigrationRunner {
    let mut runner = MigrationRunner::new();
    
    // New structure: users first, then tokens with foreign key
    let create_users_migration = CreateTableMigration::new("create_users", 1, "users".to_string())
        .column("id", "INTEGER").primary_key()
        .column("email", "TEXT").not_null().unique()
        .column("name", "TEXT").not_null()
        .column("is_active", "INTEGER").not_null().default("1")
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP")
        .column("updated_at", "DATETIME").default("CURRENT_TIMESTAMP");

    let create_tokens_migration = CreateTableMigration::new("create_tokens", 2, "tokens".to_string())
        .column("id", "INTEGER").primary_key()
        .column("user_id", "INTEGER").not_null()
        .column("calendar_email", "TEXT").not_null()
        .column("access_token", "TEXT").not_null()
        .column("refresh_token", "TEXT").not_null()
        .column("token_type", "TEXT").not_null()
        .column("is_primary", "INTEGER").not_null().default("0")
        .column("expiry", "DATETIME").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP")
        .column("updated_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    let create_calendars_migration = CreateTableMigration::new("create_calendars", 3, "calendars".to_string())
        .column("id", "INTEGER").primary_key()
        .column("user_id", "INTEGER").not_null()
        .column("title", "TEXT").not_null()
        .column("description", "TEXT")
        .column("timezone", "TEXT").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    let create_working_hours_migration = CreateTableMigration::new("create_working_hours", 4, "working_hours".to_string())
        .column("id", "INTEGER").primary_key()
        .column("start_time", "TEXT").not_null()
        .column("end_time", "TEXT").not_null()
        .column("timezone", "TEXT").not_null()
        .column("working_days", "TEXT").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP")
        .column("updated_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    let create_bookings_migration = CreateTableMigration::new("create_bookings", 5, "bookings".to_string())
        .column("id", "INTEGER").primary_key()
        .column("calendar_id", "INTEGER").not_null()
        .column("guest_email", "TEXT").not_null()
        .column("guest_name", "TEXT").not_null()
        .column("title", "TEXT").not_null()
        .column("notes", "TEXT")
        .column("start_time", "DATETIME").not_null()
        .column("end_time", "DATETIME").not_null()
        .column("status", "TEXT").not_null()
        .column("created_at", "DATETIME").default("CURRENT_TIMESTAMP");
    
    runner.add_migration(Box::new(create_users_migration));
    runner.add_migration(Box::new(create_tokens_migration));
    runner.add_migration(Box::new(create_calendars_migration));
    runner.add_migration(Box::new(create_working_hours_migration));
    runner.add_migration(Box::new(create_bookings_migration));
    
    runner
}

pub async fn initialize_database(db: &D1Client) -> WorkerResult<()> {
    // Use atomic flag to prevent multiple simultaneous initializations
    if MIGRATIONS_INITIALIZED.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        console_log!("Starting database initialization");
        
        let mut runner = get_migration_runner();
        
        match runner.run_pending_migrations(db).await {
            Ok(_) => {
                console_log!("Database initialization completed successfully");
                Ok(())
            }
            Err(e) => {
                console_error!("Database initialization failed: {}", e);
                // Reset flag on error to allow retry
                MIGRATIONS_INITIALIZED.store(false, Ordering::SeqCst);
                Err(worker::Error::RustError(format!("Migration failed: {}", e)))
            }
        }
    } else {
        console_log!("Database initialization already in progress or completed");
        Ok(())
    }
}

pub fn get_database(ctx: &worker::RouteContext<()>) -> WorkerResult<D1Client> {
    let d1_db = ctx.env.get_binding::<D1Database>("DB")?;
    Ok(D1Client::new(d1_db))
}