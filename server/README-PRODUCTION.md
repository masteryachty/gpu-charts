# GPU Charts Server - Production Deployment with Let's Encrypt

A high-performance financial data server with automatic SSL certificate management using Let's Encrypt.

## Features

- 🚀 **Ultra-low latency** HTTP/2 TLS server
- 🔐 **Automatic SSL certificates** with Let's Encrypt
- 🔄 **Auto-renewal** of certificates
- 📊 **Real-time financial data** serving
- 🐳 **Docker-ready** with production optimizations
- 🎯 **Memory-mapped** file serving for zero-copy performance

## Quick Start

### Using Docker Compose (Recommended)

```bash
# Clone the repository
git clone https://github.com/masteryachty/gpu-charts.git
cd gpu-charts/server

# Configure environment
cp .env.example .env
# Edit .env with your domain and email

# Start the server
docker-compose -f docker-compose.production.yml up -d
```

### Using Docker Run

```bash
docker run -d \
  --name gpu-charts-server-production \
  --restart unless-stopped \
  -p 80:80 \
  -p 8443:8443 \
  -v /var/lib/letsencrypt:/etc/letsencrypt \
  -v /var/log/letsencrypt:/var/log \
  -v /mnt/md/data:/mnt/md/data:ro \
  -e DOMAIN=api.rednax.io \
  -e EMAIL=admin@rednax.io \
  -e USE_LETSENCRYPT=true \
  masteryachty/gpu-charts-server-production:latest
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DOMAIN` | `api.rednax.io` | Domain name for SSL certificate |
| `EMAIL` | `admin@rednax.io` | Email for Let's Encrypt registration |
| `USE_LETSENCRYPT` | `true` | Enable Let's Encrypt certificates |
| `LETSENCRYPT_STAGING` | `false` | Use Let's Encrypt staging environment |
| `AUTO_RENEW` | `true` | Enable automatic certificate renewal |
| `DATA_PATH` | `/mnt/md/data` | Path to financial data files |
| `SSL_CERT_PATH` | `/app/certs/localhost.crt` | SSL certificate path |
| `SSL_KEY_PATH` | `/app/certs/localhost.key` | SSL private key path |

### SSL Certificate Options

#### Let's Encrypt (Production)
- **Automatic certificates** for your domain
- **90-day validity** with auto-renewal
- **Trusted by all browsers**
- **Rate limits** apply (50 certificates per week)

#### Let's Encrypt Staging
- **Testing environment** for development
- **No rate limits**
- **Not trusted by browsers** (for testing only)

#### Self-Signed Certificates
- **Development only**
- **No external dependencies**
- **Browser warnings** required

## API Endpoints

### Data Endpoint
```bash
GET https://api.rednax.io:8443/api/data
```

**Parameters:**
- `symbol`: Trading symbol (e.g., "BTC-USD")
- `type`: Data type (e.g., "MD")
- `start`: Start timestamp (Unix)
- `end`: End timestamp (Unix)
- `columns`: Comma-separated column names

**Example:**
```bash
curl "https://api.rednax.io:8443/api/data?symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time,best_bid,best_ask"
```

### Symbols Endpoint
```bash
GET https://api.rednax.io:8443/api/symbols
```

Returns available trading symbols.

## Deployment

### Prerequisites

1. **Domain name** pointing to your server
2. **Ports 80 and 8443** open in firewall
3. **Docker** installed on server
4. **Data files** mounted at `/mnt/md/data`

### Production Deployment

1. **Set up DNS** for your domain to point to your server
2. **Configure GitHub Secrets** for automated deployment:
   ```
   DOCKER_USERNAME          # Docker Hub username
   DOCKER_TOKEN            # Docker Hub token
   PRODUCTION_HOST         # Server hostname/IP
   PRODUCTION_USER         # SSH username
   PRODUCTION_SSH_KEY      # SSH private key
   PRODUCTION_DOMAIN       # Domain name (optional)
   PRODUCTION_EMAIL        # Email for certificates (optional)
   ```

3. **Deploy using GitHub Actions**:
   - Push to main branch
   - GitHub Actions will build and deploy automatically
   - Monitor deployment in Actions tab

### Manual Deployment

```bash
# Pull latest image
docker pull masteryachty/gpu-charts-server-production:latest

# Stop existing container
docker stop gpu-charts-server-production || true
docker rm gpu-charts-server-production || true

# Start new container
docker run -d \
  --name gpu-charts-server-production \
  --restart unless-stopped \
  -p 80:80 \
  -p 8443:8443 \
  -v /var/lib/letsencrypt:/etc/letsencrypt \
  -v /var/log/letsencrypt:/var/log \
  -v /mnt/md/data:/mnt/md/data:ro \
  -e DOMAIN=api.rednax.io \
  -e EMAIL=admin@rednax.io \
  -e USE_LETSENCRYPT=true \
  masteryachty/gpu-charts-server-production:latest
```

## Certificate Management

### Automatic Renewal

Certificates are automatically renewed daily at 2 AM via cron job.

### Manual Certificate Operations

```bash
# Check certificate status
docker exec gpu-charts-server-production /app/letsencrypt-setup.sh check

# Force certificate renewal
docker exec gpu-charts-server-production /app/letsencrypt-setup.sh renew

# Verify certificate details
docker exec gpu-charts-server-production /app/letsencrypt-setup.sh verify
```

### Certificate Renewal Script

Set up external monitoring with the provided renewal script:

```bash
# Copy renewal script to server
wget https://raw.githubusercontent.com/masteryachty/gpu-charts/main/server/renew-certs.sh
chmod +x renew-certs.sh

# Add to crontab (runs daily at 3 AM)
echo "0 3 * * * /path/to/renew-certs.sh >> /var/log/cert-renewal.log 2>&1" | crontab -

# Test renewal
./renew-certs.sh check
```

## Monitoring

### Health Checks

```bash
# Container health
docker ps
docker logs gpu-charts-server-production

# API health
curl -f "https://api.rednax.io:8443/api/symbols"

# Certificate health
echo | openssl s_client -servername api.rednax.io -connect api.rednax.io:8443 | openssl x509 -noout -dates
```

### Notifications

Configure Slack or Discord notifications for certificate renewal:

```bash
# Set webhook URLs
export SLACK_WEBHOOK="https://hooks.slack.com/services/..."
export DISCORD_WEBHOOK="https://discord.com/api/webhooks/..."

# Test notifications
./renew-certs.sh check
```

## Troubleshooting

### Common Issues

#### SSL Certificate Issues
```bash
# Check certificate expiry
openssl x509 -in /var/lib/letsencrypt/live/api.rednax.io/cert.pem -noout -dates

# Check DNS resolution
nslookup api.rednax.io

# Check port accessibility
telnet api.rednax.io 80
telnet api.rednax.io 8443
```

#### Let's Encrypt Rate Limiting
```bash
# Use staging environment
docker run -e LETSENCRYPT_STAGING=true ...

# Check rate limits
curl -s "https://crt.sh/?q=api.rednax.io&output=json" | jq length
```

#### Container Issues
```bash
# Check container logs
docker logs gpu-charts-server-production

# Check container health
docker inspect --format='{{.State.Health.Status}}' gpu-charts-server-production

# Restart container
docker restart gpu-charts-server-production
```

### Log Locations

- **Application logs**: `docker logs gpu-charts-server-production`
- **Certificate logs**: `/var/log/letsencrypt/letsencrypt.log`
- **Renewal logs**: `/var/log/cert-renewal.log`

## Security

### Best Practices

1. **Use non-root user** inside container
2. **Mount data as read-only**
3. **Regular security updates**
4. **Monitor certificate expiry**
5. **Use strong firewall rules**

### Firewall Configuration

```bash
# Allow HTTP (for Let's Encrypt challenges)
sudo ufw allow 80/tcp

# Allow HTTPS (for API)
sudo ufw allow 8443/tcp

# Allow SSH (for management)
sudo ufw allow 22/tcp

# Enable firewall
sudo ufw enable
```

## Performance

### Optimization Tips

1. **Use SSD storage** for data files
2. **Increase memory** for large datasets
3. **Monitor CPU usage** during peak loads
4. **Use CDN** for static assets
5. **Enable HTTP/2** push for better performance

### Resource Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 1 core | 2+ cores |
| Memory | 256MB | 1GB+ |
| Storage | 1GB | 10GB+ |
| Network | 1Mbps | 10Mbps+ |

## Support

- **GitHub Issues**: [Report issues](https://github.com/masteryachty/gpu-charts/issues)
- **Documentation**: [Wiki](https://github.com/masteryachty/gpu-charts/wiki)
- **Docker Hub**: [masteryachty/gpu-charts-server-production](https://hub.docker.com/r/masteryachty/gpu-charts-server-production)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.