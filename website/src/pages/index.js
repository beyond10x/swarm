import Layout from '@theme/Layout';
import HeroBanner from '@site/src/components/HeroBanner';

// Rewritten 2026-09-12. The previous text described the 2026-09-11 draft that was reverse-engineered
// from a live 16-agent run: ess/1, eleven domains, a `Flow` DAG and a six-level noun tower. All of
// that was deleted when the spec was stripped to a kernel, so the page was claiming things that had
// stopped being true. It also never said what actually executes a turn, which is the one question a
// reader arrives with.
export default function Home() {
  return (
    <Layout
      title="About"
      description="Swarm — an Executable System Specification of a swarm manager, and the runtime that executes it.">
      <HeroBanner
        title="Swarm"
        tagline="A swarm starts bare — one coordinator, one goal — and builds the rest."
      />
      <main className="container about-section">
        <section>
          <p>
            Swarm is a swarm manager. Each swarm is one coordinator pursuing
            one goal, with whatever boxes and connections it draws for itself
            along the way. It is written as an{' '}
            <strong>Executable System Specification</strong> (ess/4): not
            documentation that describes a runtime, but the thing the runtime
            reads at boot and executes. Add a command to a domain file, restart,
            and it is reachable — no application code is touched.
          </p>
          <p>
            Five domains, eight entities, forty-eight commands, twenty-two
            views. The interpreter that runs them is about four thousand lines
            and owns no rules of its own.
          </p>
        </section>

        <section>
          <h2>The shape a swarm is born with</h2>
          <p>
            Three nodes, and nothing else:{' '}
            <code>[loop] → [coordinator] → [goal]</code>.
          </p>
          <p>
            The loop is not a workflow engine. It is a lifecycle whose back
            edge is the loop: a goal moves <code>Open → Pursuing</code> on a
            tick, <code>Pursuing → Evaluating</code> when the coordinator
            reports "not yet", and <code>Evaluating → Pursuing</code> on the
            next tick. The exit is a guarded transition,{' '}
            <code>Pursuing → Reached</code>, taken only when the coordinator
            says the goal is met.
          </p>
          <p>
            The tick itself is declared in the specification as a periodic
            binding, so its cadence and its overlap rule are reviewable beside
            everything else the system promises.{' '}
            <code>serial_per_instance</code> is the one worth reading: one turn
            of one goal at a time, which is what stops a slow coordinator being
            asked to pursue the same goal twice.
          </p>
        </section>

        <section>
          <h2>How a turn is executed</h2>
          <pre>
            {`you ──▶ swarm-server ──▶ metaharness ──▶ claude ──▶ the work
          interpreter      hermetic       the run    a directory
          + event log      launch                    on this machine`}
          </pre>
          <p>
            A turn is one <code>metaharness run claude</code> process. It is
            handed a prompt naming the goal, the turn number and a work
            directory it keeps between turns. Its event stream comes back line
            by line, is written to disk and forwarded to every browser
            watching, so tokens, cache reads, cost and every tool call are
            visible while the turn is still running. The turn ends when the
            model writes a verdict line.
          </p>
          <p>
            The runtime never writes that line itself. A timeout, a crash or an
            exhausted budget leaves the goal in <code>Pursuing</code>, where a
            person can see it waiting — a runtime that answered on the
            coordinator's behalf would be reporting work nobody did.
          </p>
          <table>
            <thead>
              <tr>
                <th>program</th>
                <th>what it is</th>
                <th>in a swarm</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <code>substrate</code>
                </td>
                <td>
                  the execution data plane — confined workspaces, bounded
                  processes, cgroups and namespaces. Where the machine cannot
                  confine, it refuses rather than running unconfined
                </td>
                <td>not reached</td>
              </tr>
              <tr>
                <td>
                  <code>harness</code>
                </td>
                <td>
                  the b10x agent loop. It owns turns, tools, budgets and
                  approvals, and its published toolset is computed from what
                  the machine can confine — so the toolset is the safety
                  boundary
                </td>
                <td>not used</td>
              </tr>
              <tr>
                <td>
                  <code>metaharness</code>
                </td>
                <td>
                  drives a vendor binary into a scratch config home with a
                  seven-key environment allowlist, and turns whatever it emits
                  into one event format
                </td>
                <td>runs every turn</td>
              </tr>
              <tr>
                <td>
                  <code>claude</code>
                </td>
                <td>the vendor harness, with its own tools and its own loop</td>
                <td>the coordinator</td>
              </tr>
            </tbody>
          </table>
          <p>
            <strong>What confines a coordinator today: nothing.</strong> It is a
            Claude Code run under metaharness, which gives hermeticity and a
            complete event record, not confinement. metaharness implements no
            sandbox of its own, and its <code>--substrate</code> flag is refused
            by name for this arm — a socket configured there would be accepted,
            never consulted, and read as containment nobody applied. The
            coordinator runs on the host machine with the vendor's own tools,
            inside its work directory by convention rather than by force.
          </p>
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
      </main>
    </Layout>
  );
}
