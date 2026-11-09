import qs.modules.common
import qs.services
import qs.modules.common.widgets
import QtQuick

// Clock in the shared bar anatomy: text only, same size and weight as every value.
Item {
    id: root
    property bool showDate: true

    implicitWidth: item.implicitWidth
    implicitHeight: 28

    BarItem {
        id: item
        anchors.centerIn: parent
        label: root.showDate ? DateTime.time + "  " + DateTime.longDate : DateTime.time
        valueWidth: 48
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
