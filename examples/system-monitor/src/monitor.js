// The monitor page: draws the two histories as lines. The numbers come from the main process.
const GRAPH_WIDTH = 59 // 60 samples = 59 gaps

/** Points for a polyline: the newest value at the right edge, 0 at the bottom, 100 at the top. */
function points(values) {
  const offset = GRAPH_WIDTH - (values.length - 1)
  return values.map((v, i) => `${offset + i},${100 - Math.min(100, Math.max(0, v))}`)
}

function draw(name, values) {
  if (values.length === 0) return
  const line = points(values)
  document.getElementById(`${name}-line`).setAttribute("points", line.join(" "))
  const first = line[0].split(",")[0]
  const last = line[line.length - 1].split(",")[0]
  document
    .getElementById(`${name}-area`)
    .setAttribute("points", `${first},100 ${line.join(" ")} ${last},100`)
}

function show(data) {
  if (!data) return
  document.getElementById("cpu").textContent = `${data.cpu}%`
  document.getElementById("ram").textContent = `${data.ram}% (${data.usedGb} of ${data.totalGb} GB)`
  draw("cpu", data.cpuHistory)
  draw("ram", data.ramHistory)
}

window.monitor.onSample(show)
window.monitor.load().then(show)
