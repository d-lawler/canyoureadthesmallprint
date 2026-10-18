<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { analyze, listFormats } from './engine'
import type { Category, FormatInfo, Market, MediumHint, Options, Rect, Report } from './types'
import Artwork from './components/Artwork.vue'
import ContextChip from './components/ContextChip.vue'
import Verdict from './components/Verdict.vue'
import Memo from './components/Memo.vue'
import Blocks from './components/Blocks.vue'

const bytes = ref<ArrayBuffer | null>(null)
const fileName = ref('')
const src = ref('')
const report = ref<Report | null>(null)
const busy = ref(false)
const error = ref('')
const hover = ref<number | null>(null)
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
  delete opts.format_id; delete opts.medium_hint
  opts.legal_rects = [] // nothing is assessed until the user picks the copy
  report.value = null
  hover.value = null
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
const legalRects = () => (report.value?.blocks ?? []).filter((b) => b.legal).map((b) => b.rect)
function setRect(r: Rect) { opts.legal_rects = [...legalRects(), r]; run() }
function toggle(i: number) {
  const blocks = report.value?.blocks ?? []
  opts.legal_rects = blocks.filter((b, j) => (j === i ? !b.legal : b.legal)).map((b) => b.rect)
  hover.value = null
  run()
}
function selectSuggested() {
  opts.legal_rects = (report.value?.blocks ?? []).filter((b) => b.suggested).map((b) => b.rect)
  run()
}
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
        <Artwork :src="src" :report="report" :hover="hover" @rect="setRect" @toggle="toggle" />
      </div>

      <div class="info" :aria-busy="busy">
        <p v-if="busy && !report" class="grey">Reading the artwork…</p>
        <p v-if="error" class="err" role="alert">{{ error }}</p>
        <template v-if="report">
          <section v-if="!report.verdict" class="pick">
            <p class="strong">Select the legal copy</p>
            <p class="grey">Click the outlined text on the artwork, or drag a box around it. Select every block of small print you need checked.</p>
            <p v-if="report.blocks.some((b) => b.suggested)" class="no-print">
              <button class="act" @click="selectSuggested">Select the smallest text for me</button>
            </p>
          </section>

          <template v-else>
            <dl class="rows">
              <Verdict :report="report" />
            </dl>
            <Blocks :report="report" :src="src" @remove="toggle" @hover="(i) => (hover = i)" />
          </template>

          <dl class="rows">
            <ContextChip :report="report" :formats="formats" @format="setFormat" @hint="setHint" />
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
  display: flex; align-items: baseline; column-gap: 12px; row-gap: 4px; flex-wrap: wrap;
  padding-bottom: calc(var(--gap) * 3);
}
.name { background: none; border: 0; padding: 0; cursor: pointer; font-weight: 500; }
.upload { margin-left: auto; }

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

.grid { display: grid; grid-template-columns: minmax(0, max-content) minmax(320px, 520px); column-gap: calc(var(--gap) * 3); align-items: start; }
.art { position: sticky; top: var(--gap); max-width: 58vw; }
.info { display: grid; row-gap: calc(var(--gap) * 2); min-width: 0; }
.pick { display: grid; row-gap: 6px; max-width: 40ch; }
.strong { font-weight: 500; }

.rows { display: grid; grid-template-columns: 7.5em 1fr; column-gap: var(--gap); row-gap: 6px; }
.rows :deep(dt) { color: var(--grey); }
.rows :deep(dd) { min-width: 0; }

@media (max-width: 800px) {
  .top { padding-bottom: calc(var(--gap) * 2); }
  .about { order: 3; flex-basis: 100%; }
  .grid { grid-template-columns: 1fr; }
  .art { position: static; max-width: none; margin-bottom: calc(var(--gap) * 2); }
}
</style>
