# Calendar Booking App

A multi-tenant calendar booking system built with Rust, Leptos, and Tailwind CSS. Supports both local SQLite development and Cloudflare Workers with D1 for production.

## Features

- 🔐 **Multi-tenant authentication** - Users can register and manage their own calendars
- 📅 **Calendar management** - Create, configure, and manage booking calendars
- ⏰ **Working hours** - Set availability by day of the week
- 🔗 **Google Calendar integration** - Connect Google Calendar accounts
- 📱 **Public booking interface** - Clean, responsive booking experience
- 🌐 **Dual deployment** - Run locally with SQLite or deploy to Cloudflare Workers with D1

## Tech Stack

- **Backend**: Rust, Leptos (SSR), SeaORM (auto-migrations)
- **Frontend**: Leptos (CSR), Tailwind CSS
- **Database**: SQLite (local), Cloudflare D1 (production)
- **Authentication**: JWT tokens, bcrypt password hashing
- **OAuth**: Google Calendar integration
- **Deployment**: Cloudflare Workers, GitHub Actions

## Prerequisites

- **Rust** (nightly toolchain)
- **cargo-leptos** build tool
- **Git** for version control

## Local Development Setup

### 1. Install Dependencies

```bash
# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup default nightly

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install cargo-leptos
cargo install --locked cargo-leptos
```

### 2. Clone and Setup

```bash
git clone <your-repo-url>
cd calendar-rust

# Create environment file
cp .env.example .env
```

### 3. Configure Environment Variables

Create a `.env` file in the project root:

```bash
# Database
DATABASE_URL=sqlite://./calendar.db?mode=rwc

# JWT Secret (generate a random string)
JWT_SECRET=your-super-secret-jwt-key-here

# Google OAuth (optional - for calendar integration)
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret
GOOGLE_REDIRECT_URL=http://localhost:3000/auth/callback

# Server Configuration
PORT=3000
```

### 4. Run Development Server

```bash
# Start development server with hot reload
cargo leptos watch

# Or with hot reload enabled
cargo leptos watch --hot-reload
```

The app will be available at:
- **Frontend**: http://localhost:3000
- **Backend API**: http://localhost:3000/api

### 5. Database Migrations

The app uses SeaORM with auto-migrations. The database will be created and migrated automatically when you first run the app.

## Project Structure

```
calendar-rust/
├── src/
│   ├── components/          # Leptos frontend components
│   │   ├── app.rs          # Main app component
│   │   ├── auth.rs         # Authentication components
│   │   ├── calendar_management.rs  # Admin dashboard
│   │   ├── public_calendar.rs      # Public booking interface
│   │   └── booking.rs      # Booking components
│   ├── models/             # Database models
│   ├── server/             # Backend services
│   │   ├── auth.rs         # Authentication service
│   │   ├── calendar_service.rs     # Calendar management
│   │   ├── oauth_service.rs        # Google OAuth integration
│   │   └── booking_service.rs      # Booking logic
│   ├── db/                 # Database connection
│   └── lib.rs              # Main library
├── migration/              # Database migrations
├── style/                  # Tailwind CSS files
├── public/                 # Static assets
├── .github/workflows/      # GitHub Actions
├── Cargo.toml             # Rust dependencies
├── tailwind.config.js     # Tailwind configuration
└── README.md
```

## Usage

### 1. Create an Account

1. Visit http://localhost:3000
2. Click "Create a new account"
3. Fill in your details and register
4. You'll be automatically logged in

### 2. Create a Calendar

1. Go to Dashboard → Calendars
2. Click "Create Calendar"
3. Set up your calendar name, timezone, and booking settings
4. Configure working hours for each day

### 3. Connect Google Calendar (Optional)

1. Set up Google OAuth credentials in your `.env` file
2. In calendar settings, connect your Google Calendar
3. This will sync bookings to your Google Calendar

### 4. Share Your Booking Link

Your calendar will have a public booking URL like:
`http://localhost:3000/book/your-calendar-slug`

Share this link with clients to allow them to book appointments.

## Building for Production

### Local Build

```bash
# Build optimized version
cargo leptos build --release

# Serve production build
cargo leptos serve --release
```

### Cloudflare Workers Build

```bash
# Build for Cloudflare Workers
cargo leptos build --release --bin-features="workers"
```

## Deployment

### Cloudflare Workers (Recommended)

1. **Setup Cloudflare Account**:
   - Create a Cloudflare account
   - Set up a D1 database
   - Get your API token and Account ID

2. **Configure Wrangler**:
   ```bash
   npx wrangler d1 create calendar-db
   ```

3. **Set GitHub Secrets**:
   - `CLOUDFLARE_API_TOKEN`
   - `CLOUDFLARE_ACCOUNT_ID`

4. **Deploy**:
   Push to the `main` branch and GitHub Actions will automatically deploy.

## Development Commands

```bash
# Start development server
cargo leptos watch

# Build for production
cargo leptos build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Clean build artifacts
cargo clean
```

## Environment Variables

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `DATABASE_URL` | SQLite database path | Yes | `sqlite://./calendar.db?mode=rwc` |
| `JWT_SECRET` | JWT signing secret | Yes | - |
| `GOOGLE_CLIENT_ID` | Google OAuth client ID | No | - |
| `GOOGLE_CLIENT_SECRET` | Google OAuth secret | No | - |
| `GOOGLE_REDIRECT_URL` | OAuth redirect URL | No | `http://localhost:3000/auth/callback` |
| `PORT` | Server port | No | `3000` |

## API Endpoints

### Authentication
- `POST /api/register` - User registration
- `POST /api/login` - User login
- `POST /api/logout` - User logout

### Calendar Management
- `GET /api/calendars` - Get user calendars
- `POST /api/calendars` - Create calendar
- `GET /api/calendars/{id}` - Get calendar details
- `PUT /api/calendars/{id}` - Update calendar
- `DELETE /api/calendars/{id}` - Delete calendar

### Public Booking
- `GET /api/public/calendar/{slug}` - Get public calendar info
- `GET /api/public/slots/{slug}` - Get available time slots
- `POST /api/public/book/{slug}` - Create booking

## Troubleshooting

### Common Issues

1. **Database Connection Error**:
   - Ensure SQLite database path is correct
   - Check file permissions

2. **WASM Target Missing**:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **cargo-leptos Not Found**:
   ```bash
   cargo install --locked cargo-leptos
   ```

4. **Tailwind Styles Not Loading**:
   - Ensure `tailwind.config.js` content paths are correct
   - Check that `tailwind-input-file` is set in `Cargo.toml`

### Development Tips

- Use `cargo leptos watch --hot-reload` for faster development
- Check browser console for client-side errors
- Use `RUST_LOG=debug` for detailed logging
- Database schema changes require restarting the dev server

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Format code: `cargo fmt`
6. Submit a pull request

## License

MIT License - see LICENSE file for details