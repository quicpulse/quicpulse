#!/bin/bash
# test-termux-docker.sh - Test QuicPulse in Termux Docker environment
#
# NOTE: For easier testing, use Docker Compose instead:
#   docker compose -f platform/android-not-supported/docker-compose.termux.yml run test
#
# This script is useful for CI/CD or when Docker Compose isn't available.
# Run this script from the repository root.
set -e

echo "=== QuicPulse Termux Docker Test ==="
echo ""

# Check if binary exists
if [ ! -f "target/aarch64-unknown-linux-musl/debug/quicpulse" ]; then
    echo "Error: Binary not found at target/aarch64-unknown-linux-musl/debug/quicpulse"
    echo "Please build it first with: cross build --target aarch64-unknown-linux-musl"
    exit 1
fi

# Build Docker image
echo "Step 1: Building Docker image..."
docker build -f platform/android-not-supported/Dockerfile.termux -t quicpulse-termux .

echo ""
echo "Step 2: Testing version command..."
docker run --rm --network host \
    -v "$(pwd)/target/aarch64-unknown-linux-musl/debug/quicpulse:/tmp/quicpulse:ro" \
    quicpulse-termux \
    /tmp/quicpulse --version

echo ""
echo "Step 3: Testing HTTPS GET request..."
docker run --rm --network host \
    -v "$(pwd)/target/aarch64-unknown-linux-musl/debug/quicpulse:/tmp/quicpulse:ro" \
    quicpulse-termux \
    /tmp/quicpulse https://httpbin.org/get

echo ""
echo "Step 4: Testing POST with JSON..."
docker run --rm --network host \
    -v "$(pwd)/target/aarch64-unknown-linux-musl/debug/quicpulse:/tmp/quicpulse:ro" \
    quicpulse-termux \
    /tmp/quicpulse POST https://httpbin.org/post name=test value:=42

echo ""
echo "✅ All tests passed!"
echo ""
echo "To run interactively:"
echo "  docker run -it --network host \\"
echo "    -v \$(pwd)/target/aarch64-unknown-linux-musl/debug/quicpulse:/tmp/quicpulse:ro \\"
echo "    quicpulse-termux bash"
