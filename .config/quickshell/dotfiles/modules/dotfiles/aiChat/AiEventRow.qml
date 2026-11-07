import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import QtQuick.Layouts

// One tool call, rendered as a compact Codex-style event row: a status glyph,
// the command in monospace, and its duration on the right. Clicking expands the
// short output preview underneath.
Item {
    id: root
    property string command: ""
    property string toolName: ""
    property string preview: ""
    property string duration: ""
    property string state: "pending" // pending | running | done | failed
    property bool expanded: false

    implicitHeight: content.implicitHeight

    ColumnLayout {
        id: content
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
        }
        spacing: 2

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            // Status glyph
            Item {
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: 14
                implicitHeight: 14

                MaterialSymbol {
                    anchors.centerIn: parent
                    visible: root.state === "done"
                    text: "check"
                    iconSize: 14
                    color: Appearance.colors.colPrimary
                }
                MaterialSymbol {
                    anchors.centerIn: parent
                    visible: root.state === "failed"
                    text: "close"
                    iconSize: 14
                    color: Appearance.colors.colError
                }
                MaterialSymbol {
                    anchors.centerIn: parent
                    visible: root.state === "pending"
                    text: "circle"
                    iconSize: 12
                    color: Appearance.colors.colOnLayer1Inactive
                }
                MaterialSymbol {
                    anchors.centerIn: parent
                    visible: root.state === "running"
                    text: "progress_activity"
                    iconSize: 14
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

            StyledText {
                visible: root.toolName.length > 0 && root.toolName !== "bash"
                text: root.toolName
                font {
                    family: Appearance.font.family.monospace
                    pixelSize: Appearance.font.pixelSize.smallest
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

            MaterialSymbol {
                visible: root.preview.length > 0
                text: root.expanded ? "expand_less" : "expand_more"
                iconSize: 14
                color: Appearance.colors.colSubtext
            }
        }

        StyledText {
            Layout.fillWidth: true
            visible: root.expanded && root.preview.length > 0
            text: root.preview
            wrapMode: Text.WrapAnywhere
            font {
                family: Appearance.font.family.monospace
                pixelSize: Appearance.font.pixelSize.smallest
            }
            color: Appearance.colors.colSubtext
            opacity: 0.85
            leftPadding: 22
        }
    }

    MouseArea {
        anchors.fill: parent
        enabled: root.preview.length > 0
        cursorShape: Qt.PointingHandCursor
        onClicked: root.expanded = !root.expanded
    }
}
