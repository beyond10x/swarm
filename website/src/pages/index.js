import Layout from '@theme/Layout';
import {
  Callout,
  CardGrid,
  CodeExample,
  CommandExample,
  ContentCard,
  Diagram,
  FactGrid,
  ScrollableTable,
  SectionHeader,
  StatusBadge,
} from '@beyond10x/docs-system/components';
import HeroBanner from '@site/src/components/HeroBanner';
import facts from '@site/src/data/spec-facts.json';

// Every number on this page comes from `facts`, which website/scripts/spec-facts.mjs derives from
// the tree at build time. None are typed here, and that is deliberate: the page this replaced said
// "5 domains, 8 entities, 48 commands, 22 views" and was wrong about all four for its entire
// published life, because a mailbox domain landed minutes after somebody typed them. It also said
// the interpreter was "about four thousand lines", which matched no count in the tree.

const {counts, domains, lines, surfaces} = facts;

const n = (value) => value.toLocaleString('en-US');

const COUNT_ORDER = [
  ['domains', 'domains'],
  ['entities', 'entities'],
  ['commands', 'commands'],
  ['events', 'events'],
  ['views', 'views'],
  ['types', 'types'],
  ['errors', 'errors'],
  ['actors', 'actors'],
  ['bindings', 'bindings'],
];

// Verbatim from src/core/components.yaml. The tick is not a runtime constant hidden in Rust; it is
// declared beside everything else the system promises, which is the point of quoting it.
const TICK_YAML = `- id: turn-the-loop
  summary: A goal waiting for a turn gets one.
  when:
    periodic:
      every: PT30S
      anchor: host_activation
      first: after_period
      cadence: fixed_rate
      overlap: serial_per_instance
      missed: one_pending_drop_excess`;

const GOAL_LIFECYCLE = `stateDiagram-v2
    [*] --> Open
    Open --> Pursuing: a tick
    Pursuing --> Evaluating: the coordinator reports "not yet"
    Evaluating --> Pursuing: the next tick
    Pursuing --> Reached: the coordinator reports the goal met
    Reached --> [*]`;

export default function Home() {
  return (
    <Layout
      title="About"
      description="Swarm — a swarm manager whose kernel is an Executable System Specification, compiled at boot and interpreted rather than implemented.">
      <main className="container swarm-page">
        <HeroBanner
          eyebrow="Executable System Specification"
          title="The kernel is a specification"
          tagline={
            <>
              Swarm is a <strong>swarm manager</strong>. A swarm is born bare — one coordinator, one
              goal, and a loop that turns — and builds whatever else it needs as it goes. What makes
              it worth looking at is not the agent. It is where the rules live: the runtime compiles
              the specification at boot and interprets it, so nothing is hand-coded per feature.
            </>
          }
          actions={
            <>
              <a href="https://github.com/beyond10x/swarm">Read the source</a>
              <a href="#specification">What is in the specification</a>
            </>
          }>
          <StatusBadge maturity="development">operational, pre-v1</StatusBadge>
          <StatusBadge maturity="stable">{facts.essVersion}</StatusBadge>
          <StatusBadge maturity="stable">{facts.specFiles} spec files</StatusBadge>
        </HeroBanner>

        <section className="swarm-section">
          <p className="swarm-lede">
            The kernel is written as an <strong>Executable System Specification</strong> (
            {facts.specFormat}): {facts.specFiles} files of YAML that are not documentation about a
            runtime, but the thing the runtime reads. <code>swarm-server</code> compiles{' '}
            <code>src/core/</code> at boot and refuses to start if it does not resolve. A command
            added to a domain file is reachable over HTTP, in the CLI and in the UI without a line
            of code being written.
          </p>
          <Callout title="Interpreted, not generated" tone="note">
            <p>
              There is no codegen step, no generated file in the tree and no build that emits one.{' '}
              <code>ess-runtime</code> is an interpreter: it holds the compiled specification in
              memory and executes against it. That is the whole trick, and it is a different claim
              from scaffolding — there is no generated code to drift, because there is no generated
              code.
            </p>
          </Callout>
        </section>

        <section className="swarm-section">
          <SectionHeader
            id="specification"
            eyebrow="Derived at build time"
            title="What is actually in the specification"
            description="No number below is typed by hand. Each is counted out of the compiled specification when the site is built, because the page this replaced published four counts that were wrong from the day they were written."
          />
          <FactGrid
            label="Counts in the compiled specification"
            items={COUNT_ORDER.map(([key, label]) => ({label, value: n(counts[key])}))}
          />
          <CommandExample
            title="Where the counts come from"
            command="ess specify compile --path src/core --format json"
          />
          <p className="swarm-note">
            {facts.essVersion}, {facts.specFiles} files,{' '}
            {facts.validates ? 'valid' : 'INVALID'}.
          </p>

          <CardGrid label="Domains in the kernel" columns={3}>
            {domains.map((domain) => (
              <ContentCard
                key={domain.name}
                title={domain.display}
                eyebrow={domain.name}
                meta={`${domain.commands} cmd · ${domain.events} ev · ${domain.views} views`}
                description={domain.summary}>
                <ul className="swarm-chips">
                  {domain.entities.map((entity) => (
                    <li key={entity}>
                      <code>{entity}</code>
                    </li>
                  ))}
                </ul>
              </ContentCard>
            ))}
          </CardGrid>
        </section>

        <section className="swarm-section">
          <SectionHeader
            id="runtime"
            eyebrow="Three crates"
            title="What the runtime does, and how little of it there is"
            description={`${n(lines.runtimeTotal)} hand-written lines between them, against ${n(
              lines.specYaml,
            )} lines of specification YAML. The interpreter is the smallest of the three and owns no rules of its own.`}
          />
          <ScrollableTable label="The runtime crates">
            <thead>
              <tr>
                <th>crate</th>
                <th className="num">lines</th>
                <th>what it does</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <code>ess-runtime</code>
                </td>
                <td className="num">{n(lines.essRuntime)}</td>
                <td>
                  The interpreter. Compiles the specification, folds events into entities, computes
                  views, routes bindings. Knows nothing about swarms.
                </td>
              </tr>
              <tr>
                <td>
                  <code>swarm-server</code>
                </td>
                <td className="num">{n(lines.swarmServer)}</td>
                <td>
                  {surfaces.httpRoutes} HTTP routes, an event stream per swarm, and the trigger that
                  fires the periodic bindings.
                </td>
              </tr>
              <tr>
                <td>
                  <code>swarm-cli</code>
                </td>
                <td className="num">{n(lines.swarmCli)}</td>
                <td>
                  {surfaces.cliVerbs} verbs, the ones a coordinator uses from inside a turn — mail,
                  views, and <code>do</code>, which issues any command the specification declares.
                </td>
              </tr>
            </tbody>
          </ScrollableTable>
          <p>
            Beside them: {n(lines.integrationTests)} lines of integration tests across{' '}
            {lines.integrationTestFiles} files, and {n(lines.web)} lines of Vue for the canvas, which
            reads <code>/spec</code> at load rather than hard-coding the entities it draws. Add an
            entity and the UI knows about it.
          </p>
        </section>

        <section className="swarm-section">
          <SectionHeader
            id="loop"
            eyebrow="The lifecycle whose back edge is the loop"
            title="The loop"
            description="A swarm starts with three nodes and nothing else: a loop, a coordinator and a goal."
          />
          <Diagram
            kind="mermaid"
            source={GOAL_LIFECYCLE}
            minWidth="32rem"
            title="A goal, from Open to Reached"
            description="A goal moves Open to Pursuing on a tick, Pursuing to Evaluating when the coordinator reports not yet, and Evaluating back to Pursuing on the next tick. The exit is a guarded transition, Pursuing to Reached, taken only when the coordinator says the goal is met."
            alternative={{
              content: (
                <p>
                  <code>Open</code> → <code>Pursuing</code> on a tick. <code>Pursuing</code> →{' '}
                  <code>Evaluating</code> when the coordinator reports &ldquo;not yet&rdquo;.{' '}
                  <code>Evaluating</code> → <code>Pursuing</code> on the next tick. The only exit is
                  the guarded transition <code>Pursuing</code> → <code>Reached</code>, taken when the
                  coordinator says the goal is met.
                </p>
              ),
            }}
          />
          <p>
            The loop is not a workflow engine. The tick is declared in the specification as a
            periodic binding, so its cadence and its overlap rule are reviewable beside everything
            else the system promises.
          </p>
          <CodeExample language="yaml" title="src/core/components.yaml — the tick, as declared">
            {TICK_YAML}
          </CodeExample>
          <p>
            <code>serial_per_instance</code> is the line worth reading: one turn of one goal at a
            time, which is what stops a slow coordinator from being asked to pursue the same goal
            twice. A tick missed while a turn was running is taken once when it finishes, not banked
            and replayed.
          </p>
          <p>
            A turn is one <code>metaharness run claude</code> process, handed a prompt naming the
            goal, the turn number, and a work directory it keeps between turns. Its event stream
            comes back line by line, is written to disk and forwarded to every browser watching, so
            tokens, cache reads, cost and every tool call are visible while the turn is still
            running. The turn ends when the model writes a verdict line.
          </p>
          <Callout title="The runtime never writes the verdict" tone="warning">
            <p>
              A timeout, a crash or an exhausted budget leaves the goal in <code>Pursuing</code>,
              where a person can see it waiting. A runtime that answered on the coordinator&rsquo;s
              behalf would be reporting work nobody did.
            </p>
          </Callout>
        </section>

        <section className="swarm-section">
          <SectionHeader
            id="status"
            eyebrow="The honest reading"
            title="What is true today, and what is not yet"
            description="Written out rather than implied, because the pitch matters less here than the reading."
          />
          <ScrollableTable label="Claims and their state">
            <thead>
              <tr>
                <th>claim</th>
                <th>state</th>
                <th>detail</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>The specification is the source of truth</td>
                <td>
                  <StatusBadge maturity="stable">true</StatusBadge>
                </td>
                <td>
                  The server compiles <code>src/core/</code> at boot and refuses to start if it does
                  not resolve. The UI reads <code>/spec</code> rather than hard-coding entities.
                </td>
              </tr>
              <tr>
                <td>It is operational</td>
                <td>
                  <StatusBadge maturity="stable">true</StatusBadge>
                </td>
                <td>
                  Server, CLI and UI run. {lines.integrationTestFiles} integration tests,{' '}
                  {n(lines.integrationTests)} lines. Real recorded coordinator turns on disk under{' '}
                  <code>data/swarms/</code>.
                </td>
              </tr>
              <tr>
                <td>The kernel is fully specified</td>
                <td>
                  <StatusBadge maturity="development">with a caveat</StatusBadge>
                </td>
                <td>
                  True of the kernel, not of the host. The HTTP route table, the coordinator process
                  protocol, retry bounds, the pump depth cap, budget caps, the UI component library
                  and every clock read sit outside the specification — by design, and each is named
                  where it lives.
                </td>
              </tr>
              <tr>
                <td>A working agent swarm</td>
                <td>
                  <StatusBadge maturity="development">depth one, breadth two</StatusBadge>
                </td>
                <td>
                  Demonstrated on 2026-09-13, and no further than it says. A coordinator was given a
                  goal it could not meet alone, spawned a second agent, posted it an assignment and
                  waited while the runtime ran that agent as its own session with its own work
                  directory and transcript. The evidence is exported into the repository and a
                  checker read the log to reach the verdict: seven clauses, seven met. <code>AssignmentTaken</code>,{' '}
                  <code>AssignmentDone</code> and <code>GoalReached</code> had each fired zero times
                  before that run. Still undemonstrated: agents that spawn agents, more than two at
                  once, and a swarm that extends its own specification.
                </td>
              </tr>
              <tr>
                <td>Anything confines a coordinator</td>
                <td>
                  <StatusBadge maturity="experimental">no — but a turn is narrowed</StatusBadge>
                </td>
                <td>
                  A coordinator is a Claude Code run under <code>metaharness</code>, which gives
                  hermeticity and a complete event record, not containment. It runs on the host
                  machine with the host&rsquo;s access: no namespace, no cgroup, no network
                  isolation, and the harness&rsquo;s own attestation says so. Since 2026-09-13 every
                  turn does launch under a sealed frame naming the operations it admits and the one
                  directory it may write, so a call outside that set is refused when the model
                  attempts it — measured, 1 of 41 decided calls. That is a refusal at a decision
                  seam, which is a weaker claim than confinement and is the reason this row still
                  says no.
                </td>
              </tr>
            </tbody>
          </ScrollableTable>
        </section>

        <section className="swarm-section">
          <SectionHeader
            id="kernel"
            eyebrow="Boxes and connections"
            title="What is in the kernel, and why nothing else is"
            description="The kernel is only what a swarm cannot build for itself: the swarm record and its lifecycle, how an agent is launched, the agent itself, the goal, and what anything may reach."
          />
          <CardGrid columns={2} label="Why the kernel is this small">
            <ContentCard title="Everything else is a box" headingLevel={3}>
              <p>
                A tool is a box. An MCP server is a box. A UI panel is a box. Two boxes connect only
                when their ports cite the same published schema, so what a coordinator may reach is
                the set of connections drawn to it: granting a tool is drawing an edge, revoking it
                is removing one.
              </p>
            </ContentCard>
            <ContentCard title="Eleven domains were removed" headingLevel={3}>
              <p>
                An earlier draft was reverse-engineered from a live sixteen-agent run and carried a
                board, a scheduler, a work ledger, an incident log, a decisions register and a DAG
                flow engine. Each described something that run had built for itself, and a swarm that
                starts with them has not started bare.
              </p>
            </ContentCard>
          </CardGrid>
        </section>

        <section className="swarm-section">
          <SectionHeader
            id="checked"
            eyebrow="Two rules that are not retrofittable"
            title="Specified, checked"
            description="The specification validates and compiles with ess, and two further rules are checked on every change."
          />
          <p>
            Every field an outcome writes must appear on an event that outcome emits, and every event
            must carry its subject&rsquo;s identity. Together they are what make state a fold over
            the log rather than a cache with a log beside it. Neither is retrofittable — once a log
            exists, its events are already missing whatever they were allowed to omit.
          </p>
          <Callout title="A consequence worth knowing" tone="note">
            <p>
              The model does not read a clock. The specification decides what an outcome writes but
              not what time it is, so timestamps are handed in as command inputs and the runtime
              reads the clock once, at the edge.
            </p>
          </Callout>
          <p>
            The source, the specification and the instructions for running it are at{' '}
            <a href="https://github.com/beyond10x/swarm">github.com/beyond10x/swarm</a>. Start with{' '}
            <code>src/core/</code> — it is {facts.specFiles} files of YAML, and it is the system.
          </p>
        </section>
      </main>
    </Layout>
  );
}
