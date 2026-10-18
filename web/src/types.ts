// Mirrors smallpint-core's serialised Report.

export type Rgb = [number, number, number]
export interface Rect { x: number; y: number; w: number; h: number }
export type Medium = 'mobile' | 'display' | 'video' | 'dooh' | 'ooh' | 'print'
export type MediumHint = 'phone' | 'web' | 'billboard' | 'print'
export type Confidence = 'low' | 'medium' | 'high' | 'confirmed'
export type Level = 'AAA' | 'AA' | 'Fail'
export type Market = 'uk' | 'us' | 'eu' | 'other'
export type Category = 'financial' | 'gambling' | 'alcohol' | 'healthcare'

export type Scale =
  | { kind: 'screen'; css_px_per_px: number; css_width: number }
  | { kind: 'physical'; mm_per_px: number; viewing_m: number; width_mm: number }

export interface Candidate {
  format_id: string
  name: string
  medium: Medium
  confidence: Confidence
  scale: Scale
  reason: string
}

export interface Report {
  meta: { width_px: number; height_px: number; format: string; dpi: number | null; dpi_source: string | null }
  context: { chosen: Candidate; alternatives: Candidate[]; needs_question: boolean }
  legal: { rect: Rect; auto_detected: boolean } | null
  blocks: Block[]
  text_lines: { rect: Rect; glyph_px: number; glyphs: number }[]
  measurement: {
    fg: Rgb; bg_typical: Rgb; bg_worst: Rgb
    ratio_typical: number; ratio_worst: number
    bg_varied: boolean; worst_at: Rect
    font_px: number; lines: number; confidence: number
  } | null
  verdict: {
    level: Level; ratio: number; required_aa: number; required_aaa: number
    large_text: boolean; font_pt: number; suggested_fg: Rgb | null; needs_review: boolean
  } | null
  path: { question: string; answer: string }[]
  memo: { summary: string; paragraphs: string[]; footer: string; text: string; fragment_ids: string[] }
  ruleset: string
  hash: string
}

export interface Block {
  rect: Rect
  legal: boolean
  manual: boolean
  suggested: boolean
  position: string
  font_pt: number | null
  ratio: number | null
  level: Level | null
}

export interface Options {
  format_id?: string
  medium_hint?: MediumHint
  legal_rects?: Rect[]
  market?: Market
  category?: Category
  assessed_at?: string
}

export interface FormatInfo { id: string; name: string; medium: Medium }

export const hex = (c: Rgb) => '#' + c.map((v) => v.toString(16).padStart(2, '0')).join('').toUpperCase()
