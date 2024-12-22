#!/usr/bin/env bash

# Was intended for CI where rust scripts are not allowed but decided to just
# use `cargo run`` instead of maintaining two files and because I also needed
# to switch the .env file and there was beginning to be too much duplication

set -e

# Validate argument is one of the two expected values
case $1 in

  "Standalone" | "mysql")
    MODE="Standalone"
    ;;

  "Shuttle" | "postgres")
    MODE="Shuttle"
    ;;
    
    *)
    echo "Error: Got \"$1\" but expected one of: Standalone | Shuttle | mysql | postgres"
    exit 1
esac

SQLX_DIR=".sqlx"
SRC_DIR=""$SQLX_DIR"_"$MODE""
echo "Source Directory: $SRC_DIR"

if [ -d "$SRC_DIR" ] 
then
    : # Do nothing (was getting trouble with negating the condition)
else
  echo "Error: Source directory does not exist"
  exit 1
fi


if [ -d "$SQLX_DIR" ] 
then
    rm -r $SQLX_DIR
fi

cp -r "$SRC_DIR" "$SQLX_DIR"

echo "Set sqlx files for mode: $MODE"
echo COMPLETED