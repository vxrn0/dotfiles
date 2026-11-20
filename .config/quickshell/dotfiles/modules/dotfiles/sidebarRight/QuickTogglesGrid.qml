pragma ComponentBehavior: Bound
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.models.quickToggles as Models
import QtQuick
import QtQuick.Layouts

// Every quick toggle the shell ships. Each one is a QuickToggleModel (name, icon, toggled, mainAction).
Item {
    id: root
    implicitHeight: grid.implicitHeight

    signal openWifiDialog()
    signal openBluetoothDialog()
    signal openNightLightDialog()

    Models.BluetoothToggle { id: bluetooth }
    Models.ColorPickerToggle { id: colorPicker }
    Models.GameModeToggle { id: gameMode }
    Models.IdleInhibitorToggle { id: idleInhibitor }
    Models.NetworkToggle { id: network }
    Models.NightLightToggle { id: nightLight }
    Models.NotificationToggle { id: notifications }

    readonly property var all: [
        network, bluetooth, idleInhibitor, nightLight, gameMode, colorPicker
    ]

    GridLayout {
        id: grid
        width: root.width
        columns: 2
        columnSpacing: 8
        rowSpacing: 8

        Repeater {
            model: root.all

            RippleButton {
                id: tile
                required property var modelData
                visible: modelData.available
                Layout.fillWidth: true
                implicitHeight: 56
                padding: 14
                buttonRadius: Appearance.rounding.normal
                // The default 1.2 s ripple plus its 2.4 s fade left a slow wash over the
                // tile; a quick toggle should flash and be done.
                rippleDuration: Appearance.animation.elementMoveFast.duration
                colBackground: Appearance.colors.colLayer2
                colBackgroundHover: Appearance.colors.colLayer2Hover
                // Toggled state only changes tone, never hue: neutral surfaces only
                colBackgroundToggled: Appearance.colors.colLayer3
                colBackgroundToggledHover: Appearance.colors.colLayer3Hover
                // The default ripple is colLayer1Active, 85% toward the foreground: on a
                // layer-2 tile that reads as a white flash. Keep it in the tile's own tone.
                colRipple: Appearance.colors.colLayer2Active
                colRippleToggled: Appearance.colors.colLayer3Active
                toggled: modelData.toggled
                onClicked: modelData.mainAction()
                altAction: () => {
                    if (modelData === network)
                        root.openWifiDialog();
                    else if (modelData === bluetooth)
                        root.openBluetoothDialog();
                    else if (modelData === nightLight)
                        root.openNightLightDialog();
                }

                contentItem: RowLayout {
                    spacing: 10

                    MaterialSymbol {
                        Layout.alignment: Qt.AlignVCenter
                        text: tile.modelData.icon
                        iconSize: 22
                        fill: 1
                        color: tile.toggled ? Appearance.colors.colPrimary : Appearance.colors.colOnLayer2
                    }
                    StyledText {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        text: tile.modelData.name
                        elide: Text.ElideRight
                        font.pixelSize: Appearance.font.pixelSize.small
                        color: Appearance.colors.colOnLayer2
                    }
                }
            }
        }
    }
}
