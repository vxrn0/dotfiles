import qs.modules.common
import qs.modules.common.widgets
import qs.services
import qs
import QtQuick
import QtQuick.Layouts
import Qt5Compat.GraphicalEffects
import Quickshell

Rectangle {
    id: root
    radius: Appearance.rounding.normal
    color: Appearance.colors.colLayer1
    implicitHeight: content.implicitHeight + 20

    readonly property string preview: {
        const path = Config.options.wallpaper.thumbnailPath;
        return path.length > 0 ? path : Config.options.wallpaper.path;
    }
    readonly property bool dark: Appearance.m3colors.darkmode

    ColumnLayout {
        id: content
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: 10
        }
        spacing: 10

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            MaterialSymbol {
                text: "wallpaper"
                iconSize: Appearance.font.pixelSize.normal
                color: Appearance.colors.colOnLayer1
            }
            StyledText {
                text: Translation.tr("Wallpaper")
                font.pixelSize: Appearance.font.pixelSize.small
                color: Appearance.colors.colOnLayer1
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            RippleButton {
                implicitWidth: 64
                implicitHeight: 40
                buttonRadius: Appearance.rounding.small
                colBackground: Appearance.colors.colLayer2
                onClicked: {
                    // Respect the same choice the Super+N shortcut and the IPC handler use
                    if (Config.options.wallpaperSelector.useSystemFileDialog)
                        Wallpapers.openFallbackPicker(root.dark)
                    else
                        GlobalStates.wallpaperSelectorOpen = true
                }

                Image {
                    id: wallpaperThumbnail
                    anchors.fill: parent
                    source: root.preview
                    sourceSize.width: 128
                    sourceSize.height: 80
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    layer.enabled: true
                    layer.effect: OpacityMask {
                        maskSource: Rectangle {
                            width: wallpaperThumbnail.width
                            height: wallpaperThumbnail.height
                            radius: Appearance.rounding.small
                        }
                    }
                }
            }

            RippleButton { // Theme switch; the only one in the panel
                Layout.fillWidth: true
                implicitHeight: 40
                buttonRadius: Appearance.rounding.small
                colBackground: Appearance.colors.colLayer2
                colBackgroundHover: Appearance.colors.colLayer2Hover
                onClicked: Quickshell.execDetached([
                    Directories.wallpaperSwitchScriptPath,
                    "--mode", root.dark ? "light" : "dark",
                    "--noswitch"
                ])

                Row {
                    anchors.centerIn: parent
                    spacing: 8

                    MaterialSymbol {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.dark ? "light_mode" : "dark_mode"
                        iconSize: 18
                        color: Appearance.colors.colOnLayer2
                    }
                    StyledText {
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.dark ? Translation.tr("Light") : Translation.tr("Dark")
                        font.pixelSize: Appearance.font.pixelSize.small
                        color: Appearance.colors.colOnLayer2
                    }
                }
            }
        }
    }
}
