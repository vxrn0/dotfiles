import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import QtQuick.Layouts

// One system metric, as a Material 3 stat: the number carries the meaning, the icon
// only labels it. Over threshold the segment becomes an error container; the padded
// hit area stays the same, so crossing the threshold never shifts the bar.
Item {
    id: root
    required property string iconName
    required property double percentage
    property int warningThreshold: 100
    property bool shown: true
    readonly property bool warning: percentage * 100 >= warningThreshold

    clip: true
    implicitWidth: shown ? content.width + 12 : 0
    implicitHeight: Appearance.sizes.barHeight

    Rectangle {
        id: stateLayer
        anchors.verticalCenter: parent.verticalCenter
        x: content.x - 6
        width: content.width + 12
        height: 26
        radius: height / 2
        color: root.warning ? Appearance.colors.colErrorContainer : "transparent"

        Behavior on color {
            ColorAnimation {
                duration: Appearance.animation.elementMoveFast.duration
                easing.type: Easing.OutCubic
            }
        }
    }

    RowLayout {
        id: content
        anchors.verticalCenter: parent.verticalCenter
        x: root.shown ? 6 : -width
        spacing: 5

        MaterialSymbol {
            Layout.alignment: Qt.AlignVCenter
            text: root.iconName
            iconSize: 16
            fill: 0
            color: root.warning ? Appearance.colors.colOnErrorContainer : Appearance.colors.colOnLayer1
        }

        StyledText {
            Layout.alignment: Qt.AlignVCenter
            // Fixed box, centred: values tick every second and must not push the bar
            // around, and short values should not leave one side of the metric empty.
            Layout.preferredWidth: 32
            horizontalAlignment: Text.AlignHCenter
            text: Math.round(root.percentage * 100) + "%"
            color: root.warning ? Appearance.colors.colOnErrorContainer : Appearance.colors.colOnLayer0
            font {
                pixelSize: Appearance.font.pixelSize.smallie
                weight: Font.Medium
            }
        }
    }

    Behavior on x {
        animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
    }

    Behavior on implicitWidth {
        animation: Appearance.animation.elementMove.numberAnimation.createObject(this)
    }
}
