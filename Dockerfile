FROM rust:1.84 AS builder

WORKDIR /app

# Copy manifests for dependency caching
COPY Cargo.toml Cargo.lock ./

# Build dependencies only (cached until Cargo.toml changes)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy source code, migrations, and SQLx offline cache
COPY . .

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary and migrations from builder
COPY --from=builder /app/target/release/noteflow-backend .
COPY --from=builder /app/migrations ./migrations

EXPOSE 8080

CMD ["./noteflow-backend"]
