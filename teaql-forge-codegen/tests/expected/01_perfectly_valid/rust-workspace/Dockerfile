# Build stage
FROM rust:1.80-slim as builder
WORKDIR /usr/src/app
# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
# Copy workspace members
COPY . .
# Build the axum web app
RUN cargo build --release --bin perfect-service-core-web-axum
# Production stage
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/perfect-service-core-web-axum /app/server
ENV PERFECT_SERVICE_CORE_BIND_ADDR="0.0.0.0:3000"
EXPOSE 3000
CMD ["./server"]