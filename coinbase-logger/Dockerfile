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
    echo "pub mod connection; pub mod data_types; pub mod file_handlers; pub mod websocket; pub use connection::ConnectionHandler; pub use data_types::TickerData; pub use file_handlers::FileHandles; use std::error::Error; pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;" > src/lib.rs && \
    mkdir src/connection src/data_types src/file_handlers src/websocket && \
    echo "pub struct ConnectionHandler; impl ConnectionHandler { pub async fn new(_: usize, _: Vec<String>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> { Ok(Self) } pub async fn run(&mut self) {} }" > src/connection.rs && \
    echo "pub struct TickerData;" > src/data_types.rs && \
    echo "pub struct FileHandles;" > src/file_handlers.rs && \
    echo "pub async fn get_all_products() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> { Ok(vec![]) }" > src/websocket.rs && \
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
RUN mkdir -p /mnt/md/data && \
    chown -R logger:logger /mnt/md/data

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
VOLUME ["/mnt/md/data"]

# Health check to ensure the process is running
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD pgrep -f coinbase-logger || exit 1

# Run the application
CMD ["coinbase-logger"]