pragma ComponentBehavior: Bound
import qs
import qs.services
import qs.modules.common
import qs.modules.common.models
import qs.modules.common.widgets
import qs.modules.common.functions
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Hyprland

ButtonMouseArea {
    id: root

    readonly property HyprlandMonitor monitor: Hyprland.monitorFor(root.QsWindow.window?.screen)
    WorkspaceModel {
        id: wsModel
        monitor: root.monitor
    }

    readonly property real barThickness: Appearance.sizes.barHeight
    readonly property real widgetPadding: 4
    readonly property real dotSize: 12
    readonly property real activeWidth: 30
    readonly property int workspaceIndexInGroup: Math.max(0, (monitor?.activeWorkspace?.id - 1) % Math.max(1, wsModel.shownCount))

    Layout.alignment: Qt.AlignVCenter
    Layout.fillWidth: false
    Layout.fillHeight: true
    implicitWidth: track.implicitWidth
    implicitHeight: barThickness

    function workspaceIndexAt(px) {
        let best = 0;
        let bestDistance = Infinity;
        const items = indicatorRow.children;
        for (let i = 0; i < items.length; i++) {
            const item = items[i];
            if (item.width === undefined || item.width <= 0)
                continue;
            const distance = Math.abs(item.x + item.width / 2 - (px - indicatorRow.x));
            if (distance < bestDistance) {
                bestDistance = distance;
                best = i;
            }
        }
        return best;
    }

    function switchWorkspaceAt(index) {
        Hyprland.dispatch(`hl.dsp.focus({workspace = ${wsModel.getWorkspaceIdAt(index)}})`);
    }

    acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.BackButton
    hoverEnabled: true

    onPressed: mouse => {
        if (mouse.button === Qt.LeftButton)
            switchWorkspaceAt(workspaceIndexAt(mouseX));
        else if (mouse.button === Qt.RightButton)
            GlobalStates.overviewOpen = !GlobalStates.overviewOpen;
        else if (mouse.button === Qt.BackButton)
            Hyprland.dispatch(`hl.dsp.workspace.toggle_special("special")`);
    }
    onWheel: event => {
        if (event.angleDelta.y < 0)
            Hyprland.dispatch(`hl.dsp.focus({workspace = "r+1"})`);
        else if (event.angleDelta.y > 0)
            Hyprland.dispatch(`hl.dsp.focus({workspace = "r-1"})`);
    }

    Rectangle { // Track
        id: track
        anchors.centerIn: parent
        implicitWidth: indicatorRow.implicitWidth + root.widgetPadding * 2
        implicitHeight: root.dotSize + 8
        radius: height / 2
        color: ColorUtils.transparentize(Appearance.colors.colLayer1, 0.4)

        Row {
            id: indicatorRow
            anchors.centerIn: parent
            spacing: 7

            Repeater {
                model: wsModel.shownCount

                Rectangle {
                    id: indicator
                    required property int index
                    readonly property bool active: index === root.workspaceIndexInGroup
                    readonly property bool hovered: root.containsMouse && root.workspaceIndexAt(root.mouseX) === index

                    width: active ? root.activeWidth : root.dotSize
                    height: root.dotSize
                    radius: height / 2
                    color: active
                        ? Appearance.colors.colPrimary
                        : ColorUtils.transparentize(Appearance.colors.colOnLayer1, hovered ? 0.25 : 0.55)

                    Behavior on width {
                        animation: Appearance.animation.elementMoveFast.numberAnimation.createObject(this)
                    }
                    Behavior on color {
                        animation: Appearance.animation.elementMoveFast.colorAnimation.createObject(this)
                    }
                }
            }
        }
    }
}
