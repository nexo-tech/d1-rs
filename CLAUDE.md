# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Cloudflare Worker application built with Rust that implements a Google Calendar-integrated booking system with D1 database integration. The project features Google OAuth authentication, real-time calendar sync, working hours management, and a public booking interface. It uses a custom D1ORM (Object-Relational Mapping) library for type-safe database operations and WASM compilation for edge deployment.

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

## Architecture

### Core Components

1. **Main Worker Application** (`src/lib.rs:1-1400+`)
   - Cloudflare Worker entry point using the worker crate
   - Google OAuth 2.0 authentication flow with automatic token refresh
   - Google Calendar API integration for event management
   - Working hours management system
   - Public booking interface with real-time availability checking
   - Database initialization with migration system on worker startup

2. **Google Integration** 
   - **OAuth Flow**: Complete Google OAuth 2.0 implementation with state validation
   - **Calendar API**: Fetch events, create bookings, handle conflicts
   - **Token Management**: Automatic refresh, secure storage, error handling
   - **Multi-Calendar Support**: Connect multiple Google accounts

3. **D1ORM Library** (`d1orm/`)
   - Custom ORM built specifically for Cloudflare D1
   - Type-safe query builders generated via procedural macros
   - Migration system with distributed locking support
   - Entity derive macro for automatic CRUD method generation

4. **Key Design Patterns**
   - **OAuth Token Management**: Secure token storage with automatic refresh
   - **Calendar Synchronization**: Real-time event fetching and conflict detection
   - **Working Hours System**: Flexible scheduling with timezone support
   - **Public Booking Interface**: Email-based calendar access and booking

### API Routes

#### Authentication
- `GET /` - Home page with Google OAuth login
- `GET /auth/google` - Initiate Google OAuth flow
- `GET /auth/callback` - Handle OAuth callback and token exchange

#### Management Dashboard
- `GET /dashboard` - Dashboard showing connected calendars and management options
- `GET /working-hours` - Working hours configuration interface
- `POST /api/working-hours` - Save/update working hours settings
- `GET /api/emails` - List all connected calendar email addresses

#### Public Booking
- `GET /calendar/:email` - Public booking interface for specific calendar
- `GET /api/slots/:email` - Get available time slots for date (with Google Calendar sync)
- `POST /api/book` - Create booking (creates event in Google Calendar)

### Database Schema

The application uses multiple tables:

1. **tokens** - OAuth token storage
   - `id` (INTEGER PRIMARY KEY)
   - `user_email` (TEXT UNIQUE NOT NULL) - Google account email
   - `access_token` (TEXT NOT NULL) - Google API access token
   - `refresh_token` (TEXT NOT NULL) - Token refresh capability
   - `token_type` (TEXT NOT NULL) - Usually "Bearer"
   - `expiry` (DATETIME NOT NULL) - Token expiration time
   - `created_at`, `updated_at` (DATETIME)

2. **working_hours** - Global availability settings
   - `id` (INTEGER PRIMARY KEY)
   - `start_time` (TEXT NOT NULL) - Format: "HH:MM"
   - `end_time` (TEXT NOT NULL) - Format: "HH:MM"  
   - `timezone` (TEXT NOT NULL) - Timezone identifier
   - `working_days` (TEXT NOT NULL) - Comma-separated days: "monday,tuesday,wednesday,thursday,friday"
   - `created_at`, `updated_at` (DATETIME)

3. **calendars** - Calendar metadata (legacy, mostly unused now)
4. **bookings** - Local booking records (for reference, actual events in Google Calendar)

## Configuration

### Required Environment Variables
Set these in Cloudflare Workers environment or `.env` for local development:

- `GOOGLE_CLIENT_ID` - Google OAuth client ID from Google Cloud Console
- `GOOGLE_CLIENT_SECRET` - Google OAuth client secret
- `GOOGLE_REDIRECT_URL` - OAuth redirect URL (e.g., https://your-worker.your-subdomain.workers.dev/auth/callback)

### Required Setup
1. **Google Cloud Setup**:
   - Create project in Google Cloud Console
   - Enable Google Calendar API
   - Create OAuth 2.0 credentials (Web application)
   - Add redirect URL to authorized redirect URIs
   - Note client ID and secret

2. **Cloudflare Setup**:
   - Create D1 database: `just db-create`
   - Update `wrangler.toml` with database ID
   - Set environment variables in Cloudflare dashboard or `wrangler.toml`

3. **Local Development**:
   - Create `.env` file with Google OAuth credentials
   - Install wasm32-unknown-unknown target: `rustup target add wasm32-unknown-unknown`

### Build Configuration
- Target: `wasm32-unknown-unknown`
- Build tool: `worker-build` (automatically installed)
- Output: `build/worker/shim.mjs`
- Dependencies: reqwest, url, base64 for HTTP and OAuth operations

## Usage Workflow

### For Calendar Owners
1. **Connect Google Calendar**:
   - Visit `/` and click "Sign in with Google Calendar"
   - Authorize calendar access
   - Redirected to success page

2. **Configure Working Hours**:
   - Visit `/dashboard` → "Configure Working Hours"
   - Set start/end times, timezone, working days
   - Save configuration

3. **Share Booking Link**:
   - Copy public booking URL: `/calendar/your-email@gmail.com`
   - Share with people who want to book time

### For Booking Guests
1. **Access Public Calendar**:
   - Visit `/calendar/owner-email@gmail.com`
   - View available time slots for any date

2. **Make Booking**:
   - Click available time slot
   - Enter name, email, meeting title
   - Booking creates event in owner's Google Calendar
   - Both parties receive email notifications

## Google Calendar Integration

### Event Management
- **Conflict Detection**: Fetches existing events to mark unavailable slots
- **Real-time Sync**: Checks Google Calendar for each slot request
- **Automatic Booking**: Creates events directly in Google Calendar
- **Email Notifications**: Uses Google's built-in meeting invitations

### Token Management
- **Automatic Refresh**: Expired tokens refreshed transparently
- **Error Handling**: Graceful degradation when tokens invalid
- **Multi-Account**: Support for multiple connected Google accounts
- **Secure Storage**: Tokens encrypted at rest in D1 database

### Working Hours Logic
- **Timezone Support**: Proper timezone handling for global users
- **Flexible Schedules**: Day-specific availability settings
- **Slot Generation**: Creates 30 and 60-minute booking slots
- **Conflict Resolution**: Respects existing Google Calendar events

## Development Workflow

1. **Initial Setup**: Configure Google OAuth and create D1 database
2. **Local Development**: Use `just dev` with proper environment variables
3. **Database Changes**: Modify migrations in `src/lib.rs:get_migration_runner()`
4. **Google Integration**: Test OAuth flow and Calendar API calls
5. **Deployment**: Use `just deploy` after testing locally

## Important Security Notes

- OAuth state parameter validation prevents CSRF attacks
- Tokens stored securely in D1 database
- No permanent storage of Google Calendar data
- HTTPS required for OAuth redirect URLs
- API requests use proper authentication headers

## Troubleshooting

### OAuth Issues
- Verify redirect URL matches Google Cloud Console settings
- Check that Google Calendar API is enabled
- Ensure environment variables are set correctly

### Calendar Sync Issues  
- Check token expiration and refresh logic
- Verify Google Calendar API quotas not exceeded
- Test with different Google accounts

### Deployment Issues
- Ensure D1 database binding is configured
- Verify all environment variables set in production
- Check Cloudflare Workers logs for errors