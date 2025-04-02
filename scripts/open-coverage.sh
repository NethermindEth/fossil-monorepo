#!/bin/bash
set -e

COVERAGE_PATH=".coverage/html/index.html"
ABSOLUTE_PATH="$PWD/$COVERAGE_PATH"

if [ ! -f "$COVERAGE_PATH" ]; then
    echo "Coverage report not found at $COVERAGE_PATH"
    echo "Please run 'make coverage' first"
    exit 1
fi

# Windows handling
if [[ "$OSTYPE" == "cygwin" ]] || [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    # On Windows, use start command with empty string as first arg to force browser
    echo "Opening coverage report on Windows..."
    # The empty string forces Windows to use the default browser
    start "" "$(cygpath -w "$ABSOLUTE_PATH")"
    exit 0
fi

# Function that tries to launch a browser - returns 0 if successful
launch_browser() {
    local browser_cmd="$1"
    local browser_name="$2"
    
    if command -v "$browser_cmd" &> /dev/null; then
        echo "Opening coverage report with $browser_name..."
        "$browser_cmd" "$ABSOLUTE_PATH" &> /dev/null &
        return 0
    fi
    return 1
}

# Try to find and launch a browser, in order of preference
# Add multiple possible names for Brave
if launch_browser "brave" "Brave" ||
   launch_browser "brave-browser" "Brave" ||
   launch_browser "/snap/bin/brave" "Brave (Snap)" ||
   launch_browser "firefox" "Firefox" ||
   launch_browser "google-chrome" "Chrome" ||
   launch_browser "chromium-browser" "Chromium" ||
   launch_browser "chromium" "Chromium" ||
   launch_browser "opera" "Opera" ||
   launch_browser "epiphany" "GNOME Web" ||
   launch_browser "konqueror" "Konqueror" ||
   launch_browser "vivaldi" "Vivaldi" ||
   launch_browser "microsoft-edge" "Microsoft Edge"; then
    echo "Browser launched successfully!"
    exit 0
fi

# Try common Snap browser paths on Linux
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    for snap_browser in "/snap/bin/brave" "/snap/bin/firefox" "/snap/bin/chromium"; do
        if [ -x "$snap_browser" ]; then
            echo "Opening with Snap-installed browser: $snap_browser"
            "$snap_browser" "$ABSOLUTE_PATH" &> /dev/null &
            exit 0
        fi
    done
fi

# If we're on macOS, try macOS-specific browsers
if [[ "$OSTYPE" == "darwin"* ]]; then
    if [ -d "/Applications/Firefox.app" ]; then
        echo "Opening with Firefox..."
        open -a Firefox "$COVERAGE_PATH"
        exit 0
    elif [ -d "/Applications/Google Chrome.app" ]; then
        echo "Opening with Google Chrome..."
        open -a "Google Chrome" "$COVERAGE_PATH"
        exit 0
    elif [ -d "/Applications/Brave Browser.app" ]; then
        echo "Opening with Brave Browser..."
        open -a "Brave Browser" "$COVERAGE_PATH"
        exit 0
    elif [ -d "/Applications/Safari.app" ]; then
        echo "Opening with Safari..."
        open -a Safari "$COVERAGE_PATH"
        exit 0
    fi
fi

# Last resort - print the path and instructions
echo "Could not find a suitable browser to launch."
echo "" 
echo "Please open the coverage report manually at:"
echo "file://$ABSOLUTE_PATH"
echo ""
echo "You can do this by copying the above path and pasting it into your browser's address bar." 