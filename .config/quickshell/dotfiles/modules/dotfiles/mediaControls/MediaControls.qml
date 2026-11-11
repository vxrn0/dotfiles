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
Scope {
    id: root
    readonly property real widgetWidth: 360

    Connections {
        target: MprisController
        function onPlayersChanged() {
            if (MprisController.players.length === 0)
                GlobalStates.mediaControlsOpen = false;
        }
    }

    Loader {
        id: loader
        active: GlobalStates.mediaControlsOpen

        sourceComponent: PanelWindow {
            id: panelWindow
            visible: true
            screen: Quickshell.screens.find(s => s.name === Hyprland.focusedMonitor?.name) ?? null
            WlrLayershell.namespace: "quickshell:mediaControls"
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.OnDemand
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
                left: (panelWindow.screen.width / 2) - (root.widgetWidth / 2)
            }
            mask: Region {
                item: columnLayout
            }

            Component.onCompleted: {
                focusScope.forceActiveFocus();
                GlobalFocusGrab.addDismissable(panelWindow);
            }
            Component.onDestruction: {
                GlobalFocusGrab.removeDismissable(panelWindow);
            }
            Connections {
                target: GlobalFocusGrab
                function onDismissed() {
                    GlobalStates.mediaControlsOpen = false;
                }
            }

            FocusScope {
                id: focusScope
                anchors.fill: parent
                focus: true

                Keys.onPressed: (event) => {
                    if (event.key === Qt.Key_Escape) {
                        GlobalStates.mediaControlsOpen = false;
                        event.accepted = true;
                    }
                }

                ColumnLayout {
                    id: columnLayout
                    anchors.fill: parent
                    spacing: 0

                    Repeater {
                        model: MprisController.players
                        delegate: PlayerControl {
                            required property MprisPlayer modelData
                            player: modelData
                            Layout.fillWidth: true
                        }
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
