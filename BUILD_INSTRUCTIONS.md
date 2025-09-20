# Build Instructions for MacOS Key Sound

## Prerequisites

- macOS (required for building macOS app)
- Rust and Cargo (installed via rustup)
- cargo-bundle (for app packaging)
- Node.js and npm (for DMG creation)

## Building the Application

### 1. Install Dependencies

```bash
# Source Rust environment
source "$HOME/.cargo/env"

# Install cargo-bundle
cargo install cargo-bundle

# Install create-dmg for DMG packaging
npm install -g create-dmg
```

### 2. Build and Run Locally

```bash
# Build in debug mode
cargo build

# Build in release mode (optimized)
cargo build --release

# Run the application
cargo run --release
```

### 3. Create macOS App Bundle

```bash
# Package as .app
cargo bundle --release

# The app will be created at:
# target/release/bundle/osx/MacOS Key Sound.app
```

### 4. Create DMG Installer

```bash
# Create DMG file
create-dmg 'target/release/bundle/osx/MacOS Key Sound.app' \
  --overwrite \
  --dmg-title="MacOS Key Sound" \
  dist/

# The DMG will be created in the dist/ directory
```

## Important Notes

### Accessibility Permissions

On macOS, the application requires Accessibility permissions to monitor keyboard events:

1. Run the application
2. macOS will prompt for Accessibility permissions
3. Go to System Preferences → Security & Privacy → Privacy → Accessibility
4. Add your application to the allowed list

### Sound File

The application looks for `assets/sound.mp3` file. Make sure this file exists:

- The build process includes a default sound file
- You can replace it with any WAV, MP3, or OGG file
- Keep the filename as `sound.mp3` or modify the code accordingly

### Running the App

- The application runs in the background
- Press any key to hear the sound effect
- Press Ctrl+C in the terminal to quit (when run via `cargo run`)
- When run as a packaged app, it will run until manually quit

## Troubleshooting

### "Permission denied" or "Input monitoring" errors

- Grant Accessibility permissions in System Preferences
- Restart the application after granting permissions

### "Sound file not found" errors

- Check that `assets/sound.mp3` exists
- Verify the sound file format is supported (WAV, MP3, OGG)

### Build errors

- Ensure you have the latest Rust version: `rustup update`
- Clean and rebuild: `cargo clean && cargo build`

## Distribution

The final DMG file can be distributed to users who can:

1. Download and mount the DMG
2. Drag the app to Applications folder
3. Run the app and grant Accessibility permissions when prompted
