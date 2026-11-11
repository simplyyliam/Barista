// The only bridge the player page gets. A sandboxed preload must be plain CommonJS.
// The Spotify tokens never come this way: the page only asks for actions and receives state.
const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("player", {
  playback: (action) => ipcRenderer.invoke("playback", action),
  connect: () => ipcRenderer.invoke("connect"),
  onState: (listener) => {
    ipcRenderer.on("state", (_event, state) => listener(state))
    ipcRenderer.send("ready")
  },
})
