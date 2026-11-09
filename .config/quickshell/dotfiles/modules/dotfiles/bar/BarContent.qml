import QtQuick
import QtQuick.Layouts
import Quickshell
import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets

Item { // Bar content region
    id: root

    property var screen: root.QsWindow.window?.screen
    // The media glyph only exists while a player is present
    readonly property bool hasMedia: MprisController.activePlayer != null
    property real useShortenedForm: (Appearance.sizes.barHellaShortenScreenWidthThreshold >= screen?.width) ? 2 : (Appearance.sizes.barShortenScreenWidthThreshold >= screen?.width) ? 1 : 0

    // Full-bleed surface: the bar is attached to the screen edge, not a floating card.
    Rectangle {
        id: barBackground
        anchors.fill: parent
        color: Config.options.bar.showBackground ? Appearance.colors.colLayer0 : "transparent"
    }

    // Left: clock first, like the Pixel status bar, then running agent CLIs.
    RowLayout {
        id: leftRow
        anchors {
            top: parent.top
            bottom: parent.bottom
            left: parent.left
            right: centerRow.left
        }
        anchors.leftMargin: 12
        spacing: 10

        ClockWidget {
            showDate: false
            Layout.alignment: Qt.AlignVCenter
        }

        HarnessIcons {
            id: harnessIcons
            Layout.alignment: Qt.AlignVCenter
            visible: anyRunning
        }

        Item { Layout.fillWidth: true }
    }


    // Centre: workspaces as flat dots on the bar surface.
    RowLayout {
        id: centerRow
        anchors {
            top: parent.top
            bottom: parent.bottom
            horizontalCenter: parent.horizontalCenter
        }
        spacing: 0

        Workspaces {
            Layout.fillHeight: true
        }
    }

    // Right: status glyphs, Pixel order — the battery is outermost. A plain Row packs
    // the items against the right edge; RowLayout spread them across the leftover space.
    Row {
        id: rightRow
        anchors {
            right: parent.right
            rightMargin: 12
            verticalCenter: parent.verticalCenter
        }
        spacing: 12

        Resources {
            id: resBox
            alwaysShowAllResources: root.useShortenedForm === 2
            visible: root.useShortenedForm < 2
        }

        Media {
            id: mediaBox
            visible: root.hasMedia && root.useShortenedForm < 2
        }

        SysTray {
            id: trayBox
            visible: root.useShortenedForm < 2 && (pinnedItems.length + unpinnedItems.length) > 0
        }

        StatusIcons {
            id: statusBox
            visible: root.useShortenedForm < 2
        }

        BatteryIndicator {
            visible: Battery.available && root.useShortenedForm < 2
        }
    }

    // Tapping the status icons opens quick settings, the way the Android shade works.
    component StatusIcons: Item {
        implicitWidth: icons.implicitWidth
        implicitHeight: 24

        RowLayout {
            id: icons
            anchors.fill: parent
            spacing: 12

            Revealer {
                reveal: Audio.sink?.audio?.muted ?? false
                Layout.fillHeight: true
                Layout.rightMargin: reveal ? 12 : 0
                Behavior on Layout.rightMargin {
                    animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
                }
                MaterialSymbol {
                    text: "volume_off"
                    iconSize: 16
                    fill: 1
                    color: Appearance.colors.colOnLayer0
                }
            }
            Revealer {
                reveal: Audio.source?.audio?.muted ?? false
                Layout.fillHeight: true
                Layout.rightMargin: reveal ? 12 : 0
                Behavior on Layout.rightMargin {
                    animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
                }
                MaterialSymbol {
                    text: "mic_off"
                    iconSize: 16
                    fill: 1
                    color: Appearance.colors.colOnLayer0
                }
            }
            Revealer {
                reveal: Notifications.silent || Notifications.unread > 0
                Layout.fillHeight: true
                Layout.rightMargin: reveal ? 12 : 0
                implicitHeight: reveal ? notificationUnreadCount.implicitHeight : 0
                implicitWidth: reveal ? notificationUnreadCount.implicitWidth : 0
                Behavior on Layout.rightMargin {
                    animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
                }
                NotificationUnreadCount {
                    id: notificationUnreadCount
                }
            }
            MaterialSymbol {
                text: Network.materialSymbol
                iconSize: 16
                fill: 1
                color: Appearance.colors.colOnLayer0
            }
            MaterialSymbol {
                // Android shows a bluetooth icon only while the adapter is on
                visible: BluetoothStatus.available && BluetoothStatus.enabled
                text: BluetoothStatus.connected ? "bluetooth_connected" : "bluetooth"
                iconSize: 16
                fill: 1
                color: Appearance.colors.colOnLayer0
            }
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: GlobalStates.sidebarRightOpen = !GlobalStates.sidebarRightOpen
        }
    }
}
