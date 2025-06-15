#!/bin/bash
set -e

echo "🔧 Building eBPF programs for HyperProcessor..."

# Check for required tools
if ! command -v cargo-bpf &> /dev/null; then
    echo "❌ cargo-bpf not found. Installing..."
    cargo install cargo-bpf
fi

if ! command -v bpf-linker &> /dev/null; then
    echo "❌ bpf-linker not found. Installing..."
    cargo install bpf-linker
fi

# Create target directory if it doesn't exist
mkdir -p target/bpfel-unknown-none/release

# Build eBPF program
cd hyper_processor_ebpf
echo "📦 Building eBPF program..."
cargo +nightly build --target bpfel-unknown-none -Z build-std=core --release

# Copy the built eBPF program to expected location
cp target/bpfel-unknown-none/release/hyper_processor_ebpf ../target/bpfel-unknown-none/release/

echo "✅ eBPF programs built successfully!"
echo "📍 Output: target/bpfel-unknown-none/release/hyper_processor_ebpf" 