<script setup lang="ts">
// The component-library showcase at /ui: every component from '@/components/ui' with 2–3 live
// variants, a hand-written prop table, and its slots/emits. Owned by W3.
import { ref } from 'vue'
import '@/components/ui/ui.css'
import {
  UiBadge, UiButton, UiCard, UiEmptyState, UiField, UiFileList, UiIconButton, UiKeyValue, UiModal,
  UiNumberInput, UiPanel, UiSelect, UiSpinner, UiStateBadge, UiTable, UiTabs, UiTag, UiTextArea,
  UiTextInput, UiTile, UiToolbar,
} from '@/components/ui'
import { iconNames } from '@/components/ui/icons'

type PropRow = { prop: string; type: string; default: string; notes: string }
type Meta = { desc: string; slots: string; emits: string; props: PropRow[] }

const propCols = [
  { key: 'prop', label: 'Prop', width: '20%' },
  { key: 'type', label: 'Type', width: '34%' },
  { key: 'default', label: 'Default', width: '14%' },
  { key: 'notes', label: 'Notes' },
]
const p = (prop: string, type: string, def = '—', notes = ''): PropRow => ({ prop, type, default: def, notes })

const m = {
  UiButton: { desc: 'The button. Always a <button type="button">; pass type="submit" to override.', slots: 'default', emits: 'click(MouseEvent)', props: [
    p('variant', "'primary' | 'secondary' | 'ghost' | 'danger'", "'secondary'", 'primary = accent bg, danger = fault bg'),
    p('size', "'sm' | 'md' | 'lg'", "'md'"),
    p('disabled', 'boolean', 'false'),
    p('loading', 'boolean', 'false', 'shows UiSpinner inline, disables, aria-busy'),
    p('icon', 'string', '—', 'glyph name from icons.ts, rendered before the label'),
  ] },
  UiIconButton: { desc: 'A square, icon-only button. `label` is the accessible name and the tooltip.', slots: '—', emits: 'click(MouseEvent)', props: [
    p('icon', 'string', 'required'), p('label', 'string', 'required', 'aria-label + title'), p('size', "'sm' | 'md' | 'lg'", "'md'"),
  ] },
  UiTile: { desc: 'A large card (min 280×180) for grids of swarms or actions. Keyboard-activatable when clickable.', slots: 'default, footer', emits: 'click(MouseEvent | KeyboardEvent)', props: [
    p('title', 'string', 'required'), p('subtitle', 'string', '—', 'clamped to 2 lines'),
    p('accent', 'string', '—', "token colour name -> left bar, e.g. 'accent', 'ok', 'warn', 'info'"),
    p('big', 'boolean', 'false', 'doubles padding, title 16px -> 28px'), p('clickable', 'boolean', 'false', 'role=button, tabindex, hover lift'),
  ] },
  UiCard: { desc: 'A titled box with optional header extras and footer.', slots: 'default, header (right of the title), footer', emits: '—', props: [p('title', 'string')] },
  UiPanel: { desc: 'A titled section, optionally collapsible. Collapse state is internal; `open` seeds and drives it.', slots: 'default, actions (in the header)', emits: '—', props: [
    p('title', 'string'), p('collapsible', 'boolean', 'false', 'header becomes a toggle button with chevron'), p('open', 'boolean', 'true'),
  ] },
  UiBadge: { desc: 'A small tinted pill with a dot.', slots: '—', emits: '—', props: [p('tone', "'ok' | 'warn' | 'fault' | 'info' | 'muted'", "'muted'"), p('text', 'string', 'required')] },
  UiTag: { desc: 'A neutral chip, optionally removable.', slots: '—', emits: 'remove()', props: [p('text', 'string', 'required'), p('removable', 'boolean', 'false')] },
  UiTable: { desc: 'A data table with a sticky header. Generic over the row type; rows become clickable only when @row-click is bound.', slots: 'cell-<key> { row, value }, empty', emits: 'row-click(row)', props: [
    p('columns', '{ key, label, width?, align? }[]', 'required', "align: 'left' | 'center' | 'right'"), p('rows', 'object[]', 'required'),
    p('rowKey', 'string', "'id'", 'falls back to the index'), p('dense', 'boolean', 'false'),
  ] },
  UiFileList: { desc: 'Files and directories with kind glyph, mono path, human size and relative mtime. Arrow keys move, Enter selects.', slots: '—', emits: 'select(path)', props: [
    p('files', "{ path, size?, mtime?, kind?: 'file' | 'dir' }[]", 'required', 'mtime: ISO string | epoch ms | Date'), p('selected', 'string', '—', 'path of the highlighted row'),
  ] },
  UiModal: { desc: 'Teleported dialog with backdrop. Escape and backdrop click emit close; focus is trapped and restored.', slots: 'default, footer', emits: 'close()', props: [
    p('open', 'boolean', 'required'), p('title', 'string'), p('width', 'string', "'560px'"),
  ] },
  UiTextInput: { desc: 'Single-line text input. With `label` it renders through UiField; other attrs reach the <input>.', slots: '—', emits: 'update:modelValue(string)', props: [
    p('modelValue', 'string', 'required', 'v-model'), p('label', 'string'), p('placeholder', 'string'), p('error', 'string', '—', 'red text + aria-invalid'), p('mono', 'boolean', 'false'),
  ] },
  UiTextArea: { desc: 'Multi-line text input, vertically resizable.', slots: '—', emits: 'update:modelValue(string)', props: [
    p('modelValue', 'string', 'required', 'v-model'), p('label', 'string'), p('placeholder', 'string'), p('rows', 'number', '3'), p('error', 'string'), p('mono', 'boolean', 'false', 'extension'),
  ] },
  UiNumberInput: { desc: 'Numeric input with an optional unit suffix. Cleared -> undefined, never NaN.', slots: '—', emits: 'update:modelValue(number | undefined)', props: [
    p('modelValue', 'number | undefined', 'required', 'v-model'), p('label', 'string'), p('min', 'number'), p('max', 'number'), p('step', 'number'), p('unit', 'string', '—', 'suffix inside the box'), p('error', 'string'), p('mono', 'boolean', 'false', 'extension'),
  ] },
  UiSelect: { desc: 'Native <select> with a custom chevron. An unmatched model shows an empty row.', slots: '—', emits: 'update:modelValue(string)', props: [
    p('modelValue', 'string', 'required', 'v-model'), p('options', '{ value, label }[]', 'required'), p('label', 'string'), p('error', 'string'), p('mono', 'boolean', 'false', 'extension'),
  ] },
  UiField: { desc: 'Label + control + hint/error. The default slot gets { id, describedBy, invalid } for the control to bind.', slots: 'default { id, describedBy, invalid }', emits: '—', props: [
    p('label', 'string', 'required'), p('hint', 'string'), p('error', 'string', '—', 'replaces the hint'),
  ] },
  UiToolbar: { desc: 'A flex row: default slot left, `right` slot pushed to the end.', slots: 'default (left), right', emits: '—', props: [] },
  UiEmptyState: { desc: 'Centered icon, title, text and actions for empty lists.', slots: 'default (actions)', emits: '—', props: [
    p('title', 'string', 'required'), p('text', 'string'), p('icon', 'string', "'info'"),
  ] },
  UiSpinner: { desc: 'An inline loading indicator (role=status).', slots: '—', emits: '—', props: [p('size', "'sm' | 'md'", "'md'")] },
  UiTabs: { desc: 'role=tablist with roving tabindex; Left/Right/Home/End move and select.', slots: '—', emits: 'update:modelValue(string)', props: [
    p('modelValue', 'string', 'required', 'v-model'), p('tabs', '{ value, label }[]', 'required'),
  ] },
  UiKeyValue: { desc: 'A two-column definition list.', slots: '—', emits: '—', props: [
    p('items', '{ key: string, value: unknown }[]', 'required', 'null/undefined -> —, booleans -> yes/no, objects -> JSON'), p('mono', 'boolean', 'false', 'monospace values'),
  ] },
  UiStateBadge: { desc: 'UiBadge coloured by lifecycle state: Running/Working/Succeeded/Answered -> ok; Blocked/Parked/Paused/Held -> warn; Faulted/Failed/Escalated -> fault; Idle/Created/Draft/Open -> info; else muted.', slots: '—', emits: '—', props: [
    p('state', 'string', 'required', 'case-insensitive lookup; the text is the state as given'),
  ] },
} satisfies Record<string, Meta>

const names = Object.keys(m) as (keyof typeof m)[]

// --- live state for the demos -------------------------------------------------------------
const log = ref<string[]>([])
function note(msg: string) {
  log.value = [`${new Date().toLocaleTimeString()}  ${msg}`, ...log.value].slice(0, 8)
}

const loading = ref(false)
function fakeLoad() {
  loading.value = true
  setTimeout(() => { loading.value = false }, 1500)
}

const tags = ref(['claude', 'codex', 'b10x'])

const swarmCols = [
  { key: 'displayName', label: 'Swarm' },
  { key: 'state', label: 'State', width: '120px' },
  { key: 'agents', label: 'Agents', align: 'right' as const, width: '90px' },
  { key: 'createdAt', label: 'Created', width: '140px' },
]
interface SwarmRow { swarmId: string; displayName: string; state: string; agents: number; createdAt: string }
const swarms: SwarmRow[] = [
  { swarmId: 's-1', displayName: 'harness-builder', state: 'Running', agents: 4, createdAt: '2026-09-11' },
  { swarmId: 's-2', displayName: 'org-brain-ingest', state: 'Paused', agents: 2, createdAt: '2026-09-08' },
  { swarmId: 's-3', displayName: 'daemonloom-release', state: 'Faulted', agents: 1, createdAt: '2026-09-02' },
  { swarmId: 's-4', displayName: 'aep-drive-wave-7', state: 'Created', agents: 0, createdAt: '2026-09-11' },
]

const now = Date.now()
const files = [
  { path: '.agents', kind: 'dir' as const, mtime: now - 3 * 3600e3 },
  { path: '.agents/orchestrator.md', size: 2311, mtime: now - 5 * 60e3 },
  { path: '.agents/improver.md', size: 14872, mtime: now - 2 * 3600e3 },
  { path: '.engineering/planning', kind: 'dir' as const, mtime: now - 26 * 3600e3 },
  { path: 'swarm/seed.json', size: 1_204_331, mtime: now - 4 * 86400e3 },
  { path: 'README.md', size: 812, mtime: '2026-07-14T09:30:00Z' },
]
const selectedFile = ref('.agents/orchestrator.md')

const modalOpen = ref(false)
const modalName = ref('')

const name = ref('harness-builder')
const objective = ref('Build the swarm web UI: splash, swarm canvas, flow editor, component library.')
const cwd = ref('/home/timo/beyond10x/harness-builder')
const budget = ref<number | undefined>(200000)
const maxAgents = ref<number | undefined>(undefined)
const harness = ref('claude')
const harnessOptions = [
  { value: 'claude', label: 'Claude Code' },
  { value: 'codex', label: 'Codex' },
  { value: 'b10x', label: 'b10x' },
]
const badHarness = ref('')

const tab = ref('agents')
const tabs = [
  { value: 'agents', label: 'Agents' },
  { value: 'boxes', label: 'Blackboxes' },
  { value: 'decisions', label: 'Decisions' },
]

const panelOpen = ref(true)

const states = ['Running', 'Working', 'Succeeded', 'Answered', 'Blocked', 'Parked', 'Paused', 'Held', 'Faulted', 'Failed', 'Escalated', 'Idle', 'Created', 'Draft', 'Open', 'Retired', 'Moot']
</script>

<template>
  <div class="ui-showcase">
    <nav class="side" aria-label="Components">
      <div class="side-title">components</div>
      <a v-for="n in names" :key="n" :href="`#${n}`">{{ n }}</a>
      <a href="#icons">icons.ts</a>
      <div class="side-title log-title">events</div>
      <ul class="log" aria-live="polite">
        <li v-for="(l, i) in log" :key="i">{{ l }}</li>
        <li v-if="!log.length" class="muted">interact with a demo</li>
      </ul>
    </nav>

    <div class="main">
      <h1>Component library</h1>
      <p class="lead">
        21 components, plain Vue 3 + scoped CSS, every colour and spacing from <code>styles/tokens.css</code>.
        Import from <code>@/components/ui</code>.
      </p>

      <!-- UiButton -->
      <section id="UiButton" class="demo">
        <h2>UiButton</h2>
        <p class="desc">{{ m.UiButton.desc }}</p>
        <div class="stage">
          <div class="row">
            <UiButton variant="primary" @click="note('UiButton primary click')">Primary</UiButton>
            <UiButton @click="note('UiButton secondary click')">Secondary</UiButton>
            <UiButton variant="ghost" @click="note('UiButton ghost click')">Ghost</UiButton>
            <UiButton variant="danger" @click="note('UiButton danger click')">Danger</UiButton>
          </div>
          <div class="row">
            <UiButton size="sm" icon="plus">Small</UiButton>
            <UiButton size="md" icon="plus">Medium</UiButton>
            <UiButton size="lg" icon="plus">Large</UiButton>
          </div>
          <div class="row">
            <UiButton variant="primary" icon="play" :loading="loading" @click="fakeLoad">{{ loading ? 'Starting' : 'Start swarm' }}</UiButton>
            <UiButton disabled>Disabled</UiButton>
            <UiButton variant="danger" icon="trash" size="sm">Delete</UiButton>
            <UiButton variant="ghost" icon="search" />
          </div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiButton.slots }, { key: 'emits', value: m.UiButton.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiButton.props" row-key="prop" dense />
      </section>

      <!-- UiIconButton -->
      <section id="UiIconButton" class="demo">
        <h2>UiIconButton</h2>
        <p class="desc">{{ m.UiIconButton.desc }}</p>
        <div class="stage">
          <div class="row">
            <UiIconButton icon="play" label="Run" @click="note('UiIconButton run')" />
            <UiIconButton icon="pause" label="Pause" @click="note('UiIconButton pause')" />
            <UiIconButton icon="stop" label="Stop" @click="note('UiIconButton stop')" />
            <UiIconButton icon="trash" label="Delete" @click="note('UiIconButton delete')" />
          </div>
          <div class="row">
            <UiIconButton icon="plus" label="Add (sm)" size="sm" />
            <UiIconButton icon="plus" label="Add (md)" size="md" />
            <UiIconButton icon="plus" label="Add (lg)" size="lg" />
            <UiIconButton icon="close" label="Disabled" disabled />
          </div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiIconButton.slots }, { key: 'emits', value: m.UiIconButton.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiIconButton.props" row-key="prop" dense />
      </section>

      <!-- UiTile -->
      <section id="UiTile" class="demo">
        <h2>UiTile</h2>
        <p class="desc">{{ m.UiTile.desc }}</p>
        <div class="stage">
          <div class="grid">
            <UiTile title="harness-builder" subtitle="Build the swarm web UI: splash, swarm canvas, flow editor and the component library, all against the seed." accent="accent" clickable @click="note('UiTile harness-builder click')">
              <div class="row"><UiStateBadge state="Running" /><UiBadge tone="muted" text="4 agents" /></div>
              <template #footer><span class="muted">started 2 h ago</span></template>
            </UiTile>
            <UiTile title="daemonloom-release" subtitle="Cut v0.23 after the full pre-release suite." accent="fault" clickable @click="note('UiTile daemonloom click')">
              <UiStateBadge state="Faulted" />
              <template #footer><UiButton size="sm" variant="ghost" icon="play">Resume</UiButton></template>
            </UiTile>
            <UiTile title="Static tile" subtitle="Not clickable, no accent." />
          </div>
          <UiTile title="Create a swarm" subtitle="A big tile: double padding, 28px title. Objective, budget and harness go in a modal." accent="ok" big clickable @click="note('UiTile big click')">
            <template #footer><UiButton variant="primary" icon="plus">New swarm</UiButton></template>
          </UiTile>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiTile.slots }, { key: 'emits', value: m.UiTile.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiTile.props" row-key="prop" dense />
      </section>

      <!-- UiCard -->
      <section id="UiCard" class="demo">
        <h2>UiCard</h2>
        <p class="desc">{{ m.UiCard.desc }}</p>
        <div class="stage">
          <div class="grid">
            <UiCard title="Plain card">Body content goes here. Cards do not lift on hover; tiles do.</UiCard>
            <UiCard title="With header slot and footer">
              <template #header><UiBadge tone="info" text="3 open" /><UiIconButton icon="close" label="Dismiss" size="sm" /></template>
              Header extras sit to the right of the title.
              <template #footer><UiButton size="sm">Cancel</UiButton><UiButton size="sm" variant="primary">Save</UiButton></template>
            </UiCard>
            <UiCard>No title: the header row is skipped entirely.</UiCard>
          </div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiCard.slots }, { key: 'emits', value: m.UiCard.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiCard.props" row-key="prop" dense />
      </section>

      <!-- UiPanel -->
      <section id="UiPanel" class="demo">
        <h2>UiPanel</h2>
        <p class="desc">{{ m.UiPanel.desc }}</p>
        <div class="stage">
          <div class="grid">
            <UiPanel title="Limits">
              <UiKeyValue :items="[{ key: 'budgetTokens', value: 200000 }, { key: 'maxAgents', value: 6 }, { key: 'diskFloorGb', value: undefined }]" mono />
            </UiPanel>
            <UiPanel title="Agents" collapsible :open="panelOpen">
              <template #actions><UiIconButton icon="plus" label="Add agent" size="sm" @click="note('UiPanel action: add agent')" /></template>
              Collapsible: click the header to fold. The actions slot stays clickable on its own.
            </UiPanel>
            <UiPanel title="Starts collapsed" collapsible :open="false">Hidden until expanded.</UiPanel>
          </div>
          <div class="row"><UiButton size="sm" @click="panelOpen = !panelOpen">Toggle the Agents panel from outside ({{ panelOpen ? 'open' : 'closed' }})</UiButton></div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiPanel.slots }, { key: 'emits', value: m.UiPanel.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiPanel.props" row-key="prop" dense />
      </section>

      <!-- UiBadge -->
      <section id="UiBadge" class="demo">
        <h2>UiBadge</h2>
        <p class="desc">{{ m.UiBadge.desc }}</p>
        <div class="stage">
          <div class="row">
            <UiBadge tone="ok" text="ok" /><UiBadge tone="warn" text="warn" /><UiBadge tone="fault" text="fault" /><UiBadge tone="info" text="info" /><UiBadge tone="muted" text="muted" /><UiBadge text="default tone" />
          </div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiBadge.slots }, { key: 'emits', value: m.UiBadge.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiBadge.props" row-key="prop" dense />
      </section>

      <!-- UiTag -->
      <section id="UiTag" class="demo">
        <h2>UiTag</h2>
        <p class="desc">{{ m.UiTag.desc }}</p>
        <div class="stage">
          <div class="row">
            <UiTag text="static" />
            <UiTag v-for="t in tags" :key="t" :text="t" removable @remove="tags = tags.filter((x) => x !== t); note(`UiTag remove ${t}`)" />
            <UiButton v-if="tags.length < 3" size="sm" variant="ghost" icon="plus" @click="tags = ['claude', 'codex', 'b10x']">Reset</UiButton>
          </div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiTag.slots }, { key: 'emits', value: m.UiTag.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiTag.props" row-key="prop" dense />
      </section>

      <!-- UiTable -->
      <section id="UiTable" class="demo">
        <h2>UiTable</h2>
        <p class="desc">{{ m.UiTable.desc }}</p>
        <div class="stage">
          <UiTable :columns="swarmCols" :rows="swarms" row-key="swarmId" @row-click="(r) => note(`UiTable row-click ${r.displayName}`)">
            <template #cell-state="{ value }"><UiStateBadge :state="String(value)" /></template>
            <template #cell-displayName="{ row }"><span class="ui-mono">{{ row.displayName }}</span></template>
          </UiTable>
          <UiTable :columns="swarmCols" :rows="swarms.slice(0, 2)" row-key="swarmId" dense class="short" />
          <UiTable :columns="swarmCols" :rows="[]">
            <template #empty><UiEmptyState title="No swarms yet" text="The empty slot takes anything; here it is a UiEmptyState." icon="folder"><UiButton size="sm" variant="primary" icon="plus">Create one</UiButton></UiEmptyState></template>
          </UiTable>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiTable.slots }, { key: 'emits', value: m.UiTable.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiTable.props" row-key="prop" dense />
      </section>

      <!-- UiFileList -->
      <section id="UiFileList" class="demo">
        <h2>UiFileList</h2>
        <p class="desc">{{ m.UiFileList.desc }}</p>
        <div class="stage">
          <UiFileList :files="files" :selected="selectedFile" @select="(path) => { selectedFile = path; note(`UiFileList select ${path}`) }" />
          <p class="muted">selected: <code>{{ selectedFile }}</code> — focus the list, use Up/Down and Enter.</p>
          <UiFileList :files="[]" />
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiFileList.slots }, { key: 'emits', value: m.UiFileList.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiFileList.props" row-key="prop" dense />
      </section>

      <!-- UiModal -->
      <section id="UiModal" class="demo">
        <h2>UiModal</h2>
        <p class="desc">{{ m.UiModal.desc }}</p>
        <div class="stage">
          <div class="row"><UiButton variant="primary" icon="plus" @click="modalOpen = true">Open modal</UiButton></div>
          <UiModal :open="modalOpen" title="New swarm" @close="modalOpen = false; note('UiModal close')">
            <div class="stack">
              <UiTextInput v-model="modalName" label="Display name" placeholder="e.g. harness-builder" />
              <UiSelect v-model="harness" label="Harness" :options="harnessOptions" />
              <UiNumberInput v-model="budget" label="Budget" unit="tokens" :min="0" :step="1000" />
            </div>
            <template #footer>
              <UiButton @click="modalOpen = false">Cancel</UiButton>
              <UiButton variant="primary" @click="modalOpen = false; note(`UiModal create ${modalName || '(unnamed)'}`)">Create</UiButton>
            </template>
          </UiModal>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiModal.slots }, { key: 'emits', value: m.UiModal.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiModal.props" row-key="prop" dense />
      </section>

      <!-- UiTextInput -->
      <section id="UiTextInput" class="demo">
        <h2>UiTextInput</h2>
        <p class="desc">{{ m.UiTextInput.desc }}</p>
        <div class="stage">
          <div class="form-grid">
            <UiTextInput v-model="name" label="Display name" placeholder="swarm name" />
            <UiTextInput v-model="cwd" label="Working directory" mono />
            <UiTextInput v-model="name" label="With error" error="Must be unique across swarms" />
            <UiTextInput v-model="name" placeholder="no label" />
          </div>
          <p class="muted">model: <code>{{ name }}</code></p>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiTextInput.slots }, { key: 'emits', value: m.UiTextInput.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiTextInput.props" row-key="prop" dense />
      </section>

      <!-- UiTextArea -->
      <section id="UiTextArea" class="demo">
        <h2>UiTextArea</h2>
        <p class="desc">{{ m.UiTextArea.desc }}</p>
        <div class="stage">
          <div class="form-grid">
            <UiTextArea v-model="objective" label="Objective" :rows="4" />
            <UiTextArea v-model="objective" label="Mono, with error" mono :rows="4" error="Objective too vague" />
          </div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiTextArea.slots }, { key: 'emits', value: m.UiTextArea.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiTextArea.props" row-key="prop" dense />
      </section>

      <!-- UiNumberInput -->
      <section id="UiNumberInput" class="demo">
        <h2>UiNumberInput</h2>
        <p class="desc">{{ m.UiNumberInput.desc }}</p>
        <div class="stage">
          <div class="form-grid">
            <UiNumberInput v-model="budget" label="Budget" unit="tokens" :min="0" :step="1000" />
            <UiNumberInput v-model="maxAgents" label="Max agents" :min="1" :max="32" />
            <UiNumberInput v-model="budget" label="With error" unit="GB" error="Above the disk floor" />
          </div>
          <p class="muted">budget: <code>{{ budget === undefined ? 'undefined' : budget }}</code> · maxAgents: <code>{{ maxAgents === undefined ? 'undefined' : maxAgents }}</code> — clear a field to get undefined.</p>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiNumberInput.slots }, { key: 'emits', value: m.UiNumberInput.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiNumberInput.props" row-key="prop" dense />
      </section>

      <!-- UiSelect -->
      <section id="UiSelect" class="demo">
        <h2>UiSelect</h2>
        <p class="desc">{{ m.UiSelect.desc }}</p>
        <div class="stage">
          <div class="form-grid">
            <UiSelect v-model="harness" label="Harness" :options="harnessOptions" />
            <UiSelect v-model="badHarness" label="Unmatched model (empty row)" :options="harnessOptions" error="Pick a harness" />
            <UiSelect v-model="harness" :options="harnessOptions" />
          </div>
          <p class="muted">harness: <code>{{ harness }}</code></p>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiSelect.slots }, { key: 'emits', value: m.UiSelect.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiSelect.props" row-key="prop" dense />
      </section>

      <!-- UiField -->
      <section id="UiField" class="demo">
        <h2>UiField</h2>
        <p class="desc">{{ m.UiField.desc }}</p>
        <div class="stage">
          <div class="form-grid">
            <UiField label="Native input via the slot" hint="The slot passes id/describedBy/invalid" v-slot="{ id, describedBy }">
              <input :id="id" class="ui-control" :aria-describedby="describedBy" placeholder="type here" />
            </UiField>
            <UiField label="With error" error="Something is wrong" v-slot="{ id, describedBy, invalid }">
              <input :id="id" class="ui-control" :class="{ invalid }" :aria-describedby="describedBy" :aria-invalid="invalid" value="bad value" />
            </UiField>
            <UiField label="Wrapping anything" hint="A field can hold non-input content too">
              <div class="row"><UiTag text="claude" /><UiTag text="codex" /></div>
            </UiField>
          </div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiField.slots }, { key: 'emits', value: m.UiField.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiField.props" row-key="prop" dense />
      </section>

      <!-- UiToolbar -->
      <section id="UiToolbar" class="demo">
        <h2>UiToolbar</h2>
        <p class="desc">{{ m.UiToolbar.desc }}</p>
        <div class="stage">
          <UiToolbar>
            <UiButton variant="primary" icon="plus" size="sm">New</UiButton>
            <UiButton icon="play" size="sm">Start</UiButton>
            <UiButton icon="pause" size="sm">Pause</UiButton>
            <template #right>
              <UiTextInput v-model="name" placeholder="search" class="search" />
              <UiIconButton icon="search" label="Search" />
            </template>
          </UiToolbar>
          <UiToolbar>
            <UiTabs v-model="tab" :tabs="tabs" />
            <template #right><UiBadge tone="ok" text="live" /></template>
          </UiToolbar>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiToolbar.slots }, { key: 'emits', value: m.UiToolbar.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiToolbar.props" row-key="prop" dense>
          <template #empty>no props</template>
        </UiTable>
      </section>

      <!-- UiEmptyState -->
      <section id="UiEmptyState" class="demo">
        <h2>UiEmptyState</h2>
        <p class="desc">{{ m.UiEmptyState.desc }}</p>
        <div class="stage">
          <div class="grid">
            <UiCard><UiEmptyState title="No agents" text="Spawn one from the toolbar or let the orchestrator assign work." icon="folder"><UiButton variant="primary" icon="plus" size="sm">Spawn agent</UiButton><UiButton variant="ghost" size="sm">Learn more</UiButton></UiEmptyState></UiCard>
            <UiCard><UiEmptyState title="Nothing matches" text="Try a different filter." icon="search" /></UiCard>
            <UiCard><UiEmptyState title="Title only" :icon="''" /></UiCard>
          </div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiEmptyState.slots }, { key: 'emits', value: m.UiEmptyState.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiEmptyState.props" row-key="prop" dense />
      </section>

      <!-- UiSpinner -->
      <section id="UiSpinner" class="demo">
        <h2>UiSpinner</h2>
        <p class="desc">{{ m.UiSpinner.desc }}</p>
        <div class="stage">
          <div class="row"><UiSpinner size="sm" /><UiSpinner /><span class="accent"><UiSpinner /></span><span class="muted">inherits currentColor</span></div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiSpinner.slots }, { key: 'emits', value: m.UiSpinner.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiSpinner.props" row-key="prop" dense />
      </section>

      <!-- UiTabs -->
      <section id="UiTabs" class="demo">
        <h2>UiTabs</h2>
        <p class="desc">{{ m.UiTabs.desc }}</p>
        <div class="stage">
          <UiTabs v-model="tab" :tabs="tabs" @update:modelValue="(v) => note(`UiTabs -> ${v}`)" />
          <p class="muted">active: <code>{{ tab }}</code> — focus a tab and use Left/Right/Home/End.</p>
          <UiTabs v-model="tab" :tabs="[{ value: 'agents', label: 'One' }, { value: 'boxes', label: 'Two' }]" />
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiTabs.slots }, { key: 'emits', value: m.UiTabs.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiTabs.props" row-key="prop" dense />
      </section>

      <!-- UiKeyValue -->
      <section id="UiKeyValue" class="demo">
        <h2>UiKeyValue</h2>
        <p class="desc">{{ m.UiKeyValue.desc }}</p>
        <div class="stage">
          <div class="grid">
            <UiCard title="Proportional"><UiKeyValue :items="[{ key: 'swarmId', value: 's-1' }, { key: 'state', value: 'Running' }, { key: 'startedAt', value: null }, { key: 'paused', value: false }]" /></UiCard>
            <UiCard title="Mono"><UiKeyValue mono :items="[{ key: 'tmuxSession', value: 'swarm-harness-builder' }, { key: 'home', value: '/home/timo/beyond10x/harness-builder' }, { key: 'limits', value: { budgetTokens: 200000, maxAgents: 6 } }]" /></UiCard>
          </div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiKeyValue.slots }, { key: 'emits', value: m.UiKeyValue.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiKeyValue.props" row-key="prop" dense />
      </section>

      <!-- UiStateBadge -->
      <section id="UiStateBadge" class="demo">
        <h2>UiStateBadge</h2>
        <p class="desc">{{ m.UiStateBadge.desc }}</p>
        <div class="stage">
          <div class="row"><UiStateBadge v-for="s in states" :key="s" :state="s" /></div>
        </div>
        <UiKeyValue class="meta" :items="[{ key: 'slots', value: m.UiStateBadge.slots }, { key: 'emits', value: m.UiStateBadge.emits }]" />
        <UiTable class="props" :columns="propCols" :rows="m.UiStateBadge.props" row-key="prop" dense />
      </section>

      <!-- icons -->
      <section id="icons" class="demo">
        <h2>icons.ts</h2>
        <p class="desc">The names accepted by every <code>icon</code> prop. Inline SVG, 24×24, stroke = currentColor.</p>
        <div class="stage">
          <div class="icon-grid">
            <div v-for="n in iconNames" :key="n" class="icon-cell">
              <UiIconButton :icon="n" :label="n" size="lg" />
              <code>{{ n }}</code>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.ui-showcase { display: flex; flex: 1; min-height: 0; }
.side {
  position: sticky; top: 0; align-self: flex-start; max-height: 100vh; overflow: auto;
  display: flex; flex-direction: column; gap: 2px; width: 220px; flex: none;
  padding: var(--space-4); border-right: 1px solid var(--color-border); background: var(--color-surface);
}
.side a { color: var(--color-text-muted); text-decoration: none; font-size: 13px; padding: 2px var(--space-2); border-radius: var(--radius-1); }
.side a:hover { color: var(--color-text); background: var(--color-surface-2); }
.side-title { font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: .06em; color: var(--color-text-muted); padding: 0 var(--space-2); margin-bottom: var(--space-1); }
.log-title { margin-top: var(--space-4); }
.log { list-style: none; margin: 0; padding: 0 var(--space-2); font-family: var(--font-mono); font-size: 11px; color: var(--color-text-muted); display: flex; flex-direction: column; gap: 2px; }
.log li { overflow-wrap: anywhere; }

.main { flex: 1; min-width: 0; padding: var(--space-5) var(--space-6) var(--space-8); }
h1 { margin: 0 0 var(--space-2); font-size: 24px; }
.lead { margin: 0 0 var(--space-6); color: var(--color-text-muted); }
code { font-family: var(--font-mono); font-size: 12px; background: var(--color-surface-2); padding: 1px var(--space-1); border-radius: var(--radius-1); }

.demo { padding: var(--space-5) 0; border-top: 1px solid var(--color-border); scroll-margin-top: var(--space-4); }
.demo h2 { margin: 0 0 var(--space-1); font-size: 18px; font-family: var(--font-mono); }
.desc { margin: 0 0 var(--space-3); color: var(--color-text-muted); max-width: 72em; }
.stage { display: flex; flex-direction: column; gap: var(--space-4); padding: var(--space-4); border: 1px dashed var(--color-border); border-radius: var(--radius-2); background: var(--color-bg); }
.row { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-2); }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: var(--space-4); align-items: start; }
.form-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: var(--space-4); align-items: start; }
.stack { display: flex; flex-direction: column; gap: var(--space-3); }
.meta { margin-top: var(--space-3); font-size: 13px; }
.props { margin-top: var(--space-2); }
.short { max-height: 96px; }
.search { width: 220px; }
.muted { color: var(--color-text-muted); margin: 0; font-size: 13px; }
.accent { color: var(--color-accent); display: inline-flex; }
.icon-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(96px, 1fr)); gap: var(--space-3); }
.icon-cell { display: flex; flex-direction: column; align-items: center; gap: var(--space-1); }
</style>
