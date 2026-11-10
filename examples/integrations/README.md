# Barista integrations example

A Node.js client built with `@barista/sdk` that demonstrates three real service integrations:

- **Spotify:** current track, playback state, pause/play, previous and next.
- **GitHub:** latest commit for a configured repository, with links to the repository and commit.
- **Claude:** on-demand summary of the latest public GitHub commit.

The client does not store API credentials. Configure them as environment variables in the
PowerShell session where you run the example.

## Configure Spotify

1. Create an app in the [Spotify developer dashboard](https://developer.spotify.com/dashboard).
2. Add this exact redirect URI in the app settings: `http://127.0.0.1:8888/callback`.
3. Set its client ID:

   ```powershell
   $env:SPOTIFY_CLIENT_ID = "your-client-id"
   ```

On startup, the example opens Spotify sign-in in your browser and receives the authorization
callback on the local loopback address. It requests permission to read the current track and
control playback. Tokens stay in memory and are not written to disk, so signing in again is
required after restarting the example. Keep local port `8888` available while signing in.

## Configure GitHub

Set a public or private repository in `owner/name` format:

```powershell
$env:GITHUB_REPOSITORY = "owner/repo"
```

Public repositories work without a token. For private repositories or higher API rate limits,
set `GITHUB_TOKEN` to a fine-grained token with read-only access to the selected repository.
The example checks for a new commit about once a minute.

## Configure Claude

Set an Anthropic API key to enable the **Summarize latest GitHub commit** menu action:

```powershell
$env:ANTHROPIC_API_KEY = "your-api-key"
```

The default model is `claude-sonnet-5-5`. Override it with `ANTHROPIC_MODEL` if needed. A summary
is requested only when you choose the menu action, and only the repository name, author, commit
id, and commit message (up to 3,000 characters) are sent to Anthropic. API usage may incur
charges.

## Run

From the repository root, build the local SDK and start the example:

```powershell
pnpm --filter @barista/sdk build
pnpm --filter @barista/integrations-demo start
```

Run `pnpm --filter @barista/integrations-demo typecheck` to type-check the example.

The status items can be registered before the menu frontend is finished. To see the playback,
refresh, navigation, and Claude actions, the bar UI must render the `menu` window and connect it
to the existing v0.3 menu commands and events.
