# ClickClack - Mechanical Keyboard Sound Simulator

ClickClack is a macOS application that simulates the satisfying sound of a mechanical keyboard while you type. It runs in your menu bar and provides real-time sound synthesis for each keypress.

## Features

- Real-time mechanical keyboard sound synthesis
- Customizable sound parameters:
  - Volume control
  - Click frequency adjustment
  - Sound decay rate
- Minimal system tray interface
- Low latency audio playback
- Privacy-focused (does not log or store keystrokes)

## Installation

### Prerequisites

- macOS 10.15 or later
- Rust toolchain (if building from source)

### Building from Source

1. Clone the repository:
```bash
git clone https://github.com/yourusername/clickclack.git
cd clickclack
```

2. Build the release version:
```bash
cargo build --release
```

3. The binary will be available at `target/release/clickclack`

### Running

Simply run the binary:
```bash
./target/release/clickclack
```

The application will appear in your menu bar as a keyboard icon.

## Usage

1. Click the keyboard icon in the menu bar to access settings
2. Use the menu to:
   - Enable/disable the click sound
   - Adjust volume (25%, 50%, 75%, 100%)
   - Change click frequency (Low, Medium, High)
   - Modify decay rate (Fast, Medium, Slow)
3. Type normally and enjoy the mechanical keyboard sounds!

## Sound Customization

The application synthesizes a realistic mechanical keyboard sound using:
- Base click frequency (adjustable)
- Harmonic overtones for mechanical character
- Subtle noise component for authenticity
- Customizable decay rate

## Privacy

ClickClack only detects key press events to trigger sound playback. It does not:
- Log or store any keystrokes
- Track which keys are pressed
- Send any data over the network

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. 