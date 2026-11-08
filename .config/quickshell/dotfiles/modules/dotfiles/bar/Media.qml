import qs.modules.common
import qs.modules.common.functions
import qs.modules.common.widgets
import qs.services
import qs

import QtQuick
import QtQuick.Layouts
import Quickshell.Services.Mpris

// Material 3 media chip: rounded album art, one line of title, and a trailing tonal
// play/pause button. The split keeps the artwork readable and the action quiet; the
// old position ring read as a fourth resource meter.
Item {
    id: root
    readonly property MprisPlayer activePlayer: MprisController.activePlayer
    readonly property string cleanedTitle: StringUtils.cleanMusicTitle(activePlayer?.trackTitle) || Translation.tr("No media")
    readonly property string artUrl: activePlayer?.trackArtUrl ?? ""
    readonly property bool hasArt: artUrl.length > 0
    readonly property real maxTextWidth: Math.max(120, Math.min(280, (root.Window.window?.screen?.width ?? 1920) * 0.22))

    Layout.fillHeight: true
    implicitWidth: rowLayout.implicitWidth
    implicitHeight: Appearance.sizes.barHeight

    Behavior on implicitWidth {
        animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
    }

    TextMetrics {
        id: titleMetrics
        text: root.cleanedTitle
        font {
            family: Appearance.font.family.main
            pixelSize: Appearance.font.pixelSize.smallie
            weight: Font.Medium
            hintingPreference: Font.PreferDefaultHinting
            variableAxes: Appearance.font.variableAxes.main
        }
    }

    MouseArea { // Chip-wide playback control; the visible button owns the left click
        anchors.fill: parent
        acceptedButtons: Qt.MiddleButton | Qt.BackButton | Qt.ForwardButton | Qt.RightButton
        cursorShape: Qt.PointingHandCursor
        onPressed: (event) => {
            if (!root.activePlayer)
                return;
            if (event.button === Qt.MiddleButton) {
                root.activePlayer.togglePlaying();
            } else if (event.button === Qt.BackButton) {
                root.activePlayer.previous();
            } else if (event.button === Qt.ForwardButton || event.button === Qt.RightButton) {
                root.activePlayer.next();
            }
        }
    }

    RowLayout {
        id: rowLayout

        spacing: 8
        anchors.fill: parent

        Rectangle { // Album art; the fallback is a tonal tile, not a stock note glyph
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: 26
            implicitHeight: 26
            radius: 8
            color: Appearance.colors.colSecondaryContainer
            border.width: 1
            border.color: Appearance.colors.colOutlineVariant
            clip: true

            Image {
                anchors.fill: parent
                visible: root.hasArt
                source: root.hasArt ? root.artUrl : ""
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                sourceSize.width: 52
                sourceSize.height: 52
            }

            MaterialSymbol {
                anchors.centerIn: parent
                visible: !root.hasArt
                text: "music_note"
                iconSize: 15
                fill: 1
                color: Appearance.colors.colOnSecondaryContainer
            }
        }

        StyledText {
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: Math.min(Math.ceil(titleMetrics.advanceWidth), root.maxTextWidth)
            elide: Text.ElideRight
            color: Appearance.colors.colOnLayer0
            text: root.cleanedTitle
            font {
                pixelSize: Appearance.font.pixelSize.smallie
                weight: Font.Medium
            }
        }

        Rectangle { // Trailing segment of the split button
            id: playButton
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: 26
            implicitHeight: 26
            radius: width / 2
            color: playArea.pressed
                ? Appearance.colors.colSecondaryContainerActive
                : playArea.containsMouse
                    ? Appearance.colors.colSecondaryContainerHover
                    : Appearance.colors.colSecondaryContainer

            Behavior on color {
                ColorAnimation {
                    duration: Appearance.animation.elementMoveFast.duration
                    easing.type: Easing.OutCubic
                }
            }

            // Expressive press: the button dips, then springs back.
            scale: playArea.pressed ? 0.86 : 1
            Behavior on scale {
                NumberAnimation {
                    duration: Appearance.animation.elementMoveFast.duration
                    easing.type: Easing.OutBack
                }
            }

            MaterialSymbol {
                anchors.centerIn: parent
                text: root.activePlayer?.isPlaying ? "pause" : "play_arrow"
                iconSize: 16
                fill: 1
                color: Appearance.colors.colOnSecondaryContainer
            }

            MouseArea {
                id: playArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.activePlayer?.togglePlaying()
            }
        }
    }
}
