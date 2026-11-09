import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.UPower
import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions

Item { // Bar content region
    id: root

    property var screen: root.QsWindow.window?.screen
    property var brightnessMonitor: Brightness.getMonitorForScreen(screen)
    // The media pill only exists while a player is present
    readonly property bool hasMedia: MprisController.activePlayer != null
    property real useShortenedForm: (Appearance.sizes.barHellaShortenScreenWidthThreshold >= screen?.width) ? 2 : (Appearance.sizes.barShortenScreenWidthThreshold >= screen?.width) ? 1 : 0
    readonly property int centerSideModuleWidth: (useShortenedForm == 2) ? Appearance.sizes.barCenterSideModuleWidthHellaShortened : (useShortenedForm == 1) ? Appearance.sizes.barCenterSideModuleWidthShortened : Appearance.sizes.barCenterSideModuleWidth

    component VerticalBarSeparator: Rectangle {
        Layout.topMargin: Appearance.sizes.baseBarHeight / 3
        Layout.bottomMargin: Appearance.sizes.baseBarHeight / 3
        Layout.fillHeight: true
        implicitWidth: 1
        color: Appearance.colors.colOutlineVariant
    }

    // Background shadow
    Loader {
        active: Config.options.bar.showBackground
        anchors.fill: barBackground
        sourceComponent: StyledRectangularShadow {
            anchors.fill: undefined // The loader's anchors act on this, and this should not have any anchor
            target: barBackground
        }
    }
    // Background
    Rectangle {
        id: barBackground
        anchors {
            fill: parent
            margins: Appearance.sizes.hyprlandGapsOut
        }
        color: Config.options.bar.showBackground ? Appearance.colors.colLayer0 : "transparent"
        radius: Appearance.rounding.windowRounding
        border.width: 1
        border.color: Appearance.colors.colLayer0Border
    }

    RowLayout { // Left section: clock where the Pilot mark used to be, then harness badges
        id: leftSectionRowLayout
        anchors {
            top: parent.top
            bottom: parent.bottom
            left: parent.left
            right: middleSection.left
        }
        spacing: 0

        BarGroup {
            Layout.alignment: Qt.AlignVCenter
            Layout.leftMargin: 8

            ClockWidget {
                showDate: false
                Layout.alignment: Qt.AlignVCenter
            }
        }

        BarGroup {
            Layout.alignment: Qt.AlignVCenter
            Layout.leftMargin: 4
            visible: harnessIcons.anyRunning

            HarnessIcons {
                id: harnessIcons
                Layout.alignment: Qt.AlignVCenter
                Layout.leftMargin: 4
                Layout.rightMargin: 4
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }

    Row { // Middle section
        id: middleSection
        anchors {
            top: parent.top
            bottom: parent.bottom
            horizontalCenter: parent.horizontalCenter
        }
        spacing: 4

        BarGroup {
            id: leftCenterGroup
            anchors.verticalCenter: parent.verticalCenter

            Resources {
                alwaysShowAllResources: root.useShortenedForm === 2
                Layout.fillWidth: root.useShortenedForm === 2
            }
        }

        VerticalBarSeparator {
            visible: Config.options?.bar.borderless && root.hasMedia
        }

        BarGroup {
            id: mediaGroup
            anchors.verticalCenter: parent.verticalCenter
            visible: root.useShortenedForm < 2 && root.hasMedia

            Media {
                Layout.fillWidth: true
            }
        }

        VerticalBarSeparator {
            visible: Config.options?.bar.borderless
        }

        BarGroup {
            id: middleCenterGroup
            anchors.verticalCenter: parent.verticalCenter
            padding: workspacesWidget.widgetPadding

            Workspaces {
                id: workspacesWidget
                Layout.fillHeight: true
                MouseArea {
                    // Right-click to toggle overview
                    anchors.fill: parent
                    acceptedButtons: Qt.RightButton

                    onPressed: event => {
                        if (event.button === Qt.RightButton) {
                            GlobalStates.overviewOpen = !GlobalStates.overviewOpen;
                        }
                    }
                }
            }
        }

        VerticalBarSeparator {
            visible: Config.options?.bar.borderless
        }

        BarGroup { // Battery stays in the centre now that the clock moved left
            id: rightCenterGroup
            anchors.verticalCenter: parent.verticalCenter
            visible: root.useShortenedForm < 2 && Battery.available

            BatteryIndicator {
                visible: (root.useShortenedForm < 2 && Battery.available)
                Layout.alignment: Qt.AlignVCenter
            }
        }
    }

    RowLayout { // Right section: status pill and tray pill, same item scale as the rest
        id: rightSectionRowLayout
        anchors {
            top: parent.top
            bottom: parent.bottom
            left: middleSection.right
            right: parent.right
        }
        spacing: 4
        layoutDirection: Qt.RightToLeft

        BarGroup { // Sidebar toggle with indicators
            Layout.alignment: Qt.AlignVCenter
            Layout.rightMargin: 8

            RippleButton {
                id: rightSidebarButton

                Layout.alignment: Qt.AlignVCenter
                implicitWidth: indicatorsRowLayout.implicitWidth + 8
                implicitHeight: 24

                buttonRadius: Appearance.rounding.full
                // No fill and no ripple: the click wave would read as a lit-up background
                rippleEnabled: false
                colBackground: "transparent"
                colBackgroundHover: "transparent"
                colRipple: Appearance.colors.colLayer1Active
                colBackgroundToggled: "transparent"
                colBackgroundToggledHover: "transparent"
                colRippleToggled: Appearance.colors.colLayer1Active
                toggled: GlobalStates.sidebarRightOpen
                // Same hover rule as BarItem: the glyph goes primary, the pill never lights up
                property color colText: (toggled || hovered) ? Appearance.colors.colPrimary : Appearance.colors.colOnLayer1

                Behavior on colText {
                    animation: Appearance.animation.elementMoveFast.colorAnimation.createObject(this)
                }

                onPressed: {
                    GlobalStates.sidebarRightOpen = !GlobalStates.sidebarRightOpen;
                }

                RowLayout {
                    id: indicatorsRowLayout
                    anchors.centerIn: parent
                    property real realSpacing: 10
                    spacing: 0

                    Revealer {
                        reveal: Audio.sink?.audio?.muted ?? false
                        Layout.fillHeight: true
                        Layout.rightMargin: reveal ? indicatorsRowLayout.realSpacing : 0
                        Behavior on Layout.rightMargin {
                            animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
                        }
                        MaterialSymbol {
                            text: "volume_off"
                            iconSize: 16
                            color: rightSidebarButton.colText
                        }
                    }
                    Revealer {
                        reveal: Audio.source?.audio?.muted ?? false
                        Layout.fillHeight: true
                        Layout.rightMargin: reveal ? indicatorsRowLayout.realSpacing : 0
                        Behavior on Layout.rightMargin {
                            animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
                        }
                        MaterialSymbol {
                            text: "mic_off"
                            iconSize: 16
                            color: rightSidebarButton.colText
                        }
                    }
                    Revealer {
                        reveal: Notifications.silent || Notifications.unread > 0
                        Layout.fillHeight: true
                        Layout.rightMargin: reveal ? indicatorsRowLayout.realSpacing : 0
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
                        color: rightSidebarButton.colText
                    }
                    MaterialSymbol {
                        Layout.leftMargin: indicatorsRowLayout.realSpacing
                        // Android shows a bluetooth icon only while the adapter is on; a slashed glyph reads as broken
                        visible: BluetoothStatus.available && BluetoothStatus.enabled
                        text: BluetoothStatus.connected ? "bluetooth_connected" : "bluetooth"
                        iconSize: 16
                        color: rightSidebarButton.colText
                    }
                }
            }
        }

        BarGroup { // Tray
            Layout.alignment: Qt.AlignVCenter
            visible: tray.implicitWidth > 0

            SysTray {
                id: tray
                Layout.fillHeight: true
                Layout.leftMargin: 4
                Layout.rightMargin: 4
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }
}
