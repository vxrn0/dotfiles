import qs.modules.common
import qs.modules.common.widgets
import QtQuick

// Seek bar for the media panel. Deliberately not StyledSlider: its SmoothedAnimation
// behaviour freezes when the value is updated every frame by a running track. The look
// copies StyledSlider's Wavy configuration: 4px round track, 24px handle, stop dot.
Item {
    id: root
    property real value: 0 // 0..1
    property bool interactive: true
    property bool playing: false // animate the wave only while audio runs
    property color fillColor: Appearance.colors.colPrimary
    property color trackColor: Appearance.colors.colSecondaryContainer
    signal moved(real value)

    implicitHeight: 24
    readonly property real margin: 4
    readonly property real usableWidth: Math.max(0, width - margin * 2)
    readonly property real position: Math.max(0, Math.min(1, root.value))
    readonly property real trackHeight: 4
    readonly property real handleWidth: mouseArea.dragging ? 6 : 3

    Rectangle { // Track
        anchors.verticalCenter: parent.verticalCenter
        x: root.margin
        width: root.usableWidth
        height: root.trackHeight
        radius: height / 2
        color: root.trackColor
    }

    WavyLine { // Fill, same wave StyledSlider draws
        id: waveFill
        anchors.verticalCenter: parent.verticalCenter
        x: root.margin
        width: Math.max(0, root.position * root.usableWidth - root.handleWidth / 2)
        height: root.height // the wave needs the full height, a 4px canvas clips it flat
        lineWidth: root.trackHeight
        fullLength: root.usableWidth
        amplitudeMultiplier: 0.5
        frequency: 6
        color: root.fillColor
        visible: root.position > 0

        Connections {
            target: root
            function onPositionChanged() { waveFill.requestPaint(); }
            function onFillColorChanged() { waveFill.requestPaint(); }
        }

        FrameAnimation {
            running: root.playing
            onTriggered: waveFill.requestPaint()
        }
    }

    Rectangle { // Stop dot at the end of the track
        anchors.verticalCenter: parent.verticalCenter
        x: root.margin + root.usableWidth - width
        width: 3
        height: 3
        radius: height / 2
        color: Appearance.m3colors.m3onSecondaryContainer
    }

    Rectangle { // Handle
        anchors.verticalCenter: parent.verticalCenter
        x: root.margin + root.position * root.usableWidth - width / 2
        width: root.handleWidth
        height: 24
        radius: Appearance.rounding.full
        color: root.fillColor

        Behavior on width {
            animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
        }
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        property bool dragging: false
        enabled: root.interactive
        cursorShape: root.interactive ? Qt.PointingHandCursor : Qt.ArrowCursor

        onPressed: (mouse) => {
            dragging = true;
            apply(mouse.x);
        }
        onPositionChanged: (mouse) => {
            if (dragging)
                apply(mouse.x);
        }
        onReleased: dragging = false

        function apply(px) {
            root.moved(Math.max(0, Math.min(1, (px - root.margin) / root.usableWidth)));
        }
    }
}
