// The monitor page. Everything is dots: LED-style numerals, a dot-matrix history and one dot per
// core (the look of the "Found on Cosmos" design). The numbers come from the main process.

// A 5 x 7 dot font for the digits (1 = lit).
const FONT = {
  0: ["01110", "10001", "10011", "10101", "11001", "10001", "01110"],
  1: ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
  2: ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
  3: ["11110", "00001", "00001", "01110", "00001", "00001", "11110"],
  4: ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
  5: ["11111", "10000", "11110", "00001", "00001", "10001", "01110"],
  6: ["00110", "01000", "10000", "11110", "10001", "10001", "01110"],
  7: ["11111", "00001", "00010", "00100", "01000", "01000", "01000"],
  8: ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
  9: ["01110", "10001", "10001", "01111", "00001", "00010", "01100"],
  ".": ["0", "0", "0", "0", "0", "0", "1"],
  "-": ["00000", "00000", "00000", "11111", "00000", "00000", "00000"],
}

const ROWS = 7

/** Sizes a canvas to its CSS box at the screen's pixel density. Returns the context and size in CSS px. */
function prepare(canvas) {
  const ratio = window.devicePixelRatio || 1
  const width = canvas.clientWidth
  const height = canvas.clientHeight
  if (canvas.width !== Math.round(width * ratio) || canvas.height !== Math.round(height * ratio)) {
    canvas.width = Math.round(width * ratio)
    canvas.height = Math.round(height * ratio)
  }
  const ctx = canvas.getContext("2d")
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0)
  ctx.clearRect(0, 0, width, height)
  return { ctx, width, height }
}

function dot(ctx, x, y, r, color) {
  ctx.fillStyle = color
  ctx.beginPath()
  ctx.arc(x, y, r, 0, Math.PI * 2)
  ctx.fill()
}

/** The columns of the text in the dot font: a row of 0/1 per column, one blank column between characters. */
function matrixOf(text) {
  const rows = Array.from({ length: ROWS }, () => [])
  for (const [index, char] of [...text].entries()) {
    const glyph = FONT[char] ?? FONT["-"]
    if (index > 0) for (const row of rows) row.push(0)
    glyph.forEach((line, y) => {
      for (const bit of line) rows[y].push(bit === "1" ? 1 : 0)
    })
  }
  return rows
}

/** A big number made of dots, centred, with the unlit dots left faintly visible like an LED panel. */
function drawNumber(canvas, text, lit, dim) {
  const { ctx, width, height } = prepare(canvas)
  const matrix = matrixOf(text)
  const cols = matrix[0].length
  const pitch = Math.min(height / ROWS, width / cols)
  const x0 = (width - pitch * cols) / 2
  const y0 = (height - pitch * ROWS) / 2
  const r = pitch * 0.34
  matrix.forEach((row, y) =>
    row.forEach((on, x) => dot(ctx, x0 + (x + 0.5) * pitch, y0 + (y + 0.5) * pitch, r, on ? lit : dim)),
  )
}

/** Mixes two [r, g, b] colours. */
const mix = (a, b, t) => a.map((v, i) => Math.round(v + (b[i] - v) * t))
const rgb = (c, alpha = 1) => `rgb(${c[0]} ${c[1]} ${c[2]} / ${alpha})`

/** A dot-matrix bar chart: one column per sample, the newest at the right, rows lit from the bottom. */
function drawHistory(canvas, values, low, high) {
  const { ctx, width, height } = prepare(canvas)
  const cols = 60
  const rows = 7
  const px = width / cols
  const py = height / rows
  const r = Math.min(px, py) * 0.3
  const start = cols - values.length
  for (let x = 0; x < cols; x++) {
    const value = x >= start ? values[x - start] : null
    const litRows = value === null ? 0 : Math.round((Math.min(100, Math.max(0, value)) / 100) * rows)
    for (let y = 0; y < rows; y++) {
      const fromBottom = rows - 1 - y
      const on = fromBottom < litRows
      const color = on ? rgb(mix(low, high, fromBottom / (rows - 1))) : "rgb(255 255 255 / 0.1)"
      dot(ctx, (x + 0.5) * px, (y + 0.5) * py, r, color)
    }
  }
}

/** One dot per core: bigger and brighter the busier it is (a halftone). */
function drawCores(canvas, cores) {
  const { ctx, width, height } = prepare(canvas)
  const count = Math.max(1, cores.length)
  const cols = Math.max(1, Math.round(Math.sqrt(count * (width / height))))
  const rows = Math.ceil(count / cols)
  const pitch = Math.min(width / cols, height / rows)
  const x0 = (width - pitch * cols) / 2
  const y0 = (height - pitch * rows) / 2
  const max = pitch * 0.42
  for (let i = 0; i < count; i++) {
    const load = Math.min(100, Math.max(0, cores[i] ?? 0)) / 100
    const x = x0 + ((i % cols) + 0.5) * pitch
    const y = y0 + (Math.floor(i / cols) + 0.5) * pitch
    const hot = mix([240, 240, 245], [255, 150, 90], Math.max(0, (load - 0.55) / 0.45))
    dot(ctx, x, y, 1.4 + (max - 1.4) * load, rgb(hot, 0.3 + 0.7 * load))
  }
}

function uptime(seconds) {
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  if (days > 0) return `${days}d ${hours}h`
  if (hours > 0) return `${hours}h ${minutes}m`
  return `${minutes}m`
}

const $ = (id) => document.getElementById(id)
let last

function show(data) {
  if (!data) return
  last = data
  drawNumber($("cpu-big"), String(data.cpu), "rgb(255 255 255)", "rgb(255 255 255 / 0.13)")
  drawHistory($("cpu-history"), data.cpuHistory, [255, 255, 255], [255, 120, 215])
  $("threads").textContent = `${data.cores.length} threads`

  drawNumber($("ram-big"), String(data.usedGb), "rgb(255 255 255)", "rgb(255 255 255 / 0.13)")
  $("ram-of").textContent = `of ${data.totalGb} GB`
  $("ram-percent").textContent = `${data.ram}%`
  $("ram-fill").style.width = `${data.ram}%`
  $("ram-thumb").style.left = `${data.ram}%`

  drawCores($("cores"), data.cores)
  $("busiest").textContent = `${Math.max(0, ...data.cores)}% peak`
  $("uptime").textContent = uptime(data.uptime)
  // Later changes glide; the first draw must not slide in from zero.
  requestAnimationFrame(() => document.body.classList.add("ready"))
}

// The popover can be shown at another screen density: draw again with the new pixel ratio.
window.addEventListener("resize", () => last && show(last))

window.monitor.onSample(show)
window.monitor.load().then(show)
