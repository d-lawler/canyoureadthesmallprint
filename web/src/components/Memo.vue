<script setup lang="ts">
import { ref } from 'vue'
import type { Report } from '../types'

const props = defineProps<{ report: Report; fileName: string }>()
const copied = ref(false)

async function copy() {
  await navigator.clipboard.writeText(props.report.memo.text)
  copied.value = true
  setTimeout(() => (copied.value = false), 2000)
}
const print = () => window.print()
function download() {
  const blob = new Blob([props.report.memo.text + '\n'], { type: 'text/plain' })
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = props.fileName.replace(/\.[^.]+$/, '') + '-rationale.txt'
  a.click()
  URL.revokeObjectURL(a.href)
}
</script>

<template>
  <section class="memo">
    <header>
      <h2 class="grey">Rationale</h2>
      <p class="no-print actions">
        <button class="act" @click="copy">{{ copied ? 'Copied' : 'Copy' }}</button>
        <button class="act" @click="download">Download</button>
        <button class="act" @click="print">Print</button>
      </p>
    </header>
    <div class="text">
      <p>{{ report.memo.summary }}</p>
      <p v-for="(p, i) in report.memo.paragraphs" :key="i">{{ p }}</p>
      <p class="grey">{{ report.memo.footer }}</p>
    </div>
    <details v-if="report.path.length" class="path no-print">
      <summary class="grey">How this was decided</summary>
      <ol>
        <li v-for="(s, i) in report.path" :key="i"><span class="grey">{{ s.question }}</span><span>{{ s.answer }}</span></li>
      </ol>
    </details>
  </section>
</template>

<style scoped>
header { display: flex; justify-content: space-between; gap: var(--gap); padding-bottom: 6px; border-bottom: 1px solid var(--rule); margin-bottom: 12px; }
.actions { display: flex; gap: 12px; }
.text { max-width: 62ch; }
.text p + p { margin-top: var(--lead); }
.path { margin-top: calc(var(--gap) * 1.5); }
.path summary { cursor: pointer; list-style: none; }
.path summary::-webkit-details-marker { display: none; }
.path summary::after { content: ' +'; }
.path[open] summary::after { content: ' –'; }
.path ol { list-style: none; padding: 0; margin-top: 6px; }
.path li { display: grid; grid-template-columns: 12em 1fr; column-gap: var(--gap); }
</style>
