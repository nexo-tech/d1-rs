# Fast test execution with nextest (recommended for iteration)
test:
    @echo "🚀 Running ALL D1 ORM tests with nextest (fast parallel execution)..."
    @pkill -f "rustc|cargo" > /dev/null 2>&1 || true
    @sleep 1
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p')

# Legacy cargo test (slower but sometimes needed for compatibility)
test-legacy:
    @echo "🧪 Running ALL D1 ORM tests with legacy cargo test..."
    cargo test --target $(rustc -vV | sed -n 's|host: ||p')

# Run a specific test by name (super fast for iteration)
test-one TEST_NAME:
    @echo "🎯 Running specific test: {{TEST_NAME}}..."
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') -E 'test({{TEST_NAME}})'

# Run tests matching a pattern (great for focusing on specific areas)
test-filter PATTERN:
    @echo "🔍 Running tests matching pattern: {{PATTERN}}..."
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') -E 'test(~{{PATTERN}})'

# Run only tests that failed in the last run (super useful for debugging)
test-failed:
    @echo "⚠️  Re-running previously failed tests..."
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') --last-run-id failed

# Run tests with timing information (helps identify slow tests)
test-timing:
    @echo "⏱️  Running tests with detailed timing..."
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') --profile slow-timeout

# Quick smoke test (runs fast tests only)
test-quick:
    @echo "💨 Running quick smoke tests..."
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') -E 'test(basic) or test(crud) or test(simple)'

# Ultra-fast test for rapid iteration
test-fast:
    @echo "⚡ Running tests with aggressive timeouts for rapid iteration..."
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') --profile fast

# Run single test with maximum detail and output
test-debug TEST_NAME:
    @echo "🔍 Debugging test: {{TEST_NAME}} with full output..."
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') --profile single -E 'test({{TEST_NAME}})'

# Show all available tests (useful for finding test names)
test-list:
    @echo "📋 Listing all available tests..."
    cargo nextest list --target $(rustc -vV | sed -n 's|host: ||p')

# Show nextest version info
test-config:
    @echo "📊 Showing nextest version and configuration..."
    cargo nextest show-config version

# Clean locks and restart tests (use if tests hang)
test-clean:
    @echo "🧹 Cleaning build locks and target directory..."
    @pkill -f "rustc|cargo" > /dev/null 2>&1 || true
    @rm -rf target/
    @echo "✅ Clean complete. Try running tests again."

# === Multi-Database Testing Commands ===

# Start all test databases (optimized)
start-test-dbs:
    @echo "🐳 Starting test database containers..."
    @if ! docker ps | grep -q "d1rs-postgres-test\|d1rs-mysql-test"; then \
        echo "📦 Containers not running, starting them..."; \
        docker compose -f docker-compose.test.yml up -d; \
    else \
        echo "📦 Containers already running, skipping start..."; \
    fi
    @echo "⏳ Waiting for databases to be ready..."
    @echo "🐘 Checking PostgreSQL readiness..."
    @timeout 30 bash -c 'until pg_isready -h localhost -p 5434 -U d1rs_user >/dev/null 2>&1; do echo "⏳ PostgreSQL starting..."; sleep 1; done' || (echo "❌ PostgreSQL failed to start" && exit 1)
    @echo "🐬 Checking MySQL readiness..."
    @timeout 30 bash -c 'until mysql -h localhost -P 3308 -u d1rs_user -pd1rs_pass -e "SELECT 1" >/dev/null 2>&1; do echo "⏳ MySQL starting..."; sleep 1; done' || (echo "⚠️  MySQL failed to start, continuing anyway")
    @echo "✅ Test databases ready!"

# Start PostgreSQL only (for PostgreSQL tests)
start-postgres-only:
    @echo "🐳 Starting PostgreSQL container..."
    @if ! docker ps | grep -q "d1rs-postgres-test"; then \
        echo "📦 Starting PostgreSQL container..."; \
        docker compose -f docker-compose.test.yml up -d postgres-test; \
    else \
        echo "📦 PostgreSQL already running, skipping start..."; \
    fi
    @echo "🐘 Checking PostgreSQL readiness..."
    @timeout 30 bash -c 'until pg_isready -h localhost -p 5434 -U d1rs_user >/dev/null 2>&1; do echo "⏳ PostgreSQL starting..."; sleep 1; done' || (echo "❌ PostgreSQL failed to start" && exit 1)
    @echo "✅ PostgreSQL ready!"

# Start MySQL only (for MySQL tests)
start-mysql-only:
    @echo "🐳 Starting MySQL container..."
    @if ! docker ps | grep -q "d1rs-mysql-test"; then \
        echo "📦 Starting MySQL container..."; \
        docker compose -f docker-compose.test.yml up -d mysql-test; \
    else \
        echo "📦 MySQL already running, skipping start..."; \
    fi
    @echo "🐬 Checking MySQL readiness..."
    @timeout 30 bash -c 'while ! docker exec d1rs-mysql-test mysqladmin ping -h localhost -u d1rs_user -pd1rs_pass --silent >/dev/null 2>&1; do echo "⏳ MySQL starting..."; sleep 2; done' || (echo "❌ MySQL failed to start" && exit 1)
    @echo "✅ MySQL ready!"

# Stop all test databases
stop-test-dbs:
    @echo "🛑 Stopping test database containers..."
    docker compose -f docker-compose.test.yml down
    @echo "✅ Test databases stopped!"

# Clean test database volumes
clean-test-dbs:
    @echo "🧹 Cleaning test database volumes..."
    docker compose -f docker-compose.test.yml down -v
    docker volume prune -f || true
    @echo "✅ Test databases cleaned!"

# Run tests on PostgreSQL
test-postgres: start-postgres-only
    @echo "🐘 Running tests on PostgreSQL..."
    @export POSTGRES_TEST_URL="postgresql://d1rs_user:d1rs_pass@localhost:5434/d1rs_test" && \
    export DATABASE_URL="$$POSTGRES_TEST_URL" && \
    export TEST_DATABASE_TYPE="postgres" && \
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') --features postgres
    @echo "✅ PostgreSQL tests completed!"

# Run tests on MySQL  
test-mysql: start-mysql-only
    @echo "🐬 Running tests on MySQL..."
    @export MYSQL_TEST_URL="mysql://d1rs_user:d1rs_pass@localhost:3308/d1rs_test" && \
    export DATABASE_URL="$$MYSQL_TEST_URL" && \
    export TEST_DATABASE_TYPE="mysql" && \
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') --features mysql
    @echo "✅ MySQL tests completed!"

# Run tests on all three databases sequentially
test-all-dbs: clean-test-dbs start-test-dbs
    @echo "🎯 Running comprehensive tests on ALL databases..."
    @echo "\n=== 1/3: SQLite Tests ==="
    @export TEST_DATABASE_TYPE="sqlite" && \
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p')
    @echo "\n=== 2/3: PostgreSQL Tests ==="
    @export POSTGRES_TEST_URL="postgresql://d1rs_user:d1rs_pass@localhost:5433/d1rs_test" && \
    export DATABASE_URL="$$POSTGRES_TEST_URL" && \
    export TEST_DATABASE_TYPE="postgres" && \
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') --features postgres
    @echo "\n=== 3/3: MySQL Tests ==="
    @export MYSQL_TEST_URL="mysql://d1rs_user:d1rs_pass@localhost:3307/d1rs_test" && \
    export DATABASE_URL="$$MYSQL_TEST_URL" && \
    export TEST_DATABASE_TYPE="mysql" && \
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') --features mysql
    just stop-test-dbs
    @echo "\n✅ All database tests completed successfully!"

# Run performance benchmarks on all databases (if available)
bench-all-dbs: start-test-dbs
    @echo "📊 Running performance benchmarks on all databases..."
    @echo "\n=== SQLite Benchmarks ==="
    @export TEST_DATABASE_TYPE="sqlite" && \
    cargo bench --features sqlite || echo "⚠️ SQLite benchmarks not available or failed"
    @echo "\n=== PostgreSQL Benchmarks ==="
    @export POSTGRES_TEST_URL="postgresql://d1rs_user:d1rs_pass@localhost:5433/d1rs_test" && \
    export DATABASE_URL="$$POSTGRES_TEST_URL" && \
    export TEST_DATABASE_TYPE="postgres" && \
    cargo bench --features postgres || echo "⚠️ PostgreSQL benchmarks not available or failed"
    @echo "\n=== MySQL Benchmarks ==="
    @export MYSQL_TEST_URL="mysql://d1rs_user:d1rs_pass@localhost:3307/d1rs_test" && \
    export DATABASE_URL="$$MYSQL_TEST_URL" && \
    export TEST_DATABASE_TYPE="mysql" && \
    cargo bench --features mysql || echo "⚠️ MySQL benchmarks not available or failed"
    just stop-test-dbs
    @echo "✅ All benchmarks completed!"

# Health check for all databases
check-test-dbs:
    @echo "🔍 Checking database health..."
    @echo -n "PostgreSQL: "
    @if docker exec d1rs-postgres-test pg_isready -U d1rs_user -d d1rs_test >/dev/null 2>&1; then echo "✅ Ready"; else echo "❌ Not ready"; fi
    @echo -n "MySQL: "  
    @if docker exec d1rs-mysql-test mysqladmin ping -h localhost -u d1rs_user -pd1rs_pass --silent >/dev/null 2>&1; then echo "✅ Ready"; else echo "❌ Not ready"; fi
    @echo "✅ Database health check completed!"

# Show database logs
logs-postgres:
    @echo "📋 PostgreSQL container logs (last 50 lines, following):"
    docker logs d1rs-postgres-test --tail 50 -f

logs-mysql:
    @echo "📋 MySQL container logs (last 50 lines, following):"
    docker logs d1rs-mysql-test --tail 50 -f

# Run specific test on specific database
test-one-postgres TEST_NAME: start-test-dbs  
    @echo "🎯 Running specific test on PostgreSQL: {{TEST_NAME}}..."
    @export POSTGRES_TEST_URL="postgresql://d1rs_user:d1rs_pass@localhost:5433/d1rs_test" && \
    export DATABASE_URL="$$POSTGRES_TEST_URL" && \
    export TEST_DATABASE_TYPE="postgres" && \
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') --features postgres -E 'test({{TEST_NAME}})'
    @echo "✅ PostgreSQL test completed!"

test-one-mysql TEST_NAME: start-test-dbs
    @echo "🎯 Running specific test on MySQL: {{TEST_NAME}}..."
    @export MYSQL_TEST_URL="mysql://d1rs_user:d1rs_pass@localhost:3307/d1rs_test" && \
    export DATABASE_URL="$$MYSQL_TEST_URL" && \
    export TEST_DATABASE_TYPE="mysql" && \
    cargo nextest run --target $(rustc -vV | sed -n 's|host: ||p') --features mysql -E 'test({{TEST_NAME}})'
    @echo "✅ MySQL test completed!"

