#!/bin/bash

# shellcheck disable=SC2046
if [ $(which codesign) ]
then
  #echo "codesign detected"
  if [ $(which security) ]
  then
    SELFCERT=$(security find-certificate -c "self" 2>&1 | grep "self")
    if [ -n "$SELFCERT" ]
    then
      #echo "self certificate detected"
      #code sign the test binary
      codesign -s self "$1"
      echo "MacOS test runner: Signed executable '$1'"
    fi
  fi
fi

#execute the test binary that was passed as an arg
$1