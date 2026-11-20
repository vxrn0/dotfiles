#!/usr/bin/env bash

XDG_CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}"
XDG_STATE_HOME="${XDG_STATE_HOME:-$HOME/.local/state}"
STATE_DIR="$XDG_STATE_HOME/quickshell"

apply_term() {
  # matugen renders this file from the wallpaper palette; foot understands OSC 4/10/11
  local sequences="$STATE_DIR/user/generated/terminal/sequences.txt"
  if [ ! -f "$sequences" ]; then
    echo "Terminal color sequences not found. Skipping that."
    return
  fi
  for file in /dev/pts/*; do
    if [[ $file =~ ^/dev/pts/[0-9]+$ ]]; then
      {
      cat "$sequences" >"$file"
      } & disown || true
    fi
  done
}

# Check if terminal theming is enabled in config
CONFIG_FILE="$XDG_CONFIG_HOME/dotfiles/config.json"
if [ -f "$CONFIG_FILE" ]; then
  enable_terminal=$(jq -r '.appearance.wallpaperTheming.enableTerminal' "$CONFIG_FILE")
  if [ "$enable_terminal" = "true" ]; then
    apply_term &
  fi
else
  echo "Config file not found at $CONFIG_FILE. Applying terminal theming by default."
  apply_term &
fi
