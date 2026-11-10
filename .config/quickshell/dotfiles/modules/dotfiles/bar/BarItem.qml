import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import QtQuick.Layouts

// The bar's single item anatomy: optional 16px glyph, 13px medium label, optional
// trailing glyph. Every cluster uses this, so icons, text and values stay on one scale.
// The glyph is the label (onSurfaceVariant), the text is the content (onSurface);
// state is shown by color only, never by adding a second surface inside the pill.
Item {
    id: root
    property string glyph: ""
    property string trailingGlyph: ""
    property string label: ""
    property string iconSource: "" // brand marks, e.g. the harness badges
    property real valueWidth: 0 // fixed box for values that change every second
    property bool active: false
    property bool warning: false
    property bool interactive: false
    property bool shown: true

    signal clicked()
    signal middleClicked()
    signal rightClicked()

    readonly property bool hovered: itemMouse.containsMouse && interactive
    readonly property color glyphColor: warning ? Appearance.colors.colError
        : (active || hovered) ? Appearance.colors.colPrimary
        : Appearance.colors.colOnLayer1
    readonly property color labelColor: warning ? Appearance.colors.colError
        : (active || hovered) ? Appearance.colors.colPrimary
        : Appearance.colors.colOnLayer0

    implicitWidth: shown ? content.implicitWidth + 10 : 0
    implicitHeight: 28
    clip: true
    transformOrigin: Item.Center

    scale: interactive && itemMouse.pressed ? 0.94 : 1
    Behavior on scale {
        NumberAnimation {
            duration: 120
            easing.type: Easing.OutCubic
        }
    }
    Behavior on implicitWidth {
        animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
    }

    RowLayout {
        id: content
        anchors.centerIn: parent
        spacing: 6
        visible: root.shown

        MaterialSymbol {
            visible: root.iconSource.length === 0 && root.glyph.length > 0
            text: root.glyph
            iconSize: 20
            fill: 0
            color: root.glyphColor
        }

        CustomIcon {
            visible: root.iconSource.length > 0
            Layout.preferredWidth: 18
            Layout.preferredHeight: 18
            width: 18
            height: 18
            source: root.iconSource
            colorize: true
            color: root.glyphColor
        }

        StyledText {
            visible: root.label.length > 0
            Layout.preferredWidth: root.valueWidth > 0 ? root.valueWidth : -1
            horizontalAlignment: Text.AlignLeft
            text: root.label
            color: root.labelColor
            font {
                pixelSize: Appearance.font.pixelSize.normal
                weight: Font.DemiBold
                features: ["tnum"]
                variableAxes: ({ "wght": 650 })
            }
        }

        MaterialSymbol {
            visible: root.trailingGlyph.length > 0
            text: root.trailingGlyph
            iconSize: 20
            fill: 0
            color: root.glyphColor
        }
    }

    MouseArea {
        id: itemMouse
        anchors.fill: parent
        enabled: root.interactive
        hoverEnabled: root.interactive
        cursorShape: root.interactive ? Qt.PointingHandCursor : Qt.ArrowCursor
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
        onPressed: (event) => {
            if (event.button === Qt.LeftButton)
                root.clicked();
            else if (event.button === Qt.MiddleButton)
                root.middleClicked();
            else
                root.rightClicked();
        }
    }
}
