#!/bin/bash
set -e

# Step 1: Generate advanced test data
echo "Generating advanced tests..."
./gen_advanced_tests.sh

# Step 2: Build the project in release mode
echo "Building project (release mode)..."
cargo build --release

# Step 3: Run the binary against test_advanced_ws
echo "Running mr_hedgehog analysis..."
TARGET_BIN="target/release/mr_hedgehog"

if [ ! -f "$TARGET_BIN" ]; then
    echo "Error: Binary not found at $TARGET_BIN"
    exit 1
fi

set -x # Enable debug printing of commands
./"$TARGET_BIN" --workspace test_advanced_ws/Cargo.toml --output result.dot --expand-paths --debug --store mem || {
    echo "Memory store run failed!"
    exit 1
}

echo "Testing disk storage backend..."
./"$TARGET_BIN" --workspace test_advanced_ws/Cargo.toml --output result_disk.dot --expand-paths --debug --store disk || {
    echo "Disk store run failed!"
    exit 1
}

if [ -d "mr_hedgehog_db" ]; then
    echo "Success: mr_hedgehog_db folder created by disk store."
else
    echo "Error: mr_hedgehog_db folder not found after disk store run."
    exit 1
fi
set +x

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
