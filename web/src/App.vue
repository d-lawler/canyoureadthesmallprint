<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { analyze, listFormats } from './engine'
import type { Category, FormatInfo, Market, MediumHint, Options, Rect, Report } from './types'
import Artwork from './components/Artwork.vue'
import ContextChip from './components/ContextChip.vue'
import Verdict from './components/Verdict.vue'
import Memo from './components/Memo.vue'

const bytes = ref<ArrayBuffer | null>(null)
const fileName = ref('')
const src = ref('')
const report = ref<Report | null>(null)
const busy = ref(false)
const error = ref('')
const drawing = ref(false)
const over = ref(false)
const input = ref<HTMLInputElement>()
const formats = ref<FormatInfo[]>([])
const opts = reactive<Options>({ market: guessMarket() })

function guessMarket(): Market {
  const l = navigator.language.toLowerCase()
  if (l.endsWith('-gb') || l.endsWith('-ie')) return 'uk'
  if (l.endsWith('-us')) return 'us'
  return /-(de|fr|es|it|nl|pt|be|at|se|dk|fi|pl)$/.test(l) ? 'eu' : 'uk'
}

onMounted(async () => {
  formats.value = await listFormats().catch(() => [])
})

async function load(file: File) {
  error.value = ''
  if (!/^image\/(png|jpeg|webp)$/.test(file.type)) {
    error.value = `${file.name} isn’t a PNG, JPG or WebP.`
    return
  }
  if (src.value) URL.revokeObjectURL(src.value)
  fileName.value = file.name
  src.value = URL.createObjectURL(file)
  bytes.value = await file.arrayBuffer()
  delete opts.format_id; delete opts.medium_hint; delete opts.legal_rect
  report.value = null
  drawing.value = false
  await run()
}

async function run() {
  if (!bytes.value) return
  busy.value = true
  try {
    report.value = await analyze(bytes.value, { ...opts, assessed_at: new Date().toISOString().slice(0, 10) })
  } catch (e) {
    error.value = `Couldn’t check this file: ${(e as Error).message}`
  } finally {
    busy.value = false
  }
}

const pick = () => input.value?.click()
function onPick(e: Event) {
  const f = (e.target as HTMLInputElement).files?.[0]
  if (f) load(f)
  ;(e.target as HTMLInputElement).value = ''
}
function onDrop(e: DragEvent) {
  over.value = false
  const f = e.dataTransfer?.files[0]
  if (f) load(f)
}
async function example() {
  const r = await fetch(`${import.meta.env.BASE_URL}examples/story.png`)
  load(new File([await r.blob()], 'example-story.png', { type: 'image/png' }))
}
function setFormat(id: string) { opts.format_id = id; delete opts.medium_hint; run() }
function setHint(h: MediumHint) { opts.medium_hint = h; delete opts.format_id; run() }
function setRect(r: Rect) { opts.legal_rect = r; drawing.value = false; run() }
function autoRect() { delete opts.legal_rect; run() }
function setMarket(e: Event) { opts.market = (e.target as HTMLSelectElement).value as Market; run() }
function setCategory(e: Event) {
  const v = (e.target as HTMLSelectElement).value
  v ? (opts.category = v as Category) : delete opts.category
  run()
}
function reset() { report.value = null; bytes.value = null; src.value = ''; error.value = '' }
</script>

<template>
  <div class="page" @dragover.prevent="over = true" @dragleave.self="over = false" @drop.prevent="onDrop">
    <header class="top">
      <button class="name act-plain" @click="reset">Smallpint</button>
      <p class="grey about">Legal copy accessibility check. Files stay on this device.</p>
      <button class="act no-print upload" @click="pick">{{ src ? 'Upload another' : 'Upload' }}</button>
      <input ref="input" type="file" accept="image/png,image/jpeg,image/webp" hidden @change="onPick" />
    </header>

    <main v-if="!src" class="empty">
      <button class="drop" :class="{ over }" @click="pick">
        <span>Drop an image here, or click to choose one</span>
        <span class="grey">PNG, JPG or WebP</span>
      </button>
      <p class="under">
        <button class="act" @click="example">Try an example</button>
        <span v-if="error" class="err" role="alert">{{ error }}</span>
      </p>
    </main>

    <main v-else class="grid">
      <div class="art">
        <Artwork :src="src" :report="report" :drawing="drawing" @rect="setRect" />
        <p v-if="drawing" class="grey hint no-print">
          Drag a box around the legal copy. <button class="act" @click="drawing = false">Cancel</button>
        </p>
      </div>

      <div class="info" :aria-busy="busy">
        <p v-if="busy && !report" class="grey">Checking…</p>
        <p v-if="error" class="err" role="alert">{{ error }}</p>
        <template v-if="report">
          <dl class="rows">
            <Verdict v-if="report.verdict" :report="report" />
            <template v-else>
              <dt>Result</dt>
              <dd>No legal copy found. <button class="act" @click="drawing = true">Mark it</button></dd>
            </template>

            <ContextChip :report="report" :formats="formats" @format="setFormat" @hint="setHint" />

            <dt>Legal copy</dt>
            <dd>
              <span class="grey">{{ report.legal ? (report.legal.auto_detected ? 'Found automatically' : 'Marked by you') : 'Not found' }}</span>
              <span class="no-print">
                &ensp;<button class="act" @click="drawing = true">Redraw</button>
                <template v-if="opts.legal_rect">&ensp;<button class="act" @click="autoRect">Auto</button></template>
              </span>
            </dd>

            <dt>Market</dt>
            <dd>
              <select class="inline" :value="opts.market" @change="setMarket">
                <option value="uk">UK</option><option value="us">US</option>
                <option value="eu">EU</option><option value="other">Other</option>
              </select>
            </dd>
            <dt>Category</dt>
            <dd>
              <select class="inline" :value="opts.category ?? ''" @change="setCategory">
                <option value="">Not regulated</option><option value="financial">Financial services</option>
                <option value="gambling">Gambling</option><option value="alcohol">Alcohol</option>
                <option value="healthcare">Healthcare</option>
              </select>
            </dd>
          </dl>

          <Memo v-if="report.verdict" :report="report" :file-name="fileName" />
        </template>
      </div>
    </main>
  </div>
</template>

<style scoped>
.page { min-height: 100vh; padding: var(--gap); }
.top {
  display: grid; grid-template-columns: repeat(12, 1fr); column-gap: var(--gap);
  align-items: baseline; padding-bottom: calc(var(--gap) * 3);
}
.name { grid-column: 1 / span 2; justify-self: start; background: none; border: 0; padding: 0; cursor: pointer; font-weight: 500; }
.about { grid-column: 3 / span 6; }
.upload { grid-column: 11 / span 2; justify-self: end; }

.empty { max-width: 100%; }
.drop {
  width: 100%; height: min(62vh, 560px);
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 2px;
  background: none; border: 1px solid var(--rule); cursor: pointer;
  transition: border-color 0.15s;
}
.drop:hover, .drop.over { border-color: var(--ink); }
.under { margin-top: 12px; display: flex; gap: var(--gap); }
.err { color: var(--ink); }
.err::before { content: '— '; }

.grid { display: grid; grid-template-columns: repeat(12, 1fr); column-gap: var(--gap); align-items: start; }
.art { grid-column: 1 / span 6; position: sticky; top: var(--gap); }
.info { grid-column: 8 / span 5; display: grid; row-gap: calc(var(--gap) * 2); }
.hint { margin-top: 8px; }

.rows { display: grid; grid-template-columns: 7.5em 1fr; column-gap: var(--gap); row-gap: 6px; }
.rows :deep(dt) { color: var(--grey); }
.rows :deep(dd) { min-width: 0; }

@media (max-width: 800px) {
  .top { grid-template-columns: 1fr auto; row-gap: 4px; padding-bottom: calc(var(--gap) * 2); }
  .name { grid-column: 1; }
  .upload { grid-column: 2; grid-row: 1; }
  .about { grid-column: 1 / -1; }
  .art, .info { grid-column: 1 / -1; }
  .art { position: static; margin-bottom: calc(var(--gap) * 2); }
}
</style>
