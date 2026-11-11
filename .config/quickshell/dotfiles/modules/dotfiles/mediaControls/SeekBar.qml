import qs.modules.common
import qs.modules.common.widgets
import QtQuick

// Seek bar for the media panel. Deliberately not StyledSlider: its SmoothedAnimation
// behaviour freezes when the value is updated every frame by a running track. The wavy
// fill is the same WavyLine StyledSlider draws, so the look is unchanged.
Item {
    id: root
    property real value: 0 // 0..1
    property bool interactive: true
    property bool playing: false // animate the wave only while audio runs
    property color fillColor: Appearance.colors.colPrimary
    property color trackColor: Appearance.colors.colSecondaryContainer
    property real trackHeight: 4
    signal moved(real value)

    implicitHeight: 24
    readonly property real margin: 4
    readonly property real usableWidth: Math.max(0, width - margin * 2)
    readonly property real position: Math.max(0, Math.min(1, root.value))

    Rectangle { // Track
        anchors.verticalCenter: parent.verticalCenter
        x: root.margin
        width: root.usableWidth
        height: root.trackHeight
        radius: height / 2
        color: root.trackColor
    }

    WavyLine { // Fill
        id: waveFill
        anchors.verticalCenter: parent.verticalCenter
        x: root.margin
        width: root.position * root.usableWidth
        height: root.trackHeight
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

    Rectangle { // Handle
        anchors.verticalCenter: parent.verticalCenter
        x: root.margin + root.position * root.usableWidth - width / 2
        width: mouseArea.dragging ? 6 : 3
        height: 20
        radius: width / 2
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
