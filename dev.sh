#!/bin/bash

# Development script for Cloudflare Worker with D1

echo "🚀 Setting up local development environment..."

# Check if wrangler is installed
if ! command -v wrangler &> /dev/null; then
    echo "❌ Wrangler CLI not found. Please install it first:"
    echo "npm install -g wrangler"
    exit 1
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust first:"
    echo "https://rustup.rs/"
    exit 1
fi

# Check if worker-build is installed
echo "📦 Checking worker-build installation..."
if ! command -v worker-build &> /dev/null; then
    echo "Installing worker-build..."
    cargo install worker-build
else
    echo "worker-build already installed"
fi

# Create local D1 database for development
echo "🗄️ Setting up local D1 database..."
wrangler d1 execute cloudflare-worker-db --local --file=./schema.sql 2>/dev/null || echo "Database already initialized or schema applied"

echo "✅ Development environment ready!"
echo ""
echo "📋 Available commands:"
echo "  npm run dev        - Start local development server"
echo "  npm run deploy     - Deploy to Cloudflare"
echo "  wrangler d1 execute cloudflare-worker-db --local --command='SELECT * FROM users' - Query local DB"
echo ""
echo "🌐 Starting local development server..."
echo "   Visit http://localhost:8787 to see your worker"
echo ""

# Start local development server
wrangler dev --local