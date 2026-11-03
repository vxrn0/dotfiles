import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import QtQuick.Layouts

// One tool call, rendered like a console transcript line: status glyph, the command in
// monospace, its duration on the right, and a short output preview underneath.
Item {
    id: root
    property string command: ""
    property string toolName: ""
    property string summary: ""
    property string preview: ""
    property string duration: ""
    property string state: "pending" // pending | running | done | failed

    implicitHeight: content.implicitHeight

    RowLayout {
        id: content
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
        }
        spacing: 10

        Item { // Status glyph
            Layout.alignment: Qt.AlignTop | Qt.AlignLeft
            implicitWidth: 16
            implicitHeight: 18

            MaterialSymbol {
                anchors.centerIn: parent
                visible: root.state === "done"
                text: "check_circle"
                iconSize: 16
                color: Appearance.colors.colPrimary
            }
            MaterialSymbol {
                anchors.centerIn: parent
                visible: root.state === "failed"
                text: "cancel"
                iconSize: 16
                color: Appearance.colors.colError
            }
            MaterialSymbol {
                anchors.centerIn: parent
                visible: root.state === "pending"
                text: "circle"
                iconSize: 14
                color: Appearance.colors.colOnLayer1Inactive
            }
            MaterialSymbol {
                anchors.centerIn: parent
                visible: root.state === "running"
                text: "progress_activity"
                iconSize: 16
                color: Appearance.colors.colPrimary

                RotationAnimator on rotation {
                    from: 0
                    to: 360
                    duration: 900
                    loops: Animation.Infinite
                    running: root.state === "running"
                }
            }
        }

        ColumnLayout { // Command, summary, output
            Layout.fillWidth: true
            spacing: 2

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                StyledText {
                    visible: root.toolName.length > 0
                    text: root.toolName
                    font {
                        family: Appearance.font.family.monospace
                        pixelSize: Appearance.font.pixelSize.smaller
                    }
                    color: Appearance.colors.colSubtext
                }
                StyledText {
                    Layout.fillWidth: true
                    text: root.command.length > 0 ? root.command : Translation.tr("working…")
                    elide: Text.ElideRight
                    font {
                        family: Appearance.font.family.monospace
                        pixelSize: Appearance.font.pixelSize.smaller
                    }
                    color: root.state === "pending"
                        ? Appearance.colors.colOnLayer1Inactive
                        : Appearance.colors.colOnLayer1
                }
                StyledText {
                    visible: root.duration.length > 0
                    text: root.duration
                    font {
                        family: Appearance.font.family.monospace
                        pixelSize: Appearance.font.pixelSize.smallest
                    }
                    color: Appearance.colors.colSubtext
                }
            }

            StyledText {
                Layout.fillWidth: true
                visible: root.summary.length > 0
                text: root.summary
                elide: Text.ElideRight
                font.pixelSize: Appearance.font.pixelSize.smallest
                color: Appearance.colors.colSubtext
            }

            StyledText {
                Layout.fillWidth: true
                visible: root.preview.length > 0
                text: root.preview
                wrapMode: Text.WrapAnywhere
                maximumLineCount: 3
                elide: Text.ElideRight
                font {
                    family: Appearance.font.family.monospace
                    pixelSize: Appearance.font.pixelSize.smallest
                }
                color: Appearance.colors.colSubtext
                opacity: 0.85
            }
        }
    }
}
