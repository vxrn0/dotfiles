import qs
import qs.modules.common
import qs.modules.common.widgets
import qs.services
import Qt5Compat.GraphicalEffects
import QtQuick
import QtQuick.Layouts

// Pixel-style media capsule: album art, track title, play/pause. It only exists
// while a player is present. Left click opens the media panel, the other buttons
// are transport shortcuts.
Item {
    id: root
    readonly property var player: MprisController.activePlayer
    readonly property var track: MprisController.activeTrack
    readonly property string artUrl: root.track?.artUrl ?? ""

    implicitWidth: rowLayout.implicitWidth
    implicitHeight: rowLayout.implicitHeight

    RowLayout {
        id: rowLayout
        anchors.centerIn: parent
        spacing: 10

        Rectangle { // Album art, or a note glyph when the player sends none
            id: artFrame
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: 30
            implicitHeight: 30
            radius: 9
            color: Appearance.colors.colLayer2

            StyledImage {
                id: art
                anchors.fill: parent
                fillMode: Image.PreserveAspectCrop
                source: root.artUrl.startsWith("/") ? "file://" + root.artUrl : root.artUrl
                layer.enabled: true
                layer.effect: OpacityMask {
                    maskSource: Rectangle {
                        width: artFrame.width
                        height: artFrame.height
                        radius: artFrame.radius
                    }
                }
            }

            MaterialSymbol {
                anchors.centerIn: parent
                visible: art.status !== Image.Ready
                text: "music_note"
                iconSize: 18
                color: Appearance.colors.colOnLayer2
            }
        }

        StyledText {
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: Math.min(implicitWidth, 300)
            Layout.maximumWidth: 300
            elide: Text.ElideRight
            text: root.track?.title ?? ""
            color: Appearance.colors.colOnLayer0
            font {
                pixelSize: Appearance.font.pixelSize.normal
                weight: Font.DemiBold
                variableAxes: ({ "wght": 650 })
            }
        }

        MaterialSymbol {
            Layout.alignment: Qt.AlignVCenter
            text: root.player?.isPlaying ? "pause" : "play_arrow"
            iconSize: 22
            fill: root.player?.isPlaying ? 1 : 0
            color: root.player?.isPlaying ? Appearance.colors.colPrimary : Appearance.colors.colOnLayer1
        }
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
        cursorShape: Qt.PointingHandCursor
        onClicked: (event) => {
            if (event.button === Qt.RightButton)
                root.player?.next();
            else if (event.button === Qt.MiddleButton)
                root.player?.togglePlaying();
            else {
                GlobalStates.mediaControlsAnchorX = root.mapToItem(null, 0, 0).x;
                GlobalStates.mediaControlsOpen = !GlobalStates.mediaControlsOpen;
            }
        }
    }
}
