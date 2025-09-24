use std::time::Duration;

/// Main database configuration struct that contains all configuration options
/// for different database backends including connection pooling, SSL, and timeouts.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database connection URL/string
    pub url: String,
    /// Optional connection pool configuration (not applicable for SQLite)
    pub pool_config: Option<PoolConfig>,
    /// Optional SSL configuration (not applicable for SQLite)
    pub ssl_config: Option<SslConfig>,
    /// Timeout configuration for various operations
    pub timeout_config: TimeoutConfig,
}

/// Connection pool configuration for databases that support pooling
/// (PostgreSQL, MySQL). Not used for SQLite.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections to maintain in the pool
    pub min_connections: u32,
    /// Timeout for establishing new connections
    pub connect_timeout: Duration,
    /// How long a connection can be idle before being closed
    pub idle_timeout: Option<Duration>,
    /// Maximum lifetime of a connection before forced closure
    pub max_lifetime: Option<Duration>,
}

/// SSL/TLS configuration for secure database connections
/// (PostgreSQL, MySQL). Not used for SQLite.
#[derive(Debug, Clone)]
pub struct SslConfig {
    /// SSL mode/requirement level
    pub mode: SslMode,
    /// Path to CA certificate for server verification
    pub ca_cert_path: Option<String>,
    /// Path to client certificate for client authentication
    pub client_cert_path: Option<String>,
    /// Path to client private key for client authentication  
    pub client_key_path: Option<String>,
}

/// SSL mode enumeration defining the level of SSL enforcement
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SslMode {
    /// Disable SSL completely
    Disable,
    /// Allow SSL but don't require it
    Allow,
    /// Prefer SSL but fall back to non-SSL
    Prefer,
    /// Require SSL but don't verify certificates
    Require,
    /// Require SSL and verify CA certificate
    VerifyCA,
    /// Require SSL and verify full certificate chain
    VerifyFull,
}

/// Timeout configuration for various database operations
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Timeout for establishing database connections
    pub connect_timeout: Duration,
    /// Timeout for individual query execution
    pub query_timeout: Duration,
    /// Timeout for database transactions
    pub transaction_timeout: Duration,
}

/// Configuration error types that can occur during config parsing or validation
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("DATABASE_URL environment variable not found")]
    MissingUrl,
    #[error("Unsupported database type: {0}")]
    UnsupportedDatabase(&'static str),
    #[error("Invalid database URL: {0}")]
    InvalidUrl(String),
    #[error("Invalid timeout value: {0}")]
    InvalidTimeout(String),
    #[error("Invalid pool configuration: {0}")]
    InvalidPoolConfig(String),
}

impl DatabaseConfig {
    /// Create configuration for SQLite in-memory database
    pub fn sqlite_memory() -> Self {
        Self {
            url: ":memory:".to_string(),
            pool_config: None,
            ssl_config: None,
            timeout_config: TimeoutConfig::default(),
        }
    }
    
    /// Create configuration for SQLite file-based database
    pub fn sqlite_file(path: &str) -> Self {
        let url = if path.starts_with("sqlite:") {
            path.to_string()
        } else {
            format!("sqlite:{}", path)
        };
        
        Self {
            url,
            pool_config: None,
            ssl_config: None,
            timeout_config: TimeoutConfig::default(),
        }
    }
    
    /// Create configuration for PostgreSQL database
    #[cfg(feature = "postgres")]
    pub fn postgres(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool_config: Some(PoolConfig::postgres_default()),
            ssl_config: Some(SslConfig::default()),
            timeout_config: TimeoutConfig::default(),
        }
    }
    
    /// Create configuration for MySQL database
    #[cfg(feature = "mysql")]
    pub fn mysql(url: &str) -> Self {
        Self {
            url: url.to_string(),
            pool_config: Some(PoolConfig::mysql_default()),
            ssl_config: Some(SslConfig::default()),
            timeout_config: TimeoutConfig::default(),
        }
    }
    
    /// Parse configuration from DATABASE_URL environment variable
    pub fn from_env() -> Result<Self, ConfigError> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingUrl)?;
            
        Self::from_url(&url)
    }
    
    /// Parse configuration from a database URL string
    pub fn from_url(url: &str) -> Result<Self, ConfigError> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            #[cfg(feature = "postgres")]
            return Ok(Self::postgres(url));
            #[cfg(not(feature = "postgres"))]
            return Err(ConfigError::UnsupportedDatabase("PostgreSQL"));
        }
        
        if url.starts_with("mysql://") {
            #[cfg(feature = "mysql")]
            return Ok(Self::mysql(url));
            #[cfg(not(feature = "mysql"))]
            return Err(ConfigError::UnsupportedDatabase("MySQL"));
        }
        
        if url.starts_with("sqlite:") || url == ":memory:" || !url.contains("://") {
            if url == ":memory:" {
                return Ok(Self::sqlite_memory());
            } else {
                return Ok(Self::sqlite_file(url));
            }
        }
        
        Err(ConfigError::InvalidUrl(url.to_string()))
    }
    
    /// Get the database type based on the URL
    pub fn database_type(&self) -> &'static str {
        if self.url.starts_with("postgres://") || self.url.starts_with("postgresql://") {
            "PostgreSQL"
        } else if self.url.starts_with("mysql://") {
            "MySQL"
        } else {
            "SQLite"
        }
    }
    
    /// Check if this configuration requires connection pooling
    pub fn requires_pooling(&self) -> bool {
        self.pool_config.is_some()
    }
    
    /// Check if this configuration requires SSL
    pub fn requires_ssl(&self) -> bool {
        self.ssl_config.as_ref()
            .map(|ssl| ssl.mode != SslMode::Disable)
            .unwrap_or(false)
    }
}

impl PoolConfig {
    /// Create default pool configuration for PostgreSQL
    pub fn postgres_default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)), // 10 minutes
            max_lifetime: Some(Duration::from_secs(3600)), // 1 hour
        }
    }
    
    /// Create default pool configuration for MySQL
    pub fn mysql_default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connect_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)), // 10 minutes
            max_lifetime: Some(Duration::from_secs(1800)), // 30 minutes (MySQL has shorter default)
        }
    }
    
    /// Create a custom pool configuration with validation
    pub fn new(
        max_connections: u32,
        min_connections: u32,
        connect_timeout: Duration,
        idle_timeout: Option<Duration>,
        max_lifetime: Option<Duration>,
    ) -> Result<Self, ConfigError> {
        if max_connections == 0 {
            return Err(ConfigError::InvalidPoolConfig("max_connections must be > 0".to_string()));
        }
        
        if min_connections > max_connections {
            return Err(ConfigError::InvalidPoolConfig("min_connections cannot exceed max_connections".to_string()));
        }
        
        if connect_timeout == Duration::ZERO {
            return Err(ConfigError::InvalidPoolConfig("connect_timeout must be > 0".to_string()));
        }
        
        Ok(Self {
            max_connections,
            min_connections,
            connect_timeout,
            idle_timeout,
            max_lifetime,
        })
    }
}

impl SslConfig {
    /// Create a new SSL configuration with specified mode
    pub fn new(mode: SslMode) -> Self {
        Self {
            mode,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
        }
    }
    
    /// Create SSL configuration with certificate paths for client authentication
    pub fn with_client_certs(
        mode: SslMode,
        ca_cert_path: Option<String>,
        client_cert_path: Option<String>,
        client_key_path: Option<String>,
    ) -> Self {
        Self {
            mode,
            ca_cert_path,
            client_cert_path,
            client_key_path,
        }
    }
}

impl TimeoutConfig {
    /// Create a new timeout configuration with custom values
    pub fn new(
        connect_timeout: Duration,
        query_timeout: Duration,
        transaction_timeout: Duration,
    ) -> Result<Self, ConfigError> {
        if connect_timeout == Duration::ZERO {
            return Err(ConfigError::InvalidTimeout("connect_timeout must be > 0".to_string()));
        }
        
        if query_timeout == Duration::ZERO {
            return Err(ConfigError::InvalidTimeout("query_timeout must be > 0".to_string()));
        }
        
        if transaction_timeout == Duration::ZERO {
            return Err(ConfigError::InvalidTimeout("transaction_timeout must be > 0".to_string()));
        }
        
        Ok(Self {
            connect_timeout,
            query_timeout,
            transaction_timeout,
        })
    }
}

// Default implementations
impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(60),
            transaction_timeout: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl Default for SslConfig {
    fn default() -> Self {
        Self {
            mode: SslMode::Prefer,
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
        }
    }
}

impl std::fmt::Display for SslMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SslMode::Disable => write!(f, "disable"),
            SslMode::Allow => write!(f, "allow"),
            SslMode::Prefer => write!(f, "prefer"),
            SslMode::Require => write!(f, "require"),
            SslMode::VerifyCA => write!(f, "verify-ca"),
            SslMode::VerifyFull => write!(f, "verify-full"),
        }
    }
}

impl std::str::FromStr for SslMode {
    type Err = ConfigError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "disable" => Ok(SslMode::Disable),
            "allow" => Ok(SslMode::Allow),
            "prefer" => Ok(SslMode::Prefer),
            "require" => Ok(SslMode::Require),
            "verify-ca" => Ok(SslMode::VerifyCA),
            "verify-full" => Ok(SslMode::VerifyFull),
            _ => Err(ConfigError::InvalidUrl(format!("Invalid SSL mode: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_sqlite_memory_config() {
        let config = DatabaseConfig::sqlite_memory();
        assert_eq!(config.url, ":memory:");
        assert!(config.pool_config.is_none());
        assert!(config.ssl_config.is_none());
        assert!(!config.requires_pooling());
        assert!(!config.requires_ssl());
        assert_eq!(config.database_type(), "SQLite");
    }
    
    #[test]
    fn test_sqlite_file_config() {
        let config = DatabaseConfig::sqlite_file("test.db");
        assert_eq!(config.url, "sqlite:test.db");
        assert!(config.pool_config.is_none());
        assert!(config.ssl_config.is_none());
        assert_eq!(config.database_type(), "SQLite");
        
        // Test with sqlite: prefix already present
        let config2 = DatabaseConfig::sqlite_file("sqlite:another.db");
        assert_eq!(config2.url, "sqlite:another.db");
    }
    
    #[test]
    fn test_from_url_sqlite() {
        let config = DatabaseConfig::from_url("sqlite:test.db").expect("Should parse SQLite URL");
        assert_eq!(config.url, "sqlite:test.db");
        assert_eq!(config.database_type(), "SQLite");
        
        let config2 = DatabaseConfig::from_url(":memory:").expect("Should parse memory URL");
        assert_eq!(config2.url, ":memory:");
        
        let config3 = DatabaseConfig::from_url("test.db").expect("Should parse bare filename");
        assert_eq!(config3.url, "sqlite:test.db");
    }
    
    #[test]
    fn test_from_url_invalid() {
        let result = DatabaseConfig::from_url("invalid://test");
        assert!(matches!(result, Err(ConfigError::InvalidUrl(_))));
    }
    
    #[test]
    fn test_from_env_missing() {
        // Ensure DATABASE_URL is not set for this test
        env::remove_var("DATABASE_URL");
        let result = DatabaseConfig::from_env();
        assert!(matches!(result, Err(ConfigError::MissingUrl)));
    }
    
    #[test]
    fn test_from_env_sqlite() {
        env::set_var("DATABASE_URL", "sqlite:test.db");
        let config = DatabaseConfig::from_env().expect("Should parse from env");
        assert_eq!(config.url, "sqlite:test.db");
        env::remove_var("DATABASE_URL");
    }
    
    #[test]
    fn test_timeout_config_default() {
        let config = TimeoutConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.query_timeout, Duration::from_secs(60));
        assert_eq!(config.transaction_timeout, Duration::from_secs(300));
    }
    
    #[test]
    fn test_timeout_config_new_valid() {
        let config = TimeoutConfig::new(
            Duration::from_secs(10),
            Duration::from_secs(30),
            Duration::from_secs(60),
        ).expect("Should create valid timeout config");
        
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.query_timeout, Duration::from_secs(30));
        assert_eq!(config.transaction_timeout, Duration::from_secs(60));
    }
    
    #[test]
    fn test_timeout_config_new_invalid() {
        let result = TimeoutConfig::new(
            Duration::ZERO,
            Duration::from_secs(30),
            Duration::from_secs(60),
        );
        assert!(matches!(result, Err(ConfigError::InvalidTimeout(_))));
    }
    
    #[test]
    fn test_pool_config_postgres_default() {
        let config = PoolConfig::postgres_default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
        assert_eq!(config.max_lifetime, Some(Duration::from_secs(3600)));
    }
    
    #[test]
    fn test_pool_config_mysql_default() {
        let config = PoolConfig::mysql_default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Some(Duration::from_secs(600)));
        assert_eq!(config.max_lifetime, Some(Duration::from_secs(1800))); // MySQL has shorter default
    }
    
    #[test]
    fn test_pool_config_new_valid() {
        let config = PoolConfig::new(
            5,
            1,
            Duration::from_secs(20),
            Some(Duration::from_secs(300)),
            Some(Duration::from_secs(1800)),
        ).expect("Should create valid pool config");
        
        assert_eq!(config.max_connections, 5);
        assert_eq!(config.min_connections, 1);
    }
    
    #[test]
    fn test_pool_config_new_invalid_zero_max() {
        let result = PoolConfig::new(
            0,
            1,
            Duration::from_secs(20),
            None,
            None,
        );
        assert!(matches!(result, Err(ConfigError::InvalidPoolConfig(_))));
    }
    
    #[test]
    fn test_pool_config_new_invalid_min_exceeds_max() {
        let result = PoolConfig::new(
            5,
            10,
            Duration::from_secs(20),
            None,
            None,
        );
        assert!(matches!(result, Err(ConfigError::InvalidPoolConfig(_))));
    }
    
    #[test]
    fn test_ssl_config_default() {
        let config = SslConfig::default();
        assert_eq!(config.mode, SslMode::Prefer);
        assert!(config.ca_cert_path.is_none());
        assert!(config.client_cert_path.is_none());
        assert!(config.client_key_path.is_none());
    }
    
    #[test]
    fn test_ssl_config_new() {
        let config = SslConfig::new(SslMode::Require);
        assert_eq!(config.mode, SslMode::Require);
        assert!(config.ca_cert_path.is_none());
    }
    
    #[test]
    fn test_ssl_config_with_client_certs() {
        let config = SslConfig::with_client_certs(
            SslMode::VerifyFull,
            Some("ca.pem".to_string()),
            Some("client.pem".to_string()),
            Some("client.key".to_string()),
        );
        
        assert_eq!(config.mode, SslMode::VerifyFull);
        assert_eq!(config.ca_cert_path.as_deref(), Some("ca.pem"));
        assert_eq!(config.client_cert_path.as_deref(), Some("client.pem"));
        assert_eq!(config.client_key_path.as_deref(), Some("client.key"));
    }
    
    #[test]
    fn test_ssl_mode_display() {
        assert_eq!(SslMode::Disable.to_string(), "disable");
        assert_eq!(SslMode::Allow.to_string(), "allow");
        assert_eq!(SslMode::Prefer.to_string(), "prefer");
        assert_eq!(SslMode::Require.to_string(), "require");
        assert_eq!(SslMode::VerifyCA.to_string(), "verify-ca");
        assert_eq!(SslMode::VerifyFull.to_string(), "verify-full");
    }
    
    #[test]
    fn test_ssl_mode_from_str() {
        assert_eq!("disable".parse::<SslMode>().unwrap(), SslMode::Disable);
        assert_eq!("allow".parse::<SslMode>().unwrap(), SslMode::Allow);
        assert_eq!("prefer".parse::<SslMode>().unwrap(), SslMode::Prefer);
        assert_eq!("require".parse::<SslMode>().unwrap(), SslMode::Require);
        assert_eq!("verify-ca".parse::<SslMode>().unwrap(), SslMode::VerifyCA);
        assert_eq!("verify-full".parse::<SslMode>().unwrap(), SslMode::VerifyFull);
        
        // Test case insensitive
        assert_eq!("DISABLE".parse::<SslMode>().unwrap(), SslMode::Disable);
        assert_eq!("Prefer".parse::<SslMode>().unwrap(), SslMode::Prefer);
        
        // Test invalid
        let result = "invalid".parse::<SslMode>();
        assert!(matches!(result, Err(ConfigError::InvalidUrl(_))));
    }
    
    #[test]
    fn test_config_error_display() {
        let error = ConfigError::MissingUrl;
        assert_eq!(error.to_string(), "DATABASE_URL environment variable not found");
        
        let error = ConfigError::UnsupportedDatabase("PostgreSQL");
        assert_eq!(error.to_string(), "Unsupported database type: PostgreSQL");
        
        let error = ConfigError::InvalidUrl("bad://url".to_string());
        assert_eq!(error.to_string(), "Invalid database URL: bad://url");
        
        let error = ConfigError::InvalidTimeout("timeout must be > 0".to_string());
        assert_eq!(error.to_string(), "Invalid timeout value: timeout must be > 0");
        
        let error = ConfigError::InvalidPoolConfig("max_connections must be > 0".to_string());
        assert_eq!(error.to_string(), "Invalid pool configuration: max_connections must be > 0");
    }
    
    #[cfg(feature = "postgres")]
    #[test]
    fn test_postgres_config() {
        let config = DatabaseConfig::postgres("postgres://user:pass@localhost/db");
        assert_eq!(config.url, "postgres://user:pass@localhost/db");
        assert!(config.pool_config.is_some());
        assert!(config.ssl_config.is_some());
        assert!(config.requires_pooling());
        assert!(config.requires_ssl());
        assert_eq!(config.database_type(), "PostgreSQL");
    }
    
    #[cfg(feature = "postgres")]
    #[test]
    fn test_from_url_postgres() {
        let config = DatabaseConfig::from_url("postgres://user:pass@localhost/db")
            .expect("Should parse PostgreSQL URL");
        assert_eq!(config.database_type(), "PostgreSQL");
        
        let config2 = DatabaseConfig::from_url("postgresql://user:pass@localhost/db")
            .expect("Should parse PostgreSQL URL with postgresql:// scheme");
        assert_eq!(config2.database_type(), "PostgreSQL");
    }
    
    #[cfg(not(feature = "postgres"))]
    #[test]
    fn test_from_url_postgres_unsupported() {
        let result = DatabaseConfig::from_url("postgres://user:pass@localhost/db");
        assert!(matches!(result, Err(ConfigError::UnsupportedDatabase("PostgreSQL"))));
    }
    
    #[cfg(feature = "mysql")]
    #[test]
    fn test_mysql_config() {
        let config = DatabaseConfig::mysql("mysql://user:pass@localhost/db");
        assert_eq!(config.url, "mysql://user:pass@localhost/db");
        assert!(config.pool_config.is_some());
        assert!(config.ssl_config.is_some());
        assert!(config.requires_pooling());
        assert!(config.requires_ssl());
        assert_eq!(config.database_type(), "MySQL");
    }
    
    #[cfg(feature = "mysql")]
    #[test]
    fn test_from_url_mysql() {
        let config = DatabaseConfig::from_url("mysql://user:pass@localhost/db")
            .expect("Should parse MySQL URL");
        assert_eq!(config.database_type(), "MySQL");
    }
    
    #[cfg(not(feature = "mysql"))]
    #[test]
    fn test_from_url_mysql_unsupported() {
        let result = DatabaseConfig::from_url("mysql://user:pass@localhost/db");
        assert!(matches!(result, Err(ConfigError::UnsupportedDatabase("MySQL"))));
    }
    
    #[test]
    fn test_database_config_debug() {
        let config = DatabaseConfig::sqlite_memory();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("DatabaseConfig"));
        assert!(debug_str.contains(":memory:"));
    }
    
    #[test]
    fn test_requires_ssl_with_disable() {
        let mut config = DatabaseConfig::sqlite_memory();
        config.ssl_config = Some(SslConfig::new(SslMode::Disable));
        assert!(!config.requires_ssl());
    }
    
    #[test]
    fn test_requires_ssl_with_require() {
        let mut config = DatabaseConfig::sqlite_memory();
        config.ssl_config = Some(SslConfig::new(SslMode::Require));
        assert!(config.requires_ssl());
    }
}