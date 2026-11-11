import qs.modules.common
import qs.modules.common.functions
import qs.modules.common.widgets
import qs.services
import Qt5Compat.GraphicalEffects
import QtQuick
import QtQuick.Layouts
import Quickshell.Services.Mpris

// One player card in the media panel, laid out like end-4's: art on the left,
// title and artist on top, transport around the seek bar.
Item {
    id: root
    required property MprisPlayer player
    readonly property string artUrl: root.player?.trackArtUrl ?? ""
    readonly property real length: root.player?.length ?? 0
    readonly property bool playing: root.player?.isPlaying ?? false
    property real displayPosition: 0
    property double seekGuardUntil: 0 // ignore stale MPRIS positions right after a seek

    implicitHeight: 140

    // MPRIS only answers on request, and polling it every few seconds makes the bar
    // jump. Let the local clock run the progress and resync it on real events.
    function runProgress() {
        progressAnimation.stop();
        if (root.playing && root.length > root.displayPosition) {
            progressAnimation.from = root.displayPosition;
            progressAnimation.to = root.length;
            progressAnimation.duration = (root.length - root.displayPosition) * 1000;
            progressAnimation.start();
        }
    }

    function syncProgress() {
        root.displayPosition = Math.max(0, root.player?.position ?? 0);
        root.runProgress();
    }

    function seek(seconds) {
        if (root.length <= 0)
            return;
        root.displayPosition = Math.max(0, Math.min(root.length, seconds));
        root.player.position = root.displayPosition;
        root.seekGuardUntil = Date.now() + 1000;
        root.runProgress();
    }

    NumberAnimation {
        id: progressAnimation
        target: root
        property: "displayPosition"
        easing.type: Easing.Linear
    }

    Timer { // Refresh MPRIS position now and then; the local clock keeps the bar smooth
        running: root.playing
        interval: 10000
        repeat: true
        onTriggered: {
            root.player?.positionChanged();
            syncDebounce.restart();
        }
    }

    Timer { // The refresh is async; read the value after it lands
        id: syncDebounce
        interval: 200
        onTriggered: {
            if (Date.now() >= root.seekGuardUntil)
                root.syncProgress();
        }
    }

    Connections {
        target: root.player

        function onPositionChanged() {
            syncDebounce.restart();
        }
        function onLengthChanged() {
            syncDebounce.restart();
        }
        function onIsPlayingChanged() {
            root.player?.positionChanged();
            syncDebounce.restart();
        }
        function onTrackTitleChanged() {
            root.displayPosition = 0;
            root.player?.positionChanged();
            syncDebounce.restart();
        }
    }

    Component.onCompleted: {
        root.player?.positionChanged();
        root.syncProgress();
    }

    StyledRectangularShadow {
        target: background
    }

    Rectangle {
        id: background
        anchors.fill: parent
        radius: Appearance.rounding.windowRounding
        color: Appearance.colors.colLayer0
        border.width: 1
        border.color: Appearance.colors.colLayer0Border

        RowLayout {
            anchors.fill: parent
            anchors.margins: 14
            spacing: 15

            Rectangle { // Art frame; the image is masked, a plain clip would stay square
                id: artFrame
                Layout.fillHeight: true
                implicitWidth: height
                radius: Appearance.rounding.verysmall
                color: Appearance.colors.colLayer1

                StyledImage {
                    id: art
                    anchors.fill: parent
                    cache: false
                    fillMode: Image.PreserveAspectCrop
                    source: root.artUrl.startsWith("/") ? "file://" + root.artUrl : root.artUrl
                    layer.enabled: true
                    layer.effect: OpacityMask {
                        maskSource: Rectangle {
                            width: artFrame.width
                            height: artFrame.height
                            radius: artFrame.radius
                        }
                    }
                }

                MaterialSymbol {
                    anchors.centerIn: parent
                    visible: art.status !== Image.Ready
                    text: "music_note"
                    iconSize: 32
                    color: Appearance.colors.colOnLayer1
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 2

                StyledText {
                    Layout.fillWidth: true
                    wrapMode: Text.Wrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    text: StringUtils.cleanMusicTitle(root.player?.trackTitle) || Translation.tr("Unknown Title")
                    color: Appearance.colors.colOnLayer0
                    font {
                        pixelSize: Appearance.font.pixelSize.large
                        weight: Font.DemiBold
                        variableAxes: ({ "wght": 650 })
                    }
                }

                StyledText {
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                    text: root.player?.trackArtist ?? ""
                    color: Appearance.colors.colSubtext
                    font.pixelSize: Appearance.font.pixelSize.smaller
                }

                Item {
                    Layout.fillHeight: true
                }

                Item { // Time above the bar, play/pause above its right end
                    Layout.fillWidth: true
                    implicitHeight: Math.max(timeText.implicitHeight, playPauseButton.implicitHeight)
                        + 5 + sliderRow.implicitHeight

                    RowLayout { // Previous, seek, next
                        id: sliderRow
                        anchors {
                            left: parent.left
                            right: parent.right
                            bottom: parent.bottom
                        }
                        spacing: 8

                        RippleButton {
                            implicitWidth: 24
                            implicitHeight: 24
                            buttonRadius: Appearance.rounding.full
                            colBackground: "transparent"
                            colBackgroundHover: Appearance.colors.colLayer1Hover
                            colRipple: Appearance.colors.colLayer1Active
                            downAction: () => root.player?.previous()

                            contentItem: MaterialSymbol {
                                anchors.centerIn: parent
                                text: "skip_previous"
                                iconSize: 20
                                fill: 1
                                color: Appearance.colors.colOnLayer1
                            }
                        }

                        SeekBar {
                            id: seekBar
                            Layout.fillWidth: true
                            Layout.alignment: Qt.AlignVCenter
                            interactive: root.player?.canSeek ?? false
                            value: root.length > 0 ? root.displayPosition / root.length : 0
                            onMoved: (value) => root.seek(value * root.length)
                        }

                        RippleButton {
                            implicitWidth: 24
                            implicitHeight: 24
                            buttonRadius: Appearance.rounding.full
                            colBackground: "transparent"
                            colBackgroundHover: Appearance.colors.colLayer1Hover
                            colRipple: Appearance.colors.colLayer1Active
                            downAction: () => root.player?.next()

                            contentItem: MaterialSymbol {
                                anchors.centerIn: parent
                                text: "skip_next"
                                iconSize: 20
                                fill: 1
                                color: Appearance.colors.colOnLayer1
                            }
                        }
                    }

                    StyledText {
                        id: timeText
                        anchors {
                            left: parent.left
                            bottom: sliderRow.top
                            bottomMargin: 5
                        }
                        text: `${StringUtils.friendlyTimeForSeconds(root.displayPosition)} / ${StringUtils.friendlyTimeForSeconds(root.length)}`
                        color: Appearance.colors.colSubtext
                        font.pixelSize: Appearance.font.pixelSize.small
                    }

                    RippleButton {
                        id: playPauseButton
                        anchors {
                            right: parent.right
                            bottom: sliderRow.top
                            bottomMargin: 5
                        }
                        implicitWidth: 40
                        implicitHeight: 40
                        buttonRadius: root.playing ? Appearance.rounding.normal : Appearance.rounding.full
                        colBackground: root.playing ? Appearance.colors.colPrimary : Appearance.colors.colSecondaryContainer
                        colBackgroundHover: root.playing ? Appearance.colors.colPrimaryHover : Appearance.colors.colSecondaryContainerHover
                        colRipple: root.playing ? Appearance.colors.colPrimaryActive : Appearance.colors.colSecondaryContainerActive
                        downAction: () => root.player?.togglePlaying()

                        contentItem: MaterialSymbol {
                            anchors.centerIn: parent
                            text: root.playing ? "pause" : "play_arrow"
                            iconSize: 24
                            fill: 1
                            color: root.playing ? Appearance.colors.colOnPrimary : Appearance.colors.colOnSecondaryContainer
                        }
                    }
                }
            }
        }
    }
}
