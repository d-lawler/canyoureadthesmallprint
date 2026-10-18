// The engine runs here so a 7000px billboard file doesn't freeze the page.
import init, { analyze, formats } from '../pkg/smallpint.js'

const ready = init()

self.onmessage = async (e: MessageEvent) => {
  const { id, kind, bytes, options } = e.data
  try {
    await ready
    const result = kind === 'formats' ? formats() : analyze(new Uint8Array(bytes), options)
    self.postMessage({ id, result })
  } catch (err) {
    self.postMessage({ id, error: err instanceof Error ? err.message : String(err) })
  }
}
