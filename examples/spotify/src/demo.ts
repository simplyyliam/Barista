// A pretend player for `pnpm --filter @barista/spotify-demo start:demo`: no Spotify account needed,
// so the popover can be tried (and tested) without credentials.
import type { SpotifyTrack } from "./spotify.ts"

const TRACKS: Array<Omit<SpotifyTrack, "isPlaying">> = [
  { name: "Late Night Build", artists: ["The Linters"], url: "https://open.spotify.com/" },
  { name: "Green Checks", artists: ["CI Orchestra", "Cargo"] },
  { name: "Merge Conflict", artists: ["Rebase"] },
]

export class DemoPlayer {
  #index = 0
  #playing = true

  async currentlyPlaying(): Promise<SpotifyTrack | null> {
    return { ...TRACKS[this.#index]!, isPlaying: this.#playing }
  }

  async playback(action: "play" | "pause" | "next" | "previous"): Promise<void> {
    if (action === "play") this.#playing = true
    else if (action === "pause") this.#playing = false
    else {
      const step = action === "next" ? 1 : TRACKS.length - 1
      this.#index = (this.#index + step) % TRACKS.length
    }
  }
}
