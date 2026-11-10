import QtQuick
import Quickshell
import Quickshell.Wayland
import qs
import qs.modules.common
import qs.modules.common.functions
import qs.services

PanelWindow {
    id: root
    WlrLayershell.layer: WlrLayer.Background
    WlrLayershell.namespace: "quickshell:background"
    exclusionMode: ExclusionMode.Ignore

    anchors {
        top: true
        bottom: true
        left: true
        right: true
    }

    readonly property string path: Config.options.wallpaper.path
    // Video wallpapers are drawn by mpvpaper from Hyprland, not by the shell
    readonly property bool isVideo: /\.(mp4|webm|mkv|mov|avi)$/i.test(path)
    color: root.isVideo ? "transparent" : Appearance.colors.colLayer0

    Image {
        anchors.fill: parent
        visible: !root.isVideo && root.path.length > 0
        source: (!root.isVideo && root.path.length > 0) ? `file://${root.path}` : ""
        sourceSize.width: root.width
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        cache: false
    }
}
