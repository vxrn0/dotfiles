pragma ComponentBehavior: Bound
import qs.modules.common
import qs.modules.common.models
import qs.modules.common.widgets
import qs.services
import qs.modules.common.functions
import Qt5Compat.GraphicalEffects
import QtQuick
import QtQuick.Effects
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris

Item { // Player instance
    id: root
    required property MprisPlayer player
    property var artUrl: player?.trackArtUrl
    // Some players send an empty art URL mid-track (#3580); keep the last real one.
    property string lastArtUrl: ""
    property string lastGoodArtPath: "" // stays until the next cover is ready
    property string artDownloadLocation: Directories.coverArt
    property string artFileName: Qt.md5(root.lastArtUrl)
    property string artFilePath: `${artDownloadLocation}/${artFileName}`
    property color artDominantColor: ColorUtils.mix((colorQuantizer?.colors[0] ?? Appearance.colors.colPrimary), Appearance.colors.colPrimaryContainer, 0.8) || Appearance.m3colors.m3secondaryContainer
    property bool downloaded: false
    property real radius
    // MPRIS position is only re-read on demand; animate it locally so the bar is smooth.
    property real displayPosition: 0
    property double seekGuardUntil: 0
    // Some streams report a bogus length (YouTube sends ~2^63 ns); treat it as unknown.
    readonly property real length: (root.player?.length > 0 && root.player.length < 86400)
        ? root.player.length : 0

    onArtUrlChanged: {
        if (root.artUrl && root.artUrl.length > 0)
            root.lastArtUrl = root.artUrl;
    }

    readonly property bool artIsLocalFile: root.lastArtUrl.startsWith("file://") || root.lastArtUrl.startsWith("/")
    property string displayedArtFilePath: root.artIsLocalFile
        ? (root.lastArtUrl.startsWith("/") ? "file://" + root.lastArtUrl : root.lastArtUrl)
        : root.lastGoodArtPath

    function syncProgress() {
        progressAnimation.stop();
        root.displayPosition = Math.max(0, root.player?.position ?? 0);
        if (root.player?.isPlaying && root.length > root.displayPosition) {
            progressAnimation.from = root.displayPosition;
            progressAnimation.to = root.length;
            progressAnimation.duration = (root.length - root.displayPosition) * 1000;
            progressAnimation.start();
        }
    }

    NumberAnimation {
        id: progressAnimation
        target: root
        property: "displayPosition"
        easing.type: Easing.Linear
    }

    Timer { // Periodic MPRIS resync; the animation keeps the bar smooth in between
        running: root.player?.playbackState == MprisPlaybackState.Playing
        interval: 10000
        repeat: true
        onTriggered: {
            root.player.positionChanged();
            syncDebounce.restart();
        }
    }

    Timer {
        id: syncDebounce
        interval: 200
        onTriggered: {
            if (Date.now() >= root.seekGuardUntil)
                root.syncProgress();
        }
    }

    Connections {
        target: root.player
        function onPositionChanged() { syncDebounce.restart(); }
        function onLengthChanged() { syncDebounce.restart(); }
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
        if (root.artUrl)
            root.lastArtUrl = root.artUrl;
        root.player?.positionChanged();
        root.syncProgress();
    }

    onArtFilePathChanged: {
        if (root.lastArtUrl.length == 0) {
            root.artDominantColor = Appearance.m3colors.m3secondaryContainer
            return;
        }
        if (root.artIsLocalFile)
            return; // nothing to download

        // Binding does not work in Process
        coverArtDownloader.targetFile = root.lastArtUrl
        coverArtDownloader.artFilePath = root.artFilePath
        root.downloaded = false
        coverArtDownloader.running = true
    }

    Process { // Cover art downloader
        id: coverArtDownloader
        property string targetFile: root.lastArtUrl
        property string artFilePath: root.artFilePath
        // Quote the URL: it comes from the player and could break the shell command.
        command: [ "bash", "-c", `[ -f "$2" ] || curl -4 -sSL "$1" -o "$2"`, "--", targetFile, artFilePath ]
        onExited: (exitCode, exitStatus) => {
            root.downloaded = exitCode === 0
            // Keep the previous cover on screen until the new file is really there.
            if (exitCode === 0)
                root.lastGoodArtPath = Qt.resolvedUrl(root.artFilePath)
        }
    }

    ColorQuantizer {
        id: colorQuantizer
        source: root.displayedArtFilePath
        depth: 0 // 2^0 = 1 color
        rescaleSize: 1 // Rescale to 1x1 pixel for faster processing
    }

    property QtObject blendedColors: AdaptedMaterialScheme {
        color: artDominantColor
    }

    StyledRectangularShadow {
        target: background
    }
    Rectangle { // Background
        id: background
        anchors.fill: parent
        anchors.margins: Appearance.sizes.elevationMargin
        color: ColorUtils.applyAlpha(blendedColors.colLayer0, 1)
        radius: root.radius

        layer.enabled: true
        layer.effect: OpacityMask {
            maskSource: Rectangle {
                width: background.width
                height: background.height
                radius: background.radius
            }
        }

        StyledImage {
            id: blurredArt
            anchors.fill: parent
            source: root.displayedArtFilePath
            fillMode: Image.PreserveAspectCrop
            cache: false
            antialiasing: true
            asynchronous: true

            layer.enabled: true
            layer.effect: StyledBlurEffect {
                source: blurredArt
            }

            Rectangle {
                anchors.fill: parent
                color: ColorUtils.transparentize(blendedColors.colLayer0, 0.3)
                radius: root.radius
            }
        }

        RowLayout {
            anchors.fill: parent
            anchors.margins: 13
            spacing: 15

            Rectangle { // Art background
                id: artBackground
                Layout.fillHeight: true
                implicitWidth: height
                radius: Appearance.rounding.verysmall
                color: ColorUtils.transparentize(blendedColors.colLayer1, 0.5)

                layer.enabled: true
                layer.effect: OpacityMask {
                    maskSource: Rectangle {
                        width: artBackground.width
                        height: artBackground.height
                        radius: artBackground.radius
                    }
                }

                StyledImage { // Art image
                    id: mediaArt
                    property int size: parent.height
                    anchors.fill: parent

                    source: root.displayedArtFilePath
                    fillMode: Image.PreserveAspectCrop
                    cache: false
                    antialiasing: true

                    width: size
                    height: size
                }
            }

            ColumnLayout { // Info & controls
                Layout.fillHeight: true
                spacing: 2

                StyledText {
                    id: trackTitle
                    Layout.fillWidth: true
                    font.pixelSize: Appearance.font.pixelSize.large
                    color: blendedColors.colOnLayer0
                    elide: Text.ElideRight
                    text: StringUtils.cleanMusicTitle(root.player?.trackTitle) || "Untitled"
                    animateChange: true
                    animationDistanceX: 6
                    animationDistanceY: 0
                }
                StyledText {
                    id: trackArtist
                    Layout.fillWidth: true
                    font.pixelSize: Appearance.font.pixelSize.smaller
                    color: blendedColors.colSubtext
                    elide: Text.ElideRight
                    text: root.player?.trackArtist
                    animateChange: true
                    animationDistanceX: 6
                    animationDistanceY: 0
                }
                Item { Layout.fillHeight: true }
                Item {
                    Layout.fillWidth: true
                    implicitHeight: trackTime.implicitHeight + sliderRow.implicitHeight

                    StyledText {
                        id: trackTime
                        anchors.bottom: sliderRow.top
                        anchors.bottomMargin: 5
                        anchors.left: parent.left
                        font.pixelSize: Appearance.font.pixelSize.small
                        color: blendedColors.colSubtext
                        elide: Text.ElideRight
                        text: root.length > 0
                            ? `${StringUtils.friendlyTimeForSeconds(root.displayPosition)} / ${StringUtils.friendlyTimeForSeconds(root.length)}`
                            : StringUtils.friendlyTimeForSeconds(root.displayPosition)
                    }
                    RowLayout {
                        id: sliderRow
                        anchors {
                            bottom: parent.bottom
                            left: parent.left
                            right: parent.right
                        }
                        TrackChangeButton {
                            blendedColors: root.blendedColors
                            iconName: "skip_previous"
                            downAction: () => root.player?.previous()
                        }
                        Item {
                            id: progressBarContainer
                            Layout.fillWidth: true
                            implicitHeight: Math.max(sliderLoader.implicitHeight, progressBarLoader.implicitHeight)

                            Loader {
                                id: sliderLoader
                                anchors.fill: parent
                                active: root.player?.canSeek ?? false
                                sourceComponent: StyledSlider { 
                                    configuration: StyledSlider.Configuration.Wavy
                                    highlightColor: blendedColors.colPrimary
                                    trackColor: blendedColors.colSecondaryContainer
                                    handleColor: blendedColors.colPrimary
                                    value: root.length > 0 ? root.displayPosition / root.length : 0
                                    onMoved: {
                                        if (root.length > 0) {
                                            root.displayPosition = value * root.length;
                                            root.player.position = root.displayPosition;
                                            root.seekGuardUntil = Date.now() + 1000;
                                            root.syncProgress();
                                        }
                                    }
                                }
                            }

                            Loader {
                                id: progressBarLoader
                                anchors {
                                    verticalCenter: parent.verticalCenter
                                    left: parent.left
                                    right: parent.right
                                }
                                active: !(root.player?.canSeek ?? false)
                                sourceComponent: StyledProgressBar { 
                                    wavy: root.player?.isPlaying
                                    highlightColor: blendedColors.colPrimary
                                    trackColor: blendedColors.colSecondaryContainer
                                    value: root.length > 0 ? root.displayPosition / root.length : 0
                                }
                            }

                            
                        }
                        TrackChangeButton {
                            blendedColors: root.blendedColors
                            iconName: "skip_next"
                            downAction: () => root.player?.next()
                        }
                    }

                    RippleButton {
                        id: playPauseButton
                        anchors.right: parent.right
                        anchors.bottom: sliderRow.top
                        anchors.bottomMargin: 5
                        property real size: 44
                        implicitWidth: size
                        implicitHeight: size
                        downAction: () => root.player.togglePlaying();

                        buttonRadius: root.player?.isPlaying ? Appearance?.rounding.normal : size / 2
                        colBackground: root.player?.isPlaying ? blendedColors.colPrimary : blendedColors.colSecondaryContainer
                        colBackgroundHover: root.player?.isPlaying ? blendedColors.colPrimaryHover : blendedColors.colSecondaryContainerHover
                        colRipple: root.player?.isPlaying ? blendedColors.colPrimaryActive : blendedColors.colSecondaryContainerActive

                        contentItem: MaterialSymbol {
                            iconSize: Appearance.font.pixelSize.huge
                            fill: 1
                            horizontalAlignment: Text.AlignHCenter
                            color: root.player?.isPlaying ? blendedColors.colOnPrimary : blendedColors.colOnSecondaryContainer
                            text: root.player?.isPlaying ? "pause" : "play_arrow"

                            Behavior on color {
                                animation: Appearance.animation.elementMoveFast.colorAnimation.createObject(this)
                            }
                        }
                    }
                }
            }
        }
    }
}
