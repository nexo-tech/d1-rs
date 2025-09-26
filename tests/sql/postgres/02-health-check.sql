-- PostgreSQL health check script
-- Verifies that the database is ready for d1-rs testing

\c d1rs_test;

-- Test basic functionality
SELECT 1 as connectivity_test;

-- Test user permissions
CREATE TEMPORARY TABLE temp_permissions_test (id SERIAL PRIMARY KEY, data TEXT);
INSERT INTO temp_permissions_test (data) VALUES ('test');
SELECT COUNT(*) as permission_test FROM temp_permissions_test;

-- Test UUID extension
SELECT uuid_generate_v4() as uuid_test;

-- Verify JSON support
SELECT '{"test": "value"}'::json as json_test;

-- Test timestamp functionality
SELECT NOW() as timestamp_test;

-- Show current configuration
SELECT 
    current_database() as database,
    current_user as user,
    inet_server_addr() as server_addr,
    inet_server_port() as server_port;

-- All tests passed
SELECT 'PostgreSQL health check passed - ready for d1-rs testing' as final_status;