#!/bin/bash

# Run tests for D1 ORM with SQLite backend
echo "🧪 Running D1 ORM Tests..."
echo ""

# Set environment for macOS
export LDFLAGS="-L/opt/homebrew/opt/libiconv/lib"
export CPPFLAGS="-I/opt/homebrew/opt/libiconv/include"

# Run tests with native target
echo "📦 Building and running tests..."
cargo test --features test-utils --no-default-features 2>&1 | grep -E "test result:|running|test.*\.\.\.|PASSED|FAILED|ok|error" || cargo test --features test-utils --no-default-features

echo ""
echo "✅ Tests complete!"