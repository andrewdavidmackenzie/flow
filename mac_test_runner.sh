#!/usr/bin/env bash
#code sign the test binary
codesign -s self $1

echo "MacOS test runner: Signed executable $1, now running it"

#execute the test binary that was passed as an arg
$1