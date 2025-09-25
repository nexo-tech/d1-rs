-- Create test database
CREATE DATABASE IF NOT EXISTS d1rs_test;

-- Grant permissions  
GRANT ALL PRIVILEGES ON d1rs_dev.* TO 'd1rs_user'@'%';
GRANT ALL PRIVILEGES ON d1rs_test.* TO 'd1rs_user'@'%';

FLUSH PRIVILEGES;