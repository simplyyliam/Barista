// The player page. It only draws the state it is sent and asks for actions through `window.player`.
// Text goes in with textContent, never innerHTML: track names come from the internet.
const $ = (id) => document.getElementById(id)

let state = { status: "setup", title: "Spotify", subtitle: "", playing: false }

function render() {
  $("title").textContent = state.title
  $("subtitle").textContent = state.subtitle
  const connected = state.status !== "setup" && state.status !== "error"
  $("connect").hidden = connected
  for (const id of ["previous", "toggle", "next"]) {
    $(id).hidden = !connected
    $(id).disabled = state.status === "idle"
  }
  $("toggle").textContent = state.playing ? "⏸" : "▶"
}

$("previous").addEventListener("click", () => window.player.playback("previous"))
$("next").addEventListener("click", () => window.player.playback("next"))
$("toggle").addEventListener("click", () =>
  window.player.playback(state.playing ? "pause" : "play"),
)
$("connect").addEventListener("click", () => window.player.connect())

window.player.onState((next) => {
  state = next
  render()
})
render()
