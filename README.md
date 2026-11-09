# hyprswitch_rust

A high-performance native **Rust + GTK4** on-screen display (OSD) projection switcher for Hyprland that replaces manual editing of `hyprland.lua`. Designed to be triggered with **`Super + P`** (matching the Windows projection flyout workflow), it provides instantaneous (<5ms) multi-instance cycling across standard display modes with zero screen flickering, auto-detects external monitors' optimal resolutions and refresh rates, and features a frosted-glass **Caelestia shell aesthetic**.

---

## Features

- **Blazing Fast Native Performance**:
  - Compiled self-contained Rust binary with instant sub-millisecond startup time.
  - No Python runtime overhead, no virtual environments, and no `LD_PRELOAD` linking workarounds.
- **Caelestia Shell Aesthetic & Material 3 Styling**:
  - Dynamically extracts Material 3 color tokens from `~/.local/state/caelestia/scheme.json` (fallback to `~/.config/hypr/scheme/current.lua`).
  - Frosted translucent glass background (`alpha(#surfaceContainer, 0.72)`) with deep drop shadows and smooth 32px corner rounding.
  - Hyprland layer-shell blur (`layerrule = blur true, match:namespace hyprswitch.*` and `ignorealpha 0.2`) to blur wallpaper behind the switcher window.
  - Active/selected mode lights up with a vibrant neon primary glowing border (`#primary` / `#f5b6c3`) and tinted container fill.
  - Status display and footer controls styled in Caelestia pill chips (`border-radius: 9999px`).
- **Auto-Detection of External Resolutions & Refresh Rates**:
  - Automatically queries connected monitors and their supported modes from `hyprctl -j monitors all`.
  - Intelligently distinguishes built-in panel (`eDP-1`, `LVDS`, `DSI`) from external monitors (`HDMI-A-1`, `DP-1`, etc.).
  - Selects the highest supported native resolution and peak refresh rate (e.g. `1920x1080@144Hz`, `2560x1440@165Hz`).
- **4 Display Projection Modes**:
  1. **PC Screen Only**: Built-in monitor active; external monitor disabled.
  2. **Duplicate**: Mirror built-in display onto external monitor at preferred aspect ratio.
  3. **Extend**: Dual-screen desktop spans across displays side-by-side at optimal resolutions and refresh rates.
  4. **Second Screen Only**: External monitor active; built-in display disabled (with auto-failsafe if no external screen is present).
- **Fast Single-Instance IPC Cycling & Enter-to-Select**:
  - Uses `gtk4::gio::Application` single-instance IPC: pressing `Super + P` opens the OSD highlighting the current mode; repeated presses cycle forward instantly.
  - **Zero Screen Flicker**: No monitor reconfiguration is triggered while cycling through choices.
  - Press **`Enter`** (or **`Space`**, or click any card) to commit and apply immediately.
  - Press **`Escape`** to dismiss without making changes.
- **Hyprland Lua Integration & Wildcard Fallbacks**:
  - Applies settings live and atomically via `hyprctl eval "hl.monitor(...)"`.
  - Persists configuration cleanly to `~/.config/hypr/monitors.lua`.
  - Uses Hyprland's `output = ""` wildcard fallback rules so Duplicate, Extend, and Second Screen modes configure any external screen whether plugged in now or later.

---

## Installation & Setup

Clone the repository and run the installer:
```bash
./install
```
*(Alternatively: `./install.sh`)*

The installation script will:
1. Verify required build dependencies (`cargo`, `gtk4`, `gtk4-layer-shell`, `hyprctl`).
2. Build an optimized release binary (`cargo build --release`).
3. Symlink `target/release/hyprswitch_rust` into `~/.local/bin/hyprswitch_rust`.
4. Add `pcall(require, "monitors")` to `~/.config/hypr/hyprland.lua`.
5. Add the `SUPER + P` keybind (`hl.bind("SUPER +  P", hl.dsp.exec_cmd("hyprswitch_rust"))`) to `~/.config/hypr/hyprland/keybinds.lua`.
6. Add layer rules (blur, popin animation, ignorealpha) to `~/.config/hypr/hyprland/rules.conf`.
7. Reload the active Hyprland configuration automatically.

---

## Controls

| Key / Action | Description |
| --- | --- |
| `Super + P` | Open switcher OSD / Cycle forward to next mode |
| `Right` / `Down` / `Tab` | Cycle forward |
| `Left` / `Up` / `Shift + Tab` | Cycle backward |
| `Enter` / `Space` | Apply highlighted mode immediately and close |
| `Escape` | Cancel and dismiss without changes |
| Mouse Click | Click any card to apply immediately and close |

---

## CLI Usage

`hyprswitch_rust` can also be queried or scripted directly from the terminal without opening the UI:

```bash
# Open switcher OSD / cycle mode (called by Super + P)
hyprswitch_rust

# Check monitor auto-detection and current active mode (<5ms)
hyprswitch_rust --status

# Apply a mode directly via CLI without UI
hyprswitch_rust --apply pc
hyprswitch_rust --apply duplicate
hyprswitch_rust --apply extend
hyprswitch_rust --apply second
```

---

## Build Dependencies

- **Rust & Cargo** (`rust >= 1.85`)
- **GTK4** (`gtk4 >= 4.0`)
- **GTK4 Layer Shell** (`gtk4-layer-shell >= 1.0`)
- **Hyprland** (`hyprctl`)

On Arch Linux:
```bash
sudo pacman -S rust cargo gtk4 gtk4-layer-shell
```

---

## Running Tests

Run the test suite:
```bash
cargo test
```
