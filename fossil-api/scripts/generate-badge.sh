#!/bin/bash
set -e

# Check if required files exist
if [ ! -f .coverage/html/index.html ] || [ ! -f .coverage/lcov.info ]; then
    echo "Coverage reports not found."
    echo "Please run 'make coverage' first to generate the reports."
    exit 1
fi

# Extract coverage information from lcov.info
LINES_TOTAL=$(grep -c "DA:" .coverage/lcov.info || echo 0)
LINES_HIT=$(grep "DA:" .coverage/lcov.info | grep -v "DA:0,0" | grep -v ",0$" | wc -l || echo 0)

if [ "$LINES_TOTAL" -gt 0 ]; then
    COVERAGE_PCT=$(echo "scale=1; 100 * $LINES_HIT / $LINES_TOTAL" | bc)
else
    COVERAGE_PCT="0.0"
fi

# Determine badge color based on coverage percentage
if (( $(echo "$COVERAGE_PCT >= 80" | bc -l) )); then
    COLOR="brightgreen"
elif (( $(echo "$COVERAGE_PCT >= 70" | bc -l) )); then
    COLOR="green"
elif (( $(echo "$COVERAGE_PCT >= 60" | bc -l) )); then
    COLOR="yellowgreen"
elif (( $(echo "$COVERAGE_PCT >= 50" | bc -l) )); then
    COLOR="yellow"
elif (( $(echo "$COVERAGE_PCT >= 40" | bc -l) )); then
    COLOR="orange"
else
    COLOR="red"
fi

# Create .coverage/badge directory if it doesn't exist
mkdir -p .coverage/badge

# Generate the badge URL
BADGE_URL="https://img.shields.io/badge/coverage-${COVERAGE_PCT}%25-${COLOR}"

echo "Coverage: ${COVERAGE_PCT}%"
echo ""
echo "Add this badge to your README.md:"
echo "[![Coverage](${BADGE_URL})](https://github.com/NethermindEth/fossil-prover-service)"

# Save badge information
echo "${BADGE_URL}" > .coverage/badge/url.txt
echo "${COVERAGE_PCT}" > .coverage/badge/percentage.txt 