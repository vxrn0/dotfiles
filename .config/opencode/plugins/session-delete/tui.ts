import { Plugin } from "@opencode/plugin/tui"

// Delete the session currently open in the TUI: first press arms, second press deletes.
export default Plugin.define({
  id: "session-delete",
  setup(context) {
    let armed: string | undefined
    let timer: ReturnType<typeof setTimeout> | undefined

    context.keymap.layer(() => ({
      mode: "global",
      commands: [
        {
          id: "session-delete.current",
          title: "Delete current session",
          group: "Session",
          bind: "ctrl+shift+d",
          palette: true,
          run: async () => {
            const route = context.ui.router.current()
            if (route.type !== "session") return

            if (armed !== route.sessionID) {
              armed = route.sessionID
              clearTimeout(timer)
              timer = setTimeout(() => {
                armed = undefined
              }, 4000)
              const key = context.keymap.shortcuts("session-delete.current")[0] ?? "ctrl+shift+d"
              context.ui.toast.show({ message: `Press ${key} again to delete this session` })
              return
            }

            armed = undefined
            clearTimeout(timer)
            try {
              await context.client.session.remove({ sessionID: route.sessionID })
              context.ui.toast.show({ message: "Session deleted", variant: "success" })
              context.ui.router.navigate({ type: "home" })
            } catch (error) {
              context.ui.toast.show({ message: `Failed to delete session: ${error}`, variant: "error" })
            }
          },
        },
      ],
    }))

    return () => clearTimeout(timer)
  },
})
