import qs.modules.common
import qs.modules.common.widgets
import qs.services
import QtQuick
import QtQuick.Layouts

// Pixel-style media capsule: album art, title over artist, play/pause. It only exists
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
        spacing: 9

        Rectangle { // Album art, or a note glyph when the player sends none
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: 30
            implicitHeight: 30
            radius: 9
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
                iconSize: 18
                color: Appearance.colors.colOnLayer2
            }
        }

        ColumnLayout { // Title over artist, like Android's now-playing row
            Layout.alignment: Qt.AlignVCenter
            Layout.maximumWidth: 210
            spacing: -2

            StyledText {
                Layout.fillWidth: true
                Layout.preferredWidth: Math.min(implicitWidth, 210)
                Layout.maximumWidth: 210
                elide: Text.ElideRight
                text: root.track?.title ?? ""
                color: Appearance.colors.colOnLayer0
                font {
                    pixelSize: Appearance.font.pixelSize.small
                    weight: Font.DemiBold
                    variableAxes: ({ "wght": 650 })
                }
            }

            StyledText {
                Layout.fillWidth: true
                Layout.preferredWidth: Math.min(implicitWidth, 210)
                Layout.maximumWidth: 210
                elide: Text.ElideRight
                text: root.track?.artist ?? ""
                color: Appearance.colors.colOnLayer2
                font.pixelSize: Appearance.font.pixelSize.smaller
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
                root.player?.previous();
            else
                root.player?.togglePlaying();
        }
    }
}
