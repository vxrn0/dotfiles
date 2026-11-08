# TODO

Work on the Quickshell shell (`~/.config/quickshell/dotfiles`, mirrored into `extra/git/dotfiles`).

## Open

- [ ] Re-check blur and `QT_QPA_PLATFORMTHEME` after a full re-login: the current session's
      Qt apps still run with the old environment
- [ ] `opencode-symbolic.svg` in the bar renders as a white square: it is the actual logo, but it
      does not read at 20 px

## Done

- [x] Pilot panel removed entirely: the bar button, SUPER+O and SUPER+Tab, the `aiChat` module,
      its IPC handlers, the `ai` config block and the `scripts/ai` adapters are gone. Working
      directly in the harness terminal is more convenient; the live harness badges stay.

- [x] Restored the 37 packages `pacman -Rns` removed with the illogical-impulse metapackages on
      2026-11-05: kitty, hyprland and the lock/idle/screenshot tools were only declared by those
      metapackages, so `-s` took them as unneeded. `extras/packages-explicit.txt` now lists every
      one of them, so a fresh install no longer depends on those metapackages

- [x] Region binds no longer screenshot the selector. The fallback guard was
      `pgrep -x qs`, which is false while the shell restarts, so `hyprshot` ran against the
      selector the shell had just opened. It is now `hyprland/scripts/region_guard.sh`,
      which polls for the `quickshell:regionSelector` layer for up to ~1 s instead of
      sleeping a fixed 150 ms (the layer took 4-104 ms to appear, so a fixed delay still
      raced). Covers screenshot, image search, OCR and record

- [x] Super+Shift+S could fire twice: the fallback bind was guarded by
      `qs ipc call TEST_ALIVE`, which exits 0 even when the call fails, so `hyprshot` ran
      while the region selector was already open and screenshotted it. The guard is now
      `pgrep -x qs`, which is deterministic
- [x] Media pill in the bar is hidden entirely (with its separator) while no player exists
- [x] Wallpaper picker lists videos (mp4, webm, mkv, avi, mov) and shows a poster frame
      extracted with ffmpeg; `switchwall.sh` already hands them to mpvpaper
- [x] Clipboard history is pruned on a three-day rolling window
      (`scripts/clipboard/prune.sh` + `cliphist-prune.timer`, runs at 04:00 every third day).
      cliphist keeps no timestamps, so the script stores the newest entry id and deletes
      everything up to it on the next run
- [x] Overview: per-entry buttons restored for clipboard results (copy, delete). The earlier
      cleanup removed them along with the duplicate gear icons; the model is now filtered to
      built-in actions only
- [x] Wallpaper picker: folder tiles were drawn as a magenta checkerboard. Cause: `DirectoryIcon`
      used `Quickshell.iconPath`, which cannot resolve folder icons here (log:
      `Could not load icon "folder"`). Replaced with a Material Symbols glyph and deleted the
      now-unused `DirectoryIcon.qml`
- [x] Wallpaper picker opened in a non-existent `~/Pictures/Wallpapers`. It now starts in the
      folder of the current wallpaper and falls back to `~/Pictures`
- [x] Wallpaper picker: the thumbnail in quick settings respects
      `wallpaperSelector.useSystemFileDialog`. The config sets it to `false` so the built-in
      picker always opens instead of the foreign-looking GTK dialog
- [x] Blur removed: `decoration:blur:enabled = false`, every `blur = true` layer rule dropped,
      dead `StyledBlurEffect.qml` deleted
- [x] Colour picker works from the panel: the delay lived in a component that gets destroyed, so
      it moved into the detached process. Zoom is back and the notification is gone
- [x] Overview: action buttons to the right of app icons removed
- [x] Apps showing a light theme: `xdg-desktop-portal-gtk` failed without `WAYLAND_DISPLAY` in the
      systemd environment, so the Settings portal never answered. Now
      `dbus-update-activation-environment --systemd --all` runs at startup and the portal reports
      `prefer-dark`; `QT_QPA_PLATFORMTHEME=kde` was added for Qt apps
- [x] Audit: dead code, duplicates, crashes
- [x] Material Symbols Filled icons (static font instance)
- [x] Harness icons blinking
- [x] Tooltips disabled shell-wide
- [x] The shell runs on the distro `quickshell` package, not the patched build it shipped with
- [x] No trace of the shell this config started from: the name is gone from the repo, the docs,
      the backup tree (moved to `~/.local/share/dotfiles-backups`) and the stray icon
- [x] Brightness and volume sliders in the quick settings panel
- [x] Media player: bar only, middle click toggles playback
- [x] Clock popup: Todo and uptime removed
