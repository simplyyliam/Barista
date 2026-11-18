// The only bridge the monitor page gets: ask for the numbers, and hear about new ones.
const { contextBridge, ipcRenderer } = require("electron")

contextBridge.exposeInMainWorld("monitor", {
  load: () => ipcRenderer.invoke("load"),
  onSample: (listener) => ipcRenderer.on("sample", (_event, data) => listener(data)),
})
