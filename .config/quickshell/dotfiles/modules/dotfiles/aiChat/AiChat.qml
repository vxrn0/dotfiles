pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import qs.modules.common.functions

// System assistant panel. Talks to Codex through scripts/ai/codex-run.py and
// keeps the surface minimal: the task, what the agent is doing, and the result.
// Provider and model selection live behind slash commands, not in the chrome.
PanelWindow {
    id: root
    visible: GlobalStates.aiChatOpen
    exclusiveZone: 0
    exclusionMode: ExclusionMode.Ignore
    WlrLayershell.namespace: "quickshell:aiChat"
    WlrLayershell.keyboardFocus: GlobalStates.aiChatOpen ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
    color: "transparent"

    // Covers the whole output so a click anywhere outside the card can be caught here.
    // A Hyprland focus grab was not usable: it is cleared about two seconds after the
    // panel opens, which dismissed the panel on its own.
    anchors {
        top: true
        left: true
        right: true
        bottom: true
    }

    // Clicking anywhere outside dismisses the panel, the same way the quick settings
    // sidebar does. The grab is armed a frame later: Hyprland clears it if the layer
    // surface is not mapped yet, which would close the panel on its first frame.
    onVisibleChanged: {
        if (visible) {
            locate.running = true;
            modelsList.running = true;
            focusTimer.start();
        }
    }

    Timer {
        id: focusTimer
        interval: 120
        onTriggered: input.forceActiveFocus()
    }

    // ---- state ---------------------------------------------------------------
    property string codexPath: ""
    property string statusLine: ""
    property bool running: false
    // The whole conversation, one entry per request:
    // { user, system, events: [{tool, command, preview, duration, state}], stream,
    //   answer, meta, done }
    property var turns: []
    // Seconds since the current turn started, shown while it runs
    property string elapsed: ""
    property bool sessionStarted: false
    property string sessionId: ""
    property bool stopping: false
    // Recent sessions for /sessions, filled by sessionList
    property var sessions: []

    // Run metrics for the footer summary
    property double runStartedAt: 0
    property int runTokens: 0
    property string footer: ""

    property string model: ""
    property string effort: "medium"
    // read-only | workspace | full, passed straight to Codex as a sandbox mode
    property string sandbox: "workspace"
    // Where Codex may read and edit; empty until the locate pass fills it in
    property string workdir: ""
    // Models this account can run, filled by modelsList
    property var models: []

    // ---- slash commands ------------------------------------------------------
    property string paletteMode: "" // "" | command | model | effort | sandbox | sessions
    property string paletteQuery: ""
    property int paletteIndex: 0

    readonly property var allCommands: [
        { name: "/model", label: Translation.tr("Choose a model") },
        { name: "/effort", label: Translation.tr("Set reasoning effort") },
        { name: "/sandbox", label: Translation.tr("Choose what Codex may touch") },
        { name: "/new", label: Translation.tr("Start a new chat") },
        { name: "/sessions", label: Translation.tr("Switch to a past chat") },
        { name: "/stop", label: Translation.tr("Stop the running task") },
    ]
    readonly property var sandboxLevels: [
        { name: "read-only", label: Translation.tr("Read only, no edits") },
        { name: "workspace", label: Translation.tr("Edit files in the workspace") },
        { name: "full", label: Translation.tr("Full access, no sandbox") },
    ]
    // Codex publishes which efforts each model supports; show those, not a fixed three.
    readonly property var effortLevels: {
        const entry = root.models.find(item => item.name === root.model);
        const levels = (entry?.efforts ?? "low,medium,high").split(",").filter(level => level.length > 0);
        return levels.map(level => ({ name: level, label: level }));
    }

    readonly property var paletteItems: {
        // In the session list the input holds the command itself, so the filter only
        // applies to what was typed after it.
        const query = (paletteMode === "sessions"
            ? paletteQuery.replace(/^\/sessions\s*/, "")
            : paletteQuery).toLowerCase();
        let source = [];
        if (paletteMode === "command")
            source = allCommands;
        else if (paletteMode === "model")
            source = models;
        else if (paletteMode === "effort")
            source = effortLevels;
        else if (paletteMode === "sandbox")
            source = sandboxLevels;
        else if (paletteMode === "sessions")
            source = sessions;
        // Filter as the user types, so "/new" leaves only /new in the list
        if (query.length === 0)
            return source;
        return source.filter(item => item.name.toLowerCase().startsWith(query));
    }

    // ---- helpers -------------------------------------------------------------
    function now() {
        return Qt.formatTime(new Date(), "hh:mm");
    }

    function resetChat() {
        chat.running = false;
        root.running = false;
        root.turns = [];
        root.statusLine = "";
        root.sessionStarted = false;
        root.sessionId = "";
        root.paletteMode = "";
        root.footer = "";
        root.runTokens = 0;
        root.elapsed = "";
        input.text = "";
    }

    function switchSession(item) {
        root.sessionId = item.name;
        root.sessionStarted = true;
        root.turns = [{
            user: "",
            system: Translation.tr("Switched to %1\n%2").arg(item.name).arg(item.label),
            events: [], stream: "", answer: "", meta: "", done: true,
        }];
        root.statusLine = "";
        root.footer = "";
        root.paletteMode = "";
        input.text = "";
    }

    // The turn the engine is currently writing into.
    function patchTurn(patch) {
        const next = root.turns.slice();
        if (next.length === 0)
            return;
        next[next.length - 1] = Object.assign({}, next[next.length - 1], patch);
        root.turns = next;
    }

    function addStep(id, tool, label) {
        const turn = root.turns[root.turns.length - 1];
        if (!turn)
            return;
        const events = turn.events.slice();
        events.push({
            id: id ?? "",
            tool: tool || label || "bash",
            command: "",
            preview: "",
            duration: "",
            state: "running",
            startedAt: Date.now(),
        });
        root.patchTurn({ events: events });
    }

    function updateStep(id, patch) {
        const turn = root.turns[root.turns.length - 1];
        if (!turn)
            return;
        const events = turn.events.slice();
        for (let i = events.length - 1; i >= 0; i--) {
            if (events[i].id === id) {
                events[i] = Object.assign({}, events[i], patch);
                root.patchTurn({ events: events });
                return;
            }
        }
    }

    // The adapter wraps tool output in <untrusted-output> and prefixes a meta line
    // ("exit 0 · 0.0s · 2 lines · log <path>"). Both are noise here: the exit status is
    // already the step icon and the timing is our own measurement.
    function cleanOutput(output) {
        return (output ?? "")
            .split("\n")
            .filter(line => {
                const t = line.trim();
                return t !== "<untrusted-output>" && t !== "</untrusted-output>"
                    && !t.startsWith("exit ");
            })
            .join("\n")
            .trim();
    }

    function finishStep(id, failed, output) {
        const turn = root.turns[root.turns.length - 1];
        if (!turn)
            return;
        const events = turn.events.slice();
        for (let i = events.length - 1; i >= 0; i--) {
            if (events[i].id === id) {
                const seconds = (Date.now() - events[i].startedAt) / 1000;
                events[i] = Object.assign({}, events[i], {
                    state: failed ? "failed" : "done",
                    preview: root.cleanOutput(output).split("\n").slice(0, 3).join("\n"),
                    duration: seconds >= 0.1 ? seconds.toFixed(1) + "s" : "",
                });
                root.patchTurn({ events: events });
                return;
            }
        }
    }

    function handleEvent(line) {
        let row;
        try { row = JSON.parse(line); } catch (e) { return; }

        // Every row carries the session id; keeping it lets /sessions and later
        // turns address the same transcript.
        if (row.session_id)
            root.sessionId = row.session_id;

        if (row.type === "progress") {
            // Phases the adapter emits: generating, tool_started, tool_ran,
            // tool_finished, provider_retry, steered, persisting, compacted, compact_ceiling.
            if (row.phase === "tool_started") {
                root.addStep(row.call_id, row.tool, row.label);
            } else if (row.phase === "tool_ran") {
                // `detail` carries the command that is about to run
                root.updateStep(row.call_id, { command: row.detail || "" });
            } else if (row.phase === "tool_finished") {
                root.finishStep(row.call_id, !!row.error, row.output);
            } else if (row.phase === "text") {
                if (row.delta) {
                    const turn = root.turns[root.turns.length - 1];
                    if (turn)
                        root.patchTurn({ stream: turn.stream + row.delta });
                }
            } else if (row.phase === "generating") {
                root.statusLine = Translation.tr("Thinking…");
            } else if (row.phase === "provider_retry") {
                root.statusLine = Translation.tr("Retrying: %1").arg(row.detail || "");
            } else if (row.phase === "steered") {
                root.statusLine = Translation.tr("Adjusting to new instructions…");
            } else if (row.phase === "persisting") {
                root.statusLine = Translation.tr("Saving…");
            } else if (row.phase === "compacted" || row.phase === "compact_ceiling") {
                root.statusLine = Translation.tr("Compacted context");
            }
        } else if (row.type === "result") {
            const turn = root.turns[root.turns.length - 1];
            // The stream already carries intermediate commentary; the result is the
            // answer. Keep both: prose first, the final word last.
            const carried = (turn?.stream ?? "").trim();
            const text = row.text || carried;
            root.patchTurn({ answer: text, stream: "", done: true });
            root.statusLine = "";
            root.finishRun(row.usage);
        } else if (row.type === "error") {
            // An error row may carry a `hint` naming the command that fixes it;
            // its own docs say to read the hint instead of retrying blindly.
            root.patchTurn({
                answer: Translation.tr("Error: %1").arg(row.message || row.code || ""),
                stream: "",
                done: true,
            });
            root.statusLine = row.hint || "";
        }
    }

    // Matches the terminal summary: "12s · 120 tokens/s"
    function finishRun(usage) {
        const seconds = (Date.now() - root.runStartedAt) / 1000;
        const tokens = usage?.output_tokens ?? usage?.total_tokens ?? 0;
        root.runTokens = tokens;
        let meta = "";
        if (seconds >= 0.5)
            meta = seconds.toFixed(0) + "s";
        if (tokens > 0)
            meta += (meta.length > 0 ? " · " : "") + tokens + " tokens";
        root.footer = meta;
        root.patchTurn({ meta: meta });
    }

    function activateItem(item) {
        if (!item)
            return;
        if (root.paletteMode === "model") {
            // Straight into the running state: the next turn reads these properties,
            // there is no config file to write.
            root.model = item.name;
            root.paletteMode = "";
        } else if (root.paletteMode === "effort") {
            root.effort = item.name;
            root.paletteMode = "";
        } else if (root.paletteMode === "sandbox") {
            root.sandbox = item.name;
            root.paletteMode = "";
        } else if (root.paletteMode === "sessions") {
            root.switchSession(item);
        } else {
            input.text = "";
            root.runCommand(item.name);
        }
    }

    function runCommand(text) {
        const parts = text.trim().split(/\s+/);
        const name = parts[0];

        if (name === "/new") {
            root.resetChat();
            return;
        }
        if (name === "/stop") {
            if (!root.running) {
                // The status line, not the answer: /stop typed after a finished task
                // must not wipe the result that is already on screen.
                root.statusLine = Translation.tr("Nothing is running.");
            } else {
                root.stopping = true;
                chat.running = false;
                // The adapter runs under setsid, so its pid is the group id and one
                // signal reaches Codex and every command it started.
                cancelTask.command = ["bash", "-c", `kill -TERM -- -${chat.processId} 2>/dev/null; true`];
                cancelTask.running = true;
            }
            input.text = "";
            root.paletteMode = "";
            return;
        }
        if (name === "/sessions") {
            if (parts.length > 1) {
                const picked = root.sessions.find(item => item.name.startsWith(parts[1]));
                if (picked) {
                    root.switchSession(picked);
                    return;
                }
            }
            root.paletteMode = "sessions";
            root.paletteIndex = 0;
            input.text = "/sessions";
            sessionList.running = true;
            return;
        }
        if (name === "/model" && parts.length === 1) {
            root.paletteMode = "model";
            root.paletteIndex = 0;
            return;
        }
        if (name === "/effort" && parts.length === 1) {
            root.paletteMode = "effort";
            root.paletteIndex = 0;
            return;
        }
        if (name === "/sandbox" && parts.length === 1) {
            root.paletteMode = "sandbox";
            root.paletteIndex = 0;
            return;
        }
        input.text = "";
        root.paletteMode = "";
    }

    function send() {
        const text = input.text.trim();
        if (text.length === 0)
            return;

        if (text.startsWith("/")) {
            root.runCommand(text);
            return;
        }
        if (root.running)
            return;
        if (root.codexPath.length === 0) {
            root.turns = root.turns.concat([{
                user: "", system: Translation.tr("Codex CLI is not installed, or not logged in."),
                events: [], stream: "", answer: "", meta: "", done: true,
            }]);
            input.text = "";
            return;
        }

        root.runStartedAt = Date.now();
        root.footer = "";
        root.elapsed = "0s";
        root.turns = root.turns.concat([{
            user: text,
            system: "",
            events: [],
            stream: "",
            answer: "",
            meta: "",
            done: false,
        }]);
        root.statusLine = "";
        input.text = "";

        // setsid puts the adapter and its Codex child in one process group, so
        // /stop can take down the whole tree with a single signal.
        const args = ["setsid", "python3", `${FileUtils.trimFileProtocol(Directories.scriptPath)}/ai/codex-run.py`,
            "--prompt", text, "--model", root.model, "--effort", root.effort,
            "--sandbox", root.sandbox];
        if (root.workdir.length > 0)
            args.push("--cwd", root.workdir);
        if (root.sessionId.length > 0)
            args.push("--session", root.sessionId);
        if (root.codexPath.length > 0)
            args.push("--codex", root.codexPath);
        chat.command = args;
        chat.running = true;
    }

    // ---- processes -----------------------------------------------------------
    Process {
        id: locate
        // Labelled lines: an empty `command -v` would otherwise collapse and shift
        // the home directory into the codex slot.
        command: ["bash", "-lc", "printf 'codex=%s\\n' \"$(command -v codex)\"\nprintf 'home=%s\\n' \"$HOME\""]
        stdout: StdioCollector {
            onStreamFinished: {
                for (const line of text.split("\n")) {
                    if (line.startsWith("codex="))
                        root.codexPath = line.slice(6).trim();
                    else if (line.startsWith("home="))
                        root.workdir = line.slice(5).trim();
                }
                if (root.codexPath.length === 0)
                    root.codexPath = `${root.workdir}/.local/node/bin/codex`;
            }
        }
    }

    Process {
        id: cancelTask
    }

    Process {
        id: sessionList
        command: ["python3", `${FileUtils.trimFileProtocol(Directories.scriptPath)}/ai/codex-info.py`, "sessions", "12"]
        stdout: StdioCollector {
            onStreamFinished: {
                const rows = [];
                for (const line of text.split("\n")) {
                    if (line.length === 0)
                        continue;
                    const fields = line.split("\t");
                    // Second field is an epoch, so the time is rendered here where the
                    // panel can use the local timezone.
                    const clock = Qt.formatTime(new Date(Number(fields[1]) * 1000), "hh:mm");
                    rows.push({ name: fields[0], label: `${clock} · ${fields[2] ?? ""}` });
                }
                root.sessions = rows;
            }
        }
    }

    Process {
        id: modelsList
        command: ["python3", `${FileUtils.trimFileProtocol(Directories.scriptPath)}/ai/codex-info.py`, "models"]
        stdout: StdioCollector {
            onStreamFinished: {
                const rows = [];
                for (const line of text.split("\n")) {
                    if (line.length === 0)
                        continue;
                    const fields = line.split("\t");
                    rows.push({ name: fields[0], label: fields[1] ?? fields[0], efforts: fields[2] ?? "" });
                }
                if (rows.length === 0)
                    return;
                root.models = rows;
                if (root.model.length === 0)
                    root.model = rows[0].name;
            }
        }
    }

    Process {
        id: chat
        stdout: SplitParser {
            onRead: line => root.handleEvent(line)
        }
        stderr: StdioCollector {
            onStreamFinished: {
                const err = text.trim();
                if (err.length > 0)
                    root.statusLine = err.split("\n").slice(-1)[0];
            }
        }
        onStarted: root.running = true
        onExited: (code, status) => {
            root.running = false;
            const turn = root.turns[root.turns.length - 1];
            if (root.stopping) {
                root.stopping = false;
                root.statusLine = "";
                root.patchTurn({
                    // Whatever was mid-flight stays marked, otherwise its spinner keeps turning.
                    events: (turn?.events ?? []).map(event => event.state === "running"
                        ? Object.assign({}, event, { state: "failed" })
                        : event),
                    answer: Translation.tr("Stopped."),
                    stream: "",
                    done: true,
                });
                return;
            }
            if (turn && !turn.done && turn.answer.length === 0) {
                const carried = turn.stream.trim();
                if (carried.length > 0)
                    root.patchTurn({ answer: carried, stream: "", done: true });
            }
            if (code === 0)
                root.sessionStarted = true;
        }
    }

    // Elapsed counter for the running turn; one authored motion, the spinning glyph.
    Timer {
        interval: 1000
        running: root.running
        repeat: true
        onTriggered: root.elapsed = Math.floor((Date.now() - root.runStartedAt) / 1000) + "s"
    }

    Timer {
        interval: 400
        running: true
        repeat: false
        triggeredOnStart: true
        onTriggered: {
            locate.running = true;
            modelsList.running = true;
        }
    }

    IpcHandler {
        target: "aiChat"

        function toggle(): void {
            GlobalStates.aiChatOpen = !GlobalStates.aiChatOpen;
        }

        function close(): void {
            GlobalStates.aiChatOpen = false;
        }

        function open(): void {
            GlobalStates.aiChatOpen = true;
        }

        // Lets a keybind or a script drive the panel without typing into it.
        function ask(text: string): void {
            GlobalStates.aiChatOpen = true;
            input.text = text;
            root.send();
        }

        function stop(): void {
            root.runCommand("/stop");
        }





        // Kept for `qs ipc call aiChat newChat`; the SUPER+Tab path goes through
        // the GlobalShortcut in shell.qml.
        function newChat(): void {
            if (!GlobalStates.aiChatOpen)
                return;
            root.resetChat();
        }
    }

    Connections {
        target: GlobalStates
        function onAiChatResetRequestedChanged() {
            root.resetChat();
        }
    }

    // ---- surface -------------------------------------------------------------
    MouseArea { // Catcher: any click that misses the card closes the panel
        anchors.fill: parent
        onClicked: GlobalStates.aiChatOpen = false
    }

    StyledRectangularShadow {
        target: surface
    }
    Rectangle {
        id: surface
        anchors {
            top: parent.top
            left: parent.left
            bottom: parent.bottom
            margins: Appearance.sizes.hyprlandGapsOut
            rightMargin: 0
        }
        implicitWidth: Appearance.sizes.sidebarWidth - Appearance.sizes.hyprlandGapsOut * 2
        width: implicitWidth

        MouseArea { // Swallow clicks on the card so the catcher never sees them
            anchors.fill: parent
        }
        color: Appearance.colors.colLayer0
        border.width: 1
        border.color: Appearance.colors.colLayer0Border
        radius: Appearance.rounding.screenRounding - Appearance.sizes.hyprlandGapsOut + 1

        Keys.onPressed: event => {
            if (event.key === Qt.Key_Escape) {
                if (root.paletteMode !== "") {
                    root.paletteMode = "";
                    input.forceActiveFocus();
                } else {
                    GlobalStates.aiChatOpen = false;
                }
                event.accepted = true;
            }
        }

        ColumnLayout {
            anchors {
                fill: parent
                margins: 12
            }
            spacing: 8

            // ---- Header -----------------------------------------------------
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                StyledText {
                    text: Translation.tr("Pilot")
                    font {
                        pixelSize: Appearance.font.pixelSize.normal
                        weight: Font.DemiBold
                    }
                    color: Appearance.colors.colOnLayer0
                }

                // Live status word instead of a blinking dot: reads like an agent,
                // not a daemon.
                StyledText {
                    visible: root.running
                    text: root.statusLine.length > 0 ? root.statusLine : Translation.tr("Working")
                    font {
                        pixelSize: Appearance.font.pixelSize.small
                        weight: Font.Medium
                    }
                    color: Appearance.colors.colPrimary

                    SequentialAnimation on opacity {
                        running: root.running
                        loops: Animation.Infinite
                        NumberAnimation { to: 0.55; duration: 900; easing.type: Easing.InOutSine }
                        NumberAnimation { to: 1.0; duration: 900; easing.type: Easing.InOutSine }
                    }
                }

                StyledText {
                    visible: !root.running && root.elapsed.length > 0
                    text: root.elapsed
                    font {
                        family: Appearance.font.family.monospace
                        pixelSize: Appearance.font.pixelSize.smallest
                    }
                    color: Appearance.colors.colSubtext
                }

                Item { Layout.fillWidth: true }

                AiIconButton {
                    glyph: "close"
                    onClicked: GlobalStates.aiChatOpen = false
                }
            }

            // Workdir line, quiet.
            StyledText {
                Layout.fillWidth: true
                visible: root.workdir.length > 0
                text: root.workdir
                elide: Text.ElideMiddle
                font {
                    family: Appearance.font.family.monospace
                    pixelSize: Appearance.font.pixelSize.smallest
                }
                color: Appearance.colors.colSubtext
            }

            // ---- Transcript ---------------------------------------------
            StyledFlickable {
                id: scroller
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                contentWidth: width
                contentHeight: transcript.implicitHeight
                visible: root.turns.length > 0

                onContentHeightChanged: if (contentHeight > height) contentY = Math.max(0, contentHeight - height)

                ColumnLayout {
                    id: transcript
                    width: scroller.width
                    spacing: 14

                    Repeater {
                        model: root.turns

                        ColumnLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            spacing: 6

                            // User turn: a single readable line, right-aligned feel
                            // via a subtle leading rule, not a bubble.
                            RowLayout {
                                Layout.fillWidth: true
                                visible: modelData.user.length > 0
                                spacing: 8

                                Rectangle {
                                    Layout.alignment: Qt.AlignTop
                                    implicitWidth: 2
                                    implicitHeight: 14
                                    radius: 1
                                    color: Appearance.colors.colPrimary
                                }
                                StyledText {
                                    Layout.fillWidth: true
                                    text: modelData.user
                                    wrapMode: Text.Wrap
                                    font.pixelSize: Appearance.font.pixelSize.small
                                    color: Appearance.colors.colOnLayer0
                                }
                            }

                            // System note (session switch etc.)
                            StyledText {
                                Layout.fillWidth: true
                                visible: modelData.system.length > 0
                                text: modelData.system
                                wrapMode: Text.Wrap
                                font {
                                    family: Appearance.font.family.monospace
                                    pixelSize: Appearance.font.pixelSize.smallest
                                }
                                color: Appearance.colors.colSubtext
                            }

                            // Event rows: collapsible command/file-change list.
                            // While running, each row is visible; once done the whole
                            // group can be folded into one summary line.
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                visible: modelData.events.length > 0

                                Repeater {
                                    model: modelData.events

                                    AiEventRow {
                                        required property var modelData
                                        Layout.fillWidth: true
                                        toolName: modelData.tool
                                        command: modelData.command
                                        preview: modelData.preview
                                        duration: modelData.duration
                                        state: modelData.state
                                    }
                                }
                            }

                            // Agent answer: the hero of the screen.
                            StyledText {
                                Layout.fillWidth: true
                                visible: modelData.stream.length > 0
                                text: modelData.stream
                                textFormat: Text.MarkdownText
                                wrapMode: Text.Wrap
                                font.pixelSize: Appearance.font.pixelSize.normal
                                color: Appearance.colors.colOnLayer0
                            }

                            StyledText {
                                Layout.fillWidth: true
                                visible: modelData.answer.length > 0
                                text: modelData.answer
                                textFormat: Text.MarkdownText
                                wrapMode: Text.Wrap
                                font.pixelSize: Appearance.font.pixelSize.normal
                                color: Appearance.colors.colOnLayer0
                            }

                            // Run meta, quiet.
                            StyledText {
                                Layout.fillWidth: true
                                visible: modelData.meta.length > 0
                                text: modelData.meta
                                font {
                                    family: Appearance.font.family.monospace
                                    pixelSize: Appearance.font.pixelSize.smallest
                                }
                                color: Appearance.colors.colSubtext
                            }
                        }
                    }
                }

            // ---- Empty state --------------------------------------------
            Column {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 10
                visible: root.turns.length === 0
                topPadding: 60

                CustomIcon {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: 32
                    height: 32
                    source: "pilot-symbolic.svg"
                    colorize: true
                    color: Appearance.colors.colSubtext
                }
                StyledText {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: Translation.tr("What should I do?")
                    color: Appearance.colors.colOnLayer0
                    font.pixelSize: Appearance.font.pixelSize.normal
                    font.weight: Font.Medium
                }
                StyledText {
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: parent.width * 0.8
                    horizontalAlignment: Text.AlignHCenter
                    text: Translation.tr("Type a request, or / for commands.")
                    color: Appearance.colors.colSubtext
                    font.pixelSize: Appearance.font.pixelSize.small
                    wrapMode: Text.Wrap
                }
            }

            // ---- Status line (retries, compaction) ----------------------
            StyledText {
                Layout.fillWidth: true
                visible: root.statusLine.length > 0 && root.running
                text: root.statusLine
                wrapMode: Text.Wrap
                font.pixelSize: Appearance.font.pixelSize.smaller
                color: Appearance.colors.colSubtext
            }

            // ---- Command palette ----------------------------------------
            AiCommandPalette {
                Layout.fillWidth: true
                visible: root.paletteMode !== ""
                items: root.paletteItems
                activeIndex: root.paletteIndex
                onActivated: index => {
                    root.activateItem(root.paletteItems[index]);
                    input.forceActiveFocus();
                }
            }

            // ---- Composer -----------------------------------------------
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: composerColumn.implicitHeight + 16
                radius: Appearance.rounding.normal
                color: Appearance.colors.colLayer1
                border.width: 1
                border.color: root.running
                    ? Appearance.colors.colPrimary
                    : Appearance.colors.colLayer0Border

                Behavior on border.color {
                    ColorAnimation { duration: 180 }
                }

                ColumnLayout {
                    id: composerColumn
                    anchors {
                        fill: parent
                        margins: 8
                    }
                    spacing: 6

                    ToolbarTextField {
                        id: input
                        Layout.fillWidth: true
                        implicitHeight: 36
                        colBackground: "transparent"
                        placeholderText: root.running
                            ? Translation.tr("Working… type /stop to cancel")
                            : Translation.tr("Ask Pilot…")
                        enabled: true

                        onTextChanged: {
                            const typing = text;
                            root.paletteQuery = typing;
                            if (typing.startsWith("/") && !typing.includes(" ")) {
                                const inSubmenu = (root.paletteMode === "model" && typing === "/model")
                                    || (root.paletteMode === "effort" && typing === "/effort")
                                    || (root.paletteMode === "sessions" && typing === "/sessions");
                                if (!inSubmenu) {
                                    root.paletteMode = "command";
                                    root.paletteIndex = 0;
                                }
                            } else if (root.paletteMode === "command") {
                                root.paletteMode = "";
                            }
                        }

                        Keys.onPressed: event => {
                            if (root.paletteMode !== "" && root.paletteItems.length > 0) {
                                if (event.key === Qt.Key_Down) {
                                    root.paletteIndex = (root.paletteIndex + 1) % root.paletteItems.length;
                                    event.accepted = true;
                                    return;
                                }
                                if (event.key === Qt.Key_Up) {
                                    root.paletteIndex = (root.paletteIndex + root.paletteItems.length - 1) % root.paletteItems.length;
                                    event.accepted = true;
                                    return;
                                }
                                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                                    root.activateItem(root.paletteItems[root.paletteIndex]);
                                    event.accepted = true;
                                    return;
                                }
                            }
                        }

                        onAccepted: root.send()
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        // Chips: model · effort · sandbox
                        Rectangle {
                            implicitHeight: 20
                            implicitWidth: chipModel.implicitWidth + 10
                            radius: Appearance.rounding.full
                            color: Appearance.colors.colLayer2
                            border.width: 1
                            border.color: Appearance.colors.colLayer0Border
                            StyledText {
                                id: chipModel
                                anchors.centerIn: parent
                                text: root.model.length > 0 ? root.model : "model"
                                font { family: Appearance.font.family.monospace; pixelSize: Appearance.font.pixelSize.smallest }
                                color: Appearance.colors.colOnLayer2
                            }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: { root.paletteMode = "model"; root.paletteQuery = ""; root.paletteIndex = 0; input.text = ""; input.forceActiveFocus(); }
                            }
                        }
                        Rectangle {
                            implicitHeight: 20
                            implicitWidth: chipEffort.implicitWidth + 10
                            radius: Appearance.rounding.full
                            color: Appearance.colors.colLayer2
                            border.width: 1
                            border.color: Appearance.colors.colLayer0Border
                            StyledText {
                                id: chipEffort
                                anchors.centerIn: parent
                                text: root.effort.length > 0 ? root.effort : "effort"
                                font { family: Appearance.font.family.monospace; pixelSize: Appearance.font.pixelSize.smallest }
                                color: Appearance.colors.colOnLayer2
                            }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: { root.paletteMode = "effort"; root.paletteQuery = ""; root.paletteIndex = 0; input.text = ""; input.forceActiveFocus(); }
                            }
                        }
                        Rectangle {
                            implicitHeight: 20
                            implicitWidth: chipSandbox.implicitWidth + 10
                            radius: Appearance.rounding.full
                            color: root.sandbox === "full" ? Appearance.colors.colErrorContainer : Appearance.colors.colLayer2
                            border.width: 1
                            border.color: root.sandbox === "full" ? Appearance.colors.colError : Appearance.colors.colLayer0Border
                            StyledText {
                                id: chipSandbox
                                anchors.centerIn: parent
                                text: root.sandbox
                                font { family: Appearance.font.family.monospace; pixelSize: Appearance.font.pixelSize.smallest }
                                color: root.sandbox === "full" ? Appearance.colors.colOnErrorContainer : Appearance.colors.colOnLayer2
                            }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: { root.paletteMode = "sandbox"; root.paletteQuery = ""; root.paletteIndex = 0; input.text = ""; input.forceActiveFocus(); }
                            }
                        }

                        Item { Layout.fillWidth: true }

                        // Stop affordance: only while running.
                        AiIconButton {
                            visible: root.running
                            glyph: "stop"
                            onClicked: root.runCommand("/stop")
                        }
                        // Send hint when idle.
                        MaterialSymbol {
                            visible: !root.running
                            text: "keyboard_return"
                            iconSize: 16
                            color: Appearance.colors.colSubtext
                        }
                    }
                }
            }
        }
    }
}
}