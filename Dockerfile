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

ARG IMAGE_VERSION=unknown
RUN echo "Image version: ${IMAGE_VERSION}" > /etc/image-version

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD ["/app/tradesnap"]
