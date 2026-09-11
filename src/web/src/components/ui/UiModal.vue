<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useId, watch } from 'vue'
import UiIconButton from './UiIconButton.vue'

const props = withDefaults(defineProps<{ open: boolean; title?: string; width?: string }>(), { title: undefined, width: '560px' })
const emit = defineEmits<{ close: [] }>()
defineSlots<{ default?(): any; footer?(): any }>()

const panel = ref<HTMLElement>()
const titleId = useId()
let restoreFocus: HTMLElement | null = null
let prevOverflow = ''
let downOnBackdrop = false

const FOCUSABLE =
  'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])'

function focusables(within: Element | undefined = panel.value): HTMLElement[] {
  if (!within) return []
  return Array.from(within.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((el) => el.getClientRects().length > 0)
}

function onOpen() {
  restoreFocus = document.activeElement as HTMLElement | null
  prevOverflow = document.body.style.overflow
  document.body.style.overflow = 'hidden'
  document.addEventListener('keydown', onKeydown)
  nextTick(() => {
    // Prefer something in the body/footer over the header's close button; fall back to the panel.
    const content = [...focusables(panel.value?.querySelector('.body') ?? undefined), ...focusables(panel.value?.querySelector('.footer') ?? undefined)]
    const target = content[0] ?? focusables()[0] ?? panel.value
    target?.focus()
  })
}

function onClose() {
  document.body.style.overflow = prevOverflow
  document.removeEventListener('keydown', onKeydown)
  restoreFocus?.focus()
  restoreFocus = null
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    emit('close')
    return
  }
  if (e.key !== 'Tab') return
  const els = focusables()
  if (!els.length) {
    e.preventDefault()
    panel.value?.focus()
    return
  }
  const first = els[0]!
  const last = els[els.length - 1]!
  const active = document.activeElement
  const inside = !!panel.value && panel.value.contains(active)
  if (e.shiftKey && (active === first || !inside)) {
    e.preventDefault()
    last.focus()
  } else if (!e.shiftKey && (active === last || !inside)) {
    e.preventDefault()
    first.focus()
  }
}

function onBackdropMousedown(e: MouseEvent) {
  downOnBackdrop = e.target === e.currentTarget
}

function onBackdropClick(e: MouseEvent) {
  // Only a press that started AND ended on the backdrop closes; a drag out of the panel does not.
  if (downOnBackdrop && e.target === e.currentTarget) emit('close')
  downOnBackdrop = false
}

watch(
  () => props.open,
  (v, old) => {
    if (v && !old) onOpen()
    else if (!v && old) onClose()
  },
  { immediate: true },
)
onBeforeUnmount(() => {
  if (props.open) onClose()
})
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="ui-modal-backdrop" @mousedown="onBackdropMousedown" @click="onBackdropClick">
      <div
        ref="panel"
        class="ui-modal"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="title ? titleId : undefined"
        :style="{ width }"
        tabindex="-1"
      >
        <header class="header">
          <h2 v-if="title" :id="titleId" class="title">{{ title }}</h2>
          <UiIconButton icon="close" label="Close" size="sm" class="close" @click="emit('close')" />
        </header>
        <div class="body"><slot /></div>
        <footer v-if="$slots.footer" class="footer"><slot name="footer" /></footer>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.ui-modal-backdrop {
  position: fixed; inset: 0; z-index: 1000;
  display: flex; align-items: center; justify-content: center; padding: var(--space-5);
  background: color-mix(in srgb, var(--color-bg) 72%, transparent);
}
.ui-modal {
  display: flex; flex-direction: column; max-width: 100%; max-height: calc(100vh - var(--space-8));
  background: var(--color-surface); border: 1px solid var(--color-border); border-radius: var(--radius-3);
  box-shadow: var(--shadow-2); color: var(--color-text); outline: none;
}
.header { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3) var(--space-4); border-bottom: 1px solid var(--color-border); }
.title { margin: 0; font-size: 16px; font-weight: 600; }
.close { margin-left: auto; }
.body { padding: var(--space-4); overflow: auto; flex: 1; min-height: 0; }
.footer { display: flex; align-items: center; justify-content: flex-end; gap: var(--space-2); padding: var(--space-3) var(--space-4); border-top: 1px solid var(--color-border); }
</style>
