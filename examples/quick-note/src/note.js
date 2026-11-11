// The note page: a text box that saves itself a moment after you stop typing.
const text = document.getElementById("text")
const status = document.getElementById("status")
let timer

async function save() {
  clearTimeout(timer)
  await window.notes.save(text.value)
  status.textContent = "Saved"
}

text.addEventListener("input", () => {
  status.textContent = "Typing…"
  clearTimeout(timer)
  timer = setTimeout(save, 400)
})

// Each time the popover shows: reload the note (another window may have changed it) and focus.
async function show() {
  text.value = await window.notes.load()
  status.textContent = "Esc to close"
  text.focus()
  text.setSelectionRange(text.value.length, text.value.length)
}
window.notes.onShow(show)
void show()
