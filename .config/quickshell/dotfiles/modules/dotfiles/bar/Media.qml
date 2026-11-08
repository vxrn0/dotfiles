import qs.modules.common
import qs.modules.common.widgets
import qs.services
import qs
import qs.modules.common.functions

import QtQuick
import QtQuick.Layouts
import Quickshell.Services.Mpris

// Material 3 media chip: rounded album art, one line of title, filled play/pause
// button. The old circular ring only encoded playback position, which read as
// another resource meter.
Item {
    id: root
    readonly property MprisPlayer activePlayer: MprisController.activePlayer
    readonly property string cleanedTitle: StringUtils.cleanMusicTitle(activePlayer?.trackTitle) || Translation.tr("No media")
    readonly property string artUrl: activePlayer?.trackArtUrl ?? ""
    readonly property bool hasArt: artUrl.length > 0
    // Long titles get more room, but never more than a quarter of the screen
    readonly property real maxTextWidth: Math.max(140, Math.min(300, (root.Window.window?.screen?.width ?? 1920) * 0.24))

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

    MouseArea { // Chip-wide playback control; left click belongs to the button
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

        spacing: 6
        anchors.fill: parent

        Rectangle { // Album art, falling back to a tonal glyph
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: 24
            implicitHeight: 24
            radius: Appearance.rounding.small
            color: Appearance.colors.colSecondaryContainer
            clip: true

            Image {
                anchors.fill: parent
                visible: root.hasArt
                source: root.hasArt ? root.artUrl : ""
                fillMode: Image.PreserveAspectCrop
                asynchronous: true
                sourceSize.width: 48
                sourceSize.height: 48
            }

            MaterialSymbol {
                anchors.centerIn: parent
                visible: !root.hasArt
                text: "music_note"
                iconSize: 16
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

        Rectangle { // Filled tonal play/pause
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: 22
            implicitHeight: 22
            radius: width / 2
            color: Appearance.colors.colPrimaryContainer

            MaterialSymbol {
                anchors.centerIn: parent
                text: root.activePlayer?.isPlaying ? "pause" : "play_arrow"
                iconSize: 16
                fill: 1
                color: Appearance.colors.colOnPrimaryContainer
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.activePlayer?.togglePlaying()
            }
        }
    }
}
