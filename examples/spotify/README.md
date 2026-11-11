# Spotify mini player (Electron popover)

Left click the icon: a small player window opens right under it. Right click: a menu (open the
track, quit). Built with `@barista/sdk` and its Electron adapter (`@barista/sdk/electron`).
Electron is a dev dependency of this example only; the SDK never imports it.

- **Main process** (`src/main.ts`): the bar connection, the Spotify tokens and every network
  call. Tokens stay in memory and are never sent to the window.
- **Player page** (`src/player.html`): draws the state it is sent. A sandboxed preload
  (`src/preload.cjs`) exposes `play`, `pause`, `next`, `previous` and `connect`, and the main
  process checks the sender and the action.

## Try it without Spotify

```powershell
pnpm --filter @barista/sdk build
pnpm --filter @barista/spotify-demo start:demo
```

## Configure Spotify

1. Create an app in the [Spotify developer dashboard](https://developer.spotify.com/dashboard).
2. Add this exact redirect URI in the app settings: `http://127.0.0.1:8888/callback`.
3. Set its client ID, start the example and press **Connect Spotify** in the popover:

   ```powershell
   $env:SPOTIFY_CLIENT_ID = "your-client-id"
   pnpm --filter @barista/spotify-demo start
   ```

Sign-in opens your browser and receives the callback on the local loopback address (keep port
`8888` free). It asks to read the current track and control playback. Tokens stay in memory, so you
sign in again after restarting the example.
