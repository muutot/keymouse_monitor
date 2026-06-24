#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-release}"

case "$MODE" in
    -h|--help)
        echo "Usage: $0 [release|debug]"
        echo ""
        echo "Build keymouse-monitor and copy the exe to exe/"
        echo ""
        echo "Modes:"
        echo "  release  (default)  cargo build --release, profile = release"
        echo "  debug               cargo build (dev profile),   profile = debug"
        exit 0
        ;;
    release) FLAG="--release"; PROFILE="release" ;;
    debug)   FLAG="";         PROFILE="debug" ;;
    *)       echo "Usage: $0 [release|debug]" >&2; exit 1 ;;
esac

cargo build $FLAG

mkdir -p exe
cp "target/$PROFILE/keymouse-monitor.exe" exe/
echo "✓ exe/keymouse-monitor.exe"
