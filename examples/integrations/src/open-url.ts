import { execFile } from "node:child_process"

export function openExternalUrl(url: string): Promise<void> {
  if (process.platform !== "win32") {
    return Promise.reject(new Error("The integrations example opens links using Windows"))
  }
  const parsed = new URL(url)
  const allowedHosts = [
    "accounts.spotify.com",
    "developer.spotify.com",
    "github.com",
    "open.spotify.com",
  ]
  if (parsed.protocol !== "https:" || !allowedHosts.includes(parsed.hostname)) {
    return Promise.reject(new Error("Refusing to open an untrusted external URL"))
  }

  return new Promise((resolve, reject) => {
    execFile("rundll32.exe", ["url.dll,FileProtocolHandler", url], (error) => {
      if (error) reject(error)
      else resolve()
    })
  })
}
