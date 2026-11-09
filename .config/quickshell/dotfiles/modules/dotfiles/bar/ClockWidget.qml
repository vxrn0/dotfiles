import qs.modules.common
import qs.services
import QtQuick

// Clock: text only, the first thing in the bar, like the Pixel status bar.
Item {
    id: root
    property bool showDate: true

    implicitWidth: item.implicitWidth
    implicitHeight: Appearance.sizes.barHeight

    BarItem {
        id: item
        anchors.centerIn: parent
        label: root.showDate ? DateTime.time + "  " + DateTime.longDate : DateTime.time
    }

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: !Config.options.bar.tooltips.clickToShow

        ClockWidgetPopup {
            hoverTarget: mouseArea
        }
    }
}
