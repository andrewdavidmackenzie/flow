#!/bin/bash
echo Measuring coverage of files matching pattern: $1
cd `echo $1 | cut -f 1 -d '-'` || return
# -executable on linux
for filename in `find ../target/debug -perm +111 -type f -depth 1 -name $1`
do
  echo "    Measuring coverage of $filename"
  mkdir -p ../target/cov/`basename $filename`
#  kcov --exclude-pattern=/.cargo,/usr/lib ../target/cov/`basename $filename` "$filename"
done
