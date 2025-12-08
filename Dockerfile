FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies (cached layer)
RUN mkdir src && \\
    echo "fn main() {}" > src/main.rs && \\
    cargo build --release && \\
    rm -rf src

# Copy source code
COPY . .

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \\
    ca-certificates \\
    libssl3 \\
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/noteflow-backend .
COPY --from=builder /app/migrations ./migrations

EXPOSE 8080

CMD ["./noteflow-backend"]