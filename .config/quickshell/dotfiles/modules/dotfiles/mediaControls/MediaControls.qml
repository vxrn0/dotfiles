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
// Full-screen while open: the empty area closes the panel on click, and Esc works
// because the surface owns the keyboard.
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
        WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        exclusiveZone: 0
        color: "transparent"

        anchors {
            top: true
            bottom: true
            left: true
            right: true
        }

        // Click anywhere outside the card closes the panel.
        MouseArea {
            anchors.fill: parent
            onClicked: GlobalStates.mediaControlsOpen = false
        }

        Item {
            id: cardColumn
            property bool revealed: false
            x: {
                const anchorX = GlobalStates.mediaControlsAnchorX >= 0
                    ? GlobalStates.mediaControlsAnchorX
                    : (root.screenWidth + root.widgetWidth) / 2;
                return Math.max(Appearance.sizes.hyprlandGapsOut, Math.min(
                    root.screenWidth - root.widgetWidth - Appearance.sizes.hyprlandGapsOut,
                    anchorX - root.widgetWidth
                ));
            }
            // Slides down from under the bar; the layer itself only fades.
            y: Appearance.sizes.barHeight + (revealed ? 0 : -8)
            opacity: revealed ? 1 : 0
            width: root.widgetWidth
            height: columnLayout.implicitHeight

            Behavior on y {
                NumberAnimation {
                    duration: 220
                    easing.type: Easing.OutCubic
                }
            }
            Behavior on opacity {
                NumberAnimation {
                    duration: 160
                    easing.type: Easing.OutCubic
                }
            }

            // Swallow clicks on the card so only the empty area dismisses.
            MouseArea {
                anchors.fill: parent
            }

            Keys.onPressed: (event) => {
                if (event.key === Qt.Key_Escape) {
                    GlobalStates.mediaControlsOpen = false;
                    event.accepted = true;
                }
            }

            ColumnLayout {
                id: columnLayout
                // Anchor only three sides: filling the parent would make the parent's
                // height depend on this layout, and the card collapses to nothing.
                anchors {
                    left: parent.left
                    right: parent.right
                    top: parent.top
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

        onVisibleChanged: {
            if (visible) {
                cardColumn.revealed = false;
                Qt.callLater(() => cardColumn.revealed = true);
                cardColumn.forceActiveFocus();
            } else {
                cardColumn.revealed = false;
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
