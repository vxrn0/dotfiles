# dotfiles

Personal XDG config for a Hyprland + Quickshell desktop on Arch. The shell is called `dotfiles`.
Plain files. Derived from dots-hyprland; see `README.md` for credit and license.

Install: `git clone <repo> ~/dotfiles && ~/dotfiles/extras/install.sh`
(`--check` = preflight only, `--dry-run` = plan; `extras/verify.sh` checks the result).

## Zones

Three agents work here. Stay inside your zone.

- `main` — packages (`extras/`), Hyprland (`.config/hypr`), README, AGENTS, CI, install and verify.
- `quickshell` — the shell only: `.config/quickshell/dotfiles`. The largest area by far.
- `chore` — small cross-cutting fixes while the other two are busy: README cosmetics, CI, package
  list cleanups.

Every commit has the same author, so put the zone in the subject (`hypr:`, `pilot:`, `bar:`,
`packages:`, `docs:`). That is the only way to tell the three apart in `git log`.

## Layout

- `.config/` — the XDG config tree, mirrors `~/.config`.
- `extras/` — everything else a fresh install needs: `install.sh`, package lists, vendored
  fonts, the SDDM theme, `mango/`.
- `.config/hypr/hyprland/*.lua` — Hyprland config (Lua, not hyprlang). Tweaks go here, not into
  the header-only `custom/*.lua` hooks.
- `.config/quickshell/dotfiles/` — the shell. Upstream code; changes stay small and hand-merged.
- `.config/matugen/` — wallpaper color pipeline. Its outputs are not authored by hand.
- `extras/mango/` — MangoWM port, parked and not installed; see its README before touching it.
- `scripts/check.sh` — local CI: shell syntax, tracked secrets, README preview, `install.sh --help`.
  `tests/test_config.sh` is what it runs.

## Conventions

- Everything is written in English. Commits are authored as `vxrn <vxrn@tutamail.com>`.
- Never stage secrets or machine state: browser profiles, credentials, `fish_variables`, caches.
- Generated, do not hand-edit: `hyprland/colors.lua`, `hyprlock/colors.conf`, `kitty/colors.conf`,
  `gtk-3.0/gtk.css`, `gtk-4.0/gtk.css`. They are committed only as first-boot seeds.
- Terminal is kitty, file manager is Nautilus, editor is `~/.local/bin/monaco`, launcher and power
  menu come from the shell.
- No zsh, foot, konsole, fuzzel, wlogout, mpv, dolphin or Kvantum. Do not reintroduce them as
  fallbacks.
- The name `dotfiles` must stay in sync in `hl.env("qsConfig")`, `QUICKSHELL_CONFIG_NAME` in the
  color scripts, and the fish alias `q`.

## Extras that need a manual step

- The Codex CLI is the engine behind the Pilot panel; `scripts/ai/codex-run.py`
  translates its events. `monaco-editor` comes from `extras/install.sh`; the editor also needs
  `npm install -g monaco-editor`.
- `extras/make-material-symbols-filled.py` bakes the filled icon weight; restart the shell after it.
- `hyprland/scripts/build-dynamic-cursors.sh` rebuilds the cursor plugin.
  `hyprland/scripts/ensure-dynamic-cursors.sh` loads it at login and rebuilds it first when
  Hyprland was updated, because the ABI check rejects the old build.

- `extras/install-handy.sh` installs Handy (offline speech to text). System libraries come from
  pacman, the app itself from the release into `~/.local/opt/handy` with the binary in `bin/` and
  everything else in `lib/Handy/`, because Tauri resolves resources as `../lib/Handy`. Toggle it
  with `SUPER+ALT+D`, a Hyprland bind calling `handy --toggle-transcription`: the app's own hotkey
  is X11-only and misses keys on Wayland, so `keyboard_implementation` stays `none`.

## Verify

```sh
for f in .config/hypr/hyprland/*.lua .config/hypr/custom/*.lua; do luac -p "$f"; done
bash -n .config/quickshell/dotfiles/scripts/colors/switchwall.sh
python3 -c "import tomllib;tomllib.load(open('.config/matugen/config.toml','rb'))"
./extras/install.sh --check
git grep -lP '[\x{0400}-\x{04FF}]' -- ':!*.ttf' ':!*.png' ':!*.jpg'
```
