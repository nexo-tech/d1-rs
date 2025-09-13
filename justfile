# Development commands for Cloudflare Worker

default:
    @just --list

# Start local development server
dev:
    @./dev.sh

# Build the worker
build:
    @cargo install -q worker-build 2>/dev/null || echo "worker-build already installed"
    @worker-build --release

# Deploy to Cloudflare
deploy:
    @./deploy.sh

# Apply database schema (local)
db-local:
    @wrangler d1 execute cloudflare-worker-db --local --file=./schema.sql

# Query local database
db-query query:
    @wrangler d1 execute cloudflare-worker-db --local --command="{{query}}"

# Show database tables
db-tables:
    @wrangler d1 execute cloudflare-worker-db --local --command="SELECT name FROM sqlite_master WHERE type='table';"

# Run ORM tests (all tests - no filtering)
test-orm:
    @echo "🧪 Running ALL D1 ORM tests..."
    @cd d1orm && cargo test --target $(rustc -vV | sed -n 's|host: ||p')

# Run basic ORM tests only  
test-orm-basic:
    @echo "🧪 Running basic D1 ORM tests..."
    @cd d1orm && cargo test --target $(rustc -vV | sed -n 's|host: ||p') test_basic

# Clean build artifacts
clean:
    @cargo clean
    @rm -rf build/