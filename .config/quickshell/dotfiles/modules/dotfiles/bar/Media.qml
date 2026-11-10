import qs.modules.common
import qs.modules.common.widgets
import qs.services
import QtQuick
import QtQuick.Layouts

// Pixel-style media capsule: album art, track title, artist, play/pause. It only exists
// while a player is present; the whole capsule is the transport control.
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
        spacing: 8

        Rectangle { // Album art, or a note glyph when the player sends none
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: 26
            implicitHeight: 26
            radius: 8
            color: Appearance.colors.colLayer2
            clip: true

            Image {
                id: art
                anchors.fill: parent
                asynchronous: true
                fillMode: Image.PreserveAspectCrop
                source: root.artUrl.startsWith("/") ? "file://" + root.artUrl : root.artUrl
                visible: status === Image.Ready
            }

            MaterialSymbol {
                anchors.centerIn: parent
                visible: art.status !== Image.Ready
                text: "music_note"
                iconSize: 16
                color: Appearance.colors.colOnLayer2
            }
        }

        StyledText {
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: Math.min(implicitWidth, 150)
            Layout.maximumWidth: 150
            elide: Text.ElideRight
            text: root.track?.title ?? ""
            color: Appearance.colors.colOnLayer0
            font {
                pixelSize: Appearance.font.pixelSize.normal
                weight: Font.DemiBold
                variableAxes: ({ "wght": 650 })
            }
        }

        StyledText {
            Layout.alignment: Qt.AlignVCenter
            visible: text.length > 0
            Layout.preferredWidth: Math.min(implicitWidth, 110)
            Layout.maximumWidth: 110
            elide: Text.ElideRight
            text: root.track?.artist ?? ""
            color: Appearance.colors.colOnLayer2
            font.pixelSize: Appearance.font.pixelSize.small
        }

        MaterialSymbol {
            Layout.alignment: Qt.AlignVCenter
            text: root.player?.isPlaying ? "pause" : "play_arrow"
            iconSize: 20
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
                root.player?.previous();
            else
                root.player?.togglePlaying();
        }
    }
}
