import QtQuick
import QtQuick.Effects

MultiEffect {
    id: root
    anchors.fill: source
    saturation: 0.2
    blurEnabled: true
    blurMax: 100
    blur: 1
}
