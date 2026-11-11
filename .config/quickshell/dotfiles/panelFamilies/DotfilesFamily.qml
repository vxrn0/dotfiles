import QtQuick
import Quickshell
import qs

import qs.modules.common
import qs.modules.dotfiles.background
import qs.modules.dotfiles.bar
import qs.modules.dotfiles.cheatsheet
import qs.modules.dotfiles.lock
import qs.modules.dotfiles.mediaControls
import qs.modules.dotfiles.notificationPopup
import qs.modules.dotfiles.onScreenDisplay
import qs.modules.dotfiles.overview
import qs.modules.dotfiles.polkit
import qs.modules.dotfiles.regionSelector
import qs.modules.dotfiles.screenCorners
import qs.modules.dotfiles.sessionScreen
import qs.modules.dotfiles.sidebarRight
import qs.modules.dotfiles.overlay
import qs.modules.dotfiles.wallpaperSelector

Scope {
    PanelLoader { component: Background {} }
    PanelLoader { component: Bar {} }
    PanelLoader { component: Cheatsheet {} }
    PanelLoader { component: Lock {} }
    PanelLoader { component: MediaControls {} }
    PanelLoader { component: NotificationPopup {} }
    PanelLoader { component: OnScreenDisplay {} }
    PanelLoader { component: Overlay {} }
    PanelLoader { component: Overview {} }
    PanelLoader { component: Polkit {} }
    PanelLoader { component: RegionSelector {} }
    PanelLoader { component: ScreenCorners {} }
    PanelLoader { component: SessionScreen {} }
    PanelLoader { component: SidebarRight {} }
    PanelLoader { component: WallpaperSelector {} }
}
