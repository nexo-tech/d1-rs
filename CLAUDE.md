# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Primary Development Commands
```bash
# Start development server with hot reload
cargo leptos watch --hot-reload

# Build for production
cargo leptos build --release

# Serve production build
cargo leptos serve --release

# Build for Cloudflare Workers deployment
cargo leptos build --release --bin-features="workers"
```

### Code Quality Commands
```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Run tests
cargo test

# Run all quality checks (as in CI/CD)
cargo fmt --all -- --check
cargo clippy --all-targets --features workers -- -D warnings
cargo test --features workers
```

### Nix Environment (if using flake.nix)
```bash
# Enter development shell with all tools
nix develop
```

## Architecture Overview

This is a **multi-tenant calendar booking system** built with Rust and Leptos framework, supporting both local SQLite development and Cloudflare Workers deployment.

### Core Technology Stack
- **Framework**: Leptos (full-stack Rust framework) with SSR/hydration
- **Database**: SeaORM with auto-migrations
  - Local: SQLite (`sqlite://./calendar.db?mode=rwc`)
  - Production: Cloudflare D1 (via workers feature)
- **Frontend**: Leptos components + Tailwind CSS
- **Authentication**: JWT tokens with bcrypt password hashing
- **OAuth**: Google Calendar integration via OAuth2

### Project Structure
- `src/components/` - Leptos frontend components
  - `app.rs` - Main application router and layout
  - `auth.rs` - Login/register components
  - `calendar_management.rs` - Admin dashboard for calendar configuration
  - `public_calendar.rs` - Public booking interface
  - `booking.rs` - Booking form and slot selection
  
- `src/server/` - Backend services layer
  - `auth.rs` - JWT authentication, user management
  - `calendar_service.rs` - Calendar CRUD operations
  - `booking_service.rs` - Booking logic and slot availability
  - `oauth_service.rs` - Google Calendar OAuth integration
  - `database_service.rs` - Database abstraction (SeaORM/D1)
  - `state.rs` - Application state management
  
- `src/models/` - Database entity definitions
- `src/db/` - Database connection logic
- `migration/` - SeaORM migration files

### Feature Flags
- `ssr` (default) - Server-side rendering with Actix Web
- `hydrate` - Client-side hydration for WASM
- `workers` - Cloudflare Workers deployment with D1 database

### Key API Patterns
All API endpoints are under `/api/`:
- Authentication: `/api/register`, `/api/login`, `/api/logout`
- Calendar management: `/api/calendars` (CRUD operations)
- Public booking: `/api/public/calendar/{slug}`, `/api/public/slots/{slug}`, `/api/public/book/{slug}`

### Database Schema
The application uses SeaORM with automatic migrations. Key entities:
- `users` - User accounts with email/password
- `calendars` - Calendar configurations (name, slug, timezone, settings)
- `calendar_working_hours` - Per-calendar availability settings
- `bookings` - Appointment bookings
- `sessions` - JWT session management
- `calendar_tokens` - Google Calendar OAuth tokens

### Environment Configuration
Required environment variables (in `.env` file):
- `DATABASE_URL` - SQLite connection string
- `JWT_SECRET` - Secret key for JWT signing
- `GOOGLE_CLIENT_ID` - Google OAuth client ID (optional)
- `GOOGLE_CLIENT_SECRET` - Google OAuth secret (optional)
- `GOOGLE_REDIRECT_URL` - OAuth callback URL (optional)

### Deployment Targets
1. **Local Development**: Uses SQLite with Actix Web server
2. **Cloudflare Workers**: Uses D1 database with worker runtime
   - Configuration in `wrangler.toml`
   - GitHub Actions workflow in `.github/workflows/deploy-cloudflare.yml`

### Testing Strategy
- Unit tests run with `cargo test`
- Feature-specific tests with `cargo test --features workers`
- All tests must pass before deployment (enforced in CI/CD)