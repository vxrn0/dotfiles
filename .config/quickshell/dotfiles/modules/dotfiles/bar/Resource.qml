import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import QtQuick.Layouts

// One system metric as a Material 3 stat: a tonal glyph plus a tabular percentage.
// The ring that used to sit here showed no value, so the three metrics were
// indistinguishable blobs.
Item {
    id: root
    required property string iconName
    required property double percentage
    property int warningThreshold: 100
    property bool shown: true
    property bool warning: percentage * 100 >= warningThreshold

    clip: true
    visible: width > 0 && height > 0
    implicitWidth: resourceRowLayout.x < 0 ? 0 : resourceRowLayout.implicitWidth
    implicitHeight: Appearance.sizes.barHeight

    RowLayout {
        id: resourceRowLayout
        spacing: 4
        x: shown ? 0 : -resourceRowLayout.width
        anchors.verticalCenter: parent.verticalCenter

        MaterialSymbol {
            Layout.alignment: Qt.AlignVCenter
            text: root.iconName
            iconSize: 16
            fill: 1
            color: root.warning ? Appearance.colors.colError : Appearance.colors.colOnLayer1
        }

        StyledText {
            Layout.alignment: Qt.AlignVCenter
            // Fixed box, right-aligned: percentages change every second and must not
            // push the rest of the bar around.
            Layout.preferredWidth: 26
            horizontalAlignment: Text.AlignRight
            text: Math.round(root.percentage * 100) + "%"
            color: root.warning ? Appearance.colors.colError : Appearance.colors.colOnLayer0
            font {
                pixelSize: Appearance.font.pixelSize.smallest
                weight: Font.Medium
            }
        }
    }

    Behavior on x {
        animation: Appearance.animation.elementMove.numberAnimation.createObject(this)
    }

    Behavior on implicitWidth {
        NumberAnimation {
            duration: Appearance.animation.elementMove.duration
            easing.type: Appearance.animation.elementMove.type
            easing.bezierCurve: Appearance.animation.elementMove.bezierCurve
        }
    }
}
