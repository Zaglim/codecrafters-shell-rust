#!/bin/sh
#
# Use this script to run your program LOCALLY.
#
# Note: Changing this script WILL NOT affect how CodeCrafters runs your program.
#
# Learn more: https://codecrafters.io/program-interface

set -e # Exit early if any commands fail

BUILD_MODE="release";
if [ "$1" = "dbg" ]; then
  BUILD_MODE="debug";
fi

# Copied from .codecrafters/compile.sh
#
# - Edit this to change how your program compiles locally
# - Edit .codecrafters/compile.sh to change how your program compiles remotely
(
  cd "$(dirname "$0")" # Ensure compile steps are run within the repository directory
  if [ "$BUILD_MODE" = "debug" ]; then
    cargo build --target-dir=/tmp/codecrafters-build-shell-rust --manifest-path Cargo.toml
  else
    cargo build --release --target-dir=/tmp/codecrafters-build-shell-rust --manifest-path Cargo.toml
  fi
)

# Copied from .codecrafters/run.sh
#
# - Edit this to change how your program runs locally
# - Edit .codecrafters/run.sh to change how your program runs remotely
exec /tmp/codecrafters-build-shell-rust/"$BUILD_MODE"/codecrafters-shell "$@"
