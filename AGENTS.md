# dotfiles

Personal XDG config for a Hyprland + Quickshell desktop on Arch. The shell is called `dotfiles`.
Plain files. Derived from dots-hyprland; see `README.md` for credit and license.

`docs/ROADMAP.md` holds what is decided and what is still open.

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

## Detail

Layout, conventions, extras needing a manual step and verify commands live in
[`docs/agents/dotfiles.md`](docs/agents/dotfiles.md). Read it before editing.
