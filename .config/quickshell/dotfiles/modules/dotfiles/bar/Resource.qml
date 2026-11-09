import QtQuick

// One system metric: glyph plus value. The value is a number, not a ring or a bar;
// the shape stays identical to every other bar item.
BarItem {
    id: root
    required property string iconName
    required property double percentage
    property int warningThreshold: 100

    glyph: root.iconName
    label: Math.round(root.percentage * 100) + "%"
    valueWidth: 32
    warning: root.percentage * 100 >= root.warningThreshold
}
