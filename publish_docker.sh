#!/bin/bash
set -e

# TeaQL Forge Rust Docker Release Script
# Usage: ./publish_docker.sh [TAG]
# Example: ./publish_docker.sh v1.0.0 (Defaults to 'latest' if no tag is provided)

TAG=${1:-latest}
IMAGE_NAME="teaql/teaql-forge-rs"

echo "========================================"
echo "🚀 Building TeaQL Forge Rust Docker Image"
echo "📦 Image: ${IMAGE_NAME}:${TAG}"
echo "========================================"

# 1. Check dependencies
if ! command -v cargo-zigbuild &> /dev/null; then
    echo "❌ cargo-zigbuild not found. Please install it with:"
    echo "   cargo install cargo-zigbuild"
    exit 1
fi

echo "✅ Dependencies checked."

# 2. Build for both architectures using zigbuild
echo "🔨 Compiling for x86_64 and aarch64..."
cargo zigbuild --target x86_64-unknown-linux-musl --target aarch64-unknown-linux-musl --release -p teaql-forge-server

# 3. Find the shared target directory reliably
TARGET_DIR=$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)
echo "📂 Target directory: $TARGET_DIR"

# 4. Build and Push AMD64
echo "----------------------------------------"
echo "🐳 Building and pushing AMD64 image..."
cp "${TARGET_DIR}/x86_64-unknown-linux-musl/release/teaql-forge-server" ./teaql-forge-server-bin
docker build --platform linux/amd64 -t "${IMAGE_NAME}:${TAG}-amd64" .
docker push "${IMAGE_NAME}:${TAG}-amd64"

# 5. Build and Push ARM64
echo "----------------------------------------"
echo "🐳 Building and pushing ARM64 image..."
cp "${TARGET_DIR}/aarch64-unknown-linux-musl/release/teaql-forge-server" ./teaql-forge-server-bin
docker build --platform linux/arm64 -t "${IMAGE_NAME}:${TAG}-arm64" .
docker push "${IMAGE_NAME}:${TAG}-arm64"

# 6. Create and Push Multi-Arch Manifest
echo "----------------------------------------"
echo "🔗 Creating and pushing multi-arch manifest..."
# Remove local manifest if it exists to avoid conflicts from previous runs
docker manifest rm "${IMAGE_NAME}:${TAG}" 2>/dev/null || true

docker manifest create "${IMAGE_NAME}:${TAG}" \
    "${IMAGE_NAME}:${TAG}-amd64" \
    "${IMAGE_NAME}:${TAG}-arm64"

docker manifest push "${IMAGE_NAME}:${TAG}"

# Cleanup
rm -f ./teaql-forge-server-bin

echo "========================================"
echo "✅ Successfully published ${IMAGE_NAME}:${TAG} !"
echo "You can now run: docker run -d -p 8080:8080 ${IMAGE_NAME}:${TAG}"
echo "========================================"
