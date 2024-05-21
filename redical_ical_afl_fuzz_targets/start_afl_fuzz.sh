#!/bin/sh

# Stop on error
set -e

# Move into the redical_ical_afl_fuzz_targets crate directory.
cd ./redical_ical_afl_fuzz_targets

# Install https://crates.io/crates/afl
# Need to be build with same rust version as it is running
# cargo install --force afl

DIRECTORY_INPUT_SEEDS="./input_seeds"
DIRECTORY_FUZZ_RESULTS="./fuzz_results"

mkdir -p $DIRECTORY_INPUT_SEEDS
mkdir -p $DIRECTORY_FUZZ_RESULTS

# Build
cargo afl build --release

# Fuzz target binary
FUZZ_TARGET_BIN_PATH="../target/release/redical_ical_afl_fuzz_target"

AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES=1 AFL_SKIP_CPUFREQ=1 AFL_MAP_SIZE=131072 cargo afl fuzz -i $DIRECTORY_INPUT_SEEDS -o $DIRECTORY_FUZZ_RESULTS $FUZZ_TARGET_BIN_PATH
