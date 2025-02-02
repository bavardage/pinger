#!/bin/bash

# Build the release binary
cargo build --release

# Create the app bundle
cargo bundle --release

# Create DMG
create-dmg \
  --volname "Pinger Installer" \
  --window-pos 200 120 \
  --window-size 600 300 \
  --icon-size 100 \
  --icon "Pinger.app" 175 120 \
  --hide-extension "Pinger.app" \
  --app-drop-link 425 120 \
  "Pinger.dmg" \
  "target/release/bundle/osx/Pinger.app" 