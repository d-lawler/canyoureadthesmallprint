<script setup lang="ts">
import { computed } from 'vue'
import { hex, type Report } from '../types'

const props = defineProps<{ report: Report }>()
const v = computed(() => props.report.verdict!)
const m = computed(() => props.report.measurement!)
const ratio = (r: number) => (Math.floor(r * 100) / 100).toFixed(2)
const result = computed(() => ({ AAA: 'Meets WCAG AAA', AA: 'Meets WCAG AA', Fail: 'Does not meet WCAG AA' })[v.value.level])
const need = computed(() => (v.value.level === 'AAA' ? v.value.required_aaa : v.value.required_aa))
</script>

<template>
  <dt>Result</dt>
  <dd class="result">{{ result }}<span v-if="v.needs_review" class="grey">, check manually</span></dd>

  <dt>Contrast</dt>
  <dd>{{ ratio(m.ratio_worst) }}:1 <span class="grey">{{ v.level === 'Fail' ? 'needs' : 'against' }} {{ need }}:1</span></dd>

  <dt>Text size</dt>
  <dd>{{ v.font_pt.toFixed(1) }}pt <span class="grey">{{ v.large_text ? 'large text' : 'normal text' }}</span></dd>

  <dt>Colours</dt>
  <dd>
    <span class="chip" :style="{ background: hex(m.fg) }" />{{ hex(m.fg) }}
    <span class="grey">&ensp;on&ensp;</span>
    <span class="chip" :style="{ background: hex(m.bg_worst) }" />{{ hex(m.bg_worst) }}
    <span v-if="m.bg_varied" class="grey">, lowest point</span>
  </dd>

  <template v-if="v.suggested_fg">
    <dt>Fix</dt>
    <dd><span class="chip" :style="{ background: hex(v.suggested_fg) }" />{{ hex(v.suggested_fg) }} <span class="grey">meets AA</span></dd>
  </template>
</template>

<style scoped>
.result { font-weight: 500; }
</style>
