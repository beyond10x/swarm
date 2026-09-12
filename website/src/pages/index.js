import Layout from '@theme/Layout';
import HeroBanner from '@site/src/components/HeroBanner';
import facts from '@site/src/data/spec-facts.json';

// Every number on this page comes from `facts`, which website/scripts/spec-facts.mjs derives from
// the tree at build time. None are typed here, and that is deliberate: the page this replaced said
// "5 domains, 8 entities, 48 commands, 22 views" and was wrong about all four for its entire
// published life, because a mailbox domain landed minutes after somebody typed them. It also said
// the interpreter was "about four thousand lines", which matched no count in the tree.

const {counts, domains, lines, surfaces} = facts;

const n = (value) => value.toLocaleString('en-US');

function Stat({value, label, note}) {
  return (
    <div className="stat">
      <div className="stat__value">{value}</div>
      <div className="stat__label">{label}</div>
      {note ? <div className="stat__note">{note}</div> : null}
    </div>
  );
}

export default function Home() {
  return (
    <Layout
      title="About"
      description="Swarm — a swarm manager whose kernel is an Executable System Specification, compiled at boot and interpreted rather than implemented.">
      <HeroBanner
        title="Swarm"
        tagline="The kernel is a specification. The runtime compiles it at boot and interprets it — so nothing is hand-coded per feature."
      />

      <main className="container about-section">
        <section>
          <p className="lede">
            Swarm is a <strong>swarm manager</strong>. A swarm is born bare —
            one coordinator, one goal, and a loop that turns — and builds
            whatever else it needs as it goes. What makes it worth looking at is
            not the agent. It is where the rules live.
          </p>
          <p>
            The kernel is written as an{' '}
            <strong>Executable System Specification</strong> ({facts.specFormat}):{' '}
            {facts.specFiles} files of YAML that are not documentation about a
            runtime, but the thing the runtime reads. <code>swarm-server</code> compiles{' '}
            <code>src/core/</code> at boot and refuses to start if it does not
            resolve. <strong>Nothing is hand-coded per feature.</strong> A
            command added to a domain file is reachable over HTTP, in the CLI
            and in the UI without a line of code being written.
          </p>
          <div className="callout callout--note">
            <p>
              <strong>It is interpreted, not generated.</strong> There is no
              codegen step, no generated file in the tree and no build that
              emits one. <code>ess-runtime</code> is an interpreter: it holds
              the compiled specification in memory and executes against it.
              That is the whole trick, and it is a different claim from
              scaffolding — there is no generated code to drift, because there
              is no generated code.
            </p>
          </div>
        </section>

        <section>
          <h2>What is actually in the specification</h2>
          <div className="stat-grid">
            <Stat value={counts.domains} label="domains" />
            <Stat value={counts.entities} label="entities" />
            <Stat value={counts.commands} label="commands" />
            <Stat value={counts.events} label="events" />
            <Stat value={counts.views} label="views" />
            <Stat value={counts.types} label="types" />
            <Stat value={counts.errors} label="errors" />
            <Stat value={counts.actors} label="actors" />
            <Stat value={counts.bindings} label="bindings" />
          </div>
          <p className="stat-grid__source">
            Derived at build time from{' '}
            <code>ess specify compile --path src/core --format json</code> —{' '}
            {facts.essVersion}, {facts.specFiles} files,{' '}
            {facts.validates ? 'valid' : 'INVALID'}. No number on this page is
            typed by hand.
          </p>

          <table className="domain-table">
            <thead>
              <tr>
                <th>domain</th>
                <th>entities</th>
                <th className="num">cmds</th>
                <th className="num">events</th>
                <th className="num">views</th>
                <th>what it is</th>
              </tr>
            </thead>
            <tbody>
              {domains.map((d) => (
                <tr key={d.name}>
                  <td>
                    <code>{d.name}</code>
                  </td>
                  <td>
                    {d.entities.map((e) => (
                      <code key={e} className="chip">
                        {e}
                      </code>
                    ))}
                  </td>
                  <td className="num">{d.commands}</td>
                  <td className="num">{d.events}</td>
                  <td className="num">{d.views}</td>
                  <td className="domain-table__summary">{d.summary}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>

        <section>
          <h2>What the runtime does, and how little of it there is</h2>
          <p>
            Three Rust crates, {n(lines.runtimeTotal)} hand-written lines
            between them, against {n(lines.specYaml)} lines of specification
            YAML. The interpreter is the smallest of the three and owns no rules
            of its own.
          </p>
          <table>
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
                  The interpreter. Compiles the specification, folds events into
                  entities, computes views, routes bindings. Knows nothing about
                  swarms.
                </td>
              </tr>
              <tr>
                <td>
                  <code>swarm-server</code>
                </td>
                <td className="num">{n(lines.swarmServer)}</td>
                <td>
                  {surfaces.httpRoutes} HTTP routes, an event stream per swarm,
                  and the trigger that fires the periodic bindings.
                </td>
              </tr>
              <tr>
                <td>
                  <code>swarm-cli</code>
                </td>
                <td className="num">{n(lines.swarmCli)}</td>
                <td>
                  {surfaces.cliVerbs} verbs, the ones a coordinator uses from
                  inside a turn — mail, views, and <code>do</code>, which issues
                  any command the specification declares.
                </td>
              </tr>
            </tbody>
          </table>
          <p>
            Beside them: {n(lines.integrationTests)} lines of integration tests
            across {lines.integrationTestFiles} files, and{' '}
            {n(lines.web)} lines of Vue for the canvas, which reads{' '}
            <code>/spec</code> at load rather than hard-coding the entities it
            draws. Add an entity and the UI knows about it.
          </p>
        </section>

        <section>
          <h2>The loop</h2>
          <p>
            A swarm starts with three nodes and nothing else:{' '}
            <code>[loop] → [coordinator] → [goal]</code>.
          </p>
          <p>
            The loop is not a workflow engine. It is a lifecycle whose back edge
            is the loop. A goal moves <code>Open → Pursuing</code> on a tick,{' '}
            <code>Pursuing → Evaluating</code> when the coordinator reports "not
            yet", and <code>Evaluating → Pursuing</code> on the next tick. The
            exit is a guarded transition, <code>Pursuing → Reached</code>, taken
            only when the coordinator says the goal is met.
          </p>
          <p>
            The tick is declared in the specification as a periodic binding —{' '}
            <code>turn-the-loop</code>, every 30 seconds — so its cadence and
            its overlap rule are reviewable beside everything else the system
            promises. <code>serial_per_instance</code> is the one worth reading:
            one turn of one goal at a time, which is what stops a slow
            coordinator from being asked to pursue the same goal twice. A tick
            missed while a turn was running is taken once when it finishes, not
            banked and replayed.
          </p>
          <p>
            A turn is one <code>metaharness run claude</code> process, handed a
            prompt naming the goal, the turn number, and a work directory it
            keeps between turns. Its event stream comes back line by line, is
            written to disk and forwarded to every browser watching, so tokens,
            cache reads, cost and every tool call are visible while the turn is
            still running. The turn ends when the model writes a verdict line.
          </p>
          <p>
            The runtime never writes that line itself. A timeout, a crash or an
            exhausted budget leaves the goal in <code>Pursuing</code>, where a
            person can see it waiting — a runtime that answered on the
            coordinator's behalf would be reporting work nobody did.
          </p>
        </section>

        <section>
          <h2>What is true today, and what is not yet</h2>
          <p>
            The honest reading matters more here than the pitch, so it is
            written out rather than implied.
          </p>
          <table className="verdict-table">
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
                  <span className="tag tag--yes">true</span>
                </td>
                <td>
                  The server compiles <code>src/core/</code> at boot and refuses
                  to start if it does not resolve. The UI reads{' '}
                  <code>/spec</code> rather than hard-coding entities.
                </td>
              </tr>
              <tr>
                <td>It is operational</td>
                <td>
                  <span className="tag tag--yes">true</span>
                </td>
                <td>
                  Server, CLI and UI run. {lines.integrationTestFiles}{' '}
                  integration tests, {n(lines.integrationTests)} lines. Real
                  recorded coordinator turns on disk under{' '}
                  <code>data/swarms/</code>.
                </td>
              </tr>
              <tr>
                <td>The kernel is fully specified</td>
                <td>
                  <span className="tag tag--partly">with a caveat</span>
                </td>
                <td>
                  True of the kernel, not of the host. The HTTP route table, the
                  coordinator process protocol, retry bounds, the pump depth
                  cap, budget caps, the UI component library and every clock
                  read sit outside the specification — by design, and each is
                  named where it lives.
                </td>
              </tr>
              <tr>
                <td>A working agent swarm</td>
                <td>
                  <span className="tag tag--no">overstated</span>
                </td>
                <td>
                  What works is a swarm <em>manager</em>: one coordinator per
                  swarm, pursuing one goal in a loop. Multiple cooperating
                  agents are <em>specified</em> —{' '}
                  <code>swarm.agent.Spawn</code>, <code>Assign</code>, the whole
                  mailbox domain — and not demonstrated. Spawning a second agent
                  is still an open story.
                </td>
              </tr>
              <tr>
                <td>Anything confines a coordinator</td>
                <td>
                  <span className="tag tag--no">no</span>
                </td>
                <td>
                  A coordinator is a Claude Code run under{' '}
                  <code>metaharness</code>, which gives hermeticity and a
                  complete event record, not containment. It runs on the host
                  machine with the vendor's own tools, inside its work directory
                  by convention rather than by force.
                </td>
              </tr>
            </tbody>
          </table>
        </section>

        <section>
          <h2>What is in the kernel, and why nothing else is</h2>
          <p>
            The kernel is only what a swarm cannot build for itself: the swarm
            record and its lifecycle, how an agent is launched, the agent
            itself, the goal, and what anything may reach.
          </p>
          <p>
            Everything else is a <strong>Box</strong> the swarm draws and a{' '}
            <strong>Connection</strong> it wires. A tool is a box. An MCP server
            is a box. A UI panel is a box. Two boxes connect only when their
            ports cite the same published schema, so what a coordinator may
            reach is the set of connections drawn to it: granting a tool is
            drawing an edge, revoking it is removing one.
          </p>
          <p>
            An earlier draft was reverse-engineered from a live sixteen-agent
            run and carried eleven domains — a board, a scheduler, a work
            ledger, an incident log, a decisions register, a DAG flow engine.
            Each described something that run had built for itself, and a swarm
            that starts with them has not started bare. They were removed.
          </p>
        </section>

        <section>
          <h2>Specified, checked</h2>
          <p>
            The specification validates and compiles with <code>ess</code>, and
            two further rules are checked on every change: every field an
            outcome writes must appear on an event that outcome emits, and every
            event must carry its subject's identity. Together they are what make
            state a fold over the log rather than a cache with a log beside it.
            Neither is retrofittable — once a log exists, its events are already
            missing whatever they were allowed to omit.
          </p>
          <p>
            A consequence worth knowing: the model does not read a clock. The
            specification decides what an outcome writes but not what time it
            is, so timestamps are handed in as command inputs and the runtime
            reads the clock once, at the edge.
          </p>
        </section>

        <section>
          <h2>Look at it</h2>
          <p>
            The source, the specification and the instructions for running it
            are at{' '}
            <a href="https://github.com/beyond10x/swarm">
              github.com/beyond10x/swarm
            </a>
            . Start with <code>src/core/</code> — it is {facts.specFiles}{' '}
            files of YAML, and it is the system.
          </p>
        </section>
      </main>
    </Layout>
  );
}
