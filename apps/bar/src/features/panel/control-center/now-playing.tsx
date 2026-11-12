import { commands } from "@/shared/bindings"
import { Icon, MusicNote01Icon, NextIcon, PauseIcon, PlayIcon, PreviousIcon } from "@barista/ui/icons"
import { useMedia } from "./hooks/use-control-center"

const buttonRadius = "rounded-[max(0px,calc(var(--radius-panel-child)-var(--spacing-cc-button-pad)))]"
const control =
  "flex size-10 items-center justify-center outline-none enabled:hover:bg-cc-tile-hover focus-visible:ring-2 focus-visible:ring-cc-fg/60 disabled:opacity-40 " +
  buttonRadius

/**
 * What is playing, styled like the user row: the cover where the avatar is, the song and the
 * artist, and the play controls in a group on the right. It spans the whole card.
 */
export function NowPlaying() {
  const media = useMedia()
  const playing = media?.available === true
  const radius = "rounded-[max(0px,calc(var(--radius-panel-child)-var(--spacing-cc-avatar-pad)))]"

  return (
    <div className="flex h-cc-header gap-1.5">
      <div className="flex min-w-0 flex-1 items-center gap-2.5 rounded-panel-child bg-cc-tile py-cc-avatar-pad pr-cc-avatar-pad-end pl-cc-avatar-pad text-cc-fg">
        <div className={`flex size-10 shrink-0 items-center justify-center overflow-hidden bg-cc-tile ${radius}`}>
          {playing && media.cover ? (
            <img src={media.cover} alt="" draggable={false} className="size-full object-cover" />
          ) : (
            <Icon icon={MusicNote01Icon} size={18} />
          )}
        </div>
        <div className="flex min-w-0 flex-col leading-tight">
          <span className="truncate text-[11px] font-medium">{playing ? media.title : "Not playing"}</span>
          <span className="truncate text-[9px] text-cc-fg-muted">
            {playing ? [media.artist, media.appName].filter(Boolean).join(" · ") : "Play something to control it here"}
          </span>
        </div>
      </div>
      <div className="flex shrink-0 items-center rounded-panel-child bg-cc-tile p-cc-button-pad text-cc-fg">
        <button
          type="button"
          aria-label="Previous"
          disabled={!playing || !media.canPrevious}
          onClick={() => void commands.mediaPrevious()}
          className={control}
        >
          <Icon icon={PreviousIcon} size={16} />
        </button>
        <button
          type="button"
          aria-label={media?.playing ? "Pause" : "Play"}
          disabled={!playing}
          onClick={() => void commands.mediaPlayPause()}
          className={control}
        >
          <Icon icon={media?.playing ? PauseIcon : PlayIcon} size={16} />
        </button>
        <button
          type="button"
          aria-label="Next"
          disabled={!playing || !media.canNext}
          onClick={() => void commands.mediaNext()}
          className={control}
        >
          <Icon icon={NextIcon} size={16} />
        </button>
      </div>
    </div>
  )
}
