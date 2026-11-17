import qs
import qs.services
import qs.modules.common
import qs.modules.common.widgets
import QtQuick
import Quickshell

Rectangle {
    id: root

    property var screen
    property var brightnessMonitor: Brightness.getMonitorForScreen(screen)

    implicitHeight: column.implicitHeight + 8
    radius: Appearance.rounding.normal
    color: Appearance.colors.colLayer1

    Column {
        id: column
        anchors {
            top: parent.top
            left: parent.left
            right: parent.right
            margins: 4
        }

        Loader {
            width: column.width
            visible: active
            active: Config.options.sidebar.quickSliders.showBrightness
            sourceComponent: QuickSlider {
                materialSymbol: "light_mode"
                // The left 30% of the track is night light, the rest is brightness
                value: Hyprsunset.gamma === 100
                    ? 0.3 + (root.brightnessMonitor?.brightness ?? 0) * 0.7
                    : (Hyprsunset.gamma - Hyprsunset.gammaLowerLimit) / (100 - Hyprsunset.gammaLowerLimit) * 0.3
                onMoved: {
                    if (!root.brightnessMonitor)
                        return;
                    if (value >= 0.3) {
                        root.brightnessMonitor.setBrightness((value - 0.3) / 0.7);
                        if (Hyprsunset.gamma !== 100)
                            Hyprsunset.setGamma(100);
                    } else {
                        if (root.brightnessMonitor.brightness !== 0)
                            root.brightnessMonitor.setBrightness(0);
                        Hyprsunset.setGamma(value / 0.3 * (100 - Hyprsunset.gammaLowerLimit) + Hyprsunset.gammaLowerLimit);
                    }
                }
            }
        }

        Loader {
            width: column.width
            visible: active
            active: Config.options.sidebar.quickSliders.showVolume
            sourceComponent: QuickSlider {
                materialSymbol: "volume_up"
                value: Audio.sink?.audio?.volume ?? 0
                // Slider.moved() has no arguments; `value` is the slider's own property.
                onMoved: {
                    if (Audio.sink?.audio)
                        Audio.sink.audio.volume = value;
                }
            }
        }

        Loader {
            width: column.width
            visible: active
            active: Config.options.sidebar.quickSliders.showMic
            sourceComponent: QuickSlider {
                materialSymbol: "mic"
                value: Audio.source?.audio?.volume ?? 0
                onMoved: {
                    if (Audio.source?.audio)
                        Audio.source.audio.volume = value;
                }
            }
        }
    }

    component QuickSlider: StyledSlider {
        id: quickSlider
        required property string materialSymbol
        configuration: StyledSlider.Configuration.M

        MaterialSymbol {
            anchors {
                verticalCenter: quickSlider.verticalCenter
                right: quickSlider.right
                rightMargin: 8
            }
            iconSize: 18
            color: quickSlider.value >= 0.9 ? Appearance.colors.colOnPrimary : Appearance.colors.colOnSecondaryContainer
            text: quickSlider.materialSymbol
        }
    }
}
