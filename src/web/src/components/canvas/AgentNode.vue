<script setup lang="ts">
// Compact Vue Flow node for an Agent: slug, role, lifecycle state, unread count. One target handle
// (`inbox`) and one source handle (`events`). Rendered via the `#node-agent` slot of <VueFlow>.
import { computed } from 'vue'
import { Handle, Position } from '@vue-flow/core'
import type { Agent } from '@/model'
import { UiStateBadge } from '@/components/ui'

export interface AgentNodeData { agent: Agent }

defineOptions({ inheritAttrs: false })
const props = defineProps<{ id: string; data: AgentNodeData; selected?: boolean }>()
const agent = computed(() => props.data.agent)
</script>

<template>
  <div class="agent-node" :class="{ selected: props.selected }" :title="agent.assignment ?? agent.role">
    <Handle id="inbox" type="target" :position="Position.Left" title="inbox" />
    <div class="body">
      <div class="row">
        <span class="slug">{{ agent.agentId }}</span>
        <span v-if="agent.unread" class="unread" :title="agent.unread + ' unread'">{{ agent.unread }}</span>
      </div>
      <div class="row">
        <span class="role">{{ agent.role }}</span>
        <UiStateBadge :state="agent.state" />
      </div>
    </div>
    <Handle id="events" type="source" :position="Position.Right" title="events" />
  </div>
</template>

<style scoped>
.agent-node {
  width: 220px;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-left: 3px solid var(--color-accent-2);
  border-radius: var(--radius-2);
  box-shadow: var(--shadow-1);
  color: var(--color-text);
  font-size: 12px;
}
.agent-node.selected { border-color: var(--color-accent); box-shadow: 0 0 0 1px var(--color-accent), var(--shadow-2); }
.body { padding: var(--space-2) var(--space-3); display: flex; flex-direction: column; gap: var(--space-1); }
.row { display: flex; align-items: center; justify-content: space-between; gap: var(--space-2); min-width: 0; }
.slug { font-weight: 600; font-family: var(--font-mono); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.role { color: var(--color-text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.unread {
  min-width: 18px; height: 18px; padding: 0 6px; border-radius: 9px;
  background: var(--color-accent); color: var(--color-bg);
  font-size: 11px; font-weight: 700; line-height: 18px; text-align: center;
}
</style>
