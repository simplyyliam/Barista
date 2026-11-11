export interface GitHubCommit {
  sha: string
  message: string
  author: string
  url: string
  repository: string
}

interface CommitResponse {
  sha?: string
  html_url?: string
  commit?: {
    message?: string
    author?: { name?: string }
  }
  author?: { login?: string } | null
}

export async function fetchLatestCommit(repository: string, token?: string): Promise<GitHubCommit> {
  const [owner, name, extra] = repository.split("/")
  if (!owner || !name || extra) {
    throw new Error("Set GITHUB_REPOSITORY to a repository in owner/name format")
  }

  const response = await fetch(
    `https://api.github.com/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/commits?per_page=1`,
    {
      headers: {
        accept: "application/vnd.github+json",
        "x-github-api-version": "2022-11-28",
        "user-agent": "Barista-Integrations-Example",
        ...(token ? { authorization: `Bearer ${token}` } : {}),
      },
      signal: AbortSignal.timeout(15_000),
    },
  )
  if (!response.ok) {
    throw new Error(`GitHub API returned ${response.status}`)
  }

  const commits: unknown = await response.json()
  if (!Array.isArray(commits)) {
    throw new Error(`GitHub returned an invalid commit list for ${repository}`)
  }
  const latest = commits[0]
  const commit = latest as CommitResponse | undefined
  if (
    typeof commit?.sha !== "string" ||
    typeof commit.html_url !== "string" ||
    typeof commit.commit?.message !== "string"
  ) {
    throw new Error(`GitHub returned no commits for ${repository}`)
  }

  return {
    sha: commit.sha,
    message: commit.commit.message,
    author: commit.author?.login ?? commit.commit.author?.name ?? "Unknown author",
    url: commit.html_url,
    repository,
  }
}
