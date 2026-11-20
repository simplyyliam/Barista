// The real bar UI on mocked IPC, framed by the website (ADR 0037). `?scene=` picks what is open
// at the start (see stage.tsx). The mocks must be installed before any bar code asks Tauri anything.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks"
import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { handle } from "./backend"

mockWindows("main")
mockIPC(handle, { shouldMockEvents: true })

const params = new URLSearchParams(location.search)
const { default: Stage } = await import("./stage")
const { startBarTone } = await import("../src/shared/stores/bar-tone")
await import("./demo.css")
void startBarTone()
if (params.get("scene") === "sdk") {
  const { startCounterFx } = await import("./counter-fx")
  startCounterFx()
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Stage scene={params.get("scene") ?? "notifications"} bare={params.has("bare")} />
  </StrictMode>,
)
