#!/bin/bash
# Quick test script for Santa Tracker

echo "🎅 Testing Santa Tracker..."
echo ""

# Test 1: Check if binary exists
if [ -f "target/release/santa-tracker" ]; then
    echo "✅ Binary built successfully"
else
    echo "❌ Binary not found"
    exit 1
fi

# Test 2: Run for a few seconds
echo "🎄 Starting Santa Tracker (will run for 5 seconds)..."
timeout 5 ./target/release/santa-tracker || true

echo ""
echo "✅ Santa Tracker test completed!"
echo ""
echo "Next steps:"
echo "  1. Run locally: cargo run --release"
echo "  2. Build Docker: docker build -t santa-tracker:latest ."
echo "  3. Deploy to K8s: kubectl apply -f k8s/deployment.yaml"
