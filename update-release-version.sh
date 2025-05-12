#!/usr/bin/env zsh

set -eu

version=$1

# Swift
sed -i "" -E "s/(let releaseTag = \")[^\"]+(\")/\1$version\2/g" Package.swift

# Android
sed -i "" -E "s/(version = \")[^\"]+(\")/\1$version\2/g" android/build.gradle

# Rust
awk '{ if (!done && /version = \"/) { sub(/(version = \")[^\"]+(\")/, "version = \"" newVersion "\""); done=1 } print }' newVersion="$version" rust/unimusic-sync/Cargo.toml >tmpfile && mv tmpfile rust/unimusic-sync/Cargo.toml
cd rust && cargo check && cd ..

git add Package.swift \
    android/build.gradle \
    rust/Cargo.lock rust/unimusic-sync/Cargo.toml
