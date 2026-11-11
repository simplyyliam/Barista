// The only bridge the note page gets: read the note, and save it. A sandboxed preload is CommonJS.
const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("notes", {
  load: () => ipcRenderer.invoke("load"),
  save: (text) => ipcRenderer.invoke("save", text),
  onShow: (listener) => ipcRenderer.on("show", () => listener()),
})
