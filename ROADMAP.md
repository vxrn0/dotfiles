# Roadmap

Decisions, not ideas. Zones are in `AGENTS.md`.

## Decided, waiting for a usable connection

| # | what | size | note |
|---|------|------|------|
| 1 | Hex as the dictation app | 90 MB binary | Replaces Handy. Native Rust, no WebKit. |
| 2 | A Russian + English model for it | 469-886 MB | See the open question below. |
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

- Which model ships with Hex. Its catalog has no Whisper Medium: the whisper entry is
  `whisper-large-v3-turbo` at 886 MB, and Parakeet v3 is 705 MB. A custom Medium GGUF (469 MB)
  can be imported through "Add model file…", which accepts `general.architecture whisper` with
  Q8_0 tensors, but that path is untested here.
- Whether `webkit2gtk-4.1` (141 MB) goes. It was installed only for Handy and nothing requires it.
- Order of the Rust rewrite: colour pipeline first, then image work, then the small scripts.
