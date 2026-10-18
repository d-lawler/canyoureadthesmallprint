<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Rect, Report } from '../types'

const props = defineProps<{ src: string; report: Report | null; drawing: boolean }>()
const emit = defineEmits<{ rect: [Rect] }>()

const svg = ref<SVGSVGElement>()
const drag = ref<{ x0: number; y0: number; x1: number; y1: number } | null>(null)
const w = computed(() => props.report?.meta.width_px ?? 1)
const h = computed(() => props.report?.meta.height_px ?? 1)
// stroke widths in image px that stay ~2 screen px whatever the artwork size
const unit = computed(() => Math.max(w.value, h.value) / 600)

function toImage(e: PointerEvent) {
  const r = svg.value!.getBoundingClientRect()
  return {
    x: Math.max(0, Math.min(w.value, ((e.clientX - r.left) / r.width) * w.value)),
    y: Math.max(0, Math.min(h.value, ((e.clientY - r.top) / r.height) * h.value)),
  }
}
function down(e: PointerEvent) {
  if (!props.drawing) return
  const p = toImage(e)
  drag.value = { x0: p.x, y0: p.y, x1: p.x, y1: p.y }
  svg.value!.setPointerCapture(e.pointerId)
}
function move(e: PointerEvent) {
  if (!drag.value) return
  const p = toImage(e)
  drag.value.x1 = p.x
  drag.value.y1 = p.y
}
function up() {
  const d = drag.value
  drag.value = null
  if (!d) return
  const r = {
    x: Math.round(Math.min(d.x0, d.x1)),
    y: Math.round(Math.min(d.y0, d.y1)),
    w: Math.round(Math.abs(d.x1 - d.x0)),
    h: Math.round(Math.abs(d.y1 - d.y0)),
  }
  if (r.w > 4 && r.h > 4) emit('rect', r)
}

const legal = computed(() => props.report?.legal?.rect)
const pad = computed(() => unit.value * 6)
// crop-mark corners around the legal copy, like a proof markup
const marks = computed(() => {
  const r = legal.value
  if (!r) return ''
  const p = pad.value, l = unit.value * 14
  const x0 = r.x - p, y0 = r.y - p, x1 = r.x + r.w + p, y1 = r.y + r.h + p
  return [
    `M${x0} ${y0 + l}V${y0}H${x0 + l}`, `M${x1 - l} ${y0}H${x1}V${y0 + l}`,
    `M${x1} ${y1 - l}V${y1}H${x1 - l}`, `M${x0 + l} ${y1}H${x0}V${y1 - l}`,
  ].join('')
})
const live = computed(() => {
  const d = drag.value
  return d && { x: Math.min(d.x0, d.x1), y: Math.min(d.y0, d.y1), w: Math.abs(d.x1 - d.x0), h: Math.abs(d.y1 - d.y0) }
})
</script>

<template>
  <figure class="art" :class="{ drawing }">
    <div class="stage">
    <img :src="src" alt="Uploaded artwork" />
    <svg
      v-if="report"
      ref="svg"
      :viewBox="`0 0 ${w} ${h}`"
      preserveAspectRatio="none"
      @pointerdown="down"
      @pointermove="move"
      @pointerup="up"
      @pointercancel="drag = null"
    >
      <g v-if="!drawing">
        <rect
          v-for="(l, i) in report.text_lines" :key="i"
          :x="l.rect.x" :y="l.rect.y" :width="l.rect.w" :height="l.rect.h"
          class="line" :stroke-width="unit"
        />
        <template v-if="legal">
          <rect :x="legal.x - pad" :y="legal.y - pad" :width="legal.w + pad * 2" :height="legal.h + pad * 2" class="legal-fill" />
          <path :d="marks" class="legal" :stroke-width="unit" />
        </template>
        <rect
          v-if="report.measurement?.bg_varied"
          :x="report.measurement.worst_at.x" :y="report.measurement.worst_at.y"
          :width="report.measurement.worst_at.w" :height="report.measurement.worst_at.h"
          class="worst" :stroke-width="unit"
        />
      </g>
      <rect v-if="live" :x="live.x" :y="live.y" :width="live.w" :height="live.h" class="live" :stroke-width="unit * 2" />
    </svg>
    </div>
    <figcaption v-if="report" class="legend no-print">
      <span><i class="k-legal" /> Legal copy</span>
      <span v-if="report.text_lines.length > 1"><i class="k-line" /> Other text</span>
      <span v-if="report.measurement?.bg_varied"><i class="k-worst" /> Lowest contrast</span>
    </figcaption>
  </figure>
</template>

<style scoped>
.art { margin: 0; }
.stage { position: relative; width: fit-content; max-width: 100%; }
.art img { display: block; outline: 1px solid var(--rule); outline-offset: -1px; max-width: 100%; max-height: calc(100vh - 160px); width: auto; height: auto; }
svg { position: absolute; inset: 0; width: 100%; height: 100%; touch-action: none; }
.drawing svg { cursor: crosshair; }
/* difference blend keeps 1px markup visible on any artwork */
g, .live { mix-blend-mode: difference; }
.line { fill: none; stroke: #fff; stroke-width: 1px; vector-effect: non-scaling-stroke; stroke-dasharray: 2 3; opacity: 0.5; }
.legal { fill: none; stroke: #fff; stroke-linecap: square; }
.legal-fill { fill: none; stroke: #fff; stroke-width: 1px; vector-effect: non-scaling-stroke; }
.worst { fill: none; stroke: #fff; stroke-width: 1px; vector-effect: non-scaling-stroke; stroke-dasharray: 1 2; }
.live { fill: none; stroke: #fff; stroke-width: 1px; vector-effect: non-scaling-stroke; }
.legend { display: flex; gap: var(--gap); color: var(--grey); margin-top: 8px; }
.legend i { display: inline-block; width: 12px; height: 8px; margin-right: 6px; vertical-align: 0; }
.k-legal { border: 1px solid var(--ink); }
.k-line { border: 1px dashed var(--grey); }
.k-worst { border: 1px dotted var(--ink); }
</style>
