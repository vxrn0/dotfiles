import qs
import qs.services
import qs.modules.common
import qs.modules.dotfiles.mediaControls
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Hyprland
import Quickshell.Io
import Quickshell.Services.Mpris

// Media panel, opened from the bar's media capsule. One card per MPRIS player.
// Sized to its content like the right sidebar, so the stock Hyprland layer
// animation scales the card, not the whole screen.
Scope {
    id: root
    readonly property real widgetWidth: 360
    readonly property real screenWidth: (Quickshell.screens.find(s => s.name === Hyprland.focusedMonitor?.name)
        ?? Quickshell.screens[0])?.width ?? 0
    // ScriptModel wants a JS array; the service exposes a QML list
    readonly property var playerArray: {
        const result = [];
        for (let i = 0; i < MprisController.players.length; i++)
            result.push(MprisController.players[i]);
        return result;
    }

    Connections {
        target: MprisController
        function onPlayersChanged() {
            if (MprisController.players.length === 0)
                GlobalStates.mediaControlsOpen = false;
        }
    }

    PanelWindow {
        id: panelWindow
        visible: GlobalStates.mediaControlsOpen
        screen: Quickshell.screens.find(s => s.name === Hyprland.focusedMonitor?.name)
            ?? Quickshell.screens[0] ?? null
        WlrLayershell.namespace: "quickshell:mediaControls"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: GlobalStates.mediaControlsOpen ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        exclusiveZone: 0
        color: "transparent"
        implicitWidth: root.widgetWidth
        implicitHeight: columnLayout.implicitHeight

        anchors {
            top: true
            left: true
        }
        margins {
            top: Appearance.sizes.barHeight
            // Right edge under the media capsule's right edge, clamped to the screen.
            left: {
                const anchorX = GlobalStates.mediaControlsAnchorX >= 0
                    ? GlobalStates.mediaControlsAnchorX
                    : (root.screenWidth + root.widgetWidth) / 2;
                return Math.max(Appearance.sizes.hyprlandGapsOut, Math.min(
                    root.screenWidth - root.widgetWidth - Appearance.sizes.hyprlandGapsOut,
                    anchorX - root.widgetWidth
                ));
            }
        }

        mask: Region {
            item: columnLayout
        }

        onVisibleChanged: {
            if (visible) {
                GlobalFocusGrab.addDismissable(panelWindow);
            } else {
                GlobalFocusGrab.removeDismissable(panelWindow);
            }
        }
        Connections {
            target: GlobalFocusGrab
            function onDismissed() {
                GlobalStates.mediaControlsOpen = false;
            }
        }

        FocusScope {
            id: focusScope
            // Not fill: the window height follows the content, and filling would make
            // the height depend on the layout that lives inside it.
            anchors {
                top: parent.top
                left: parent.left
                right: parent.right
            }
            height: columnLayout.implicitHeight
            focus: GlobalStates.mediaControlsOpen

            Keys.onPressed: (event) => {
                if (event.key === Qt.Key_Escape) {
                    GlobalStates.mediaControlsOpen = false;
                    event.accepted = true;
                }
            }

            ColumnLayout {
                id: columnLayout
                anchors {
                    top: parent.top
                    left: parent.left
                    right: parent.right
                }
                spacing: 10

                Repeater {
                    // ScriptModel keeps the delegates alive across track changes; a plain
                    // list rebuild would drop the delegates and their running animations.
                    model: ScriptModel {
                        values: root.playerArray
                    }
                    delegate: PlayerControl {
                        required property int index
                        required property MprisPlayer modelData
                        player: modelData
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }

    IpcHandler {
        target: "mediaControls"

        function toggle(): void {
            GlobalStates.mediaControlsOpen = !GlobalStates.mediaControlsOpen;
        }

        function close(): void {
            GlobalStates.mediaControlsOpen = false;
        }

        function open(): void {
            GlobalStates.mediaControlsOpen = true;
        }
    }
}
