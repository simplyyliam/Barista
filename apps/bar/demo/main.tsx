// The real bar UI on mocked IPC, framed by the website (ADR 0037). `?scene=` picks what is open
// at the start (see stage.tsx). The mocks must be installed before any bar code asks Tauri anything.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks"
import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { handle } from "./backend"

mockWindows("main")
mockIPC(handle, { shouldMockEvents: true })

const { default: Stage } = await import("./stage")
const { startBarTone } = await import("../src/shared/stores/bar-tone")
await import("./demo.css")
void startBarTone()

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Stage scene={new URLSearchParams(location.search).get("scene") ?? "hero"} />
  </StrictMode>,
)
