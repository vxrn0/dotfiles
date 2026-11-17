pragma Singleton
pragma ComponentBehavior: Bound
import Quickshell

Singleton {
    id: root
    
    signal requestCenter(string identifier)

    readonly property list<var> availableWidgets: [
        { identifier: "crosshair", materialSymbol: "point_scan" },
        { identifier: "fpsLimiter", materialSymbol: "animation" },
        { identifier: "floatingImage", materialSymbol: "imagesmode" },
        { identifier: "recorder", materialSymbol: "screen_record" },
        { identifier: "resources", materialSymbol: "browse_activity" },
        { identifier: "volumeMixer", materialSymbol: "volume_up" },
    ]
    
    readonly property bool hasPinnedWidgets: root.pinnedWidgetIdentifiers.length > 0

    property list<string> pinnedWidgetIdentifiers: []
    property list<var> clickableWidgets: []

    function pin(identifier: string, pin = true) {
        if (pin) {
            if (!root.pinnedWidgetIdentifiers.includes(identifier)) {
                // Assign a new list: pushing into a list property does not notify,
                // so hasPinnedWidgets and the overlay never updated.
                root.pinnedWidgetIdentifiers = [...root.pinnedWidgetIdentifiers, identifier]
            }
        } else {
            root.pinnedWidgetIdentifiers = root.pinnedWidgetIdentifiers.filter(id => id !== identifier)
        }
    }

    function registerClickableWidget(widget: var, clickable = true) {
        if (clickable) {
            if (!root.clickableWidgets.includes(widget)) {
                // Same reason as pin(): in-place push would not notify the mask.
                root.clickableWidgets = [...root.clickableWidgets, widget]
            }
        } else {
            root.clickableWidgets = root.clickableWidgets.filter(w => w !== widget)
        }
    }
}
