#!/bin/bash
set -e

# Step 1: Generate advanced test data
echo "Generating advanced tests..."
./gen_advanced_tests.sh

# Step 2: Build the project in release mode
echo "Building project (release mode)..."
cargo build --release

# Step 3: Run the binary against test_advanced_ws
echo "Running tracecraft analysis..."
TARGET_BIN="target/release/tracecraft"

if [ ! -f "$TARGET_BIN" ]; then
    echo "Error: Binary not found at $TARGET_BIN"
    exit 1
fi

./"$TARGET_BIN" --workspace test_advanced_ws/Cargo.toml --output result.dot --expand_paths

# Step 4: Verify result.dot exists and is greater than 0 bytes
if [ -s result.dot ]; then
    echo "Success: result.dot generated and is not empty."
else
    echo "Error: result.dot is missing or empty."
    exit 1
fi

# Step 5: (Optional) Simple grep check for expected output
if grep -q "main@bin_demo" result.dot; then
     echo "Verification Passed: Found 'main@bin_demo' in result.dot"
else
     echo "Verification Failed: 'main@bin_demo' not found in result.dot"
     exit 1
fi

echo "Smoke Test Completed Successfully!"
