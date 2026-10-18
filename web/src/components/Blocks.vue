<script setup lang="ts">
import type { Block, Report } from '../types'

const props = defineProps<{ report: Report; src: string }>()
const emit = defineEmits<{ toggle: [number]; add: []; auto: [] }>()

const H = 26, MAXW = 200
function thumb(b: Block) {
  const r = b.rect
  const s = Math.min(H / r.h, MAXW / r.w, 1.5)
  const { width_px: w, height_px: h } = props.report.meta
  return {
    width: `${Math.round(r.w * s)}px`,
    height: `${Math.round(r.h * s)}px`,
    backgroundImage: `url(${props.src})`,
    backgroundSize: `${w * s}px ${h * s}px`,
    backgroundPosition: `${-r.x * s}px ${-r.y * s}px`,
  }
}
const level = (b: Block) => (b.level === 'Fail' ? 'Below AA' : b.level ?? '—')
const isWeakest = (b: Block) => {
  const l = props.report.legal?.rect
  return !!l && l.x === b.rect.x && l.y === b.rect.y && l.w === b.rect.w && l.h === b.rect.h
}
</script>

<template>
  <section class="blocks">
    <header>
      <h2 class="grey">Text found <span v-if="report.blocks.length">· tick the legal copy</span></h2>
      <p class="no-print actions">
        <button class="act" @click="emit('add')">Mark an area</button>
        <button v-if="!report.legal?.auto_detected" class="act" @click="emit('auto')">Auto</button>
      </p>
    </header>
    <p v-if="!report.blocks.length" class="grey">No text found. Mark the legal copy on the artwork.</p>
    <ol>
      <li v-for="(b, i) in report.blocks" :key="i" :class="{ off: !b.legal }">
        <label>
          <input type="checkbox" :checked="b.legal" @change="emit('toggle', i)" />
          <span class="n">{{ i + 1 }}</span>
          <span class="thumb" :style="thumb(b)" role="img" :aria-label="`Text block ${i + 1}, ${b.position}`" />
        </label>
        <span class="meta">
          <template v-if="b.ratio !== null">
            {{ level(b) }}<span class="grey">, {{ (Math.floor(b.ratio * 100) / 100).toFixed(2) }}:1, {{ b.font_pt?.toFixed(1) }}pt</span>
          </template>
          <span v-else class="grey">Couldn’t measure</span>
          <span v-if="b.legal && isWeakest(b) && report.blocks.filter((x) => x.legal).length > 1" class="grey">, weakest</span>
          <span v-if="b.manual" class="grey">, marked by you</span>
        </span>
      </li>
    </ol>
  </section>
</template>

<style scoped>
header { display: flex; justify-content: space-between; gap: var(--gap); padding-bottom: 6px; border-bottom: 1px solid var(--rule); margin-bottom: 6px; }
.actions { display: flex; gap: 12px; }
ol { list-style: none; padding: 0; }
li { display: grid; grid-template-columns: auto 1fr; column-gap: var(--gap); align-items: center; padding: 6px 0; border-bottom: 1px solid var(--rule); }
label { display: flex; align-items: center; gap: 10px; cursor: pointer; min-width: 0; }
input { margin: 0; accent-color: var(--ink); }
.n { width: 1.5em; color: var(--grey); font-feature-settings: 'tnum'; }
.thumb { display: block; background-repeat: no-repeat; outline: 1px solid var(--rule); max-width: 200px; }
li.off .thumb { opacity: 0.45; }
li.off .meta { color: var(--grey); }
.meta { text-align: right; }
</style>
