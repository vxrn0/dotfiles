import qs.modules.common
import qs.modules.common.functions
import qs.modules.common.widgets
import qs.services
import QtQuick
import QtQuick.Layouts
import Quickshell.Services.Mpris

// One player card in the media panel: art, track info, seek bar, transport.
Item {
    id: root
    required property MprisPlayer player
    readonly property string artUrl: root.player?.trackArtUrl ?? ""
    readonly property real length: root.player?.length ?? 0
    readonly property real position: root.player?.position ?? 0

    implicitHeight: 132

    Timer { // position is not signalled by MPRIS, poll it while playing
        running: root.player?.isPlaying ?? false
        interval: Config.options.resources.updateInterval
        repeat: true
        onTriggered: root.player.positionChanged()
    }

    StyledRectangularShadow {
        target: background
    }

    Rectangle {
        id: background
        anchors.fill: parent
        radius: Appearance.rounding.windowRounding
        color: Appearance.colors.colLayer0
        border.width: 1
        border.color: Appearance.colors.colLayer0Border

        RowLayout {
            anchors.fill: parent
            anchors.margins: 12
            spacing: 12

            Rectangle {
                Layout.fillHeight: true
                implicitWidth: height
                radius: Appearance.rounding.small
                color: Appearance.colors.colLayer1
                clip: true

                Image {
                    id: art
                    anchors.fill: parent
                    asynchronous: true
                    cache: false
                    fillMode: Image.PreserveAspectCrop
                    source: root.artUrl.startsWith("/") ? "file://" + root.artUrl : root.artUrl
                    visible: status === Image.Ready
                }

                MaterialSymbol {
                    anchors.centerIn: parent
                    visible: art.status !== Image.Ready
                    text: "music_note"
                    iconSize: 32
                    color: Appearance.colors.colOnLayer1
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                StyledText {
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    text: StringUtils.cleanMusicTitle(root.player?.trackTitle) || Translation.tr("Unknown Title")
                    color: Appearance.colors.colOnLayer0
                    font {
                        pixelSize: Appearance.font.pixelSize.large
                        weight: Font.DemiBold
                        variableAxes: ({ "wght": 650 })
                    }
                }

                StyledText {
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    text: root.player?.trackArtist ?? ""
                    color: Appearance.colors.colSubtext
                    font.pixelSize: Appearance.font.pixelSize.smaller
                }

                Item {
                    Layout.fillHeight: true
                }

                Loader {
                    Layout.fillWidth: true
                    active: root.player?.canSeek ?? false
                    sourceComponent: StyledSlider {
                        configuration: StyledSlider.Configuration.Wavy
                        value: root.length > 0 ? root.position / root.length : 0
                        onMoved: {
                            if (root.length > 0)
                                root.player.position = value * root.length;
                        }
                    }
                }

                StyledProgressBar {
                    Layout.fillWidth: true
                    visible: !(root.player?.canSeek ?? false)
                    wavy: root.player?.isPlaying ?? false
                    value: root.length > 0 ? root.position / root.length : 0
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    StyledText {
                        Layout.fillWidth: true
                        text: `${StringUtils.friendlyTimeForSeconds(root.position)} / ${StringUtils.friendlyTimeForSeconds(root.length)}`
                        color: Appearance.colors.colSubtext
                        font.pixelSize: Appearance.font.pixelSize.smaller
                    }

                    RippleButton {
                        implicitWidth: 32
                        implicitHeight: 32
                        buttonRadius: Appearance.rounding.full
                        downAction: () => root.player?.previous()

                        contentItem: MaterialSymbol {
                            anchors.centerIn: parent
                            text: "skip_previous"
                            iconSize: 22
                            fill: 1
                            color: Appearance.colors.colOnLayer1
                        }
                    }

                    RippleButton {
                        implicitWidth: 40
                        implicitHeight: 40
                        buttonRadius: Appearance.rounding.full
                        colBackground: root.player?.isPlaying ? Appearance.colors.colPrimary : Appearance.colors.colSecondaryContainer
                        colBackgroundHover: root.player?.isPlaying ? Appearance.colors.colPrimaryHover : Appearance.colors.colSecondaryContainerHover
                        colRipple: root.player?.isPlaying ? Appearance.colors.colPrimaryActive : Appearance.colors.colSecondaryContainerActive
                        downAction: () => root.player?.togglePlaying()

                        contentItem: MaterialSymbol {
                            anchors.centerIn: parent
                            text: root.player?.isPlaying ? "pause" : "play_arrow"
                            iconSize: 24
                            fill: 1
                            color: root.player?.isPlaying ? Appearance.colors.colOnPrimary : Appearance.colors.colOnSecondaryContainer
                        }
                    }

                    RippleButton {
                        implicitWidth: 32
                        implicitHeight: 32
                        buttonRadius: Appearance.rounding.full
                        downAction: () => root.player?.next()

                        contentItem: MaterialSymbol {
                            anchors.centerIn: parent
                            text: "skip_next"
                            iconSize: 22
                            fill: 1
                            color: Appearance.colors.colOnLayer1
                        }
                    }
                }
            }
        }
    }
}
