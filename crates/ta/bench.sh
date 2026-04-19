#!/bin/bash
# bench.sh — reproducible local benchmark run
#
# Usage:
# $ ./bench.sh # to create baseline
# # make changes
# $ ./bench.sh run-20260221-0130 # compare against baseline

set -e

echo "Warning: For best results, close all non-essential apps"

# Run with high priority, save baseline if arg provided
if [ -n "$1" ]; then
    echo "Comparing against baseline: $1"
    sudo nice -n -20 cargo bench -- --baseline "$1"
else
    BASELINE="run-$(date +%Y%m%d-%H%M)"
    echo "Saving baseline: $BASELINE"
    sudo nice -n -20 cargo bench -- --save-baseline "$BASELINE"
    echo "Re-run with: ./bench.sh $BASELINE"
fi

# Restore ownership after sudo so regular cargo commands work
sudo chown -R "$(whoami)" target/
