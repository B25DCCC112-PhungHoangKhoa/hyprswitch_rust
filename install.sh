#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"
TARGET_LINK="$BIN_DIR/hyprswitch_rust"

echo "=== Installing hyprswitch_rust (Rust Edition) ==="

# 1. Dependency checks
echo "Checking dependencies..."
MISSING_DEPS=()

if ! command -v cargo &>/dev/null; then
    MISSING_DEPS+=("cargo / rust")
fi

if ! command -v hyprctl &>/dev/null; then
    echo "Notice: 'hyprctl' command not found. Hyprland may not be running or not in PATH."
fi

if [ ! -f "/usr/lib/libgtk4-layer-shell.so" ] && [ ! -f "/usr/local/lib/libgtk4-layer-shell.so" ] && [ ! -f "/usr/lib64/libgtk4-layer-shell.so" ]; then
    echo "Warning: libgtk4-layer-shell.so was not found in standard library paths."
fi

if [ ${#MISSING_DEPS[@]} -gt 0 ]; then
    echo "Error: Missing required build dependencies: ${MISSING_DEPS[*]}"
    echo "Please install them via your package manager (e.g. pacman -S rust cargo gtk4 gtk4-layer-shell)."
    exit 1
fi

echo "✓ Build dependencies found."

# 2. Build release binary
echo "Building release binary with Cargo..."
cd "$SCRIPT_DIR"
cargo build --release

RELEASE_BIN="$SCRIPT_DIR/target/release/hyprswitch_rust"
if [ ! -f "$RELEASE_BIN" ]; then
    echo "Error: Build failed; binary not found at $RELEASE_BIN"
    exit 1
fi

# 3. Symlink binary
mkdir -p "$BIN_DIR"
ln -sf "$RELEASE_BIN" "$TARGET_LINK"
chmod +x "$TARGET_LINK"
echo "✓ Created symlink: $TARGET_LINK -> $RELEASE_BIN"

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "Notice: $BIN_DIR is not in your current PATH. Consider adding it to your ~/.bashrc or ~/.zshrc."
fi

# 4. Setup monitors.lua in hyprland.lua if not present
HYPR_LUA="$HOME/.config/hypr/hyprland.lua"
if [ -f "$HYPR_LUA" ]; then
    if ! grep -q 'monitors' "$HYPR_LUA"; then
        echo "Adding 'pcall(require, \"monitors\")' to $HYPR_LUA..."
        echo -e '\n-- Automatically load display mode managed by hyprswitch\npcall(require, "monitors")' >> "$HYPR_LUA"
        echo "✓ Updated $HYPR_LUA"
    else
        echo "✓ $HYPR_LUA already references monitors configuration."
    fi
fi

# 5. Setup keybind in ~/.config/hypr/hyprland/keybinds.lua
KEYBINDS_LUA="$HOME/.config/hypr/hyprland/keybinds.lua"
if [ -f "$KEYBINDS_LUA" ]; then
    if grep -q 'hl\.dsp\.exec_cmd("hyprswitch' "$KEYBINDS_LUA"; then
        sed -i 's/hl\.bind("SUPER.*", hl\.dsp\.exec_cmd("hyprswitch[^"]*"))/hl\.bind("SUPER +  P", hl\.dsp\.exec_cmd("hyprswitch_rust"))/g' "$KEYBINDS_LUA"
        echo "✓ Updated keybind to run 'hyprswitch_rust' in $KEYBINDS_LUA"
    else
        echo "Adding Super+P keybind to $KEYBINDS_LUA..."
        echo -e '\n-- Display switcher (hyprswitch_rust)\nhl.bind("SUPER +  P", hl.dsp.exec_cmd("hyprswitch_rust"))' >> "$KEYBINDS_LUA"
        echo "✓ Added 'SUPER +  P' bind to $KEYBINDS_LUA"
    fi
fi

# 6. Setup layer rules in ~/.config/hypr/hyprland/rules.conf if present
RULES_CONF="$HOME/.config/hypr/hyprland/rules.conf"
if [ -f "$RULES_CONF" ]; then
    if ! grep -q 'match:namespace hyprswitch' "$RULES_CONF"; then
        echo "Adding hyprswitch layer rules to $RULES_CONF..."
        echo -e '\n# hyprswitch\nlayerrule = animation popin 85%, match:namespace hyprswitch.*\nlayerrule = blur true, match:namespace hyprswitch.*\nlayerrule = ignorealpha 0.2, match:namespace hyprswitch.*' >> "$RULES_CONF"
        echo "✓ Added hyprswitch layer rules to $RULES_CONF"
    else
        sed -i 's/match:namespace hyprswitch$/match:namespace hyprswitch.*/g' "$RULES_CONF"
        echo "✓ Layer rules verified in $RULES_CONF"
    fi
fi

# 7. Reload Hyprland configuration if active
if command -v hyprctl &>/dev/null && [ -n "$HYPRLAND_INSTANCE_SIGNATURE" ]; then
    hyprctl reload config-only >/dev/null 2>&1 || true
    echo "✓ Reloaded Hyprland configuration."
fi

echo ""
echo "hyprswitch_rust installed successfully!"
echo "Run 'hyprswitch_rust --status' to inspect your monitors."
echo "Press Super + P to open the switcher and cycle display modes."
