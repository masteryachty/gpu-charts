# Docker Hub Setup for Coinbase Logger

This guide explains how to set up automatic Docker Hub publishing for the Coinbase Logger.

## Prerequisites

1. Docker Hub account (free at https://hub.docker.com)
2. GitHub repository with push access
3. Docker Hub Personal Access Token (not your password)

## Setup Steps

### 1. Create Docker Hub Repository

1. Log in to [Docker Hub](https://hub.docker.com)
2. Click "Create Repository"
3. Name it `coinbase-logger`
4. Set visibility (Public recommended for easier access)
5. Click "Create"

### 2. Generate Docker Hub Access Token

1. Go to Account Settings → Security
2. Click "New Access Token"
3. Name it (e.g., "GitHub Actions")
4. Select permissions: "Read, Write, Delete"
5. Copy the token (you won't see it again!)

### 3. Configure GitHub Secrets

In your GitHub repository:

1. Go to Settings → Secrets and variables → Actions
2. Add these repository secrets:
   - `DOCKER_USERNAME`: Your Docker Hub username
   - `DOCKER_TOKEN`: The access token from step 2

### 4. Push to Trigger Build

The GitHub Action will automatically build and publish when:
- You push to `main` or `master` branch
- Changes are made to `coinbase-logger/` directory
- You manually trigger the workflow

```bash
# Make a change and push
git add .
git commit -m "feat: setup docker hub publishing"
git push origin main
```

### 5. Verify Publication

1. Check GitHub Actions tab for build status
2. Visit your Docker Hub repository to see the published image
3. Image will be available at: `docker.io/YOUR_USERNAME/coinbase-logger:latest`

## Using the Published Image

### On Any Machine

```bash
# Pull and run directly
docker run -d \
  --name coinbase-logger \
  -v /path/to/data:/usr/src/app/data \
  YOUR_USERNAME/coinbase-logger:latest

# Or use the production docker-compose
curl -O https://raw.githubusercontent.com/YOUR_REPO/main/coinbase-logger/docker-compose.prod.yml
DOCKER_USERNAME=YOUR_USERNAME DATA_PATH=/path/to/data docker-compose -f docker-compose.prod.yml up -d
```

### With Custom Environment

Create `.env` file:
```env
DOCKER_USERNAME=your-docker-username
DATA_PATH=/path/to/your/data
TZ=UTC
```

Then run:
```bash
docker-compose -f docker-compose.prod.yml up -d
```

## Image Tags

The GitHub Action creates multiple tags:

- `latest`: Always points to the latest main/master build
- `main` or `master`: Branch-specific tags
- `main-SHA` or `master-SHA`: Commit-specific tags (first 7 chars of commit SHA)
- `pr-NUMBER`: For pull requests (not pushed to registry)

## Multi-Platform Support

The image is built for:
- `linux/amd64` (Intel/AMD processors)
- `linux/arm64` (ARM processors, including Apple Silicon)

## Updating the Image

To update to the latest version:

```bash
# Pull latest image
docker-compose -f docker-compose.prod.yml pull

# Restart with new image
docker-compose -f docker-compose.prod.yml up -d
```

## Troubleshooting

### Build Fails

1. Check GitHub Actions logs
2. Verify secrets are set correctly
3. Ensure Dockerfile builds locally

### Can't Pull Image

1. Check Docker Hub username is correct
2. For private repos, ensure you're logged in: `docker login`
3. Verify image name matches exactly

### Permission Errors

If you get permission errors when running:
```bash
# Fix data directory permissions
sudo chown -R 1000:1000 /path/to/data
```

## Security Best Practices

1. Use access tokens, never passwords
2. Rotate tokens periodically
3. Use minimal permissions needed
4. Consider private repository for sensitive applications
5. Review Docker Hub audit logs regularly