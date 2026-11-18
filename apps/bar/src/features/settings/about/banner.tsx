import { PreviewFrame } from "../preview-frame"
import { useSettings } from "@/shared/stores/settings"
import { Scene } from "./scenes"
import type { SceneId } from "./whats-new"

/**
 * The feature spotlight: the same picture the previews use (so it carries the saved look) with a
 * mock-up of the highlight in front. `play` changes when the person picks another highlight, which
 * remounts the scene so it plays once again.
 */
export function Banner({ scene, play }: { scene: SceneId; play: string }) {
  const settings = useSettings()
  if (!settings) return null
  return (
    <div aria-hidden="true">
      <PreviewFrame settings={settings} height={176} className="grid place-items-center">
        <div key={play} className="grid w-full place-items-center">
          <Scene id={scene} />
        </div>
      </PreviewFrame>
    </div>
  )
}
