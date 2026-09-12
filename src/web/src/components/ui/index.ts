// The component library's public surface. Owned by the integrator as a CONTRACT; W3 implements
// every component listed here as `./<Name>.vue` with the props documented. Consumers import from
// '@/components/ui' only.
//
// Name           props (all optional unless marked)                                   slots / emits
// UiButton       variant: 'primary'|'secondary'|'ghost'|'danger'; size: 'sm'|'md'|'lg';   default; emits click
//                disabled: boolean; loading: boolean; icon: string
// UiIconButton   icon*: string; label*: string (aria); size                            emits click
// UiTile         title*: string; subtitle: string; accent: string; big: boolean;        default, footer; emits click
//                clickable: boolean
// UiCard         title: string                                                          default, header, footer
// UiPanel        title: string; collapsible: boolean; open: boolean                    default, actions
// UiBadge        tone: 'ok'|'warn'|'fault'|'info'|'muted'; text*: string               —
// UiTag          text*: string; removable: boolean                                     emits remove
// UiTable        columns*: {key,label,width?,align?}[]; rows*: Record<string,unknown>[];  cell-<key>, empty; emits row-click(row)
//                rowKey: string; dense: boolean
// UiFileList     files*: {path,size?,mtime?,kind?:'file'|'dir'}[]; selected: string    emits select(path)
// UiModal        open*: boolean; title: string; width: string                          default, footer; emits close
// UiTextInput    modelValue*: string; label; placeholder; error; mono: boolean          emits update:modelValue
// UiTextArea     modelValue*: string; label; placeholder; rows: number; error          emits update:modelValue
// UiNumberInput  modelValue*: number|undefined; label; min; max; step; unit; error     emits update:modelValue
// UiSelect       modelValue*: string; options*: {value,label}[]; label; error         emits update:modelValue
// UiField        label*: string; hint: string; error: string                           default
// UiToolbar      —                                                                     default (left), right
// UiEmptyState   title*: string; text: string; icon: string                            default (actions)
// UiSpinner      size: 'sm'|'md'                                                       —
// UiTabs         modelValue*: string; tabs*: {value,label}[]                           emits update:modelValue
// UiKeyValue     items*: {key,value}[]; mono: boolean                                  —
// UiStateBadge   state*: string (any lifecycle state; colour by a fixed map:           —
//                Running/Working/Succeeded/Answered → ok, Blocked/Parked/Paused/Held → warn,
//                Faulted/Failed/Escalated → fault, Idle/Created/Draft/Open → info, else muted)

export { default as UiButton } from './UiButton.vue'
export { default as UiIconButton } from './UiIconButton.vue'
export { default as UiTile } from './UiTile.vue'
export { default as UiCard } from './UiCard.vue'
export { default as UiPanel } from './UiPanel.vue'
export { default as UiBadge } from './UiBadge.vue'
export { default as UiTag } from './UiTag.vue'
export { default as UiTable } from './UiTable.vue'
export { default as UiFileList } from './UiFileList.vue'
export { default as UiModal } from './UiModal.vue'
export { default as UiTextInput } from './UiTextInput.vue'
export { default as UiTextArea } from './UiTextArea.vue'
export { default as UiNumberInput } from './UiNumberInput.vue'
export { default as UiSelect } from './UiSelect.vue'
export { default as UiField } from './UiField.vue'
export { default as UiToolbar } from './UiToolbar.vue'
export { default as UiEmptyState } from './UiEmptyState.vue'
export { default as UiSpinner } from './UiSpinner.vue'
export { default as UiTabs } from './UiTabs.vue'
export { default as UiKeyValue } from './UiKeyValue.vue'
export { default as UiStateBadge } from './UiStateBadge.vue'

// The same components, by name, because a box names one with a string.
//
// A `swarm.blackbox.Box{kind: Ui}` carries its component in `ref_id` — written by an agent, read
// as text — and a string only becomes a component through a lookup. Kept here rather than beside
// the caller because this file is the contract: a component that exists is one this map has, and
// `src/components/ui/index.test.ts` fails if the two lists ever differ.
import UiButton from './UiButton.vue'
import UiIconButton from './UiIconButton.vue'
import UiTile from './UiTile.vue'
import UiCard from './UiCard.vue'
import UiPanel from './UiPanel.vue'
import UiBadge from './UiBadge.vue'
import UiTag from './UiTag.vue'
import UiTable from './UiTable.vue'
import UiFileList from './UiFileList.vue'
import UiModal from './UiModal.vue'
import UiTextInput from './UiTextInput.vue'
import UiTextArea from './UiTextArea.vue'
import UiNumberInput from './UiNumberInput.vue'
import UiSelect from './UiSelect.vue'
import UiField from './UiField.vue'
import UiToolbar from './UiToolbar.vue'
import UiEmptyState from './UiEmptyState.vue'
import UiSpinner from './UiSpinner.vue'
import UiTabs from './UiTabs.vue'
import UiKeyValue from './UiKeyValue.vue'
import UiStateBadge from './UiStateBadge.vue'

export const uiComponents = {
  UiButton,
  UiIconButton,
  UiTile,
  UiCard,
  UiPanel,
  UiBadge,
  UiTag,
  UiTable,
  UiFileList,
  UiModal,
  UiTextInput,
  UiTextArea,
  UiNumberInput,
  UiSelect,
  UiField,
  UiToolbar,
  UiEmptyState,
  UiSpinner,
  UiTabs,
  UiKeyValue,
  UiStateBadge,
}

/** The names a box's `ref_id` may legally take. Anything else names nothing and draws nothing. */
export const uiComponentNames: ReadonlySet<string> = new Set(Object.keys(uiComponents))

// What each prop MEANS, from the table above.
//
// A box carries its props as `Map<String, String>` — ess/1 has no other map — so a reader has to
// be told whether `404` is a number or the three characters an author typed. The table above is
// where that is already written down, and this is the same table as data. `index.test.ts` parses
// the comment and fails if the two disagree, so the sentence a person reads and the rule a panel
// obeys cannot come apart.
//
// A bare name in the table (`label`, `placeholder`, `min`) declares no type and is absent here:
// the contract does not say, so nothing here pretends it does.
export const uiPropTypes = {
  UiButton: { variant: 'string', size: 'string', disabled: 'boolean', loading: 'boolean', icon: 'string' },
  UiIconButton: { icon: 'string', label: 'string' },
  UiTile: { title: 'string', subtitle: 'string', accent: 'string', big: 'boolean', clickable: 'boolean' },
  UiCard: { title: 'string' },
  UiPanel: { title: 'string', collapsible: 'boolean', open: 'boolean' },
  UiBadge: { tone: 'string', text: 'string' },
  UiTag: { text: 'string', removable: 'boolean' },
  UiTable: { columns: 'json', rows: 'json', rowKey: 'string', dense: 'boolean' },
  UiFileList: { files: 'json', selected: 'string' },
  UiModal: { open: 'boolean', title: 'string', width: 'string' },
  UiTextInput: { modelValue: 'string', mono: 'boolean' },
  UiTextArea: { modelValue: 'string', rows: 'number' },
  UiNumberInput: { modelValue: 'number' },
  UiSelect: { modelValue: 'string', options: 'json' },
  UiField: { label: 'string', hint: 'string', error: 'string' },
  UiToolbar: {},
  UiEmptyState: { title: 'string', text: 'string', icon: 'string' },
  UiSpinner: { size: 'string' },
  UiTabs: { modelValue: 'string', tabs: 'json' },
  UiKeyValue: { items: 'json', mono: 'boolean' },
  UiStateBadge: { state: 'string' },
} as const

// The components a panel renders INERT, and the components it refuses outright.
//
// Both start from the table's last column, and then the one predicate that matters for a canvas
// splits them: does the component draw inside its own box and act only on it?
//
// `inert` is an attribute on a SUBTREE. It makes the markup under a panel's wrapper take no input,
// which is exactly right for a text input whose edits nothing could keep — nothing writes
// `Box.props` back. It reaches nothing a component puts OUTSIDE that subtree: `UiModal` teleports
// to `document.body`, sets `document.body.style.overflow`, and installs a document-level keydown
// handler that swallows Escape and Tab. A box naming it would cover the application with a fixed
// backdrop, with no `@close` bound and no way back — from a value an agent may put in `props`.
//
// So a component that acts outside itself is REFUSED, not inerted, and `index.test.ts` decides
// which those are by reading every component in this directory rather than by trusting this list.
export const uiEmitters: ReadonlySet<string> = new Set([
  'UiButton',
  'UiIconButton',
  'UiTile',
  'UiTag',
  'UiFileList',
  'UiTextInput',
  'UiTextArea',
  'UiNumberInput',
  'UiSelect',
  'UiTabs',
  'UiTable',
])

/** Components no panel may draw: `inert` cannot contain them, so nothing can. */
export const uiRefused: ReadonlySet<string> = new Set(['UiModal'])
