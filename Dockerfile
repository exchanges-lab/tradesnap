# Build stage
FROM rust:1.95-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install Chromium browser and CA certificates for HTTPS requests
RUN apt-get update && apt-get install -y \
    chromium \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/tradesnap /app/tradesnap

ENTRYPOINT ["/app/tradesnap"]

