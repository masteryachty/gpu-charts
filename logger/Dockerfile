# Multi-Exchange Logger - Dockerfile
# Multi-stage build for efficient image size
# Stage 1: Build the Rust application
FROM rust:1.82-slim AS builder

# Install dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/app

# Copy everything
COPY . .

# Build the application for the native platform
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash logger

# Create data directory with proper permissions
RUN mkdir -p /mnt/md/data && \
    chown -R logger:logger /mnt/md/data

# Copy binary from builder
COPY --from=builder /usr/src/app/target/release/logger /usr/local/bin/logger

# Set ownership of binary
RUN chown logger:logger /usr/local/bin/logger && \
    chmod +x /usr/local/bin/logger

# Switch to non-root user
USER logger

# Set working directory
WORKDIR /home/logger

# Volume for data persistence
VOLUME ["/mnt/md/data"]

# Expose health check port
EXPOSE 8080

# Health check using the HTTP endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["logger"]