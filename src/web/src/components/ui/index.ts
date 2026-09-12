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
// UiTable        columns*: {key,label,width?,align?}[]; rows*: Record<string,unknown>[]; cell-<key>, empty
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
