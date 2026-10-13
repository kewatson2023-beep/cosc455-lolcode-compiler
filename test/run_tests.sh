#!/bin/sh
set -eu

BASE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
COMPILER=${1:-"$BASE_DIR/../bin/lolcompiler"}
PASSED=0
FAILED=0

for SOURCE in "$BASE_DIR"/valid_*.lol; do
    HTML=${SOURCE%.lol}.html
    rm -f "$HTML"
    if LOLCODE_NO_OPEN=1 "$COMPILER" "$SOURCE" >/tmp/lolcompiler-test.out 2>/tmp/lolcompiler-test.err && [ -s "$HTML" ]; then
        printf 'PASS valid: %s\n' "$(basename "$SOURCE")"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL valid: %s\n' "$(basename "$SOURCE")"
        cat /tmp/lolcompiler-test.err
        FAILED=$((FAILED + 1))
    fi
done

for SOURCE in "$BASE_DIR"/invalid_*.lol; do
    HTML=${SOURCE%.lol}.html
    printf 'stale output' > "$HTML"
    if LOLCODE_NO_OPEN=1 "$COMPILER" "$SOURCE" >/tmp/lolcompiler-test.out 2>/tmp/lolcompiler-test.err; then
        printf 'FAIL invalid accepted: %s\n' "$(basename "$SOURCE")"
        FAILED=$((FAILED + 1))
    elif [ -e "$HTML" ]; then
        printf 'FAIL invalid left HTML: %s\n' "$(basename "$SOURCE")"
        FAILED=$((FAILED + 1))
    else
        printf 'PASS invalid: %s\n' "$(basename "$SOURCE")"
        PASSED=$((PASSED + 1))
    fi
done

printf '\nPassed: %s\nFailed: %s\n' "$PASSED" "$FAILED"
[ "$FAILED" -eq 0 ]
