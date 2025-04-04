#!/bin/bash
set -e

# Check if coverage report exists
if [ ! -f .coverage/html/index.html ]; then
    echo "Coverage report not found at .coverage/html/index.html"
    echo "Please run 'make coverage' first to generate the report."
    exit 1
fi

# Extract coverage information from lcov.info
if [ -f .coverage/lcov.info ]; then
    LINES_TOTAL=$(grep -c "DA:" .coverage/lcov.info || echo 0)
    LINES_HIT=$(grep "DA:" .coverage/lcov.info | grep -v "DA:0,0" | grep -v ",0$" | wc -l || echo 0)
    
    if [ "$LINES_TOTAL" -gt 0 ]; then
        COVERAGE_PCT=$(echo "scale=2; 100 * $LINES_HIT / $LINES_TOTAL" | bc)
    else
        COVERAGE_PCT="0.0"
    fi
    
    echo "📊 Coverage Summary"
    echo "----------------"
    echo "Lines Total:  $LINES_TOTAL"
    echo "Lines Hit:    $LINES_HIT"
    echo "Coverage:     $COVERAGE_PCT%"
    echo ""
    echo "For detailed information, view the HTML report at .coverage/html/index.html"
else
    echo "LCOV data not found at .coverage/lcov.info"
    echo "Please run 'make coverage' first to generate the report."
    exit 1
fi 