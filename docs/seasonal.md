# Seasonal desktop

Status: idea, not built. Parked until the Pilot (AI panel) work is finished.

Concept: the wallpaper scene reacts to the machine. Snow accumulates while the system is cool and
melts when the CPU heats up, so a build or a heavy session visibly changes the desktop.

Cheap by construction: the background is one `Image`, and editing `wallpaper.path` in
`~/.config/dotfiles/config.json` swaps it live, without restart and without re-running matugen
(verified). No shader, no renderer process, no extra RAM.

## Plan

1. Generate 4-5 states of one scene: clear, first snow, deep snow, melting. Same framing, only the
   snow differs.
2. `seasonal.sh`: read `/sys/class/hwmon/*/temp1_input` (coretemp), step the state up while cool
   (one step per ~10 min) and down when the CPU stays above 75 °C (one step per ~30 s); write the
   path into `config.json`.
3. Optional: take snow density from Open-Meteo so the desktop matches the weather outside.
4. Optional: shade `general.col.active_border` from ice blue to amber with temperature. Zero cost,
   visible on every window.
5. Needs the shell owner: snow shelf along the bar edge, advent calendar in the launcher.

Rejected: fullscreen falling-snow overlay. On Wayland it can only draw above windows, and on NVIDIA
it costs 40-80 MB plus 5-10% GPU continuously, which works against the memory goal.
