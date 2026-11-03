import qs.modules.common
import qs.modules.common.widgets
import QtQuick

// Round icon button for the assistant panel header.
RippleButton {
    id: root
    property string glyph
    implicitWidth: 32
    implicitHeight: 32
    buttonRadius: Appearance.rounding.full
    colBackground: "transparent"
    colBackgroundHover: Appearance.colors.colLayer2Hover
    colRipple: Appearance.colors.colLayer2Active

    contentItem: MaterialSymbol {
        anchors.centerIn: parent
        text: root.glyph
        iconSize: 18
        color: Appearance.colors.colOnLayer1
    }
}
