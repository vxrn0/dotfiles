import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions

// Which agent CLIs are running, as 20px glyphs in the shared item scale. The whole
// cluster disappears when nothing is running.
RowLayout {
    id: root
    spacing: 6

    property bool codexRunning: false
    property bool claudeRunning: false
    property bool opencodeRunning: false
    readonly property bool anyRunning: codexRunning || claudeRunning || opencodeRunning
    property var found: ({})

    Process {
        id: probe
        command: ["bash", `${FileUtils.trimFileProtocol(Directories.scriptPath)}/ai/harness-active.sh`]
        stdout: SplitParser {
            onRead: line => {
                const next = Object.assign({}, root.found);
                next[line] = true;
                root.found = next;
            }
        }
        // Apply once per probe, so icons never blink when nothing changed
        onExited: {
            root.codexRunning = !!root.found.codex;
            root.claudeRunning = !!root.found.claude;
            root.opencodeRunning = !!root.found.opencode;
            root.found = ({});
        }
    }

    Timer {
        interval: 3000
        running: true
        repeat: true
        triggeredOnStart: true
        onTriggered: probe.running = true
    }

    Repeater {
        model: [
            { running: root.codexRunning, glyph: "terminal" },
            { running: root.claudeRunning, glyph: "auto_awesome" },
            { running: root.opencodeRunning, glyph: "code" }
        ]

        delegate: Item {
            id: iconItem
            required property var modelData
            implicitWidth: modelData.running ? 20 : 0
            implicitHeight: 20
            opacity: modelData.running ? 1 : 0
            visible: opacity > 0

            Behavior on implicitWidth {
                animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
            }
            Behavior on opacity {
                animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
            }

            // Brand marks are too dense to read at 20px; the bar's own glyph set stays legible
            MaterialSymbol {
                anchors.centerIn: parent
                text: iconItem.modelData.glyph
                iconSize: 20
                fill: 0
                color: Appearance.colors.colOnLayer1
            }
        }
    }
}
