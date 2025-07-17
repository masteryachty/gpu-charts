# Comprehensive Infrastructure & API Migration - Commit Summary

## Pull Request Details
- **PR #20**: https://github.com/masteryachty/gpu-charts/pull/20
- **Branch**: `feature/migrate-api-to-domain`
- **Commits**: 2 commits with comprehensive changes
- **Files Changed**: 26 files (678 additions, 855 deletions)

## Change Categories

### 🌐 API Migration (6 files)
- `web/src/types/advanced-types.ts` - React default API URL → `api.rednax.io`
- `charting/src/renderer/data_retriever.rs` - WASM API endpoint → `api.rednax.io`
- `web/tests/data-visualization.spec.ts` - E2E tests → production API
- `charting/tests/dual_metric_tests.rs` - Unit tests → production API
- `server/test_api_production.sh` - New production API testing script
- `web/.env.example` - Environment variable configuration examples

### 🐳 Docker Infrastructure (12 files)
**Removed redundant files:**
- `docker-compose.yml` (root)
- `docker-compose.prod.yml` (root)
- `docker-deploy.sh` (root)
- `server/docker-compose.yml`
- `server/docker-compose.production.yml`
- `server/Dockerfile.production`
- `coinbase-logger/docker-compose.yml`
- `coinbase-logger/docker-compose.prod.yml`

**Added/Updated:**
- `scripts/build-server-docker.sh` - New Docker build script
- `docker-build-push.sh` - Automated deployment script
- `server/Dockerfile` - Cloudflare Tunnel optimization
- `coinbase-logger/Dockerfile` - Build improvements

### 🔧 Server Updates (4 files)
- `server/src/main.rs` - HTTP/1.1 + TLS dual mode support
- `server/docker-entrypoint.sh` - Container initialization improvements
- `server/generate-cert-for-domain.sh` - Domain certificate generation
- `server/certs/.gitkeep` - Certificate directory structure

### 📦 Build & Configuration (4 files)
- `package.json` - Docker script integration
- `.gitignore` - Certificate exclusion rules
- `CLAUDE.md` - Documentation updates
- `coinbase-logger/src/websocket.rs` - WebSocket improvements

## Key Improvements

### Infrastructure Benefits
- **Simplified Deployment**: Unified Docker configuration
- **Reduced Complexity**: Eliminated 8 redundant files
- **Better Maintenance**: Consolidated build scripts
- **Improved Security**: Proper certificate management

### API Benefits
- **Domain-based Access**: Professional API endpoint
- **Cloudflare Integration**: HTTP/1.1 tunnel support
- **Environment Flexibility**: Easy configuration override
- **Better Testing**: Separate local/production validation

### Developer Experience
- **Clear Configuration**: Environment variable examples
- **Comprehensive Testing**: Unit + integration coverage
- **Simplified Commands**: Unified npm scripts
- **Better Documentation**: Setup and configuration guides

## Testing Commands
```bash
# Production API testing
npm run test:server:api:production

# Local development testing
npm run test:server:api

# Docker deployment
npm run docker:deploy:server

# Build verification
npm run build:wasm
```

## Configuration
```bash
# Production (default)
REACT_APP_API_BASE_URL=https://api.rednax.io

# Local development override
REACT_APP_API_BASE_URL=https://localhost:8443
```

## Impact
- **Files Added**: 6 new files for better infrastructure
- **Files Removed**: 8 redundant configuration files
- **Files Modified**: 12 updated files for API migration
- **Net Change**: -177 lines (simplified codebase)
- **Docker Scripts**: 7 new npm scripts for container management
- **API Migration**: Complete transition from IP to domain-based endpoints

This comprehensive update modernizes the infrastructure while maintaining full backward compatibility for local development.