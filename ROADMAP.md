# Roadmap

Decisions, not ideas. Zones are in `AGENTS.md`.

## Decided, waiting for a usable connection

| # | what | size | note |
|---|------|------|------|
| 1 | Hex as the dictation app | 90 MB binary | Replaces Handy. Native Rust, no WebKit. |
| 2 | Nemotron 3.5 ASR for it | 716 MB | Russian and English plus 25 more, and the only multilingual entry that streams. Words appear while you speak instead of after release. Trained on FLEURS, ~25x real time on the GPU. |
| 3 | Zen from the AUR instead of flatpak | -2.5 GB disk, -20 MB RAM | Copy the profile to `~/.zen`, keep the flatpak until the copy is verified. |

## Decided, can be done offline

| # | what | scope |
|---|------|-------|
| 4 | Rewrite the backend in Rust | 19 scripts under `.config/quickshell/dotfiles/scripts`, ~1.5k lines: 1,015 bash and 630 Python. Replaces the venv (`numpy`, `cv2`, `PIL`, `materialyoucolor`) with one binary. |

`kitty/search.py` stays Python: kitty only loads kittens written in Python.

## Decided against

| what | why |
|------|-----|
| Rune for the shell | `set_exclusive_zone(0)` is hardcoded, so the bar cannot reserve space. |
| Amane | Author paused it until mid-December. |
| Cloud dictation (Groq) | Local model chosen. |

## Parked, not now

| what | what it is, and what it would take |
|------|------------------------------------|
| Pleamar | Rust shell with its own `.plm` language. It passes where Rune failed: `reserve:` is a real exclusive zone, `kind: lock` uses ext-session-lock, and it has network, bluetooth, audio, media, tray, notifications, workspaces and IPC. Missing against this shell: clipboard history, brightness, idle, night light, region capture, polkit, OSD, wallpaper and matugen. And there is no lazy loading at all, so every surface is built at startup — under the current 14 always-on panels that would raise memory, not lower it. First step would be one bar in a separate process, measured, before anything else. |
| Seasonal desktop | Wallpaper scene that reacts to the machine: snow builds while the CPU is cool and melts when it heats up. Cheap by design, since swapping `wallpaper.path` is live and needs no shader. Parked by choice, not dropped. |

## Rust backend plan

One binary per concern, built in `extras/rust/`, installed to `~/.local/bin`. The bash and Python
stay in place and keep working until the replacement is proven against them; a wrapper decides which
one runs.

| phase | replaces | size | risk |
|-------|----------|------|------|
| 1 | `colors/switchwall.sh` orchestration (matugen calls, monitors, video wallpaper, state files, thumbnails) | 514 lines bash | medium: touches the theme, but only through the same commands |
| 2a | `generate_colors_material.py --color` (Material You scheme from a colour) | done | 612/612 cases match the Python reference byte for byte. 0.9 ms against 90 ms. |
| 2b | `generate_colors_material.py --path` and `scheme_for_image.py` (scheme from an image) | done | 705/705 cases match byte for byte, including 8 real photos, greyscale, 1x1, GIF and a non-image. Links `libturbojpeg` because zune-jpeg decodes up to 8 levels off Pillow, which moves the seed. |
| 2c | switch `switchwall.sh` and `dots-theme` over to `dots-colors` | outside this zone | the venv can only go after this. |
| 3 | `images/find_regions.py` (screenshot region detection) | done | 314 lines Rust, 33-54 ms at 1080p. **No reference existed**: the script needs OpenCV, absent from the venv, so region suggestions had been failing silently. Built rather than ported and verified structurally: 5 synthetic rectangles at IoU >= 0.7, blank image returns empty, bounds and limits on real photos. |
| 4 | `thumbnails/thumbgen.py` (GNOME thumbnailer wrapper) | 122 lines Python | low, but it depends on `gi`, not on the venv |
| 5 | small scripts: `record.sh`, `prune.sh`, `keyring/*`, `applycolor.sh` | ~260 lines | low, one at a time |

## Two features that were dead before the rewrite started

Both fail in the shipped script, which is why "it worked before" is not a safe assumption:

- **Automatic scheme picking** (`colors/scheme_for_image.py`) needs `cv2`, which is not in the venv.
  `switchwall.sh` pipes it through `2>/dev/null`, gets an empty string, and falls back to the
  configured `scheme-tonal-spot`. So `auto` never picked anything. Phase 2b fixes this.
- **Region suggestions** in the screenshot selector (`images/find_regions.py`) needs the same missing
  `cv2`. The caller wraps the parse in `try`/`catch` and ends up with an empty list, so the feature
  silently never suggested anything. Phase 3 builds it rather than porting it.

Neither was noticed because both failures were swallowed: one by `2>/dev/null`, the other by a
`catch` that assigned an empty array.

Known gaps in 2b, all outside the gate because the Python reference is itself broken there: CMYK
JPEG, 16-bit PNG and greyscale PNG with alpha. The reference reads `pixels[i][2]` out of bounds on
two-element vectors, so its accent colour changes between runs; the Rust side refuses the file
instead of guessing.

Order matters: phase 1 removes the most fragile code (bash quoting, silent failures) without needing
new colour science, and phases 2a-2c are what let the venv disappear. `kitty/search.py` is out of scope
for good: kitty runs only Python kittens.

Acceptance for every phase: run the old and the new implementation over the same inputs in a sandbox
with `HOME` and `XDG_*` redirected to a temporary tree, then diff the results. Nothing ships on a
prose description of "it should work". A comparison harness that skips an axis because it found no
input must exit non-zero; a skipped axis is not a pass.

## Kitty gets its colours from two places

`~/.config/kitty/colors.conf` is a matugen template: muted colours derived from the wallpaper.
`~/.local/state/quickshell/user/generated/terminal/kitty-theme.conf` is produced by
`generate_colors_material.py` plus `applycolor.sh`: vivid colours harmonised from a catppuccin base,
pushed into running instances with `kitten @ set-colors`. The second wins at runtime, so the terminal
does not follow the wallpaper palette. That is a design choice nobody has confirmed; phase 2a does
not change it, it only replaces the implementation.

## Open

- Order of the Rust rewrite: theme driver first, then the colour science, then image work, then the small scripts.
- Live typing on Wayland is documented as unverified on Hyprland; only KWin and Mutter were tried.
  If the preedit misbehaves, use Live text (a HUD beside the cursor) or plain paste.

## Notes on the model choice

Nemotron 3.5 ASR streams: it publishes partial text while the shortcut is held, so words show up as
you speak instead of after release. It covers ru and en (27 languages), takes a language hint or
detects one, and runs at roughly 25x real time on the GPU.

The cost against Parakeet v3 is accuracy: 3.05% word error rate on LibriSpeech test-clean against
1.95%, and 25x real time against 33x. Both are far beyond what dictation needs, and streaming is
worth more than two points of WER on short utterances.

Streaming only happens on a GPU, after the load probe succeeds; on the CPU Hex transcribes the whole
clip and inserts it at the end. Hex switches to the CPU by itself when a GPU runs out of memory or
turns out slower, and it does not blacklist the driver for an out-of-memory error. This machine has
7.4 GB of 8 GB VRAM free.

Chunking the audio by hand and calling the model after every word was considered and rejected: it
cuts words at chunk edges, drops the context that decides punctuation and homophones, and cannot
revise text already inserted. Hex's streaming path keeps one continuous stream instead.
