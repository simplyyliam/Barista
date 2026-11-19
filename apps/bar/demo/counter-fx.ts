// The landing page's counter toy (SDK section): when the number changes, the old digits roll up and out
// through a soft blur while the new ones roll in from below. The bar itself never animates a label (its
// motion rules are stricter), so this lives in the demo only and works on the DOM from outside: it
// draws two short-lived copies over the label and removes them when they are done.
const SELECTOR = '[data-item-id="demo/counter"]'
const DURATION = 380
const EASE = "cubic-bezier(0.22, 1, 0.36, 1)"

/** The text React put in the item (our copies are elements, so they are not counted). */
const labelOf = (el: Element) =>
  [...el.childNodes]
    .filter((node) => node.nodeType === Node.TEXT_NODE)
    .map((node) => node.textContent ?? "")
    .join("")

export function startCounterFx() {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) return

  let last: string | null = null
  let copies: HTMLElement[] = []

  const stop = (el: HTMLElement) => {
    copies.forEach((copy) => copy.remove())
    copies = []
    el.classList.remove("fx-rolling")
  }

  const copy = (el: HTMLElement, text: string, color: string) => {
    const span = document.createElement("span")
    span.textContent = text
    span.setAttribute("aria-hidden", "true")
    Object.assign(span.style, { position: "absolute", inset: "0", display: "grid", placeItems: "center", pointerEvents: "none", color })
    el.appendChild(span)
    copies.push(span)
    return span
  }

  const roll = (el: HTMLElement, from: string, to: string) => {
    stop(el) // interruptible: a click mid-roll starts the next roll from here
    const color = getComputedStyle(el).color
    el.classList.add("fx-rolling")
    const options = { duration: DURATION, easing: EASE, fill: "forwards" as const }
    copy(el, from, color).animate(
      [
        { transform: "translateY(0)", filter: "blur(0px)", opacity: 1 },
        { transform: "translateY(-60%)", filter: "blur(3px)", opacity: 0 },
      ],
      options,
    )
    const incoming = copy(el, to, color)
    incoming.animate(
      [
        { transform: "translateY(60%)", filter: "blur(3px)", opacity: 0 },
        { transform: "translateY(0)", filter: "blur(0px)", opacity: 1 },
      ],
      options,
    ).onfinish = () => stop(el)
  }

  const check = () => {
    const el = document.querySelector<HTMLElement>(SELECTOR)
    if (!el) return
    const text = labelOf(el)
    if (last !== null && text !== last) roll(el, last, text)
    last = text
  }

  // The item mounts after the first render and its text changes in place, so watch the whole page.
  new MutationObserver(check).observe(document.body, { subtree: true, childList: true, characterData: true })
  check()
}
