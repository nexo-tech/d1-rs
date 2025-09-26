-- PostgreSQL database initialization script for d1-rs testing
-- This script is automatically executed when the PostgreSQL test container starts up

-- Ensure we're connected to the correct database
\c d1rs_test;

-- Create extensions that might be useful for testing
-- UUID extension for UUID testing
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Enable additional logging for debugging (optional)
-- ALTER SYSTEM SET log_statement = 'all';
-- SELECT pg_reload_conf();

-- Create a schema for test isolation (optional)
-- CREATE SCHEMA IF NOT EXISTS d1rs_tests;

-- Grant all privileges to the test user
GRANT ALL PRIVILEGES ON DATABASE d1rs_test TO d1rs_user;
GRANT ALL PRIVILEGES ON SCHEMA public TO d1rs_user;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO d1rs_user;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO d1rs_user;

-- Set default privileges for future objects
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO d1rs_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO d1rs_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON FUNCTIONS TO d1rs_user;

-- Verify the database is ready
SELECT 'PostgreSQL test database initialized successfully' as status;

-- Show database information
SELECT version() as postgresql_version;
SELECT current_database() as current_db, current_user as current_user;

-- List available extensions
SELECT name, default_version, installed_version 
FROM pg_available_extensions 
WHERE name IN ('uuid-ossp', 'pgcrypto', 'ltree', 'hstore') 
ORDER BY name;