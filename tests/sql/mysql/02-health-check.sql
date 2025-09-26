-- MySQL health check script
-- Verifies that the database is ready for d1-rs testing

USE d1rs_test;

-- Test basic connectivity
SELECT 1 as connectivity_test;

-- Test user permissions by creating a temporary table
CREATE TEMPORARY TABLE temp_permissions_test (
    id INT AUTO_INCREMENT PRIMARY KEY,
    data VARCHAR(255)
);

INSERT INTO temp_permissions_test (data) VALUES ('test');
SELECT COUNT(*) as permission_test FROM temp_permissions_test;

-- Test JSON support (MySQL 5.7+)
SELECT JSON_OBJECT('test', 'value') as json_test;

-- Test timestamp functionality
SELECT NOW() as timestamp_test;

-- Test UUID functionality
SELECT UUID() as uuid_test;

-- Test different data types
SELECT 
    CAST('123' AS SIGNED) as int_test,
    CAST('123.45' AS DECIMAL(10,2)) as decimal_test,
    CAST('true' AS UNSIGNED) as boolean_test;

-- Show current configuration
SELECT 
    DATABASE() as current_database,
    USER() as `current_user`,
    VERSION() as mysql_version,
    @@hostname as hostname,
    @@port as port;

-- Show important variables for d1-rs testing
SELECT 
    @@sql_mode as sql_mode,
    @@innodb_file_per_table as innodb_file_per_table,
    @@character_set_server as charset,
    @@collation_server as collation;

-- Test transaction support
START TRANSACTION;
CREATE TEMPORARY TABLE transaction_test (id INT);
INSERT INTO transaction_test VALUES (1);
SELECT COUNT(*) as transaction_test FROM transaction_test;
COMMIT;

-- All tests passed
SELECT 'MySQL health check passed - ready for d1-rs testing' as final_status;