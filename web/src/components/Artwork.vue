<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Rect, Report } from '../types'

const props = defineProps<{ src: string; report: Report | null; hover: number | null }>()
const emit = defineEmits<{ rect: [Rect]; toggle: [number] }>()

const svg = ref<SVGSVGElement>()
const drag = ref<{ x0: number; y0: number; x1: number; y1: number } | null>(null)
const w = computed(() => props.report?.meta.width_px ?? 1)
const h = computed(() => props.report?.meta.height_px ?? 1)
const unit = computed(() => Math.max(w.value, h.value) / 600)
const pad = computed(() => unit.value * 5)

function toImage(e: PointerEvent) {
  const r = svg.value!.getBoundingClientRect()
  return {
    x: Math.max(0, Math.min(w.value, ((e.clientX - r.left) / r.width) * w.value)),
    y: Math.max(0, Math.min(h.value, ((e.clientY - r.top) / r.height) * h.value)),
  }
}
function down(e: PointerEvent) {
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
  if (!d || !props.report) return
  const r = {
    x: Math.round(Math.min(d.x0, d.x1)), y: Math.round(Math.min(d.y0, d.y1)),
    w: Math.round(Math.abs(d.x1 - d.x0)), h: Math.round(Math.abs(d.y1 - d.y0)),
  }
  // a tap (no real drag) toggles the outline under the pointer
  if (r.w < unit.value * 4 && r.h < unit.value * 4) {
    const p = pad.value
    const hits = props.report.blocks
      .map((b, i) => ({ b, i }))
      .filter(({ b }) => d.x0 >= b.rect.x - p && d.x0 <= b.rect.x + b.rect.w + p && d.y0 >= b.rect.y - p && d.y0 <= b.rect.y + b.rect.h + p)
      .sort((a, z) => a.b.rect.w * a.b.rect.h - z.b.rect.w * z.b.rect.h) // smallest first
    if (hits.length) emit('toggle', hits[0].i)
    return
  }
  emit('rect', r)
}

const live = computed(() => {
  const d = drag.value
  return d && { x: Math.min(d.x0, d.x1), y: Math.min(d.y0, d.y1), w: Math.abs(d.x1 - d.x0), h: Math.abs(d.y1 - d.y0) }
})
const legalNo = computed(() => {
  // numbering follows the order of selected blocks, matching the list
  const m = new Map<number, number>()
  let n = 0
  props.report?.blocks.forEach((b, i) => { if (b.legal) m.set(i, ++n) })
  return m
})
</script>

<template>
  <figure class="art">
    <div class="stage">
      <img :src="src" alt="Uploaded artwork" draggable="false" />
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
        <g class="marks">
          <g v-for="(b, i) in report.blocks" :key="i" :class="{ on: b.legal, hot: hover === i, sugg: b.suggested }">
            <rect :x="b.rect.x - pad" :y="b.rect.y - pad" :width="b.rect.w + pad * 2" :height="b.rect.h + pad * 2" class="box" />
            <text v-if="b.legal" :x="b.rect.x - pad" :y="b.rect.y - pad - unit * 3" :font-size="unit * 11" class="num">{{ legalNo.get(i) }}</text>
          </g>
          <rect
            v-if="report.measurement?.bg_varied"
            :x="report.measurement.worst_at.x" :y="report.measurement.worst_at.y"
            :width="report.measurement.worst_at.w" :height="report.measurement.worst_at.h"
            class="worst"
          />
          <rect v-if="live" :x="live.x" :y="live.y" :width="live.w" :height="live.h" class="live" />
        </g>
      </svg>
    </div>
    <figcaption v-if="report" class="legend no-print">
      <span><i class="k-on" /> Selected</span>
      <span><i class="k-off" /> Text found, click to select</span>
      <span class="grey">Or drag around any text</span>
    </figcaption>
  </figure>
</template>

<style scoped>
.art { margin: 0; }
.stage { position: relative; width: fit-content; max-width: 100%; }
.art img { display: block; outline: 1px solid var(--rule); outline-offset: -1px; max-width: 100%; max-height: calc(100vh - 150px); width: auto; height: auto; user-select: none; }
svg { position: absolute; inset: 0; width: 100%; height: 100%; touch-action: none; cursor: crosshair; }
/* difference blend keeps 1px markup visible on any artwork */
.marks { mix-blend-mode: difference; }
.box { fill: transparent; stroke: #fff; stroke-width: 1px; vector-effect: non-scaling-stroke; stroke-dasharray: 2 3; opacity: 0.35; }
.sugg .box { opacity: 0.6; }
.hot .box, .on .box { opacity: 1; }
.on .box { stroke-dasharray: none; stroke-width: 1.5px; }
.num { fill: #fff; font-family: var(--font); }
.worst { fill: none; stroke: #fff; stroke-width: 1px; vector-effect: non-scaling-stroke; stroke-dasharray: 1 2; }
.live { fill: none; stroke: #fff; stroke-width: 1px; vector-effect: non-scaling-stroke; }
.legend { display: flex; flex-wrap: wrap; column-gap: var(--gap); color: var(--grey); margin-top: 8px; }
.legend i { display: inline-block; width: 12px; height: 8px; margin-right: 6px; vertical-align: 0; }
.k-on { border: 1.5px solid var(--ink); }
.k-off { border: 1px dashed var(--grey); }
</style>
