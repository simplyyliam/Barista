// Accelerators are only drawn, never registered as hotkeys (the protocol says so). We tidy
// Electron-style names into what Windows users expect to read.

const NAMES: Record<string, string> = {
  cmdorctrl: "Ctrl",
  commandorcontrol: "Ctrl",
  cmd: "Ctrl",
  command: "Ctrl",
  control: "Ctrl",
  super: "Win",
  meta: "Win",
  option: "Alt",
  return: "Enter",
  esc: "Esc",
  escape: "Esc",
  plus: "+",
  del: "Del",
  delete: "Del",
}

/**
 * `"CmdOrCtrl+Shift+S"` -> `"Ctrl+Shift+S"`.
 *
 * @example
 * formatAccelerator("CmdOrCtrl+Return") // "Ctrl+Enter"
 */
export function formatAccelerator(accelerator: string): string {
  return accelerator
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => NAMES[part.toLowerCase()] ?? (part.length === 1 ? part.toUpperCase() : part))
    .join("+")
}
