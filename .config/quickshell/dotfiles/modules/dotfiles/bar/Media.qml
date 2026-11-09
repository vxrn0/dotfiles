import qs.modules.common
import qs.modules.common.widgets
import qs.services

import QtQuick
import QtQuick.Layouts
import Quickshell.Services.Mpris

// Media as a single circular control, matching the metric rings beside it: a play or
// pause glyph in an outlined button. The old chip carried the album art and the track
// title, which made the bar shout; the glyph alone is enough here.
Item {
    id: root
    readonly property MprisPlayer activePlayer: MprisController.activePlayer

    Layout.fillHeight: true
    implicitWidth: button.implicitWidth
    implicitHeight: Appearance.sizes.barHeight

    Behavior on implicitWidth {
        animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
    }

    Rectangle {
        id: button
        anchors.verticalCenter: parent.verticalCenter
        implicitWidth: 22
        implicitHeight: 22
        radius: width / 2
        color: "transparent"
        border.width: 2
        // The glyph sits on the ring's stroke, not inside a filled disc: same weight as
        // the metric rings, so the cluster reads as one row.
        border.color: mouseArea.containsMouse
            ? Appearance.colors.colPrimary
            : Appearance.colors.colOnSecondaryContainer

        Behavior on border.color {
            ColorAnimation {
                duration: Appearance.animation.elementMoveFast.duration
                easing.type: Easing.OutCubic
            }
        }

        MaterialSymbol {
            anchors.centerIn: parent
            text: root.activePlayer?.isPlaying ? "pause" : "play_arrow"
            iconSize: 13
            fill: 1
            color: button.border.color
        }

        MouseArea {
            id: mouseArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.BackButton | Qt.ForwardButton | Qt.RightButton

            onPressed: (event) => {
                if (!root.activePlayer)
                    return;
                if (event.button === Qt.LeftButton || event.button === Qt.MiddleButton)
                    root.activePlayer.togglePlaying();
                else if (event.button === Qt.BackButton)
                    root.activePlayer.previous();
                else
                    root.activePlayer.next();
            }
        }
    }
}
