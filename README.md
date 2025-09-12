# Time Forge - Calendar Booking System

A Cloudflare Worker application built with Rust that implements a Google Calendar-integrated booking system with D1 database integration.

## Features

- **Google Calendar Integration** - OAuth authentication and real-time calendar sync
- **Working Hours Management** - Configure availability and working days  
- **Public Booking Interface** - Share links for others to book time with you
- **Real-time Conflict Detection** - Prevents double-booking with existing events
- **D1 Database** - Type-safe database operations with custom ORM
- **Edge Deployment** - Fast global performance via Cloudflare Workers

## Quick Start

### Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- Google Cloud Console project with Calendar API enabled
- Cloudflare account with Workers and D1 access

### Environment Setup

1. **Install Rust target:**
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. **Configure environment variables:**
   ```bash
   cp .dev.vars.example .dev.vars
   # Edit .dev.vars with your Google OAuth credentials
   ```

3. **Create D1 database:**
   ```bash
   just db-create
   # Update wrangler.toml with the returned database ID
   ```

### Google Cloud Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select existing one
3. Enable the Google Calendar API
4. Create OAuth 2.0 credentials:
   - Application type: Web application
   - Authorized redirect URI: `http://localhost:8787/auth/callback` (for development)
5. Copy Client ID and Client Secret to your `.dev.vars` file

### Development Commands

```bash
# Start local development server
just dev

# Build the worker
just build  

# Deploy to production
just deploy

# Database operations
just db-migrate      # Apply migrations to production
just db-local        # Apply migrations locally  
just db-query "SQL"  # Query production database
```

## Project Structure

```
src/
├── lib.rs           # Main router and entry point
├── models.rs        # Data models and DTOs
├── auth.rs          # OAuth authentication logic  
├── calendar.rs      # Google Calendar API integration
├── database.rs      # Database initialization and migrations
├── templates.rs     # HTML templates
└── api/             # API route handlers
    ├── auth.rs      # Authentication endpoints
    ├── booking.rs   # Booking system endpoints  
    ├── calendar.rs  # Calendar management endpoints
    └── working_hours.rs # Working hours configuration
```

## API Endpoints

### Public Routes
- `GET /` - Home page with Google OAuth login
- `GET /calendar/:email` - Public booking interface
- `GET /api/slots/:email` - Get available time slots

### Authenticated Routes  
- `GET /auth/google` - Initiate Google OAuth flow
- `GET /auth/callback` - Handle OAuth callback
- `GET /dashboard` - Management dashboard
- `GET /working-hours` - Working hours configuration
- `POST /api/working-hours` - Save working hours
- `POST /api/book` - Create booking

## Environment Variables

### Local Development (`.dev.vars`)
```env
GOOGLE_CLIENT_ID=your_client_id
GOOGLE_CLIENT_SECRET=your_client_secret  
GOOGLE_REDIRECT_URL=http://localhost:8787/auth/callback
```

### Production (Cloudflare Dashboard)
Set the same variables in your Cloudflare Workers environment settings with production URLs.

## Usage Workflow

### For Calendar Owners
1. Visit your worker URL and sign in with Google Calendar
2. Configure working hours at `/working-hours`  
3. Share your booking link: `/calendar/your-email@gmail.com`

### For Booking Guests
1. Visit the shared calendar link
2. Select an available time slot
3. Fill in booking details
4. Receive calendar invitation via email

## Database Schema

- **tokens** - OAuth token storage with automatic refresh
- **working_hours** - Global availability settings
- **calendars** - Calendar metadata (legacy)
- **bookings** - Local booking records

## Security Features

- OAuth state parameter validation prevents CSRF attacks
- Tokens stored securely in D1 database  
- No permanent storage of Google Calendar data
- HTTPS required for OAuth redirect URLs

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes following the existing code structure
4. Test locally with `just dev`
5. Submit a pull request

## License

[Add your license here]