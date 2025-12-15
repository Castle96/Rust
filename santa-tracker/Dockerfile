# Multi-stage build for minimal image size
FROM rust:1.75-slim as builder

WORKDIR /build

# Install dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml ./

# Copy source
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 santa

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/santa-tracker /app/santa-tracker

# Set ownership
RUN chown -R santa:santa /app

USER santa

# Set environment for better terminal support
ENV TERM=xterm-256color

CMD ["/app/santa-tracker"]
