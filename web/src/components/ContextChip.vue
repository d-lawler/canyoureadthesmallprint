<script setup lang="ts">
import { computed, ref } from 'vue'
import type { FormatInfo, MediumHint, Report } from '../types'

const props = defineProps<{ report: Report; formats: FormatInfo[] }>()
const emit = defineEmits<{ format: [string]; hint: [MediumHint] }>()
const open = ref(false)

const c = computed(() => props.report.context.chosen)
const seen = computed(() => {
  const s = c.value.scale
  if (s.kind === 'screen') return `${Math.round(s.css_width)}px wide on screen`
  const w = s.width_mm >= 1000 ? `${(s.width_mm / 1000).toFixed(2)}m` : `${Math.round(s.width_mm)}mm`
  const d = s.viewing_m < 1 ? `${Math.round(s.viewing_m * 100)}cm` : `${s.viewing_m}m`
  return `${w} wide, seen from ${d}`
})
const hints: { hint: MediumHint; label: string }[] = [
  { hint: 'phone', label: 'Phone' }, { hint: 'web', label: 'Web' },
  { hint: 'billboard', label: 'Billboard' }, { hint: 'print', label: 'Print' },
]
function choose(e: Event) {
  const v = (e.target as HTMLSelectElement).value
  if (v) { open.value = false; emit('format', v) }
}
</script>

<template>
  <dt>Format</dt>
  <dd>
    <template v-if="report.context.needs_question">
      <span>Unknown size. Where will it run?</span><br />
      <span class="no-print">
        <template v-for="(h, i) in hints" :key="h.hint">
          <button class="act" @click="emit('hint', h.hint)">{{ h.label }}</button><span v-if="i < hints.length - 1">&ensp;</span>
        </template>
      </span>
      <br /><span class="grey">Assumed phone until you choose.</span>
    </template>
    <template v-else>
      {{ c.name }}&ensp;<button class="act no-print" @click="open = !open">{{ open ? 'Close' : 'Change' }}</button>
      <br /><span class="grey">{{ seen }}</span>
      <div v-if="open" class="alts no-print">
        <button v-for="a in report.context.alternatives" :key="a.format_id" class="act" @click="open = false; emit('format', a.format_id)">{{ a.name }}</button>
        <select class="inline" @change="choose">
          <option value="">Other…</option>
          <option v-for="f in formats" :key="f.id" :value="f.id">{{ f.name }}</option>
        </select>
      </div>
    </template>
  </dd>
  <dt>Detected</dt>
  <dd class="grey">{{ c.reason }}</dd>
</template>

<style scoped>
.alts { display: flex; flex-direction: column; align-items: flex-start; gap: 2px; margin-top: 6px; }
</style>
