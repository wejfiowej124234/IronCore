# Multi-stage build for minimal production image
FROM rust:1.75-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static

# Create app directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "fn main() {}" > src/cli.rs
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src
COPY resources ./resources

# Build the actual application
RUN touch src/main.rs src/cli.rs && cargo build --release

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    sqlite \
    openssl \
    python3 \
    py3-pip

# Create non-root user
RUN addgroup -g 1001 -S defi && \
    adduser -S defi -u 1001 -G defi

# Create app directory and set ownership
WORKDIR /app
RUN chown defi:defi /app

# Copy binaries from builder stage
COPY --from=builder --chown=defi:defi /app/target/release/defi-wallet /usr/local/bin/
COPY --from=builder --chown=defi:defi /app/target/release/wallet-cli /usr/local/bin/

# Copy configuration and resources
COPY --from=builder --chown=defi:defi /app/resources ./resources
# Copy a simple mock RPC server (used in place of full node dependencies)
COPY --from=builder --chown=defi:defi /app/tools/mock_rpc.py /app/tools/mock_rpc.py

# Create necessary directories
RUN mkdir -p /app/data /app/keys /app/logs && \
    chown -R defi:defi /app

# Switch to non-root user
USER defi

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/api/health || exit 1

# Expose port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV WALLET_DATABASE_URL=sqlite:/app/data/wallet.db
ENV WALLET_HOST=0.0.0.0
ENV WALLET_PORT=8080

# Volume for persistent data
VOLUME ["/app/data", "/app/keys", "/app/logs"]

# Default command: start a lightweight mock RPC server in background and then the app
CMD ["/bin/sh", "-c", "python3 /app/tools/mock_rpc.py & exec /usr/local/bin/defi-wallet server --host 0.0.0.0 --port 8080"]