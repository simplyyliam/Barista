import * as BA from "@barista/sdk"
import { summarizeCommit } from "./claude.ts"
import { fetchLatestCommit, type GitHubCommit } from "./github.ts"
import { openExternalUrl } from "./open-url.ts"
import { SpotifyClient, type SpotifyTrack } from "./spotify.ts"

const MUSIC_ICON =
  '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#1db954" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M10 15V8l7-1v7"/><circle cx="8" cy="16" r="2"/><circle cx="15" cy="14" r="2"/></svg>'
const GITHUB_ICON =
  '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#f0f0f0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="6" cy="5" r="2"/><circle cx="18" cy="6" r="2"/><circle cx="12" cy="19" r="2"/><path d="M6 7v5a4 4 0 0 0 4 4h2m4-8v2a4 4 0 0 1-4 4"/></svg>'
const CLAUDE_ICON =
  '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#d97757" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 2 1.8 6.2L20 10l-6.2 1.8L12 18l-1.8-6.2L4 10l6.2-1.8L12 2Z"/><path d="m19 16 .8 2.2L22 19l-2.2.8L19 22l-.8-2.2L16 19l2.2-.8L19 16Z"/></svg>'

const spotifyClientId = process.env.SPOTIFY_CLIENT_ID?.trim()
const githubRepositoryValue = process.env.GITHUB_REPOSITORY?.trim()
const githubRepository =
  githubRepositoryValue && /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(githubRepositoryValue)
    ? githubRepositoryValue
    : undefined
const githubToken = process.env.GITHUB_TOKEN?.trim()

const bar = await BA.connect({ appName: "Barista Integrations" })
bar.on("close", () => {
  console.log("Barista closed the connection")
  process.exit(0)
})

const [spotifyItem, githubItem, claudeItem] = await Promise.all([
  BA.StatusItem.create({
    id: "spotify",
    icon: Buffer.from(MUSIC_ICON),
    label: spotifyClientId ? "Connecting Spotify…" : "Spotify setup",
    tooltip: "Now playing and playback controls",
  }),
  BA.StatusItem.create({
    id: "github",
    icon: Buffer.from(GITHUB_ICON),
    label: (githubRepository ?? "GitHub setup").slice(0, 128),
    tooltip: githubRepository
      ? `Latest activity in ${githubRepository}`
      : "Set GITHUB_REPOSITORY=owner/repo to watch a repository",
  }),
  BA.StatusItem.create({
    id: "claude",
    icon: Buffer.from(CLAUDE_ICON),
    label: process.env.ANTHROPIC_API_KEY ? "Claude ready" : "Claude setup",
    tooltip: "Ask Claude to summarize the latest GitHub commit",
  }),
])

let spotify: SpotifyClient | undefined
let currentTrack: SpotifyTrack | null = null
let latestCommit: GitHubCommit | undefined
let spotifyTimer: NodeJS.Timeout | undefined
let githubTimer: NodeJS.Timeout | undefined

function showError(service: string, error: unknown): string {
  const message = error instanceof Error ? error.message : String(error)
  console.error(`[${service}] ${message}`)
  return message.slice(0, 512)
}

async function perform(service: string, action: () => Promise<void>): Promise<void> {
  try {
    await action()
  } catch (error) {
    const message = showError(service, error)
    const item =
      service === "Spotify" ? spotifyItem : service === "GitHub" ? githubItem : claudeItem
    item.setTooltip(`${service} error: ${message}`)
  }
}

function updateSpotifyMenu(): void {
  const track = currentTrack
  const connected = spotify !== undefined
  spotifyItem.setMenu([
    {
      id: "play-pause",
      label: track?.isPlaying ? "Pause" : "Play",
      enabled: connected && track !== null,
      click: () =>
        perform("Spotify", async () => {
          if (!spotify || !track) return
          await spotify.playback(track.isPlaying ? "pause" : "play")
          await refreshSpotify()
        }),
    },
    {
      id: "previous",
      label: "Previous track",
      enabled: connected,
      click: () => perform("Spotify", () => spotify!.playback("previous")),
    },
    {
      id: "next",
      label: "Next track",
      enabled: connected,
      click: () => perform("Spotify", () => spotify!.playback("next")),
    },
    { type: "separator" },
    {
      id: "open-track",
      label: "Open track in Spotify",
      enabled: Boolean(track?.url),
      click: () => (track?.url ? perform("Spotify", () => openExternalUrl(track.url!)) : undefined),
    },
    {
      id: "connect",
      label: connected ? "Connected with Spotify" : "Spotify setup",
      enabled: !connected && Boolean(spotifyClientId),
      click: () => startSpotify(),
    },
  ])
}

async function refreshSpotify(): Promise<void> {
  if (!spotify) return
  await perform("Spotify", async () => {
    currentTrack = await spotify!.currentlyPlaying()
    if (!currentTrack) {
      spotifyItem.setLabel("Spotify idle")
      spotifyItem.setTooltip("No track is currently playing")
    } else {
      const artists = currentTrack.artists.join(", ")
      const label = `${currentTrack.isPlaying ? "" : "Paused · "}${currentTrack.name}`
      spotifyItem.setLabel(label.slice(0, 128))
      spotifyItem.setTooltip(
        (artists ? `${currentTrack.name} — ${artists}` : currentTrack.name).slice(0, 512),
      )
    }
    updateSpotifyMenu()
  })
}

function scheduleSpotifyRefresh(): void {
  clearTimeout(spotifyTimer)
  spotifyTimer = setTimeout(async () => {
    await refreshSpotify()
    scheduleSpotifyRefresh()
  }, 20_000)
}

function updateGitHubMenu(): void {
  const commit = latestCommit
  githubItem.setMenu([
    {
      id: "refresh",
      label: "Refresh repository activity",
      enabled: Boolean(githubRepository),
      click: () => refreshGitHub(),
    },
    {
      id: "open-repository",
      label: "Open repository",
      enabled: Boolean(githubRepository),
      click: () =>
        githubRepository
          ? perform("GitHub", () => openExternalUrl(`https://github.com/${githubRepository}`))
          : undefined,
    },
    {
      id: "open-commit",
      label: commit ? `Open commit ${commit.sha.slice(0, 7)}` : "No commit loaded",
      enabled: Boolean(commit),
      click: () => (commit ? perform("GitHub", () => openExternalUrl(commit.url)) : undefined),
    },
  ])
}

async function refreshGitHub(): Promise<void> {
  if (!githubRepository) return
  await perform("GitHub", async () => {
    latestCommit = await fetchLatestCommit(githubRepository, githubToken)
    const subject = latestCommit.message.split(/\r?\n/, 1)[0] ?? "Latest commit"
    githubItem.setLabel(
      `${githubRepository.split("/")[1]} · ${latestCommit.sha.slice(0, 7)}`.slice(0, 128),
    )
    githubItem.setTooltip(`${subject} — ${latestCommit.author}`.slice(0, 512))
    updateGitHubMenu()
    updateClaudeMenu()
  })
}

function updateClaudeMenu(): void {
  const commit = latestCommit
  const hasKey = Boolean(process.env.ANTHROPIC_API_KEY)
  claudeItem.setMenu([
    {
      id: "summarize-commit",
      label: "Summarize latest GitHub commit",
      enabled: hasKey && Boolean(commit),
      click: () =>
        commit
          ? perform("Claude", async () => {
              claudeItem.setLabel("Claude thinking…")
              const summary = await summarizeCommit(commit)
              claudeItem.setLabel("Claude summary")
              claudeItem.setTooltip(summary.slice(0, 512))
              console.log(`[Claude] ${summary}`)
            })
          : undefined,
    },
    { type: "separator" },
    {
      id: "about",
      label: "About this integration",
      click: () => console.log("Claude summarizes public GitHub commit metadata on request."),
    },
  ])
}

function startSpotify(): void {
  if (!spotifyClientId) {
    console.log(
      "Spotify is not configured. Set SPOTIFY_CLIENT_ID; see examples/integrations/README.md.",
    )
    updateSpotifyMenu()
    return
  }

  void SpotifyClient.authorize(spotifyClientId)
    .then(async (client) => {
      spotify = client
      spotifyItem.setLabel("Spotify connected")
      await refreshSpotify()
      scheduleSpotifyRefresh()
    })
    .catch((error: unknown) => {
      const message = showError("Spotify", error)
      spotifyItem.setLabel("Spotify sign-in failed")
      spotifyItem.setTooltip(message.slice(0, 512))
      updateSpotifyMenu()
    })
}

function scheduleGitHubRefresh(): void {
  clearTimeout(githubTimer)
  githubTimer = setTimeout(async () => {
    await refreshGitHub()
    scheduleGitHubRefresh()
  }, 65_000)
}

function shutdown(): void {
  clearTimeout(spotifyTimer)
  clearTimeout(githubTimer)
  spotifyItem.remove()
  githubItem.remove()
  claudeItem.remove()
  bar.close()
  process.exit(0)
}

if (!githubRepository) {
  if (githubRepositoryValue) {
    console.error(
      "GITHUB_REPOSITORY must use owner/repo format with letters, digits, dot, underscore or hyphen.",
    )
  } else {
    console.log(
      "GitHub is not configured. Set GITHUB_REPOSITORY=owner/repo; see examples/integrations/README.md.",
    )
  }
}
if (!process.env.ANTHROPIC_API_KEY) {
  console.log("Claude summaries are disabled. Set ANTHROPIC_API_KEY to enable them.")
}

updateSpotifyMenu()
updateGitHubMenu()
updateClaudeMenu()
startSpotify()
void refreshGitHub()
scheduleGitHubRefresh()

for (const item of [spotifyItem, githubItem, claudeItem]) {
  item.on("error", (error) =>
    console.error(`[${item.id}] bar refused: ${error.code}: ${error.message}`),
  )
}

bar.on("quit", shutdown)
process.once("SIGINT", shutdown)
process.once("SIGTERM", shutdown)
