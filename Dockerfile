# Multi-stage build for d1-rs ORM development and testing environment
FROM rust:1.75 as builder

# Install system dependencies for database connections
RUN apt-get update && apt-get install -y \
    libpq-dev \
    libmysqlclient-dev \
    libsqlite3-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/d1-rs

# Copy manifests and build dependencies first (for better layer caching)
COPY Cargo.toml Cargo.lock ./
COPY d1-rs-derive/Cargo.toml d1-rs-derive/

# Create dummy source files to build dependencies
RUN mkdir src d1-rs-derive/src \
    && echo "fn main() {}" > src/main.rs \
    && echo "fn main() {}" > d1-rs-derive/src/main.rs \
    && cargo build --release \
    && rm -rf src d1-rs-derive/src

# Copy the actual source code
COPY . .

# Build the project
RUN cargo build --release --all-features

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies for database connections
RUN apt-get update && apt-get install -y \
    libpq5 \
    libmariadb3 \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd -r d1rs && useradd -r -g d1rs d1rs

WORKDIR /app

# Copy built artifacts from builder stage
COPY --from=builder /usr/src/d1-rs/target/release/deps/* ./deps/
COPY --from=builder /usr/src/d1-rs/target/release/*.rlib ./
COPY --from=builder /usr/src/d1-rs/Cargo.toml ./
COPY --from=builder /usr/src/d1-rs/README.md ./
COPY --from=builder /usr/src/d1-rs/LICENSE* ./

# Switch to non-root user
USER d1rs

# Expose common database ports for development
EXPOSE 5432 3306 8080

# Default command for development environment
CMD ["/bin/bash"]

# Labels for metadata
LABEL maintainer="d1-rs developers"
LABEL org.opencontainers.image.title="d1-rs"
LABEL org.opencontainers.image.description="Database-agnostic, type-safe ORM for SQLite, PostgreSQL, and MySQL"
LABEL org.opencontainers.image.source="https://github.com/nexo-tech/d1-rs"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"