#\!/bin/bash

echo "Files with ONLY log removals:"
echo "==============================="

# Check each modified file
for file in $(git diff --name-only); do
    # Get diff without log-related lines
    non_log_changes=$(git diff "$file" | grep -E "^[\+\-]" |         grep -v "console\." |         grep -v "log::" |         grep -v "^---" |         grep -v "^+++" |         grep -v "^@@" |         grep -v "^[\+\-]\s*$" |         grep -v "^[\+\-]\s*//" |         wc -l)
    
    if [ "$non_log_changes" -eq 0 ]; then
        echo "$file"
    fi
done
