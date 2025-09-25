-- Create test database
CREATE DATABASE d1rs_test;

-- Grant permissions
GRANT ALL PRIVILEGES ON DATABASE d1rs_dev TO d1rs_user;
GRANT ALL PRIVILEGES ON DATABASE d1rs_test TO d1rs_user;

-- Create schemas for testing
\c d1rs_dev;
CREATE SCHEMA IF NOT EXISTS public;
GRANT ALL ON SCHEMA public TO d1rs_user;

\c d1rs_test;
CREATE SCHEMA IF NOT EXISTS public; 
GRANT ALL ON SCHEMA public TO d1rs_user;