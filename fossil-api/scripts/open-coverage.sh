#!/bin/bash
set -e

# Function to detect the appropriate browser command
function detect_browser() {
    if command -v xdg-open &> /dev/null; then
        echo "xdg-open"
    elif command -v open &> /dev/null; then
        echo "open"
    elif command -v sensible-browser &> /dev/null; then
        echo "sensible-browser"
    elif command -v firefox &> /dev/null; then
        echo "firefox"
    elif command -v chromium-browser &> /dev/null; then
        echo "chromium-browser"
    elif command -v google-chrome &> /dev/null; then
        echo "google-chrome"
    else
        echo ""
    fi
}

BROWSER=$(detect_browser)

if [ -z "$BROWSER" ]; then
    echo "No browser found to open the coverage report."
    echo "Please open .coverage/html/index.html manually in your browser."
    exit 1
fi

if [ ! -f .coverage/html/index.html ]; then
    echo "Coverage report not found at .coverage/html/index.html"
    echo "Please run 'make coverage' first to generate the report."
    exit 1
fi

echo "Opening coverage report with $BROWSER..."
$BROWSER .coverage/html/index.html 