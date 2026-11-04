local home_dir = os.getenv("HOME")

-- Wayland
hl.env("ELECTRON_OZONE_PLATFORM_HINT", "auto")

-- Applications
local xdg_data_dirs_old = os.getenv("XDG_DATA_DIRS") or ""
hl.env("XDG_DATA_DIRS", home_dir .. "/.local/share/flatpak/exports/share:/var/lib/flatpak/exports/share:/usr/local/share:/usr/share:" .. xdg_data_dirs_old)

-- Themes
hl.env("QT_QPA_PLATFORM", "wayland;xcb")
hl.env("XDG_MENU_PREFIX", "plasma-")
-- Qt apps take their palette from the GTK theme matugen writes (gtk-3.0/gtk.css plus
-- adw-gtk3-dark). This used to be "kde", which needed plasma-integration and a
-- kdeglobals file nothing regenerated; the gtk3 plugin ships with qt6-base.
hl.env("QT_QPA_PLATFORMTHEME", "gtk3")

-- Virtual environment
hl.env("DOTFILES_VIRTUAL_ENV", home_dir .. "/.local/state/quickshell/.venv")
