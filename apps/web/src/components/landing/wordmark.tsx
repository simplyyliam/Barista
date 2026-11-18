"use client"

import { useEffect, useRef, useState } from "react"
import { Bean } from "./icons"

// The big footer "Barista" with coffee inside the letters. When the footer scrolls into view the cup
// fills halfway (a little underdamped, so it sloshes), and it drains again when you scroll away.
//
// (Flat frappe experiment: a creamy drink body, thicker foam, and sparse cocoa flecks.)
// One small WebGL canvas, only drawn while visible, capped at 30fps. The letters are drawn once into a
// 2D canvas and used as a mask texture, so the shader stays cheap: a few sines.
// Without WebGL (or with a software renderer) the CSS fallback fills the same letters with a gradient.

const FILL = 0.5

const VERT = `attribute vec2 p;varying vec2 uv;void main(){uv=vec2(p.x*.5+.5,.5-p.y*.5);gl_Position=vec4(p,0.,1.);}`

const FRAG = `precision mediump float;
varying vec2 uv;
uniform sampler2D mask;
uniform vec2 res;
uniform float time;
uniform float surf;
uniform float slosh;
void main(){
  float m=texture2D(mask,uv).a;
  if(m<.002){gl_FragColor=vec4(0.);return;}
  float px=1./res.y;
  float x=uv.x*res.x/res.y;
  float amp=.005+.02*slosh;
  float w1=sin(x*7.+time*1.6)*amp+sin(x*13.-time*1.1)*amp*.5;
  float w2=sin(x*6.-time*1.2+1.7)*amp*.9+sin(x*11.+time*.9)*amp*.4;
  float surface=surf+w1;
  float under=surf-.014+w2;
  float front=smoothstep(-px*1.5,px*1.5,uv.y-surface);
  float back=smoothstep(-px*1.5,px*1.5,uv.y-under);
  float la=max(front,back);
  float depth=uv.y-surface;

  // Frappe: one flat, milky coffee tone with a thicker, softly uneven foam cap.
  vec3 drink=vec3(.66,.47,.34);
  vec3 foam=vec3(.92,.85,.76);
  vec3 liquid=drink;
  float capDepth=.078+.007*sin(x*8.+time*.3)+.004*sin(x*17.-time*.24);
  float cap=(1.-smoothstep(capDepth-px,capDepth+px,depth))*la;
  liquid=mix(liquid,foam,cap);

  // Sparse cocoa flecks give the cap a frappe-like finish, kept as crisp flat dots.
  float cell=x*6.2;
  float cellId=floor(cell);
  float seedA=fract(sin(cellId*127.1+17.0)*43758.5453);
  float seedB=fract(sin(cellId*311.7+4.0)*23421.631);
  float localX=(fract(cell)-.5)*.20;
  float fleckY=.012+seedB*.060;
  float fleckR=px*1.7+.0015;
  float fleck=1.-smoothstep(fleckR,fleckR+px,length(vec2(localX,depth-fleckY)));
  fleck*=step(.70,seedA)*cap;
  liquid=mix(liquid,vec3(.48,.31,.22),fleck);

  // A couple of larger pale pockets break up the foam without adding a gradient.
  float bubbleCell=x*2.7;
  float bubbleId=floor(bubbleCell);
  float bubbleSeed=fract(sin(bubbleId*53.3+8.1)*24634.6345);
  float bubbleX=(fract(bubbleCell)-.5)*.24;
  float bubbleY=.025+bubbleSeed*.040;
  float bubbleR=px*2.5+.002;
  float bubble=1.-smoothstep(bubbleR,bubbleR+px,length(vec2(bubbleX,depth-bubbleY)));
  bubble*=step(.72,bubbleSeed)*cap;
  liquid=mix(liquid,vec3(.98,.93,.85),bubble*.85);

  vec3 ghost=vec3(10./255.)*.07;
  vec3 col=mix(ghost,liquid,la);
  float a=mix(.07,1.,la);
  gl_FragColor=vec4(col,a)*m;
}`

// The "i" loses its dot; the bean takes its place.
const dotless = (w: string) => w.replace("i", "ı")

// Where the dot of the "i" sat, in CSS px inside the wordmark, so the bean can take its place.
function dotSpot(host: HTMLElement) {
  const cs = getComputedStyle(host)
  const fs = parseFloat(cs.fontSize)
  const c = document.createElement("canvas").getContext("2d")!
  c.font = `${cs.fontWeight} ${fs}px ${cs.fontFamily}`
  const ls = parseFloat(cs.letterSpacing)
  const hasLs = "letterSpacing" in c
  if (hasLs) (c as unknown as { letterSpacing: string }).letterSpacing = `${Number.isNaN(ls) ? 0 : ls}px`
  const word = dotless(host.dataset.word ?? "Barista")
  const at = word.indexOf("ı")
  const total = c.measureText(word).width
  const before = c.measureText(word.slice(0, at)).width
  const own = c.measureText("ı")
  const advance = own.width - (hasLs && !Number.isNaN(ls) ? ls : 0)
  const asc = own.fontBoundingBoxAscent ?? fs * 0.97
  const desc = own.fontBoundingBoxDescent ?? fs * 0.24
  const baseline = (fs * 0.9 - (asc + desc)) / 2 + asc
  const bw = fs * 0.2
  const bh = bw * (160 / 140)
  return {
    x: (host.clientWidth - total) / 2 + before + advance / 2 - bw / 2 - fs * 0.035,
    y: baseline - fs * 0.625 - bh / 2,
    w: bw,
  }
}

type Gl = { destroy: () => void; setTarget: (frac: number) => void; setVisible: (v: boolean) => void; resize: () => void }

function start(host: HTMLElement, canvas: HTMLCanvasElement, reduce: boolean, onLost: () => void): Gl | null {
  const gl = canvas.getContext("webgl", {
    alpha: true,
    premultipliedAlpha: true,
    antialias: false,
    failIfMajorPerformanceCaveat: true,
  })
  if (!gl) return null

  const sh = (type: number, src: string) => {
    const s = gl.createShader(type)!
    gl.shaderSource(s, src)
    gl.compileShader(s)
    return s
  }
  const prog = gl.createProgram()!
  gl.attachShader(prog, sh(gl.VERTEX_SHADER, VERT))
  gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FRAG))
  gl.linkProgram(prog)
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) return null
  gl.useProgram(prog)

  const buf = gl.createBuffer()
  gl.bindBuffer(gl.ARRAY_BUFFER, buf)
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW)
  const loc = gl.getAttribLocation(prog, "p")
  gl.enableVertexAttribArray(loc)
  gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0)

  const tex = gl.createTexture()
  gl.bindTexture(gl.TEXTURE_2D, tex)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)
  gl.enable(gl.BLEND)
  gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA)

  const u = (n: string) => gl.getUniformLocation(prog, n)
  const uRes = u("res"), uTime = u("time"), uSurf = u("surf"), uSlosh = u("slosh")

  let glyphTop = 0.1 // top of the letters, 0..1 from the top of the canvas
  let pos = 1.1 // surface height (uv, 0 top .. 1 bottom); starts under the bottom edge
  let vel = 0
  let target = 1.1
  let visible = false
  let raf = 0
  let last = 0
  let alive = true

  const surfFor = (frac: number) => (frac <= 0 ? 1.1 : 1 - frac * (1 - glyphTop))

  const resize = () => {
    const dpr = Math.min(window.devicePixelRatio || 1, 2)
    const w = Math.max(1, Math.round(host.clientWidth * dpr))
    const h = Math.max(1, Math.round(host.clientHeight * dpr))
    canvas.width = w
    canvas.height = h
    gl.viewport(0, 0, w, h)

    // Draw the word into a 2D canvas the same way the page lays it out, then use it as a mask.
    const cs = getComputedStyle(host)
    const fs = parseFloat(cs.fontSize) * dpr
    const lineH = fs * 0.9
    const m = document.createElement("canvas")
    m.width = w
    m.height = h
    const c = m.getContext("2d")!
    c.font = `${cs.fontWeight} ${fs}px ${cs.fontFamily}`
    const ls = parseFloat(cs.letterSpacing) * dpr
    if (!Number.isNaN(ls) && "letterSpacing" in c) (c as unknown as { letterSpacing: string }).letterSpacing = `${ls}px`
    const word = dotless(host.dataset.word ?? "Barista")
    const mt = c.measureText(word)
    const asc = mt.fontBoundingBoxAscent ?? fs * 0.97
    const desc = mt.fontBoundingBoxDescent ?? fs * 0.24
    const baseline = (lineH - (asc + desc)) / 2 + asc
    c.fillStyle = "#000"
    c.fillText(word, (w - mt.width) / 2, baseline)
    const cap = c.measureText("B").actualBoundingBoxAscent || fs * 0.73
    glyphTop = Math.max(0, (baseline - cap) / h)
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, m)
    gl.uniform2f(uRes, w, h)
    target = surfFor(targetFrac)
    if (reduce) pos = target
    draw(0)
  }

  let targetFrac = 0
  const draw = (t: number) => {
    gl.clearColor(0, 0, 0, 0)
    gl.clear(gl.COLOR_BUFFER_BIT)
    gl.uniform1f(uTime, t)
    gl.uniform1f(uSurf, pos)
    gl.uniform1f(uSlosh, Math.min(1, Math.abs(vel) * 0.7))
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4)
  }

  const frame = (now: number) => {
    raf = 0
    if (!alive || !visible || document.visibilityState !== "visible") return
    if (now - last >= 33) {
      const dt = Math.min(0.05, (now - last) / 1000)
      last = now
      // underdamped spring toward the target surface height
      const k = 38
      const d = 5.5
      vel += ((target - pos) * k - vel * d) * dt
      pos += vel * dt
      draw(now / 1000)
    }
    raf = requestAnimationFrame(frame)
  }
  const kick = () => {
    if (reduce) {
      pos = target
      draw(0)
      return
    }
    if (!raf && alive && visible) {
      last = performance.now() - 40
      raf = requestAnimationFrame(frame)
    }
  }

  const onVis = () => kick()
  document.addEventListener("visibilitychange", onVis)
  const lost = (e: Event) => {
    e.preventDefault()
    alive = false
    cancelAnimationFrame(raf)
    onLost()
  }
  canvas.addEventListener("webglcontextlost", lost)

  resize()
  // The font may load after first paint; redraw the mask once it has.
  void document.fonts?.ready.then(() => alive && resize())

  return {
    resize,
    setTarget: (frac) => {
      targetFrac = frac
      target = surfFor(frac)
      kick()
    },
    setVisible: (v) => {
      visible = v
      if (v) kick()
      else {
        cancelAnimationFrame(raf)
        raf = 0
      }
    },
    destroy: () => {
      alive = false
      cancelAnimationFrame(raf)
      document.removeEventListener("visibilitychange", onVis)
      canvas.removeEventListener("webglcontextlost", lost)
      // No loseContext() here: React remounts reuse this same canvas, and a lost context would stay dead.
    },
  }
}

export function Wordmark({ word = "Barista" }: { word?: string }) {
  const host = useRef<HTMLDivElement>(null)
  const canvas = useRef<HTMLCanvasElement>(null)
  const api = useRef<Gl | null>(null)
  const [mode, setMode] = useState<"css" | "gl">("css")
  const [inView, setInView] = useState(false)

  useEffect(() => {
    const el = host.current!
    const io = new IntersectionObserver(([e]) => setInView(!!e?.isIntersecting), { threshold: 0.35 })
    io.observe(el)
    return () => io.disconnect()
  }, [])

  useEffect(() => {
    const el = host.current!
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches
    const g = start(el, canvas.current!, reduce, () => setMode("css"))
    if (!g) return
    api.current = g
    setMode("gl")
    const ro = new ResizeObserver(() => g.resize())
    ro.observe(el)
    return () => {
      ro.disconnect()
      g.destroy()
      api.current = null
    }
  }, [])

  useEffect(() => {
    const el = host.current!
    const place = () => {
      const d = dotSpot(el)
      el.style.setProperty("--bx", `${d.x}px`)
      el.style.setProperty("--by", `${d.y}px`)
      el.style.setProperty("--bw", `${d.w}px`)
      el.dataset.ready = "true"
    }
    place()
    const ro = new ResizeObserver(place)
    ro.observe(el)
    void document.fonts?.ready.then(place)
    return () => ro.disconnect()
  }, [])

  useEffect(() => {
    api.current?.setVisible(inView)
    api.current?.setTarget(inView ? FILL : 0)
  }, [inView, mode])

  return (
    <div ref={host} className="wordmark" data-word={word} data-mode={mode} data-in={inView} aria-hidden="true">
      <span className="wordmark__text">{dotless(word)}</span>
      <canvas ref={canvas} className="wordmark__gl" />
      <Bean className="wordmark__bean" style={{ left: "var(--bx)", top: "var(--by)", width: "var(--bw)", transform: "rotate(-10deg)" }} />
    </div>
  )
}
