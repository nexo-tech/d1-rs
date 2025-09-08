# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Cloudflare Worker application built with Rust that implements a calendar system with D1 database integration. The project features a custom D1ORM (Object-Relational Mapping) library for type-safe database operations and uses WASM compilation for edge deployment.

## Development Commands

### Essential Commands
- **Start development server**: `just dev` or `./dev.sh` - Starts local server at http://localhost:8787 with hot reload
- **Build the worker**: `just build` - Compiles Rust to WASM using worker-build
- **Deploy to production**: `just deploy` or `./deploy.sh` - Builds and deploys to Cloudflare Workers
- **Run linting**: Currently no linting commands configured (add cargo clippy if needed)
- **Run tests**: Currently no test commands configured (add cargo test if needed)

### Database Commands
- **Create D1 database**: `just db-create` - Creates new D1 database instance
- **Apply migrations (prod)**: `just db-migrate` - Applies schema.sql to production
- **Apply migrations (local)**: `just db-local` - Applies schema.sql to local database
- **Query production DB**: `just db-query "SQL"` - Execute SQL on production database
- **Query local DB**: `just db-query-local "SQL"` - Execute SQL on local database
- **View tables**: `just db-tables` - List all database tables
- **View users**: `just db-users` - Display all users from database

## Architecture

### Core Components

1. **Main Worker Application** (`src/lib.rs:1-527`)
   - Cloudflare Worker entry point using the worker crate
   - Implements REST API endpoints for user CRUD operations
   - HTML forms for user interaction at `/form` and `/users`
   - Database initialization with migration system on worker startup
   - Distributed locking for migrations to prevent race conditions

2. **D1ORM Library** (`d1orm/`)
   - Custom ORM built specifically for Cloudflare D1
   - Type-safe query builders generated via procedural macros
   - Direct D1 integration without intermediate layers
   - Migration system with distributed locking support
   - Entity derive macro for automatic CRUD method generation

3. **Key Design Patterns**
   - **Entity Pattern**: User struct with `#[derive(Entity)]` macro generates type-safe database methods
   - **Migration System**: Schema-independent migrations with version tracking in `_d1orm_migrations` table
   - **Distributed Locking**: Uses advisory locks to prevent concurrent migration execution
   - **WASM Optimization**: Compiled to WebAssembly for edge deployment

### API Routes

- `GET /` - Home page with navigation
- `GET /form` - HTML form to create new users
- `GET /users` - HTML view of all users
- `GET /api/users` - JSON list of all users
- `GET /api/user/:id` - Get specific user by ID
- `POST /api/users` - Create new user (expects name, email)
- `PUT /api/user/:id` - Update user (partial updates supported)
- `DELETE /api/user/:id` - Delete user

### Database Schema

The application uses a `users` table with:
- `id` (INTEGER PRIMARY KEY)
- `name` (TEXT NOT NULL)
- `email` (TEXT NOT NULL UNIQUE)
- `created_at` (DATETIME DEFAULT CURRENT_TIMESTAMP)

Migrations are managed programmatically in `src/lib.rs:48-61` using the D1ORM migration system.

## Configuration

### Required Setup
1. Copy environment variables: Create `.env` file with Cloudflare credentials
2. Update `wrangler.toml:11` with your D1 database ID after running `just db-create`
3. The worker uses `DB` binding to access the D1 database

### Build Configuration
- Target: `wasm32-unknown-unknown`
- Build tool: `worker-build` (automatically installed)
- Output: `build/worker/shim.mjs`
- Release optimization: LTO enabled for smaller WASM bundle

## Development Workflow

1. **Initial Setup**: Run `just setup` to configure environment
2. **Local Development**: Use `just dev` for hot-reload development with local D1 simulation
3. **Database Changes**: Modify migrations in `src/lib.rs:get_migration_runner()` function
4. **Testing**: Access endpoints locally at http://localhost:8787
5. **Deployment**: Use `just deploy` after testing locally

## Important Notes

- Database migrations run automatically on worker startup with distributed locking
- The `MIGRATIONS_INITIALIZED` atomic flag prevents redundant migration checks per worker instance
- All datetime fields use custom serialization for D1 compatibility (`src/lib.rs:10-31`)
- WASM compilation requires `wasm32-unknown-unknown` target installed
- Local development uses Cloudflare's miniflare for D1 simulation