#!/bin/bash
set -e

COVERAGE_PATH=".coverage/html/index.html"
LCOV_PATH=".coverage/lcov.info"
REPO_OWNER="NethermindEth"
REPO_NAME="fossil-prover-service"
REPO_URL="https://github.com/$REPO_OWNER/$REPO_NAME"

if [ ! -f "$COVERAGE_PATH" ] || [ ! -f "$LCOV_PATH" ]; then
    echo "Coverage reports not found at $COVERAGE_PATH or $LCOV_PATH"
    echo "Please run 'make coverage' first"
    exit 1
fi

# Extract coverage directly from the HTML file
LINES_COVERAGE=$(grep -A 3 "heading\">Lines" "$COVERAGE_PATH" | grep "title" | grep -o "[0-9]\+\.[0-9]\+")

if [ -z "$LINES_COVERAGE" ]; then
    echo "Could not extract coverage percentage from HTML report"
    exit 1
fi

echo "Coverage percentage: $LINES_COVERAGE%"

# Determine badge color based on coverage
if (( $(echo "$LINES_COVERAGE < 50" | bc -l) )); then
    COLOR="red"
elif (( $(echo "$LINES_COVERAGE < 70" | bc -l) )); then
    COLOR="yellow"
elif (( $(echo "$LINES_COVERAGE < 90" | bc -l) )); then
    COLOR="green"
else
    COLOR="brightgreen"
fi

# Generate badge URL
BADGE_URL="https://img.shields.io/badge/coverage-$LINES_COVERAGE%25-$COLOR"
echo "Badge URL: $BADGE_URL"

# Save badge URL to a file
mkdir -p .coverage/badge
echo "$BADGE_URL" > .coverage/badge/url.txt

# Download the badge image
if command -v curl &> /dev/null; then
    curl -s "$BADGE_URL" > .coverage/badge/coverage.svg
    echo "Badge saved to .coverage/badge/coverage.svg"
elif command -v wget &> /dev/null; then
    wget -q -O .coverage/badge/coverage.svg "$BADGE_URL"
    echo "Badge saved to .coverage/badge/coverage.svg"
else
    echo "Could not download badge: neither curl nor wget are available"
    echo "Use the URL above to display your badge in the README"
fi

# Print instructions
echo ""
echo "To add this badge to your README.md, use:"
echo "[![Coverage]($BADGE_URL)]($REPO_URL)"
echo ""
echo "Full markdown:"
echo "[![Rust CI](https://github.com/$REPO_OWNER/$REPO_NAME/workflows/Rust%20CI/badge.svg)](https://github.com/$REPO_OWNER/$REPO_NAME/actions?query=workflow%3A%22Rust+CI%22)"
echo "[![Coverage]($BADGE_URL)]($REPO_URL)" 