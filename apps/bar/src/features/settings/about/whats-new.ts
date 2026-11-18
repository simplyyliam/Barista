export type SceneId = "bar" | "control" | "items" | "settings"

export interface Highlight {
  id: string
  title: string
  text: string
  scene: SceneId
}

/** The release these highlights are for. The full record stays in CHANGELOG.md. */
export const RELEASE = "0.8"

export const HIGHLIGHTS: Highlight[] = [
  {
    id: "settings",
    title: "Settings, in a window of their own",
    text: "Every change shows up in your bar at once. No Save button.",
    scene: "settings",
  },
  {
    id: "bar",
    title: "Arrange your menu bar",
    text: "Drag items into the order you like, or off the bar to hide them.",
    scene: "bar",
  },
  {
    id: "control",
    title: "Make the control center yours",
    text: "Reorder, resize and remove tiles and quick actions.",
    scene: "control",
  },
  {
    id: "items",
    title: "Apps that live in the bar",
    text: "Mic mute, now playing and a system monitor come with Barista.",
    scene: "items",
  },
  {
    id: "windows",
    title: "Start with Windows, tune the look",
    text: "Blur, tone, taskbar auto-hide and full-screen behaviour.",
    scene: "settings",
  },
]
