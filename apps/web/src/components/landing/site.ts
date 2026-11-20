// One place for links and copy that Liam will want to change.
// Installers live on this repo's Releases page (ADR 0038).
export const DOWNLOAD_URL = "https://github.com/simplyyliam/Barista/releases/latest"
export const NPM_URL = "https://www.npmjs.com/package/@baristabar/sdk"

export const NAV_LINKS = [
  { href: "#features", label: "Features" },
  { href: "#sdk", label: "For developers" },
  { href: "#faq", label: "FAQ" },
  { href: "/docs", label: "Docs" },
]

export const FAQ = [
  {
    q: "What happens to my taskbar?",
    a: "Barista can hide the Windows taskbar automatically, so the bar is the only thing at the top. You choose in settings, and it comes back the moment you switch the setting off.",
  },
  {
    q: "Where do my tray icons go?",
    a: "Into the bar. Apps that live in the Windows tray show up as items in the bar, and a click opens the app's own menu. Prefer the old tray? Switch it back in settings.",
  },
  {
    q: "Does it work with more than one monitor?",
    a: "Yes. The bar reserves its own space on the monitor you pick, and it is sized for every screen and display scale.",
  },
  {
    q: "Can I go back to the Windows notification center?",
    a: "Any time. Notifications and the control center each have a Windows mode that stops the bar's own version and opens the one from Windows.",
  },
  {
    q: "Will it slow my PC down?",
    a: "It idles near 0% CPU. The bar listens for events from Windows instead of polling, and it stays small in memory.",
  },
  {
    q: "Which version of Windows do I need?",
    a: "Barista is built and tested on Windows 11.",
  },
]
