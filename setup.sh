#!/bin/bash

# Setup script for Cloudflare Worker environment

echo "🛠️ Cloudflare Worker Setup"
echo "=========================="

# Check if .env already exists
if [ -f .env ]; then
    echo "⚠️ .env file already exists. Backing up to .env.backup"
    cp .env .env.backup
fi

# Copy template
echo "📄 Creating .env file from template..."
cp .env.example .env

echo ""
echo "📋 Please update the following values in your .env file:"
echo ""
echo "1. Get your Cloudflare API Token:"
echo "   - Visit: https://dash.cloudflare.com/profile/api-tokens"
echo "   - Click 'Create Token'"
echo "   - Use 'Custom token' template"
echo "   - Permissions: Account:Cloudflare Workers:Edit, Zone:Zone Settings:Read"
echo ""
echo "2. Get your Account ID:"
echo "   - Visit: https://dash.cloudflare.com"
echo "   - Find 'Account ID' in the right sidebar"
echo ""
echo "3. Create a D1 database:"
echo "   wrangler d1 create cloudflare-worker-db"
echo "   - Copy the database_id from the output"
echo ""
echo "4. Update wrangler.toml with your database_id"
echo ""
echo "🔧 After updating .env, run:"
echo "   export \$(cat .env | xargs) && wrangler auth login"
echo ""
echo "Then you can run:"
echo "   ./dev.sh        # for local development"
echo "   npm run deploy  # to deploy to Cloudflare"