pragma ComponentBehavior: Bound
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick

// Previous/next button in the media panel. Extracted from PlayerControl.qml because
// the inline `component` form failed to resolve in this shell.
RippleButton {
    id: root
    implicitWidth: 24
    implicitHeight: 24

    property var iconName
    property var blendedColors

    colBackground: ColorUtils.transparentize(root.blendedColors.colSecondaryContainer, 1)
    colBackgroundHover: root.blendedColors.colSecondaryContainerHover
    colRipple: root.blendedColors.colSecondaryContainerActive

    contentItem: MaterialSymbol {
        iconSize: Appearance.font.pixelSize.huge
        fill: 1
        horizontalAlignment: Text.AlignHCenter
        color: root.blendedColors.colOnSecondaryContainer
        text: root.iconName

        Behavior on color {
            animation: Appearance.animation.elementMoveFast.colorAnimation.createObject(this)
        }
    }
}
