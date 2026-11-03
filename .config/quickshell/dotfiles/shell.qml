//@ pragma UseQApplication
//@ pragma Env QS_NO_RELOAD_POPUP=1
//@ pragma Env QT_QUICK_CONTROLS_STYLE=Basic
//@ pragma Env QT_QUICK_FLICKABLE_WHEEL_DECELERATION=10000

// Remove two slashes below and adjust the value to change the UI scale
////@ pragma Env QT_SCALE_FACTOR=1

import "modules/common"
import "services"
import "panelFamilies"

import QtQuick
import QtQuick.Window
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

ShellRoot {
    id: root

    // Stuff for every panel family
    ReloadPopup {}

    Component.onCompleted: {
        MaterialThemeLoader.reapplyTheme()
        Hyprsunset.load()
        FirstRunExperience.load()
        ConflictKiller.load()
        Cliphist.refresh()
        Wallpapers.load()
    }


    // Global shortcuts live here so their panels can load lazily on first use
    GlobalShortcut {
        name: "aiChatToggle"

        onPressed: GlobalStates.aiChatOpen = !GlobalStates.aiChatOpen
    }

    GlobalShortcut {
        name: "aiChatNewChat"

        // Toggles Pilot with a fresh chat. Skipped while a fullscreen window is focused,
        // so the application keeps its own SUPER+Tab.
        onPressed: {
            const fullscreen = Hyprland.workspaces.values.some(workspace =>
                workspace.monitor && workspace.active
                && workspace.toplevels.values.some(toplevel => toplevel.wayland?.fullscreen));
            if (fullscreen)
                return;
            if (GlobalStates.aiChatOpen) {
                GlobalStates.aiChatOpen = false;
            } else {
                GlobalStates.aiChatResetRequested++;
                GlobalStates.aiChatOpen = true;
            }
        }
    }

    DotfilesFamily {}
}

