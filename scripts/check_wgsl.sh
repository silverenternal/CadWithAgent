#!/bin/bash
# WGSL Shader Syntax Checker
# 
# This script validates WGSL shader files using naga-cli
# Install: cargo install naga-cli
#
# Usage: ./scripts/check_wgsl.sh
#        ./scripts/check_wgsl.sh shaders/transform.wgsl

set -e

SHADER_DIR="${1:-shaders}"

echo "========================================"
echo "WGSL Shader Syntax Checker"
echo "========================================"
echo ""

# Check if naga is installed
if ! command -v naga &> /dev/null; then
    echo "❌ naga-cli not found. Installing..."
    cargo install naga-cli
fi

# Find all WGSL files
if [ -d "$SHADER_DIR" ]; then
    SHADER_FILES=$(find "$SHADER_DIR" -name "*.wgsl" -type f)
else
    SHADER_FILES="$SHADER_DIR"
fi

if [ -z "$SHADER_FILES" ]; then
    echo "No WGSL files found in $SHADER_DIR"
    exit 0
fi

ERRORS=0
SUCCESS=0

for shader in $SHADER_FILES; do
    if [ -f "$shader" ]; then
        echo -n "Checking: $shader ... "
        
        # Run naga validator
        if naga "$shader" > /dev/null 2>&1; then
            echo "✅ OK"
            ((SUCCESS++))
        else
            echo "❌ FAILED"
            naga "$shader" 2>&1 | head -20
            ((ERRORS++))
        fi
    fi
done

echo ""
echo "========================================"
echo "Summary"
echo "========================================"
echo "✅ Passed: $SUCCESS"
echo "❌ Failed: $ERRORS"
echo ""

if [ $ERRORS -gt 0 ]; then
    echo "❌ WGSL validation failed!"
    exit 1
else
    echo "✅ All WGSL shaders are valid!"
    exit 0
fi
