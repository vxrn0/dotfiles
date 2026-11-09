import qs.services
import QtQuick

// Media is one glyph-only item in the same anatomy as the metrics: the playing state
// is the active color, the glyph is the control.
BarItem {
    id: root
    readonly property var activePlayer: MprisController.activePlayer

    glyph: root.activePlayer?.isPlaying ? "pause" : "play_arrow"
    active: root.activePlayer?.isPlaying ?? false
    interactive: true

    onClicked: root.activePlayer?.togglePlaying()
    onMiddleClicked: root.activePlayer?.togglePlaying()
    onRightClicked: root.activePlayer?.next()
}
