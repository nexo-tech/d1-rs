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

