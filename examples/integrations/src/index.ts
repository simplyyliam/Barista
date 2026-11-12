// GitHub in the menu bar: the latest commit of one repository, as a menu (no popover needed).
// Spotify moved to its own Electron example, `examples/spotify`, because it has a window.
import * as BA from "@barista/sdk"
import { GitBranch } from "lucide"
import { fetchLatestCommit, type GitHubCommit } from "./github.ts"
import { openExternalUrl } from "./open-url.ts"


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

const githubItem = await BA.StatusItem.create({
  id: "github",
  icon: { icon: GitBranch, color: "#f0f0f0", height: 14}, // a Lucide icon object
  // label: (githubRepository ?? "GitHub setup").slice(0, 128),
  tooltip: githubRepository
    ? `Latest activity in ${githubRepository}`
    : "Set GITHUB_REPOSITORY=owner/repo to watch a repository",
})

let latestCommit: GitHubCommit | undefined
let githubTimer: NodeJS.Timeout | undefined

function showError(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error)
  console.error(`[GitHub] ${message}`)
  // The bar rejects control characters (API errors are multi-line JSON), so flatten to one line.
  // eslint-disable-next-line no-control-regex
  return message.replace(/[\u0000-\u001f\u007f-\u009f]+/g, " ").replace(/\s+/g, " ").trim().slice(0, 512)
}

async function perform(action: () => Promise<void>): Promise<void> {
  try {
    await action()
  } catch (error) {
    githubItem.setTooltip(`GitHub error: ${showError(error)}`)
  }
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
          ? perform(() => openExternalUrl(`https://github.com/${githubRepository}`))
          : undefined,
    },
    {
      id: "open-commit",
      label: commit ? `Open commit ${commit.sha.slice(0, 7)}` : "No commit loaded",
      enabled: Boolean(commit),
      click: () => (commit ? perform(() => openExternalUrl(commit.url)) : undefined),
    },
  ])
}

async function refreshGitHub(): Promise<void> {
  if (!githubRepository) return
  await perform(async () => {
    latestCommit = await fetchLatestCommit(githubRepository, githubToken)
    const subject = latestCommit.message.split(/\r?\n/, 1)[0] ?? "Latest commit"
    githubItem.setLabel(
      `${githubRepository.split("/")[1]} · ${latestCommit.sha.slice(0, 7)}`.slice(0, 128),
    )
    githubItem.setTooltip(`${subject} — ${latestCommit.author}`.slice(0, 512))
    updateGitHubMenu()
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
  clearTimeout(githubTimer)
  githubItem.remove()
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

updateGitHubMenu()
void refreshGitHub()
scheduleGitHubRefresh()

githubItem.on("error", (error) =>
  console.error(`[github] bar refused: ${error.code}: ${error.message}`),
)
// Refresh right when the menu opens, so "Open commit" is never stale.
githubItem.on("open", () => void refreshGitHub())

bar.on("quit", shutdown)
process.once("SIGINT", shutdown)
