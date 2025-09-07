#!/bin/bash

# Deployment script for Cloudflare Worker

echo "🚀 Deploying Cloudflare Worker..."

# Check if .env exists and load it
if [ -f .env ]; then
    export $(cat .env | xargs)
    echo "📄 Loaded environment variables from .env"
else
    echo "⚠️ No .env file found. Using system environment variables."
fi

# Check if wrangler is authenticated
echo "🔐 Checking Wrangler authentication..."
if ! wrangler whoami &>/dev/null; then
    echo "❌ Not authenticated with Wrangler. Please run: wrangler auth login"
    exit 1
fi

# Build the worker
echo "🔨 Building Rust worker..."
cargo install -q worker-build 2>/dev/null || echo "worker-build already installed"
worker-build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed"
    exit 1
fi

# Create D1 database if it doesn't exist (for first deployment)
echo "🗄️ Setting up D1 database..."
wrangler d1 create cloudflare-worker-db 2>/dev/null || echo "Database may already exist"

# Apply database schema
echo "📊 Applying database schema..."
wrangler d1 execute cloudflare-worker-db --file=./schema.sql

# Deploy to Cloudflare
echo "☁️ Deploying to Cloudflare..."
wrangler deploy

if [ $? -eq 0 ]; then
    echo "✅ Deployment successful!"
    echo "🌐 Your worker is now live at the URL shown above"
    echo ""
    echo "💡 Next steps:"
    echo "   - Update your wrangler.toml with the actual database_id if needed"
    echo "   - Set up a custom domain (optional)"
    echo "   - Monitor your worker at https://dash.cloudflare.com"
else
    echo "❌ Deployment failed"
    exit 1
fi