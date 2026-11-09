import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes

// One system metric: the value is a ring, the icon sits inside it, the number beside.
// Drawn as a stroke, not a filled disc: ClippedFilledCircularProgress always fills and
// holes out the icon, which turned every metric into a grey blob.
Item {
    id: root
    required property string iconName
    required property double percentage
    property int warningThreshold: 100
    property bool shown: true
    readonly property bool warning: percentage * 100 >= warningThreshold
    readonly property color accentColor: warning ? Appearance.colors.colError : Appearance.colors.colOnSecondaryContainer

    clip: true
    visible: width > 0 && height > 0
    implicitWidth: shown ? metric.implicitWidth : 0
    implicitHeight: Appearance.sizes.barHeight

    RowLayout {
        id: metric
        anchors.verticalCenter: parent.verticalCenter
        x: root.shown ? 0 : -metric.implicitWidth
        spacing: 5

        Item {
            id: ring
            Layout.alignment: Qt.AlignVCenter
            implicitWidth: 22
            implicitHeight: 22

            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer

                ShapePath { // track
                    strokeColor: Appearance.colors.colOutlineVariant
                    strokeWidth: 2
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap

                    PathAngleArc {
                        centerX: ring.width / 2
                        centerY: ring.height / 2
                        radiusX: ring.width / 2 - 1
                        radiusY: ring.height / 2 - 1
                        startAngle: -90
                        sweepAngle: 359.99
                    }
                }

                ShapePath { // value
                    strokeColor: root.accentColor
                    strokeWidth: 2
                    fillColor: "transparent"
                    capStyle: ShapePath.RoundCap

                    PathAngleArc {
                        centerX: ring.width / 2
                        centerY: ring.height / 2
                        radiusX: ring.width / 2 - 1
                        radiusY: ring.height / 2 - 1
                        startAngle: -90
                        sweepAngle: Math.max(0, Math.min(1, root.percentage)) * 359.99
                    }
                }
            }

            MaterialSymbol {
                anchors.centerIn: parent
                fill: 1
                text: root.iconName
                iconSize: 13
                color: root.accentColor
            }
        }

        StyledText {
            Layout.alignment: Qt.AlignVCenter
            // Fixed box: a value that ticks every second must not move the bar.
            Layout.preferredWidth: 24
            horizontalAlignment: Text.AlignLeft
            text: Math.round(root.percentage * 100)
            color: root.warning ? Appearance.colors.colError : Appearance.colors.colOnLayer1
            font {
                pixelSize: Appearance.font.pixelSize.smallie
                weight: Font.Medium
            }
        }
    }

    Behavior on x {
        animation: Appearance.animation.elementMove.numberAnimation.createObject(this)
    }

    Behavior on implicitWidth {
        animation: Appearance.animation.elementMove.numberAnimation.createObject(this)
    }
}
