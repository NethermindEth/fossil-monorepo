#!/bin/bash
# ------------------------------------------------------------------------------
# Coverage Badge Update Script
# ------------------------------------------------------------------------------
# This script updates the coverage badges in README files based on provided
# coverage values or lcov.info files.
#
# Usage:
#   ./update-badges.sh [--ps-coverage VALUE] [--op-coverage VALUE] [--ps-lcov PATH] [--op-lcov PATH]
#
# Examples:
#   ./update-badges.sh --ps-coverage 78.5 --op-coverage 82.3
#   ./update-badges.sh --ps-lcov proving-service/.coverage/lcov.info --op-lcov offchain-processor/.coverage/lcov.info
#   ./update-badges.sh --ps-coverage 78.5 # Only update PS badge
#
# If no parameters are provided, the script will use default values for demonstration.
# ------------------------------------------------------------------------------

# Default values (used for demonstration if no arguments provided)
PS_COVERAGE=""
OP_COVERAGE=""
PS_LCOV=""
OP_LCOV=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --ps-coverage)
      PS_COVERAGE="$2"
      shift 2
      ;;
    --op-coverage)
      OP_COVERAGE="$2"
      shift 2
      ;;
    --ps-lcov)
      PS_LCOV="$2"
      shift 2
      ;;
    --op-lcov)
      OP_LCOV="$2"
      shift 2
      ;;
    *)
      echo "Unknown parameter: $1"
      exit 1
      ;;
  esac
done

# Function to extract coverage from lcov file
extract_coverage_from_lcov() {
  local lcov_file=$1
  local coverage=""
  
  if [ -f "$lcov_file" ]; then
    local coverage_pct=$(grep -m 1 "LF:" "$lcov_file" | awk '{print $2}')
    local coverage_hit=$(grep -m 1 "LH:" "$lcov_file" | awk '{print $2}')
    
    if [ -n "$coverage_pct" ] && [ -n "$coverage_hit" ] && [ "$coverage_pct" -gt 0 ]; then
      coverage=$(awk "BEGIN { printf \"%.1f\", ($coverage_hit / $coverage_pct) * 100 }")
      echo "Extracted coverage: $coverage%"
    fi
  else
    echo "LCOV file not found: $lcov_file"
  fi
  
  echo "$coverage"
}

# Function to determine badge color based on coverage percentage
get_color() {
  local coverage=$1
  if (( $(echo "$coverage < 50" | bc -l) )); then
    echo "red"
  elif (( $(echo "$coverage < 70" | bc -l) )); then
    echo "yellow"
  elif (( $(echo "$coverage < 90" | bc -l) )); then
    echo "green"
  else
    echo "brightgreen"
  fi
}

# Process PS coverage
if [ -z "$PS_COVERAGE" ] && [ -n "$PS_LCOV" ]; then
  PS_COVERAGE=$(extract_coverage_from_lcov "$PS_LCOV")
fi
# Use default if still not set and in demo mode
if [ -z "$PS_COVERAGE" ] && [ -z "$PS_LCOV" ] && [ -z "$OP_LCOV" ] && [ -z "$OP_COVERAGE" ]; then
  echo "Using default PS coverage for demonstration"
  PS_COVERAGE="78.5"
fi

# Process OP coverage
if [ -z "$OP_COVERAGE" ] && [ -n "$OP_LCOV" ]; then
  OP_COVERAGE=$(extract_coverage_from_lcov "$OP_LCOV")
fi
# Use default if still not set and in demo mode
if [ -z "$OP_COVERAGE" ] && [ -z "$PS_LCOV" ] && [ -z "$OP_LCOV" ] && [ -z "$PS_COVERAGE" ]; then
  echo "Using default OP coverage for demonstration"
  OP_COVERAGE="82.3"
fi

# Create badge URLs if coverage values are available
if [ -n "$PS_COVERAGE" ]; then
  PS_COLOR=$(get_color "$PS_COVERAGE")
  PS_BADGE_URL="https://img.shields.io/badge/coverage-${PS_COVERAGE}%25-${PS_COLOR}"
  echo "PS Badge URL: $PS_BADGE_URL"
fi

if [ -n "$OP_COVERAGE" ]; then
  OP_COLOR=$(get_color "$OP_COVERAGE")
  OP_BADGE_URL="https://img.shields.io/badge/coverage-${OP_COVERAGE}%25-${OP_COLOR}"
  echo "OP Badge URL: $OP_BADGE_URL"
fi

# Update badge in README file
update_badge() {
  local component=$1
  local badge_url=$2
  local readme_path=$3
  
  if [ -z "$badge_url" ]; then
    echo "No badge URL provided for $component, skipping $readme_path"
    return
  fi
  
  if [ ! -f "$readme_path" ]; then
    echo "README file not found: $readme_path"
    return
  fi
  
  echo "Updating $component badge in $readme_path"
  sed -i".bak" "s|\\[${component}-coverage-badge\\]:.*|[${component}-coverage-badge]: $badge_url|g" "$readme_path"
  rm -f "${readme_path}.bak"
}

# Update main README
if [ -f "README.md" ]; then
  [ -n "$PS_BADGE_URL" ] && update_badge "ps" "$PS_BADGE_URL" "README.md"
  [ -n "$OP_BADGE_URL" ] && update_badge "op" "$OP_BADGE_URL" "README.md"
fi

# Update project-specific READMEs
[ -n "$PS_BADGE_URL" ] && [ -f "proving-service/README.md" ] && update_badge "ps" "$PS_BADGE_URL" "proving-service/README.md"
[ -n "$OP_BADGE_URL" ] && [ -f "offchain-processor/README.md" ] && update_badge "op" "$OP_BADGE_URL" "offchain-processor/README.md"

echo "README files have been updated"
