use async_trait::async_trait;
use sea_orm::*;
use worker::{D1Database, Error as WorkerError};
use std::collections::BTreeMap;
use std::sync::Arc;
use futures::future::BoxFuture;
use std::pin::Pin;
use std::future::Future;

// Custom D1 Connection that implements ConnectionTrait
pub struct D1Connection {
    db: Arc<D1Database>,
}

impl D1Connection {
    pub fn new(db: D1Database) -> Self {
        Self {
            db: Arc::new(db),
        }
    }

    // Initialize all tables for our calendar app
    pub async fn init_schema(&self) -> Result<(), DbErr> {
        self.execute_statement("
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                full_name TEXT NOT NULL,
                company_name TEXT,
                is_active BOOLEAN NOT NULL DEFAULT true,
                is_verified BOOLEAN NOT NULL DEFAULT false,
                verification_token TEXT,
                reset_token TEXT,
                reset_token_expires DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        ", vec![]).await?;

        self.execute_statement("
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                ip_address TEXT,
                user_agent TEXT,
                expires_at DATETIME NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
            )
        ", vec![]).await?;

        self.execute_statement("
            CREATE TABLE IF NOT EXISTS calendars (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                slug TEXT UNIQUE NOT NULL,
                description TEXT,
                timezone TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT true,
                booking_buffer_minutes INTEGER NOT NULL DEFAULT 15,
                max_booking_days_ahead INTEGER NOT NULL DEFAULT 60,
                min_booking_notice_hours INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
            )
        ", vec![]).await?;

        self.execute_statement("
            CREATE TABLE IF NOT EXISTS calendar_tokens (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                calendar_id INTEGER NOT NULL,
                provider TEXT NOT NULL,
                provider_email TEXT NOT NULL,
                access_token TEXT NOT NULL,
                refresh_token TEXT NOT NULL,
                token_type TEXT NOT NULL,
                expiry DATETIME NOT NULL,
                scopes TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (calendar_id) REFERENCES calendars (id) ON DELETE CASCADE
            )
        ", vec![]).await?;

        self.execute_statement("
            CREATE TABLE IF NOT EXISTS calendar_working_hours (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                calendar_id INTEGER NOT NULL,
                day_of_week INTEGER NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT true,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (calendar_id) REFERENCES calendars (id) ON DELETE CASCADE
            )
        ", vec![]).await?;

        self.execute_statement("
            CREATE TABLE IF NOT EXISTS bookings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                calendar_id INTEGER NOT NULL,
                external_event_id TEXT,
                guest_email TEXT NOT NULL,
                guest_name TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                start_time DATETIME NOT NULL,
                end_time DATETIME NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                meeting_link TEXT,
                cancelled_at DATETIME,
                cancellation_reason TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (calendar_id) REFERENCES calendars (id) ON DELETE CASCADE
            )
        ", vec![]).await?;

        Ok(())
    }

    async fn execute_statement(&self, sql: &str, values: Vec<Value>) -> Result<ExecResult, DbErr> {
        let stmt = self.db.prepare(sql);
        
        // Convert SeaORM Values to worker::Value
        let worker_values: Vec<worker::Value> = values.into_iter().map(|v| {
            match v {
                Value::Bool(Some(b)) => worker::Value::from(b),
                Value::TinyInt(Some(i)) => worker::Value::from(i as i32),
                Value::SmallInt(Some(i)) => worker::Value::from(i as i32),
                Value::Int(Some(i)) => worker::Value::from(i),
                Value::BigInt(Some(i)) => worker::Value::from(i),
                Value::Float(Some(f)) => worker::Value::from(f),
                Value::Double(Some(d)) => worker::Value::from(d),
                Value::String(Some(s)) => worker::Value::from(s.as_str()),
                Value::Bytes(Some(b)) => worker::Value::from(b.as_slice()),
                _ => worker::Value::Null,
            }
        }).collect();

        let query = stmt.bind(&worker_values).map_err(|e| {
            DbErr::Custom(format!("D1 bind error: {:?}", e))
        })?;

        let result = query.run().await.map_err(|e| {
            DbErr::Custom(format!("D1 execution error: {:?}", e))
        })?;

        Ok(ExecResult {
            last_insert_id: result.meta().last_row_id.unwrap_or(0) as u64,
            rows_affected: result.meta().changes as u64,
        })
    }

    async fn query_statement(&self, sql: &str, values: Vec<Value>) -> Result<Vec<QueryResult>, DbErr> {
        let stmt = self.db.prepare(sql);
        
        // Convert SeaORM Values to worker::Value  
        let worker_values: Vec<worker::Value> = values.into_iter().map(|v| {
            match v {
                Value::Bool(Some(b)) => worker::Value::from(b),
                Value::TinyInt(Some(i)) => worker::Value::from(i as i32),
                Value::SmallInt(Some(i)) => worker::Value::from(i as i32),
                Value::Int(Some(i)) => worker::Value::from(i),
                Value::BigInt(Some(i)) => worker::Value::from(i),
                Value::Float(Some(f)) => worker::Value::from(f),
                Value::Double(Some(d)) => worker::Value::from(d),
                Value::String(Some(s)) => worker::Value::from(s.as_str()),
                Value::Bytes(Some(b)) => worker::Value::from(b.as_slice()),
                _ => worker::Value::Null,
            }
        }).collect();

        let query = stmt.bind(&worker_values).map_err(|e| {
            DbErr::Custom(format!("D1 bind error: {:?}", e))
        })?;

        let result = query.all().await.map_err(|e| {
            DbErr::Custom(format!("D1 query error: {:?}", e))
        })?;

        // Convert D1 results to SeaORM QueryResult format
        let results = result.results::<serde_json::Value>().map_err(|e| {
            DbErr::Custom(format!("D1 result parsing error: {:?}", e))
        })?;

        let mut query_results = Vec::new();
        
        for row in results {
            if let serde_json::Value::Object(obj) = row {
                let mut columns = Vec::new();
                let mut values = Vec::new();
                
                for (key, value) in obj {
                    columns.push(key);
                    
                    // Convert JSON value to SeaORM Value
                    let sea_value = match value {
                        serde_json::Value::Null => Value::String(None),
                        serde_json::Value::Bool(b) => Value::Bool(Some(b)),
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                Value::BigInt(Some(i))
                            } else if let Some(f) = n.as_f64() {
                                Value::Double(Some(f))
                            } else {
                                Value::String(Some(Box::new(n.to_string())))
                            }
                        },
                        serde_json::Value::String(s) => Value::String(Some(Box::new(s))),
                        _ => Value::String(Some(Box::new(value.to_string()))),
                    };
                    values.push(sea_value);
                }
                
                query_results.push(QueryResult { columns, values });
            }
        }

        Ok(query_results)
    }
}

#[async_trait]
impl ConnectionTrait for D1Connection {
    fn get_database_backend(&self) -> DbBackend {
        DbBackend::Sqlite
    }

    async fn execute(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        let (sql, values) = stmt.build(DbBackend::Sqlite);
        self.execute_statement(&sql, values).await
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        self.execute_statement(sql, vec![]).await
    }

    async fn query_one(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        let results = self.query_all(stmt).await?;
        Ok(results.into_iter().next())
    }

    async fn query_all(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        let (sql, values) = stmt.build(DbBackend::Sqlite);
        self.query_statement(&sql, values).await
    }
}

impl StreamTrait for D1Connection {
    type Stream<'a> = futures::stream::Empty<Result<QueryResult, DbErr>>;
    
    fn stream<'a>(&'a self, _stmt: Statement) -> Pin<Box<dyn Future<Output = Result<Self::Stream<'a>, DbErr>> + Send + 'a>> {
        Box::pin(async {
            Err(DbErr::Custom("Streaming not supported in D1".to_string()))
        })
    }
}

impl MockDatabaseTrait for D1Connection {
    fn into_transaction_log(self) -> Vec<Transaction> {
        vec![]
    }

    fn append_exec_result(&mut self, _result: ExecResult) {}
    
    fn append_query_result(&mut self, _result: Vec<QueryResult>) {}
    
    fn append_exec_err(&mut self, _err: DbErr) {}
    
    fn append_query_err(&mut self, _err: DbErr) {}
}

// D1 Backend wrapper that integrates with SeaORM's architecture
#[derive(Debug)]
pub struct D1Backend {
    connection: Arc<D1Connection>,
}

impl D1Backend {
    pub fn new(db: D1Database) -> Self {
        Self {
            connection: Arc::new(D1Connection::new(db)),
        }
    }
    
    pub async fn init_schema(&self) -> Result<(), DbErr> {
        self.connection.init_schema().await
    }
    
    pub fn get_connection(&self) -> Arc<D1Connection> {
        self.connection.clone()
    }
}

// Custom D1 Database Connection that can be used with SeaORM
// This wraps our D1Connection in a way that SeaORM can understand
pub struct D1DatabaseConnection {
    backend: Arc<D1Backend>,
}

impl D1DatabaseConnection {
    pub fn new(d1_backend: D1Backend) -> Self {
        Self {
            backend: Arc::new(d1_backend),
        }
    }
}

#[async_trait]
impl ConnectionTrait for D1DatabaseConnection {
    fn get_database_backend(&self) -> DbBackend {
        DbBackend::Sqlite
    }

    async fn execute(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        self.backend.connection.execute(stmt).await
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        self.backend.connection.execute_unprepared(sql).await
    }

    async fn query_one(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        self.backend.connection.query_one(stmt).await
    }

    async fn query_all(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        self.backend.connection.query_all(stmt).await
    }
}

impl StreamTrait for D1DatabaseConnection {
    type Stream<'a> = futures::stream::Empty<Result<QueryResult, DbErr>>;
    
    fn stream<'a>(&'a self, stmt: Statement) -> Pin<Box<dyn Future<Output = Result<Self::Stream<'a>, DbErr>> + Send + 'a>> {
        self.backend.connection.stream(stmt)
    }
}

impl MockDatabaseTrait for D1DatabaseConnection {
    fn into_transaction_log(self) -> Vec<Transaction> {
        vec![]
    }
    fn append_exec_result(&mut self, _result: ExecResult) {}
    fn append_query_result(&mut self, _result: Vec<QueryResult>) {}
    fn append_exec_err(&mut self, _err: DbErr) {}
    fn append_query_err(&mut self, _err: DbErr) {}
}

// Factory function to create a proper D1 database connection
pub async fn create_d1_connection(d1_db: D1Database) -> Result<DatabaseConnection, DbErr> {
    let d1_backend = D1Backend::new(d1_db);
    
    // Initialize the database schema
    d1_backend.init_schema().await?;
    
    let d1_conn = D1DatabaseConnection::new(d1_backend);
    
    // Since SeaORM's DatabaseConnection enum is closed, we need to use MockDatabaseConnection
    // but with a twist - we'll create a custom mock that actually forwards to our D1 connection
    let mock_db = D1MockDatabase::new(d1_conn);
    
    Ok(DatabaseConnection::MockDatabaseConnection(Arc::new(mock_db)))
}

// Custom MockDatabase that forwards operations to our D1 connection
struct D1MockDatabase {
    d1_conn: D1DatabaseConnection,
    transaction_log: std::sync::Mutex<Vec<Transaction>>,
}

impl D1MockDatabase {
    fn new(d1_conn: D1DatabaseConnection) -> Self {
        Self {
            d1_conn,
            transaction_log: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ConnectionTrait for D1MockDatabase {
    fn get_database_backend(&self) -> DbBackend {
        self.d1_conn.get_database_backend()
    }

    async fn execute(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        // Log the transaction
        if let Ok(mut log) = self.transaction_log.lock() {
            let (sql, values) = stmt.build(self.get_database_backend());
            log.push(Transaction::from_sql_and_values(self.get_database_backend(), &sql, &values));
        }
        
        // Forward to D1 connection
        self.d1_conn.execute(stmt).await
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        // Log the transaction
        if let Ok(mut log) = self.transaction_log.lock() {
            log.push(Transaction::from_sql_and_values(self.get_database_backend(), sql, &[]));
        }
        
        // Forward to D1 connection
        self.d1_conn.execute_unprepared(sql).await
    }

    async fn query_one(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        // Log the transaction
        if let Ok(mut log) = self.transaction_log.lock() {
            let (sql, values) = stmt.build(self.get_database_backend());
            log.push(Transaction::from_sql_and_values(self.get_database_backend(), &sql, &values));
        }
        
        // Forward to D1 connection
        self.d1_conn.query_one(stmt).await
    }

    async fn query_all(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        // Log the transaction
        if let Ok(mut log) = self.transaction_log.lock() {
            let (sql, values) = stmt.build(self.get_database_backend());
            log.push(Transaction::from_sql_and_values(self.get_database_backend(), &sql, &values));
        }
        
        // Forward to D1 connection
        self.d1_conn.query_all(stmt).await
    }
}

impl StreamTrait for D1MockDatabase {
    type Stream<'a> = futures::stream::Empty<Result<QueryResult, DbErr>>;
    
    fn stream<'a>(&'a self, stmt: Statement) -> Pin<Box<dyn Future<Output = Result<Self::Stream<'a>, DbErr>> + Send + 'a>> {
        self.d1_conn.stream(stmt)
    }
}

impl MockDatabaseTrait for D1MockDatabase {
    fn into_transaction_log(self) -> Vec<Transaction> {
        self.transaction_log.into_inner().unwrap_or_default()
    }
    
    fn append_exec_result(&mut self, _result: ExecResult) {
        // No-op for D1 since we execute immediately
    }
    
    fn append_query_result(&mut self, _result: Vec<QueryResult>) {
        // No-op for D1 since we execute immediately
    }
    
    fn append_exec_err(&mut self, _err: DbErr) {
        // No-op for D1 since we execute immediately
    }
    
    fn append_query_err(&mut self, _err: DbErr) {
        // No-op for D1 since we execute immediately
    }
}