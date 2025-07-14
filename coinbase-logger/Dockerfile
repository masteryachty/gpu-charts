# Multi-stage build for efficient image size
# Stage 1: Build the Rust application
FROM --platform=$BUILDPLATFORM rust:1.82-slim AS builder
ARG TARGETPLATFORM
ARG BUILDPLATFORM

# Install dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/app

# Copy manifest files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source files to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    cargo build --release --target x86_64-unknown-linux-gnu && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build the actual application
RUN touch src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-gnu

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash logger

# Create data directory with proper permissions
RUN mkdir -p /usr/src/app/data && \
    chown -R logger:logger /usr/src/app/data

# Copy binary from builder
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-gnu/release/coinbase-logger /usr/local/bin/coinbase-logger

# Set ownership of binary
RUN chown logger:logger /usr/local/bin/coinbase-logger && \
    chmod +x /usr/local/bin/coinbase-logger

# Switch to non-root user
USER logger

# Set working directory
WORKDIR /home/logger

# Volume for data persistence
VOLUME ["/usr/src/app/data"]

# Health check to ensure the process is running
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD pgrep -f coinbase-logger || exit 1

# Run the application
CMD ["coinbase-logger"]