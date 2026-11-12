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
// The card window is sized to its content, so the stock Hyprland layer animation
// scales the card, not the whole screen. A second, fully transparent full-screen
// surface sits underneath to catch clicks outside the card.
Scope {
    id: root
    readonly property real widgetWidth: 360
    readonly property real shadowMargin: Appearance.sizes.elevationMargin
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

    // Click-catcher: invisible, full screen, below the card. Any click here is a
    // click outside the card.
    PanelWindow {
        id: dismissLayer
        visible: GlobalStates.mediaControlsOpen
        screen: Quickshell.screens.find(s => s.name === Hyprland.focusedMonitor?.name)
            ?? Quickshell.screens[0] ?? null
        WlrLayershell.namespace: "quickshell:mediaControlsDismiss"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        exclusiveZone: 0
        color: "transparent"

        anchors {
            top: true
            bottom: true
            left: true
            right: true
        }

        MouseArea {
            anchors.fill: parent
            onClicked: GlobalStates.mediaControlsOpen = false
        }
    }

    PanelWindow {
        id: panelWindow
        visible: GlobalStates.mediaControlsOpen
        screen: Quickshell.screens.find(s => s.name === Hyprland.focusedMonitor?.name)
            ?? Quickshell.screens[0] ?? null
        WlrLayershell.namespace: "quickshell:mediaControls"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: GlobalStates.mediaControlsOpen ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        exclusiveZone: 0
        color: "transparent"
        implicitWidth: root.widgetWidth + root.shadowMargin * 2
        implicitHeight: columnLayout.implicitHeight + root.shadowMargin * 2

        anchors {
            top: true
            left: true
        }
        margins {
            top: Appearance.sizes.barHeight - root.shadowMargin
            // Right edge under the media capsule's right edge, clamped to the screen.
            left: {
                const anchorX = GlobalStates.mediaControlsAnchorX >= 0
                    ? GlobalStates.mediaControlsAnchorX
                    : (root.screenWidth + root.widgetWidth) / 2;
                return Math.max(Appearance.sizes.hyprlandGapsOut, Math.min(
                    root.screenWidth - root.widgetWidth - Appearance.sizes.hyprlandGapsOut,
                    anchorX - root.widgetWidth
                )) - root.shadowMargin;
            }
        }

        onVisibleChanged: {
            if (visible)
                focusScope.forceActiveFocus();
        }

        FocusScope {
            id: focusScope
            anchors {
                top: parent.top
                left: parent.left
                right: parent.right
                margins: root.shadowMargin
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
