# Barista integrations example

A Node.js client built with `@baristabar/sdk` that puts **GitHub** in the menu bar: the latest commit
of a configured repository, with menu entries to refresh and to open the repository or the commit.
It is a plain menu, so it needs no window.

Looking for the Spotify mini player? It has its own window, so it moved to an Electron example:
[`examples/spotify`](../spotify). Menu or popover? A menu is enough for a list of actions
(this example); a popover is for your own UI (Spotify, `examples/quick-note`).

The client does not store API credentials. Configure them as environment variables in the
PowerShell session where you run the example.

## Configure GitHub

Set a public or private repository in `owner/name` format:

```powershell
$env:GITHUB_REPOSITORY = "owner/repo"
```

Public repositories work without a token. For private repositories or higher API rate limits,
set `GITHUB_TOKEN` to a fine-grained token with read-only access to the selected repository.
The example checks for a new commit about once a minute, and again each time the menu opens.

## Run

From the repository root, build the local SDK and start the example:

```powershell
pnpm --filter @baristabar/sdk build
pnpm --filter @barista/integrations-demo start
```

Run `pnpm --filter @barista/integrations-demo typecheck` to type-check the example.

The status item can be registered before the menu frontend is finished. To see the menu, the bar
UI must render the `menu` window and connect it to the v0.3 menu commands and events.
