# Cloudflare Worker with Rust & D1

A minimal Cloudflare Worker built with Rust featuring D1 database integration and local development support.

## Features

- 🦀 Rust-based Cloudflare Worker
- 🗄️ D1 database integration  
- 🌐 Two HTML routes (/, /about, /users)
- 🛠️ Local development with `wrangler dev`
- ⚡ Nix flake for development environment
- 🔧 Just commands for easy workflow

## Quick Start

### 1. Development Environment

```bash
# Using Nix (recommended)
nix develop

# Or install dependencies manually:
# - Rust toolchain with wasm32-unknown-unknown target
# - Node.js and wrangler CLI
# - just command runner
```

### 2. Setup

```bash
just setup  # Guides you through configuration
```

### 3. Local Development

```bash
just dev    # Start local server at http://localhost:8787
```

### 4. Deploy

```bash
just deploy # Deploy to Cloudflare
```

## Available Commands

Run `just` to see all available commands:

- `just setup` - Initial project setup
- `just dev` - Start local development server
- `just build` - Build the worker
- `just deploy` - Deploy to Cloudflare
- `just db-create` - Create D1 database
- `just db-migrate` - Apply database schema
- `just db-local` - Apply schema to local DB
- `just db-query "SQL"` - Query production database
- `just db-query-local "SQL"` - Query local database
- `just logs` - View live logs

## Configuration

1. Copy `.env.example` to `.env`
2. Set your Cloudflare API token and account ID
3. Create a D1 database: `just db-create`
4. Update `wrangler.toml` with your database ID

## Environment Variables

Create `.env` file with:

```bash
CLOUDFLARE_API_TOKEN=your_token_here
CLOUDFLARE_ACCOUNT_ID=your_account_id_here
CLOUDFLARE_DATABASE_ID=your_database_id_here
```

## Routes

- `/` - Home page with navigation
- `/about` - About page with feature list
- `/users` - Users page (demonstrates D1 database queries)

## Local Development

The worker runs locally with `wrangler dev --local` which includes:
- Local D1 database simulation
- Hot reload on file changes
- Same runtime environment as production

## Deployment

The deployment process:
1. Builds the Rust code to WASM
2. Creates/updates D1 database
3. Applies database schema
4. Deploys to Cloudflare Workers

## Database Schema

See `schema.sql` for the database structure. The example includes a `users` table with sample data for demonstration.