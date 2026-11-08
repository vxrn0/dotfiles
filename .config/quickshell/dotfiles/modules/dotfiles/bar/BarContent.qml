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

    RowLayout { // Left section: live harness badges
        id: leftSectionRowLayout
        anchors {
            top: parent.top
            bottom: parent.bottom
            left: parent.left
            right: middleSection.left
        }
        spacing: 0

        HarnessIcons {
            id: harnessIcons
            Layout.alignment: Qt.AlignVCenter
            Layout.leftMargin: 8
            Layout.rightMargin: 4
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

        MouseArea {
            id: rightCenterGroup
            anchors.verticalCenter: parent.verticalCenter
            implicitWidth: 90
            implicitHeight: rightCenterGroupContent.implicitHeight

            onPressed: {
                GlobalStates.sidebarRightOpen = !GlobalStates.sidebarRightOpen;
            }

            BarGroup {
                id: rightCenterGroupContent
                anchors.fill: parent

                ClockWidget {
                    showDate: false
                    Layout.alignment: Qt.AlignVCenter
                    Layout.fillWidth: true
                }

                BatteryIndicator {
                    visible: (root.useShortenedForm < 2 && Battery.available)
                    Layout.alignment: Qt.AlignVCenter
                }
            }
        }
    }

    RowLayout { // Right section: sidebar toggle with indicators
        id: rightSectionRowLayout
        anchors {
            top: parent.top
            bottom: parent.bottom
            left: middleSection.right
            right: parent.right
        }
        spacing: 5
        layoutDirection: Qt.RightToLeft

        RippleButton { // Right sidebar button
            id: rightSidebarButton

            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
            Layout.rightMargin: Appearance.rounding.screenRounding
            Layout.fillWidth: false

            implicitWidth: indicatorsRowLayout.implicitWidth + 10 * 2
            implicitHeight: indicatorsRowLayout.implicitHeight + 5 * 2

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
            property color colText: toggled ? Appearance.colors.colPrimary : Appearance.colors.colOnLayer0

            Behavior on colText {
                animation: Appearance.animation.elementMoveFast.colorAnimation.createObject(this)
            }

            onPressed: {
                GlobalStates.sidebarRightOpen = !GlobalStates.sidebarRightOpen;
            }

            RowLayout {
                id: indicatorsRowLayout
                anchors.centerIn: parent
                property real realSpacing: 15
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
                        iconSize: Appearance.font.pixelSize.larger
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
                        iconSize: Appearance.font.pixelSize.larger
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
                    iconSize: Appearance.font.pixelSize.larger
                    color: rightSidebarButton.colText
                }
                MaterialSymbol {
                    Layout.leftMargin: indicatorsRowLayout.realSpacing
                    // Android shows a bluetooth icon only while the adapter is on; a slashed glyph reads as broken
                    visible: BluetoothStatus.available && BluetoothStatus.enabled
                    text: BluetoothStatus.connected ? "bluetooth_connected" : "bluetooth"
                    iconSize: Appearance.font.pixelSize.larger
                    color: rightSidebarButton.colText
                }
            }
        }

        // RightToLeft row: children later in the file sit further left, so the tray goes
        // after the indicator button to appear left of the wifi/bluetooth icons.
        SysTray {
            Layout.fillWidth: false
            Layout.fillHeight: true
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }
}
