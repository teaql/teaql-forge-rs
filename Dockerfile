FROM rust:slim-bookworm AS builder

WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy the entire workspace
COPY Cargo.toml Cargo.lock ./
COPY teaql-forge-model ./teaql-forge-model
COPY teaql-forge-codegen ./teaql-forge-codegen
COPY teaql-forge-server ./teaql-forge-server

# Build the server binary
RUN cargo build --release --bin teaql-forge-server

# Create the runtime image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies (e.g., for reqwest/HTTPS if needed, and basic utils)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder
COPY --from=builder /usr/src/app/target/release/teaql-forge-server /app/teaql-forge-server

# Expose the default port
EXPOSE 8080

# Run the server on 0.0.0.0 so it can be accessed from outside the container
ENTRYPOINT ["/app/teaql-forge-server", "--host", "0.0.0.0", "--port", "8080"]
