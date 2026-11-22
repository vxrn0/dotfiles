import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import Qt5Compat.GraphicalEffects
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

RippleButton {
    id: root
    required property string materialSymbol
    required property bool current
    horizontalPadding: 10
    // The style's 24px left/right padding wins over horizontalPadding and squeezes the content.
    leftPadding: horizontalPadding
    rightPadding: horizontalPadding

    implicitHeight: 40
    implicitWidth: implicitContentWidth + horizontalPadding * 2
    buttonRadius: height / 2

    colBackground: ColorUtils.transparentize(Appearance.colors.colSurfaceContainer)
    colBackgroundHover: ColorUtils.transparentize(Appearance.colors.colOnSurface, current ? 1 : 0.95)
    colRipple: ColorUtils.transparentize(Appearance.colors.colOnSurface, 0.95)

    contentItem: Row {
        id: contentRow
        anchors.centerIn: parent
        spacing: 6

        MaterialSymbol {
            id: icon
            anchors.verticalCenter: parent.verticalCenter
            iconSize: 22
            text: root.materialSymbol
        }
        StyledText {
            id: label
            anchors.verticalCenter: parent.verticalCenter
            text: root.text
        }
    }
}
