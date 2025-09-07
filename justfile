# Development commands for Cloudflare Worker

# Show available commands
default:
    @just --list

# Set up the development environment
setup:
    @echo "🛠️ Setting up Cloudflare Worker environment..."
    @./setup.sh

# Start local development server
dev:
    @echo "🚀 Starting local development server..."
    @./dev.sh

# Build the worker
build:
    @echo "🔨 Building Rust worker..."
    @cargo install -q worker-build 2>/dev/null || echo "worker-build already installed"
    @worker-build --release

# Deploy to Cloudflare
deploy:
    @echo "☁️ Deploying to Cloudflare..."
    @./deploy.sh

# Create D1 database
db-create:
    @echo "🗄️ Creating D1 database..."
    @wrangler d1 create cloudflare-worker-db

# Apply database schema (production)
db-migrate:
    @echo "📊 Applying database schema to production..."
    @wrangler d1 execute cloudflare-worker-db --file=./schema.sql

# Apply database schema (local)
db-local:
    @echo "📊 Applying database schema to local DB..."
    @wrangler d1 execute cloudflare-worker-db --local --file=./schema.sql

# Query database (production)
db-query query:
    @wrangler d1 execute cloudflare-worker-db --command="{{query}}"

# Query local database
db-query-local query:
    @wrangler d1 execute cloudflare-worker-db --local --command="{{query}}"

# Show database tables
db-tables:
    @wrangler d1 execute cloudflare-worker-db --local --command="SELECT name FROM sqlite_master WHERE type='table';"

# Show users from database
db-users:
    @wrangler d1 execute cloudflare-worker-db --local --command="SELECT * FROM users;"

# View live logs
logs:
    @echo "📊 Viewing live logs..."
    @wrangler tail

# Clean build artifacts
clean:
    @echo "🧹 Cleaning build artifacts..."
    @cargo clean
    @rm -rf build/

# Check authentication status
auth:
    @echo "🔐 Checking authentication..."
    @wrangler whoami

# Login to Wrangler
login:
    @echo "🔑 Logging into Wrangler..."
    @wrangler auth login