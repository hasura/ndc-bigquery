#!/bin/bash

FILE=$1
for FILE in $@; do
    tmp=$(mktemp);
    cat "$FILE" | jq > "$tmp"
    if [ $? -eq 0 ]
    then
        mv "$tmp" "$FILE";
    else
        echo "Error in: $FILE";
        exit 1
    fi
done
