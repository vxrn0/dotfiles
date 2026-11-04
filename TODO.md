# TODO

Work on the Quickshell shell (`~/.config/quickshell/dotfiles`, mirrored into `extra/git/dotfiles`).

## Open

- [ ] Re-check blur and `QT_QPA_PLATFORMTHEME` after a full re-login: the current session's
      Qt apps still run with the old environment
- [ ] `opencode-symbolic.svg` in the bar renders as a white square: it is the actual logo, but it
      does not read at 20 px

## Done

- [x] Restored the 37 packages `pacman -Rns` removed with the illogical-impulse metapackages on
      2026-11-05: kitty, hyprland and the lock/idle/screenshot tools were only declared by those
      metapackages, so `-s` took them as unneeded. `extras/packages-explicit.txt` now lists every
      one of them, so a fresh install no longer depends on those metapackages

- [x] The engine is the Codex CLI, not `gray`. `scripts/ai/codex-run.py` runs
      `codex exec --json` (or `resume <id>`) and re-emits the event shape the panel and the bridge
      already parsed, so both only swapped which binary they launch. Models come from
      `~/.codex/models_cache.json`, sessions from `~/.codex/sessions`, so `/model` and `/sessions`
      list what this account can actually run. Both sides are verified end to end; Codex uses the
      ChatGPT login the CLI already had, so no key is stored anywhere
- [x] `/sandbox`: `read-only`, `workspace` (default) or `full`, passed
      straight to Codex. The panel shows the mode under the header and turns it red for `full`
- [x] `ipc call aiChat ask "<text>"` and `ipc call aiChat stop` drive the panel from a keybind or
      a script without typing into it

- [x] Pilot can stop a running task and switch between saved chats. `/stop` signals the whole
      process group, since the adapter runs under `setsid`, so the command a turn started dies
      with it; `/sessions` lists recent chats and resumes the chosen one, and `/new` clears it.
      The input stays enabled during a run so `/stop` can be typed

- [x] Pilot logo redrawn as a pixel mark in a 24x18 grid: horizontal and vertical steps only,
      wider than tall, eyes right of centre
- [x] The Pilot icon is sized with `width`/`height`, not `implicitWidth`/`implicitHeight`:
      `CustomIcon` sets explicit sizes, which take precedence, so the mark silently rendered at
      the widget default of 30 px and crowded the separator dot
- [x] Dot separator between the Pilot button and the harness badges, matching the tray's, and
      shown only while a harness is running
- [x] Click anywhere outside Pilot closes it. The panel is now one full-screen surface with a
      click catcher behind the card; a Hyprland focus grab was unusable because Hyprland
      clears it about two seconds after the panel opens, which also made the quick settings
      sidebar close itself

- [x] Pilot logo is a custom mark (`assets/icons/pilot-symbolic.svg`): a chamfered square with
      two vertical eyes looking right, drawn as one evenodd path so the eye holes render the
      same in Qt and every other renderer. The glyph sits in a 24-unit area inside a 28-unit
      viewBox, so it reads lighter than the harness icons without shrinking the eyes into a blob
- [x] Pilot no longer closes itself a second after opening: the panel asked for keyboard focus
      on demand, and Hyprland clears the focus grab when a layer surface takes focus, which
      counted as a dismissal. It now holds exclusive keyboard focus while open, so the grab
      survives and clicking outside still closes the panel

- [x] Command palette now filters as you type: `paletteQuery` tracks the field and
      `paletteItems` keeps only commands starting with it, so `/new` leaves one row
- [x] Region binds no longer screenshot the selector. The fallback guard was
      `pgrep -x qs`, which is false while the shell restarts, so `hyprshot` ran against the
      selector the shell had just opened. It is now `hyprland/scripts/region_guard.sh`,
      which polls for the `quickshell:regionSelector` layer for up to ~1 s instead of
      sleeping a fixed 150 ms (the layer took 4-104 ms to appear, so a fixed delay still
      raced). Covers screenshot, image search, OCR and record

- [x] Provider is NVIDIA NIM (`/setup nvidia KEY [MODEL]`), not Groq: the free Groq tier
      capped input at 8000 tokens per minute, which a multi-step agent turn spends within
      seconds, so tasks died with "context exhausted" after a few tool calls. NIM allows 40
      requests per minute with no token-per-minute cap
- [x] Pilot verified end to end on a real task: step timeline, tool detail, timestamps and
      the result card all render from live `gray --json` events
- [x] Pilot step rows read like a console transcript: status glyph, tool name, the command in
      monospace, its output preview, and a duration; gray's `<untrusted-output>` wrapper and
      `exit 0 · …` meta line are stripped. The result card ends with
      `worked for Ns, N tokens/s`, computed from the run's wall time and output tokens

- [x] Super+Shift+S could fire twice: the fallback bind was guarded by
      `qs ipc call TEST_ALIVE`, which exits 0 even when the call fails, so `hyprshot` ran
      while the region selector was already open and screenshotted it. The guard is now
      `pgrep -x qs`, which is deterministic
- [x] Media pill in the bar is hidden entirely (with its separator) while no player exists
- [x] AI panel renamed to Pilot, with a custom `pilot-symbolic.svg` mark replacing the
      default `auto_awesome` sparkle (header, empty state, bar button)
- [x] `/setup` feedback was invisible: its output went to the status line, which lives
      inside the activity area that is hidden when no task has run. Output now goes to the
      result card, and the help text avoids angle brackets, which MarkdownText ate as HTML
- [x] SUPER+Tab toggles Pilot: closed panel opens with a fresh chat, open panel closes.
      Skipped while a fullscreen window is focused, so the application keeps its own Tab
- [x] Pilot dismisses on a click outside: one full-screen surface with a click catcher behind
      the card. A Hyprland focus grab was unusable because Hyprland clears it about two seconds
      after the panel opens, which also closed the quick settings sidebar
- [x] Wallpaper picker lists videos (mp4, webm, mkv, avi, mov) and shows a poster frame
      extracted with ffmpeg; `switchwall.sh` already hands them to mpvpaper
- [x] AI panel rebuilt around the agent's `--json` stream: task card, live step timeline with
      per-step timestamps, result card, slash commands (`/model`, `/effort`, `/new`, `/sessions`,
      `/sandbox`, `/stop`), and a command palette with keyboard navigation
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
- [x] AI panel anchored left with the same slide animation as the quick settings panel
      (`slide left`)
- [x] AI panel rewritten from scratch: header, harness/effort/model chips, transcript, input row
- [x] Bar buttons (AI, sidebar toggle) have no background in any state and no ripple
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
