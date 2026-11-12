-- put former exec-once commands inside the func and former exec commands outside
hl.on("hyprland.start", function ()

    -- Bar, wallpaper
    hl.exec_cmd("$HOME/.config/hypr/hyprland/scripts/start_geoclue_agent.sh")
    hl.exec_cmd("qs -c $qsConfig")
    hl.exec_cmd("$HOME/.config/hypr/custom/scripts/__restore_video_wallpaper.sh")

    -- Core components (authentication, lock screen, notification daemon)
    hl.exec_cmd("gnome-keyring-daemon --start --components=secrets")
    hl.exec_cmd("hypridle")
    hl.exec_cmd("dbus-update-activation-environment --all")
    -- Portals (settings, file chooser) need the Wayland env in the systemd user manager,
    -- otherwise xdg-desktop-portal-gtk cannot open a display and apps fall back to light theme
    hl.exec_cmd("dbus-update-activation-environment --systemd --all")

    -- Audio
    hl.exec_cmd("easyeffects --hide-window --service-mode")
    hl.exec_cmd("command -v handy >/dev/null && handy --start-hidden")

    -- Clipboard: history
    --hl.exec_cmd("wl-paste --watch cliphist store")
    hl.exec_cmd("wl-paste --type text --watch bash -c 'cliphist store && qs -c $qsConfig ipc call cliphistService update'")
    hl.exec_cmd("wl-paste --type image --watch bash -c 'cliphist store && qs -c $qsConfig ipc call cliphistService update'")

    -- Cursor
    hl.exec_cmd("hyprctl setcursor Bibata-Modern-Classic 24")
    hl.exec_cmd("$HOME/.config/hypr/hyprland/scripts/ensure-dynamic-cursors.sh")
end)
