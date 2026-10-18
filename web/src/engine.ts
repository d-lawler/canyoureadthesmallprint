import type { FormatInfo, Options, Report } from './types'

const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' })
let seq = 0
const pending = new Map<number, { resolve: (v: any) => void; reject: (e: Error) => void }>()

worker.onmessage = (e) => {
  const p = pending.get(e.data.id)
  if (!p) return
  pending.delete(e.data.id)
  e.data.error ? p.reject(new Error(e.data.error)) : p.resolve(e.data.result)
}

function call<T>(msg: object): Promise<T> {
  const id = ++seq
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject })
    worker.postMessage({ id, ...msg })
  })
}

export const analyze = (bytes: ArrayBuffer, options: Options) =>
  call<Report>({ kind: 'analyze', bytes: bytes.slice(0), options })

export const listFormats = () => call<FormatInfo[]>({ kind: 'formats' })
