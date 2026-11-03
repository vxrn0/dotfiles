import qs.services
import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import QtQuick.Layouts

// Slash-command list, shown above the input while the message starts with "/".
Rectangle {
    id: root
    property var items: [] // [{ name, label, detail }]
    property int activeIndex: 0
    signal activated(int index)

    implicitHeight: list.implicitHeight + 16
    radius: Appearance.rounding.normal
    color: Appearance.colors.colLayer2
    border.width: 1
    border.color: Appearance.colors.colLayer0Border

    Column {
        id: list
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: 8
        }
        spacing: 2

        StyledText {
            leftPadding: 8
            bottomPadding: 4
            text: Translation.tr("Available commands")
            font.pixelSize: Appearance.font.pixelSize.smaller
            color: Appearance.colors.colSubtext
        }

        Repeater {
            model: root.items

            Rectangle {
                id: row
                required property var modelData
                required property int index

                width: list.width
                height: 34
                radius: Appearance.rounding.small
                color: index === root.activeIndex ? Appearance.colors.colPrimaryContainer : "transparent"

                RowLayout {
                    anchors {
                        fill: parent
                        leftMargin: 8
                        rightMargin: 8
                    }
                    spacing: 10

                    StyledText {
                        text: row.modelData.name
                        font.pixelSize: Appearance.font.pixelSize.small
                        color: row.index === root.activeIndex
                            ? Appearance.colors.colOnPrimaryContainer
                            : Appearance.colors.colOnLayer2
                    }
                    StyledText {
                        Layout.fillWidth: true
                        text: row.modelData.label
                        elide: Text.ElideRight
                        font.pixelSize: Appearance.font.pixelSize.smaller
                        color: row.index === root.activeIndex
                            ? Appearance.colors.colOnPrimaryContainer
                            : Appearance.colors.colSubtext
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onEntered: root.activeIndex = row.index
                    onClicked: root.activated(row.index)
                }
            }
        }
    }
}
