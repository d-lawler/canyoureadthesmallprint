<script setup lang="ts">
import { computed } from 'vue'
import type { Block, Report } from '../types'

const props = defineProps<{ report: Report; src: string }>()
const emit = defineEmits<{ remove: [number]; hover: [number | null] }>()

const selected = computed(() => props.report.blocks.map((b, i) => ({ b, i })).filter(({ b }) => b.legal))

// crop of the artwork, scaled so the copy is readable (up to 2x, column width)
function crop(b: Block) {
  const r = b.rect, padPx = Math.max(4, r.h * 0.15)
  const x = Math.max(0, r.x - padPx), y = Math.max(0, r.y - padPx)
  const cw = r.w + padPx * 2, ch = r.h + padPx * 2
  const { width_px: w, height_px: h } = props.report.meta
  const s = Math.min(2, 360 / cw)
  return {
    width: `${Math.round(cw * s)}px`, height: `${Math.round(ch * s)}px`,
    backgroundImage: `url(${props.src})`,
    backgroundSize: `${w * s}px ${h * s}px`,
    backgroundPosition: `${-x * s}px ${-y * s}px`,
  }
}
const ratio = (r: number) => (Math.floor(r * 100) / 100).toFixed(2)
const label = (b: Block) => (b.level === 'Fail' ? 'Below AA' : b.level ? `Meets ${b.level}` : 'Couldn’t measure')
const weakest = (b: Block) => {
  const l = props.report.legal?.rect
  return selected.value.length > 1 && !!l && l.x === b.rect.x && l.y === b.rect.y && l.w === b.rect.w && l.h === b.rect.h
}
</script>

<template>
  <section class="blocks">
    <h2 class="grey head">Legal copy assessed</h2>
    <ol>
      <li v-for="({ b, i }, n) in selected" :key="i" @mouseenter="emit('hover', i)" @mouseleave="emit('hover', null)">
        <span class="n grey">{{ n + 1 }}</span>
        <div>
          <span class="crop" :style="crop(b)" role="img" :aria-label="`Selected copy ${n + 1}, ${b.position} of the artwork`" />
          <p class="meta">
            {{ label(b) }}<span v-if="b.ratio !== null" class="grey">&ensp;{{ ratio(b.ratio) }}:1&ensp;{{ b.font_pt?.toFixed(1) }}pt&ensp;{{ b.position }}</span><span v-if="weakest(b)">&ensp;weakest</span>
          </p>
        </div>
        <button class="act no-print" @click="emit('remove', i)">Remove</button>
      </li>
    </ol>
  </section>
</template>

<style scoped>
.head { padding-bottom: 6px; border-bottom: 1px solid var(--rule); }
ol { list-style: none; padding: 0; }
li { display: grid; grid-template-columns: 1.5em 1fr auto; column-gap: 12px; align-items: start; padding: 12px 0; border-bottom: 1px solid var(--rule); }
.n { font-feature-settings: 'tnum'; }
.crop { display: block; max-width: 100%; background-repeat: no-repeat; outline: 1px solid var(--rule); }
.meta { margin-top: 6px; }
</style>
