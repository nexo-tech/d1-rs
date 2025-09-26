-- MySQL database initialization script for d1-rs testing
-- This script is automatically executed when the MySQL test container starts up

-- Switch to the test database
USE d1rs_test;

-- The d1rs_user is already created by Docker entrypoint, just grant additional privileges if needed
-- Grant all privileges to the test user on the test database (user already exists)
-- GRANT ALL PRIVILEGES ON d1rs_test.* TO 'd1rs_user'@'%'; -- User already has these from Docker entrypoint
-- GRANT ALL PRIVILEGES ON d1rs_test.* TO 'd1rs_user'@'localhost'; -- User already has these from Docker entrypoint

-- Refresh privileges (just to be sure)
FLUSH PRIVILEGES;

-- Set some optimal settings for testing
SET GLOBAL innodb_file_per_table = ON;
SET GLOBAL innodb_flush_log_at_trx_commit = 2;
SET GLOBAL sync_binlog = 0;

-- Enable more comprehensive SQL modes for testing (removed NO_AUTO_CREATE_USER for MySQL 8.0+)
SET GLOBAL sql_mode = 'STRICT_TRANS_TABLES,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION';

-- Verify the database is ready
SELECT 'MySQL test database initialized successfully' as status;

-- Show database information
SELECT VERSION() as mysql_version;
SELECT DATABASE() as current_db, USER() as `current_user`;

-- Show available storage engines
SHOW ENGINES;

-- Show current SQL mode
SELECT @@sql_mode as sql_mode;

-- Create a test table to verify everything works
CREATE TABLE IF NOT EXISTS connection_test (
    id INT AUTO_INCREMENT PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    message VARCHAR(255) DEFAULT 'Database connection successful'
);

-- Insert a test record
INSERT INTO connection_test (message) VALUES ('MySQL initialization complete');

-- Verify the test table works
SELECT COUNT(*) as test_records FROM connection_test;

-- Clean up the test table
DROP TABLE connection_test;