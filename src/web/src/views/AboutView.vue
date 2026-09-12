<script setup lang="ts">
// How everything is executed, written down where a person will read it.
//
// Half of this page is prose and half is read from the running system, on purpose. A page that
// only asserted would drift the way the website's About page drifted — it still says ess/1 and
// describes a Flow entity deleted on 2026-09-11. Every count, cadence and program name below comes
// from `/spec` or `/status`, so a number here is wrong only if the runtime is wrong.
//
// Written like ComponentsView: sticky sidebar, one section per heading, tokens-only CSS, no
// markdown dependency — the app has none installed and this page does not add one.
import { computed, onMounted, ref } from 'vue'
import * as runtime from '@/runtime'
import type { Shape } from '@/runtime'
import { useSwarmStore } from '@/stores/swarms'
import { UiBadge, UiKeyValue, UiTable } from '@/components/ui'
import '@/components/ui/ui.css'

const store = useSwarmStore()
const shape = ref<Shape | undefined>()

onMounted(async () => {
  store.wake()
  try {
    shape.value = store.shape ?? (await runtime.shape())
  } catch {
    // The runtime is not up. Every live figure below says so rather than showing a stale number.
  }
})

const status = computed(() => store.status)

/** A figure read from the runtime, or a dash when it could not be read. */
function live(value: unknown): string {
  return value === undefined || value === null ? '—' : String(value)
}

const sections = [
  { id: 'turn', label: 'A turn, end to end' },
  { id: 'layers', label: 'Three layers' },
  { id: 'spec', label: 'The spec is the system' },
  { id: 'loop', label: 'The loop' },
  { id: 'tools', label: 'Tools' },
  { id: 'mail', label: 'Messages and fan-out' },
]

/** The layers, and what each one is responsible for. */
const layers = [
  {
    layer: 'substrate',
    what: 'The execution data plane. Confined workspaces, bounded processes, cgroups, namespaces.',
    swarm: 'not reached',
  },
  {
    layer: 'harness',
    what: 'The b10x agent loop. Owns turns, tools, budgets and approvals; its published toolset is its policy.',
    swarm: 'not used',
  },
  {
    layer: 'metaharness',
    what: 'Drives a vendor binary hermetically and emits one event stream whatever is inside.',
    swarm: 'this is what runs a turn',
  },
  {
    layer: 'Claude Code',
    what: "The vendor harness. Its own tools, its own loop, spawned into a scratch config home.",
    swarm: 'the coordinator',
  },
]

const specRows = computed(() => [
  { thing: 'entities', count: live(shape.value?.entities.length), note: 'each with a lifecycle the runtime enforces' },
  { thing: 'commands', count: live(shape.value?.commands.length), note: 'every one reachable over HTTP' },
  { thing: 'views', count: live(shape.value?.views.length), note: 'filter, sort, project — computed server-side' },
  { thing: 'spec files', count: live(status.value?.spec_files), note: 'compiled at boot, never read from cache' },
])
</script>

<template>
  <section class="about">
    <nav class="side">
      <a v-for="entry in sections" :key="entry.id" :href="`#${entry.id}`">{{ entry.label }}</a>
      <div class="side-live">
        <UiBadge v-if="store.reachable" tone="ok" text="reading the live runtime" />
        <UiBadge v-else tone="warn" text="runtime unreachable" />
      </div>
    </nav>

    <div class="main">
      <h1>How a swarm is executed</h1>
      <p class="lead">
        A swarm is a specification, an interpreter that runs it, and an agent that does the work.
        This page says which program does what, and what is actually true today rather than what
        the architecture allows.
      </p>

      <!-- 1 -->
      <section id="turn" class="block">
        <h2>A turn, end to end</h2>
        <pre class="diagram">
  you ──▶ swarm-server ──▶ metaharness ──▶ claude ──▶ the work
            interpreter      hermetic        the run     a directory
            + event log      launch                      on this machine
        </pre>
        <p>
          Clicking start issues a command. The interpreter applies it against the specification,
          appends the events to the swarm's log, and runs whatever bindings those events set off.
          Every thirty seconds a timer the specification declares picks up a goal that is waiting
          and moves it to <code>Pursuing</code>. That is the runtime's cue to run a turn.
        </p>
        <p>
          A turn is one <code>metaharness run claude</code> process. It is handed a prompt naming
          the goal, the turn number and a work directory it keeps between turns. Its event stream
          comes back line by line, is written to
          <code>data/swarms/&lt;slug&gt;/turns/</code> and is forwarded to every browser watching.
          The turn ends when the model writes a verdict line:
        </p>
        <pre class="code">VERDICT {"reached": false, "note": "still two tests short"}</pre>
        <p>
          The runtime never writes that line itself. A timeout, a crash or an exhausted budget
          leaves the goal in <code>Pursuing</code>, where a person can see it waiting — a runtime
          that answered on the coordinator's behalf would be reporting work nobody did.
        </p>
        <UiKeyValue
          mono
          :items="[
            { key: 'coordinator', value: live(status?.coordinator.program) },
            { key: 'turns running now', value: live(status?.coordinator.in_flight) },
            { key: 'ticks since start', value: live(status?.ticks) },
          ]"
        />
      </section>

      <!-- 2 -->
      <section id="layers" class="block">
        <h2>Three layers, and which one you are on</h2>
        <p>
          Four programs could be in the stack. Today two of them are.
        </p>
        <UiTable
          :columns="[
            { key: 'layer', label: 'program', width: '9rem' },
            { key: 'what', label: 'what it is' },
            { key: 'swarm', label: 'in a swarm', width: '11rem' },
          ]"
          :rows="layers"
          dense
        />
        <p>
          <strong>substrate</strong> is the bottom: it turns one machine into a governed service for
          confined workspaces and bounded processes, and it reports what it observed. Where the
          machine cannot confine, it refuses with <code>exec.sandbox-unavailable</code> rather than
          running something unconfined.
        </p>
        <p>
          <strong>harness</strong> is the b10x agent loop, and the only proven substrate consumer. It
          talks to model APIs directly and owns the cycle, so tool names, budgets and approvals are
          ours. Its toolset is computed from what the machine can confine — four tools with no
          backend, six with a confined workspace, seven inside a delegated cgroup — which is why its
          published toolset <em>is</em> its safety boundary.
        </p>
        <p>
          <strong>metaharness</strong> sits above and outside both. It spawns a vendor binary into a
          scratch config home with a seven-key environment allowlist, and turns whatever that binary
          emits into one event format. It spawns the b10x loop too, but only observes it: adjudicating
          a loop whose toolset already is its policy would measure the instrument instead of the run.
        </p>
        <p class="callout">
          <strong>What confines a swarm coordinator today: nothing.</strong> It is a Claude Code run
          under metaharness, which gives hermeticity and a complete event record, not confinement.
          metaharness implements no sandbox — the design that would have given it one was rejected on
          2026-09-01 — and its <code>--substrate</code> flag is refused by name for the Claude arm,
          because a socket configured there would be accepted, never consulted, and read as
          containment nobody applied. The coordinator runs on this machine with Claude Code's own
          tools, inside its work directory by convention and not by force.
        </p>
      </section>

      <!-- 3 -->
      <section id="spec" class="block">
        <h2>The spec is the system</h2>
        <p>
          <code>src/core</code> is an Executable System Specification. It is not documentation that
          describes the runtime; it is the thing the runtime reads at boot and executes. Add a
          command to a domain file, restart, and it is reachable over HTTP and visible in this app
          without a line of TypeScript or Rust being touched.
        </p>
        <UiTable
          :columns="[
            { key: 'thing', label: 'declared', width: '9rem' },
            { key: 'count', label: 'count', width: '6rem', align: 'right' },
            { key: 'note', label: '' },
          ]"
          :rows="specRows"
          dense
        />
        <p>
          The interpreter is about four thousand lines and owns no rules of its own. Guards are
          evaluated by the compiler's own three-valued predicate evaluator; which outcome a command
          takes, what it writes and what it emits are all read from the compiled model.
        </p>
        <p>
          Two rules this system adds on top, checked by
          <code>bin/check-sets-are-emitted.py</code>: every field an outcome writes must appear on an
          event that outcome emits, and every event must carry its subject's identity. Together they
          are what make state a fold over the log rather than a cache with a log beside it. Neither
          is retrofittable — once a log exists, its events are already missing whatever they were
          allowed to omit.
        </p>
        <p>
          A consequence worth knowing: <strong>the model does not read a clock.</strong> The
          specification decides what an outcome writes but not what time it is, so timestamps are
          handed in as command inputs. The runtime reads the clock at the edge, once.
        </p>
      </section>

      <!-- 4 -->
      <section id="loop" class="block">
        <h2>The loop</h2>
        <p>
          A swarm is born with three nodes and nothing else:
        </p>
        <pre class="diagram">  [loop] ──▶ [coordinator] ──▶ [goal] ──┐
     ▲                                  │
     └──────────────────────────────────┘
                 not reached yet</pre>
        <p>
          The loop is not a workflow engine. It is a lifecycle whose back edge is the loop: a goal
          moves <code>Open → Pursuing</code> on a tick, <code>Pursuing → Evaluating</code> on a
          verdict of "not yet", and <code>Evaluating → Pursuing</code> on the next tick. The exit is
          a guarded transition, <code>Pursuing → Reached</code>, taken when the coordinator's verdict
          says so.
        </p>
        <p>
          The tick is declared in the specification, not built in the runtime, so its cadence and its
          overlap rule are reviewable beside everything else the system promises.
          <code>serial_per_instance</code> is the one worth reading: one turn of one goal at a time,
          which is what stops a slow coordinator being asked to pursue the same goal twice.
        </p>
        <UiKeyValue
          mono
          :items="[
            { key: 'binding', value: live(status?.periodic[0]?.binding) },
            { key: 'every', value: status?.periodic[0] ? `${status.periodic[0].every_s}s` : '—' },
            { key: 'next tick', value: store.nextTickIn !== undefined ? `in ${store.nextTickIn}s` : '—' },
          ]"
        />
        <p>
          The runtime supplies only what the timer contract leaves to the host: which goal an
          occurrence is for, read from a view, and whether the swarm is running at all. The cadence
          is not its business.
        </p>
      </section>

      <!-- 5 -->
      <section id="tools" class="block">
        <h2>Tools: how a command gets issued</h2>
        <p>
          The coordinator runs with the vendor's own tool surface and every call recorded:
        </p>
        <pre class="code">metaharness run claude --hermetic
  --tool-surface native --decisions observe
  --max-turns 15 --max-budget-usd 1.00
  --cwd data/swarms/&lt;slug&gt;/work -p "…"</pre>
        <p>
          <code>observe</code> means every tool call is allowed and recorded — that is what fills the
          Coordinator tab. The alternative, <code>--tool-surface owned</code>, has metaharness serve
          the tools itself and <code>--allow-program</code> name the only program a run may start;
          it needs a frame document and refuses <code>observe</code> by construction.
        </p>
        <p>
          Two facts about the hermetic launch decide how a swarm hands its agent anything:
        </p>
        <UiKeyValue
          mono
          :items="[
            { key: 'child PATH', value: '$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin' },
            { key: 'inherited env', value: 'HOME USER LOGNAME LANG LC_ALL TERM TZ — and nothing else' },
          ]"
        />
        <p>
          So a program the agent should be able to call goes in <code>~/.local/bin</code>, not
          <code>~/.cargo/bin</code>, which is not on that PATH. And nothing can be passed to it in an
          environment variable, because the allowlist is seven keys and no variable of ours is among
          them — configuration has to arrive in a file inside the work directory.
        </p>
      </section>

      <!-- 6 -->
      <section id="mail" class="block">
        <h2>Messages and fan-out</h2>
        <p>
          Data moves between parts of the system in exactly one way: an event is emitted, and a
          <em>binding</em> declared in <code>components.yaml</code> carries it to another command.
          Nothing routes an event except a binding, and the runtime reads them rather than inventing
          them.
        </p>
        <p>There are two kinds of fan-out, and they are not the same mechanism.</p>
        <UiTable
          :columns="[
            { key: 'kind', label: 'fan-out', width: '9rem' },
            { key: 'means', label: 'what it means' },
            { key: 'how', label: 'how it works' },
          ]"
          :rows="[
            {
              kind: 'static',
              means: 'one event, several different commands',
              how: 'declare several bindings naming the same event. The pump invokes every match, in order, each against the world the last one left.',
            },
            {
              kind: 'dynamic',
              means: 'one action, the same command against many instances',
              how: 'not expressible as a binding — a binding invokes one command against one instance. The host reads a view and issues the command once per row, which is exactly how the tick picks its goals.',
            },
          ]"
          dense
        />
        <p>
          Bindings in force right now:
          <code v-for="entry in status?.periodic ?? []" :key="entry.binding">{{ entry.binding }}</code>
          <span v-if="!status?.periodic.length">—</span>
        </p>
      </section>
    </div>
  </section>
</template>

<style scoped>
.about {
  display: grid;
  grid-template-columns: 15rem minmax(0, 1fr);
  gap: var(--space-6);
  padding: var(--space-5);
  align-items: start;
  max-width: 68rem;
}

.side {
  position: sticky;
  top: var(--space-4);
  display: grid;
  gap: var(--space-1);
  align-content: start;
}

.side a {
  color: var(--color-text-muted);
  text-decoration: none;
  font-size: 13px;
  padding: var(--space-1) var(--space-2);
  border-left: 2px solid var(--color-border);
}

.side a:hover {
  color: var(--color-text);
  border-left-color: var(--color-accent);
}

.side-live {
  margin-top: var(--space-3);
}

.main {
  min-width: 0;
  display: grid;
  gap: var(--space-6);
}

h1 {
  margin: 0;
  font-size: 26px;
}

.lead {
  margin: calc(-1 * var(--space-5)) 0 0;
  color: var(--color-text-muted);
  max-width: 46rem;
}

.block {
  display: grid;
  gap: var(--space-3);
}

h2 {
  margin: 0;
  font-size: 17px;
  padding-bottom: var(--space-2);
  border-bottom: 1px solid var(--color-border);
}

p {
  margin: 0;
  max-width: 46rem;
  line-height: 1.6;
}

code {
  font-family: var(--font-mono);
  font-size: 0.9em;
  background: var(--color-surface-2);
  padding: 1px var(--space-1);
  border-radius: var(--radius-1);
}

p code + code {
  margin-left: var(--space-1);
}

.diagram,
.code {
  margin: 0;
  padding: var(--space-3);
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-2);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.5;
  overflow-x: auto;
  color: var(--color-text-muted);
}

.diagram {
  color: var(--color-accent);
}

.callout {
  padding: var(--space-3);
  border-left: 2px solid var(--color-warn);
  background: color-mix(in srgb, var(--color-warn) 8%, transparent);
  border-radius: 0 var(--radius-2) var(--radius-2) 0;
  max-width: none;
}
</style>
