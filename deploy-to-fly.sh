#!/bin/bash
# One-command deployment script for Fly.io
# Rust Blockchain Wallet Backend

set -e

echo "╔════════════════════════════════════════════════╗"
echo "║   🚀 Deploying Rust Wallet to Fly.io         ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check Fly CLI
if ! command -v fly &> /dev/null; then
    echo "${YELLOW}⚠️  Fly CLI not installed${NC}"
    echo ""
    echo "Install Fly CLI:"
    echo "  Windows: powershell -Command \"iwr https://fly.io/install.ps1 -useb | iex\""
    echo "  macOS:   brew install flyctl"
    echo "  Linux:   curl -L https://fly.io/install.sh | sh"
    echo ""
    exit 1
fi

echo "${BLUE}Step 1: Check configuration files${NC}"
if [ ! -f "fly.toml" ]; then
    echo "${YELLOW}⚠️  fly.toml not found${NC}"
    exit 1
fi
echo "${GREEN}✅ fly.toml exists${NC}"

if [ ! -f "Dockerfile.fly" ]; then
    echo "${YELLOW}⚠️  Using default Dockerfile${NC}"
    DOCKERFILE="Dockerfile"
else
    DOCKERFILE="Dockerfile.fly"
    echo "${GREEN}✅ Using optimized Dockerfile.fly${NC}"
fi
echo ""

echo "${BLUE}Step 2: Check Fly.io login${NC}"
if ! fly auth whoami &> /dev/null; then
    echo "${YELLOW}⚠️  Not logged in${NC}"
    echo "Run: fly auth login"
    exit 1
fi
echo "${GREEN}✅ Logged in to Fly.io${NC}"
echo ""

echo "${BLUE}Step 3: Check secrets${NC}"
echo "Required secrets (use 'fly secrets set'):"
echo "  - WALLET_ENC_KEY"
echo "  - API_KEY"
echo "  - JWT_SECRET"
echo ""
read -p "Secrets configured? (y/n) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo "Set secrets first:"
    echo "  fly secrets set WALLET_ENC_KEY='your_key'"
    echo "  fly secrets set API_KEY='your_api_key'"
    echo "  fly secrets set JWT_SECRET='your_jwt_secret'"
    echo ""
    echo "Or run: bash setup-secrets.sh"
    exit 1
fi
echo "${GREEN}✅ Secrets confirmed${NC}"
echo ""

echo "${BLUE}Step 4: Deploying...${NC}"
echo "This may take 5-10 minutes (compiling Rust code)..."
echo ""

if [ "$DOCKERFILE" = "Dockerfile.fly" ]; then
    fly deploy --dockerfile Dockerfile.fly
else
    fly deploy
fi

echo ""
echo "╔════════════════════════════════════════════════╗"
echo "║   🎉 Deployment Complete!                     ║"
echo "╚════════════════════════════════════════════════╝"
echo ""

APP_NAME=$(grep "^app" fly.toml | cut -d'"' -f2)
echo "${GREEN}✅ Backend API: https://${APP_NAME}.fly.dev${NC}"
echo ""

echo "Quick test:"
echo "  curl https://${APP_NAME}.fly.dev/api/health"
echo ""

echo "View status:"
echo "  fly status"
echo ""

echo "View logs:"
echo "  fly logs"
echo ""

echo "Open dashboard:"
echo "  fly dashboard"
echo ""

echo "${GREEN}🎊 Deployment successful! Add URL to README.${NC}"

