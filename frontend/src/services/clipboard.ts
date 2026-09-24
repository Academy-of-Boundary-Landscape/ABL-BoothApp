// Shared clipboard helper
// Attempts navigator.clipboard first, falls back to a hidden textarea.
export async function copyLink(text: string): Promise<void> {
  if (!text) throw new Error('No text to copy')

  // Prefer modern async API when available and in secure context
  if (navigator.clipboard && window.isSecureContext) {
    console.debug('[clipboard] using navigator.clipboard')
    await navigator.clipboard.writeText(text)
    console.debug('[clipboard] copy success (navigator)')
    return
  }

  console.debug('[clipboard] using textarea fallback')
  const textarea = document.createElement('textarea')
  textarea.value = text
  textarea.setAttribute('readonly', '')
  textarea.style.position = 'fixed'
  textarea.style.top = '-9999px'
  textarea.style.left = '-9999px'
  document.body.appendChild(textarea)

  textarea.focus()
  textarea.select()

  try {
    const successful = document.execCommand('copy')
    if (!successful) throw new Error('execCommand copy failed')
    console.debug('[clipboard] copy success (fallback)')
  } finally {
    document.body.removeChild(textarea)
  }
}
