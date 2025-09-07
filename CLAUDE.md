# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Primary Development Commands
```bash
# Build for Cloudflare Workers deployment
cargo build --target wasm32-unknown-unknown --release

# Deploy to Cloudflare Workers
wrangler deploy

# Run locally with Wrangler
wrangler dev

# Run database migrations
wrangler d1 execute DB --file=./migration.sql
```

### Code Quality Commands
```bash
# Format code
cargo fmt

# Lint code
cargo clippy --target wasm32-unknown-unknown

# Check compilation
cargo check --target wasm32-unknown-unknown
```

### Local Development
```bash
# Start development server (hot reload with wrangler)
wrangler dev --local

# View logs
wrangler tail
```

## Architecture Overview

This is a **simple multi-tenant calendar booking system** built with Rust for Cloudflare Workers, using server-side HTML templates (similar to the Go version).

### Core Technology Stack
- **Runtime**: Cloudflare Workers (WASM)
- **Template Engine**: Tera (server-side HTML templates)
- **Database**: Cloudflare D1 (SQLite)
- **Authentication**: JWT tokens with Argon2 password hashing
- **No Client-Side JavaScript Framework** - Pure server-side rendering

### Key Design Principles
- **Simple like Go**: No complex frontend framework, just server-rendered HTML
- **WASM Compatible**: All dependencies work in WebAssembly environment
- **Minimal Dependencies**: Removed SeaORM, Leptos, and other complex frameworks
- **Direct D1 Integration**: Raw SQL queries instead of ORM complexity

### Project Structure
- `src/main.rs` - Main Cloudflare Worker entry point with routing
- `src/handlers.rs` - HTTP request handlers (similar to Go handlers)
- `src/templates.rs` - HTML templates embedded as strings
- `src/auth.rs` - JWT authentication and password hashing
- `src/db.rs` - Direct D1 database operations (raw SQL)
- `src/models.rs` - Data structures

### Removed Components
- ❌ Leptos framework (replaced with simple templates)
- ❌ SeaORM (replaced with raw D1 queries)
- ❌ Client-side hydration/WASM
- ❌ Complex build process
- ❌ Actix-web server (now pure Workers)

### Routes
All routes are handled server-side with HTML responses:

**Public Routes:**
- `GET /` - Redirects to login
- `GET /book/:slug` - Public calendar booking page
- `GET /api/slots/:slug` - Available time slots API
- `POST /api/book/:slug` - Create booking API

**Authentication:**
- `GET /login` - Login page
- `POST /api/login` - Login API
- `GET /register` - Registration page  
- `POST /api/register` - Registration API
- `POST /api/logout` - Logout API

**Dashboard (Protected):**
- `GET /dashboard` - Dashboard home
- `GET /dashboard/calendars` - Calendar list
- `GET /dashboard/calendars/new` - Create calendar form
- `POST /api/calendars` - Create calendar API
- `GET /dashboard/calendars/:id` - Calendar details
- `PUT /api/calendars/:id` - Update calendar API
- `DELETE /api/calendars/:id` - Delete calendar API

**Working Hours:**
- `GET /dashboard/calendars/:id/working-hours` - Working hours form
- `POST /api/calendars/:id/working-hours` - Update working hours API

### Database Schema
Direct D1 SQLite tables:
- `users` - User accounts (id, name, email, password_hash)
- `calendars` - Calendar configurations (id, user_id, name, slug, timezone, duration, buffer)  
- `calendar_working_hours` - Working hours per calendar (id, calendar_id, day_of_week, start_time, end_time, enabled)
- `bookings` - Appointments (id, calendar_id, name, email, notes, start_time, end_time)

### Environment Configuration
Set via `wrangler secret put`:
```bash
wrangler secret put JWT_SECRET
```

### Deployment
1. **Build**: `cargo build --target wasm32-unknown-unknown --release`
2. **Deploy**: `wrangler deploy`

The application compiles to WebAssembly and runs on Cloudflare's edge network.

### Key Differences from Go Version
- ✅ Same simple routing approach
- ✅ Same HTML template patterns  
- ✅ Same database operations
- ✅ Same authentication flow
- ✅ Runs on Cloudflare Workers instead of traditional server

### Testing
- No complex test setup needed
- Test locally with `wrangler dev --local`
- Deploy to staging with environment-specific configs