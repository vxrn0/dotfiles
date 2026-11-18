# Roadmap

Decisions, not ideas. Zones are in `AGENTS.md`.

## Decided, waiting for a usable connection

| # | what | size | note |
|---|------|------|------|
| 1 | Hex as the dictation app | 90 MB binary | Replaces Handy. Native Rust, no WebKit. |
| 2 | Parakeet TDT 0.6B v3 for it | 705 MB | Russian and English with automatic language detection. Non-autoregressive TDT, the fastest multilingual entry in the catalog and the smallest multilingual one. Same file that was half-downloaded before, so nothing is guessed. |
| 3 | Zen from the AUR instead of flatpak | -2.5 GB disk, -20 MB RAM | Copy the profile to `~/.zen`, keep the flatpak until the copy is verified. |

## Decided, can be done offline

| # | what | scope |
|---|------|-------|
| 4 | Rewrite the backend in Rust | 19 scripts under `.config/quickshell/dotfiles/scripts`, ~1.5k lines: 1,015 bash and 630 Python. Replaces the venv (`numpy`, `cv2`, `PIL`, `materialyoucolor`) with one binary. |

`kitty/search.py` stays Python: kitty only loads kittens written in Python.

## Decided against

| what | why |
|------|-----|
| Pleamar | Not now. Revisit only if the Rust backend feels insufficient. |
| Seasonal desktop | Dropped. |
| Rune for the shell | `set_exclusive_zone(0)` is hardcoded, so the bar cannot reserve space. |
| Amane | Author paused it until mid-December. |
| Cloud dictation (Groq) | Local model chosen. |

## Open

- Whether `webkit2gtk-4.1` (141 MB) goes. It was installed only for Handy and nothing requires it.
- Order of the Rust rewrite: colour pipeline first, then image work, then the small scripts.

## Notes on the model choice

Parakeet beats the alternatives on every axis that matters here: 705 MB against 886 MB for
Whisper large-v3-turbo, no autoregressive decoding (so no repeat loops or invented sentences), and
language detection built into the file, which means Russian and English need no setting to switch.

Hex falls back to the CPU by itself when a GPU runs out of memory or turns out slower than the CPU,
and it does not blacklist the driver for an out-of-memory error. On this machine the GPU has 7.4 GB
free of 8 GB, so the model runs on the GPU, with the CPU as the safety net.

Neither model streams, so Live typing shows no words while speaking; the finished text still lands
in the focused field through the input method instead of the clipboard.
