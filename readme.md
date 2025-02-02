# Pinger

(below readme + bulk of the code is generated using cursor, mostly as an experiment in using cursor to generate code)

A lightweight macOS system tray application that monitors network latency in real-time. Perfect for digital nomads, remote workers, and anyone who needs to keep an eye on their connection quality.

## Features

- 🎯 Real-time latency monitoring
  - Updates every second
  - Monitors connection to Google DNS
  - Visual status in system tray

- 🚦 Visual Status Indicators
  - 🟢 Green: Excellent latency (<30ms)
  - 🟡 Yellow: Moderate latency (30-100ms)
  - 🔴 Red: High latency (>100ms)
  - ❌ Red X: Connection failed

- 📈 Live Sparkline Graph
  - Shows last 10 measurements
  - Auto-scaling visualization
  - Failed pings marked with ✖
  - Historical view of connection stability

- ✈️ Airplane Mode
  - Toggle for high-latency environments
  - Normal thresholds: 30ms/100ms
  - Airplane thresholds: 600ms/1000ms
  - Perfect for satellite internet or in-flight WiFi

## Installation

1. Download the latest DMG from the [releases page](https://github.com/bavardage/pinger/releases)
2. Open the DMG file
3. Drag Pinger to your Applications folder
4. Launch Pinger from Applications
5. (Optional) Add to Login Items to start automatically

## Usage

1. Look for the circle icon in your menu bar (system tray)
2. Click the icon to see:
   - Recent latency measurements
   - Sparkline graph visualization
   - Airplane mode toggle
3. Hover over the icon for a quick status tooltip
4. Use Airplane Mode when on high-latency connections
5. Click "Quit" to exit the application

## Building from Source

### Requirements
- Rust and Cargo
- macOS
- cargo-bundle (`cargo install cargo-bundle`)
- create-dmg (`brew install create-dmg`)

