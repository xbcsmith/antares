# Campaign Builder Prototype - Quick Start

Get the Antares Campaign Builder prototype running in under 5 minutes.

## 1. Prerequisites

```bash
# Check Rust is installed
rustup --version

# If not installed, get Rust from https://rustup.rs/
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Linux Users - Install OpenGL Libraries

```bash
# Ubuntu/Debian
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev

# Fedora/RHEL
sudo dnf install libxcb-devel libxkbcommon-devel

# Arch Linux
sudo pacman -S libxcb libxkbcommon
```

## 2. Build & Run

```bash
# Navigate to antares root directory
cd antares/

# Run the prototype (will build automatically)
cargo run --bin campaign-builder
```

That's it! The Campaign Builder UI should launch.

## 3. Try These Features

### Create a New Campaign

1. Click **File → New Campaign**
2. Fill in the **Metadata** tab:
   - **Campaign ID**: `my_test_campaign`
   - **Name**: `My First Campaign`
   - **Version**: `1.0.0`
   - **Author**: Your name
   - **Description**: Any text
3. Click **Tools → Validate Campaign**
4. Check the **✅ Validation** tab for results

### Explore the UI

- Click different tabs in the left sidebar
- Try **File → Save As...** to pick a save location
- Open the **Help → About** dialog
- Notice the status bar at the bottom
- Make changes and see "● Unsaved changes" indicator

## 4. Testing Without GPU

### Linux - Software Rendering

```bash
# Force CPU-only rendering
LIBGL_ALWAYS_SOFTWARE=1 cargo run --bin campaign-builder

# Run in virtual framebuffer (headless)
xvfb-run cargo run --bin campaign-builder

# Check which backend is being used
RUST_LOG=eframe=debug cargo run --bin campaign-builder 2>&1 | grep backend
```

## 5. Expected Performance

| Your Hardware        | Expected FPS | Experience |
|---------------------|--------------|------------|
| Desktop with GPU    | 60           | Smooth ✨  |
| Laptop (integrated) | 60           | Smooth ✨  |
| VM without GPU      | 30-60        | Usable ✓   |
| Software rendering  | 30-40        | Acceptable |

## Troubleshooting

### Build Errors

```bash
# Clean and rebuild
cargo clean
cargo build --package campaign_builder
```

### "Cannot find package" Error

Make sure you're in the `antares/` root directory, not `sdk/campaign_builder/`.

### OpenGL/Graphics Errors on Linux

```bash
# Install Mesa drivers
sudo apt-get install mesa-utils libgl1-mesa-dri

# Test OpenGL
glxinfo | grep "OpenGL version"
```

### Black Screen or Doesn't Start

```bash
# Try with logging
RUST_LOG=info cargo run --bin campaign-builder

# Force software rendering
LIBGL_ALWAYS_SOFTWARE=1 cargo run --bin campaign-builder
```

## What This Prototype Demonstrates

✅ **Framework Validation** - egui works perfectly for Antares SDK
✅ **No GPU Required** - Runs with software rendering
✅ **Pure Rust** - Integrates seamlessly with Antares
✅ **Key UI Patterns** - Menus, tabs, forms, validation, file dialogs
✅ **Ready for Full SDK** - Foundation proven and tested

## Next Steps

- Read [README.md](./README.md) for detailed documentation
- See [SDK Architecture](../../docs/explanation/sdk_and_campaign_architecture.md) for the full plan
- Check [implementations.md](../../docs/explanation/implementations.md) for project status

## Need Help?

- Read the full README: `sdk/campaign_builder/README.md`
- Check architecture docs: `docs/explanation/sdk_and_campaign_architecture.md`
- Review AGENTS.md: `AGENTS.md`

---

**Ready to build campaigns for Antares!** 🚀
