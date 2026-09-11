// Glyphs for every `icon` prop in the library. 24x24 viewBox, stroke-based (Feather-like) unless
// `fill` is set. Resolved by name at render time through UiIcon.vue; unknown names render nothing.
export interface IconDef {
  d: string
  fill?: boolean
}

export const icons = {
  plus: { d: 'M12 5v14M5 12h14' },
  close: { d: 'M18 6 6 18M6 6l12 12' },
  'chevron-down': { d: 'm6 9 6 6 6-6' },
  'chevron-right': { d: 'm9 6 6 6-6 6' },
  play: { d: 'M7 4l13 8-13 8z', fill: true },
  pause: { d: 'M6 4h4v16H6zM14 4h4v16h-4z', fill: true },
  stop: { d: 'M6 6h12v12H6z', fill: true },
  trash: { d: 'M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14M10 10v7M14 10v7' },
  check: { d: 'm5 12 5 5L20 7' },
  warn: { d: 'M12 3 2 21h20L12 3zM12 10v5M12 18h.01' },
  info: { d: 'M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 11v6M12 7h.01' },
  search: { d: 'M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16zM21 21l-4.3-4.3' },
  file: { d: 'M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8l-6-6zM14 2v6h6' },
  folder: { d: 'M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z' },
} satisfies Record<string, IconDef>

export type IconName = keyof typeof icons

export const iconNames = Object.keys(icons) as IconName[]

export function resolveIcon(name: string): IconDef | undefined {
  return (icons as Record<string, IconDef>)[name]
}
