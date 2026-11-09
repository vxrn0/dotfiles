import qs.modules.common
import qs.services
import QtQuick
import QtQuick.Layouts

// Battery: a level glyph, the percentage, and a bolt while charging — the Pixel
// status-bar idiom, flat on the bar surface.
Item {
    id: root
    readonly property bool isCharging: Battery.isCharging
    readonly property real percentage: Battery.percentage
    readonly property bool isLow: percentage <= Config.options.battery.low / 100 && !isCharging

    // The glyph carries the coarse level, the number the exact one.
    readonly property string levelGlyph: {
        if (percentage >= 0.95) return "battery_full";
        if (percentage >= 0.8) return "battery_6_bar";
        if (percentage >= 0.6) return "battery_5_bar";
        if (percentage >= 0.4) return "battery_4_bar";
        if (percentage >= 0.25) return "battery_3_bar";
        if (percentage >= 0.1) return "battery_2_bar";
        return "battery_alert";
    }

    implicitWidth: item.implicitWidth
    implicitHeight: Appearance.sizes.barHeight

    BarItem {
        id: item
        anchors.centerIn: parent
        glyph: root.levelGlyph
        label: Math.round(root.percentage * 100) + "%"
        valueWidth: 30
        warning: root.isLow
        active: root.isCharging
        trailingGlyph: root.isCharging && root.percentage < 1 ? "bolt" : ""
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: !Config.options.bar.tooltips.clickToShow

        BatteryPopup {
            hoverTarget: mouseArea
        }
    }
}
