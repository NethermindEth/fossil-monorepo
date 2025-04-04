#!/bin/bash
set -e

COVERAGE_PATH=".coverage/html/index.html"

if [ ! -f "$COVERAGE_PATH" ]; then
    echo "Coverage report not found at $COVERAGE_PATH"
    echo "Please run 'make coverage' first"
    exit 1
fi

echo "Code Coverage Summary"
echo "--------------------"

# Extract coverage percentage values
LINES_LINE=$(grep -n -A 3 "heading\">Lines" "$COVERAGE_PATH" | grep -n "title" | head -1 | cut -d: -f1)
if [ -n "$LINES_LINE" ]; then
    LINES_COVERAGE=$(grep -A 3 "heading\">Lines" "$COVERAGE_PATH" | grep "abbr title" | sed -e 's/.*title="\([^/]*\)\/\([^"]*\)">\([0-9.]*\) %.*/\3/')
    LINES_COVERAGE_DETAILS=$(grep -A 3 "heading\">Lines" "$COVERAGE_PATH" | grep "abbr title" | sed -e 's/.*title="\([^/]*\)\/\([^"]*\)">.*/\1 of \2/')
    echo "Lines:     $LINES_COVERAGE% ($LINES_COVERAGE_DETAILS)"
fi

FUNCTIONS_LINE=$(grep -n -A 3 "heading\">Functions" "$COVERAGE_PATH" | grep -n "title" | head -1 | cut -d: -f1)
if [ -n "$FUNCTIONS_LINE" ]; then
    FUNCTIONS_COVERAGE=$(grep -A 3 "heading\">Functions" "$COVERAGE_PATH" | grep "abbr title" | sed -e 's/.*title="\([^/]*\)\/\([^"]*\)">\([0-9.]*\) %.*/\3/')
    FUNCTIONS_COVERAGE_DETAILS=$(grep -A 3 "heading\">Functions" "$COVERAGE_PATH" | grep "abbr title" | sed -e 's/.*title="\([^/]*\)\/\([^"]*\)">.*/\1 of \2/')
    echo "Functions: $FUNCTIONS_COVERAGE% ($FUNCTIONS_COVERAGE_DETAILS)"
fi

BRANCHES_LINE=$(grep -n -A 3 "heading\">Branches" "$COVERAGE_PATH" | grep -n "title" | head -1 | cut -d: -f1)
if [ -n "$BRANCHES_LINE" ]; then
    BRANCHES_COVERAGE=$(grep -A 3 "heading\">Branches" "$COVERAGE_PATH" | grep "abbr title" | sed -e 's/.*title="\([^/]*\)\/\([^"]*\)">\([0-9.]*\) %.*/\3/')
    BRANCHES_COVERAGE_DETAILS=$(grep -A 3 "heading\">Branches" "$COVERAGE_PATH" | grep "abbr title" | sed -e 's/.*title="\([^/]*\)\/\([^"]*\)">.*/\1 of \2/')
    echo "Branches:  $BRANCHES_COVERAGE% ($BRANCHES_COVERAGE_DETAILS)"
fi

echo ""
echo "For detailed information, open the HTML report:"
echo ".coverage/html/index.html" 