import { createServer } from "node:http"
import { createHash, randomBytes, timingSafeEqual } from "node:crypto"
import { openExternalUrl } from "./open-url.ts"

const REDIRECT_PORT = 8888
const REDIRECT_URI = `http://127.0.0.1:${REDIRECT_PORT}/callback`
const SCOPES = ["user-read-currently-playing", "user-modify-playback-state"]

interface TokenResponse {
  access_token: string
  expires_in: number
  refresh_token?: string
}

interface SpotifyPlaybackResponse {
  is_playing?: boolean
  item?: {
    name?: string
    artists?: Array<{ name?: string }>
    external_urls?: { spotify?: string }
  } | null
}

export interface SpotifyTrack {
  name: string
  artists: string[]
  url?: string
  isPlaying: boolean
}

function waitForAuthorizationCode(clientId: string): Promise<{ code: string; verifier: string }> {
  const state = randomBytes(24).toString("hex")
  const verifier = randomBytes(32).toString("base64url")
  const challenge = createHash("sha256").update(verifier).digest("base64url")
  const authorizationUrl = new URL("https://accounts.spotify.com/authorize")
  authorizationUrl.search = new URLSearchParams({
    client_id: clientId,
    response_type: "code",
    redirect_uri: REDIRECT_URI,
    code_challenge_method: "S256",
    code_challenge: challenge,
    state,
    scope: SCOPES.join(" "),
  }).toString()

  return new Promise((resolve, reject) => {
    let settled = false
    const server = createServer((request, response) => {
      const url = new URL(request.url ?? "/", REDIRECT_URI)
      if (url.pathname !== "/callback") {
        response.writeHead(404).end("Not found")
        return
      }

      const returnedState = url.searchParams.get("state") ?? ""
      const expectedState = Buffer.from(state)
      const actualState = Buffer.from(returnedState)
      if (
        expectedState.length !== actualState.length ||
        !timingSafeEqual(expectedState, actualState)
      ) {
        response.writeHead(400).end("Authorization state did not match. You can close this tab.")
        finish(new Error("Spotify authorization state did not match"))
        return
      }

      const providerError = url.searchParams.get("error")
      if (providerError) {
        response.writeHead(400).end("Spotify authorization failed. You can close this tab.")
        finish(new Error(`Spotify authorization failed: ${providerError}`))
        return
      }

      const code = url.searchParams.get("code")
      if (!code) {
        response.writeHead(400).end("Spotify did not return an authorization code.")
        finish(new Error("Spotify did not return an authorization code"))
        return
      }

      response.writeHead(200, { "content-type": "text/plain; charset=utf-8" })
      response.end("Spotify is connected to Barista. You can close this tab.")
      finish(undefined, code)
    })

    const timeout = setTimeout(
      () => finish(new Error("Spotify authorization timed out after five minutes")),
      5 * 60 * 1000,
    )
    timeout.unref()

    const finish = (error?: Error, code?: string) => {
      if (settled) return
      settled = true
      clearTimeout(timeout)
      if (server.listening) server.close()
      if (error) reject(error)
      else if (code) resolve({ code, verifier })
      else reject(new Error("Spotify authorization ended without a code"))
    }

    server.on("error", (error) =>
      finish(new Error(`Could not start Spotify's local sign-in callback: ${error.message}`)),
    )
    server.listen(REDIRECT_PORT, "127.0.0.1", () => {
      console.log(
        "Opening Spotify sign-in. If the browser does not open, visit:\n",
        authorizationUrl.href,
      )
      void openExternalUrl(authorizationUrl.href).catch((error: unknown) => {
        console.error("Could not open the browser automatically:", error)
      })
    })
  })
}

async function readTokenResponse(response: Response): Promise<TokenResponse> {
  if (!response.ok) {
    throw new Error(`Spotify API returned ${response.status}`)
  }
  const data = (await response.json()) as {
    access_token?: unknown
    expires_in?: unknown
    refresh_token?: unknown
  }
  if (
    typeof data.access_token !== "string" ||
    !data.access_token ||
    typeof data.expires_in !== "number" ||
    data.expires_in <= 0 ||
    (data.refresh_token !== undefined && typeof data.refresh_token !== "string")
  ) {
    throw new Error("Spotify returned an invalid token response")
  }
  return {
    access_token: data.access_token,
    expires_in: data.expires_in,
    ...(data.refresh_token ? { refresh_token: data.refresh_token } : {}),
  }
}

async function readPlaybackResponse(response: Response): Promise<SpotifyPlaybackResponse> {
  if (!response.ok) {
    throw new Error(`Spotify playback request returned ${response.status}`)
  }
  return (await response.json()) as SpotifyPlaybackResponse
}

export class SpotifyClient {
  private readonly clientId: string
  #accessToken: string
  #refreshToken: string
  #expiresAt: number

  private constructor(clientId: string, tokens: TokenResponse) {
    this.clientId = clientId
    this.#accessToken = tokens.access_token
    this.#refreshToken = tokens.refresh_token ?? ""
    this.#expiresAt = Date.now() + tokens.expires_in * 1000
  }

  static async authorize(clientId: string): Promise<SpotifyClient> {
    const { code, verifier } = await waitForAuthorizationCode(clientId)
    const response = await fetch("https://accounts.spotify.com/api/token", {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        client_id: clientId,
        grant_type: "authorization_code",
        code,
        redirect_uri: REDIRECT_URI,
        code_verifier: verifier,
      }),
      signal: AbortSignal.timeout(15_000),
    })
    const tokens = await readTokenResponse(response)
    return new SpotifyClient(clientId, tokens)
  }

  async currentlyPlaying(): Promise<SpotifyTrack | null> {
    const response = await this.request("/me/player/currently-playing")
    if (response.status === 204) return null

    const data = await readPlaybackResponse(response)
    const track = data.item
    if (!track?.name) return null

    return {
      name: track.name,
      artists: track.artists?.flatMap((artist) => (artist.name ? [artist.name] : [])) ?? [],
      url: track.external_urls?.spotify,
      isPlaying: data.is_playing === true,
    }
  }


  async playback(action: "play" | "pause" | "next" | "previous"): Promise<void> {
    const method = action === "next" || action === "previous" ? "POST" : "PUT"
  
    const response = await this.request(`/me/player/${action}`, {
      method,
    })
  
    if (!response.ok) {
      const details = await response.text().catch(() => "")
      throw new Error(
        `Spotify ${action} command returned ${response.status}${details ? `: ${details}` : ""}`,
      )
    }
  }


  private async request(path: string, init?: RequestInit): Promise<Response> {
    if (Date.now() >= this.#expiresAt - 30_000) await this.refresh()
    const send = () =>
      fetch(`https://api.spotify.com/v1${path}`, {
        ...init,
        headers: { ...init?.headers, authorization: `Bearer ${this.#accessToken}` },
        signal: AbortSignal.timeout(15_000),
      })

    let response = await send()
    if (response.status === 401) {
      await this.refresh()
      response = await send()
    }
    return response
  }

  private async refresh(): Promise<void> {
    if (!this.#refreshToken) {
      throw new Error(
        "Spotify did not provide a refresh token; restart the example and sign in again",
      )
    }
    const response = await fetch("https://accounts.spotify.com/api/token", {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        client_id: this.clientId,
        grant_type: "refresh_token",
        refresh_token: this.#refreshToken,
      }),
      signal: AbortSignal.timeout(15_000),
    })
    const tokens = await readTokenResponse(response)
    this.#accessToken = tokens.access_token
    this.#refreshToken = tokens.refresh_token ?? this.#refreshToken
    this.#expiresAt = Date.now() + tokens.expires_in * 1000
  }
}
