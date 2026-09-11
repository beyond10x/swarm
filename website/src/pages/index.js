import Layout from '@theme/Layout';
import HeroBanner from '@site/src/components/HeroBanner';

export default function Home() {
  return (
    <Layout
      title="About"
      description="Swarm — an Executable System Specification of a swarm manager.">
      <HeroBanner
        title="Swarm"
        tagline="Many swarms, each one orchestrator, many agents — specified, not just built."
      />
      <main className="container about-section">
        <section>
          <p>
            Swarm is a swarm manager: many swarms, each one tmux session with
            one orchestrator and many agents, composed top-down from the
            manager to the single step of a DAG. It is written as an{' '}
            <strong>Executable System Specification</strong> (ess/1) — every
            entity, field, command and view in the spec carries a comment
            naming what it was read from.
          </p>
        </section>

        <section>
          <h2>How it's being built</h2>
          <p>
            The starting shape is deliberately small. The kernel solves tool
            access and agent creation; everything an agent then builds is up
            to it.
          </p>
          <table>
            <thead>
              <tr>
                <th>kernel piece</th>
                <th>what it does</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>tool access</td>
                <td>
                  an agent is spawned with a loadout — skills, tools, MCP
                  servers, plugin dirs — and a harness/binary/model to run it
                </td>
              </tr>
              <tr>
                <td>orchestrator creates agents</td>
                <td>
                  a five-step flow: pick a role, guarantee the loadout,
                  create the inbox, post the assignment, tell the rules
                </td>
              </tr>
              <tr>
                <td>inbox</td>
                <td>
                  named inboxes per agent, a post addressed to a pair, read
                  marks, an ack terminal, broadcast fan-out
                </td>
              </tr>
              <tr>
                <td>eventbus-like connections</td>
                <td>
                  boxes with only inputs and outputs, schemas known to
                  everyone, connections other agents make themselves
                </td>
              </tr>
              <tr>
                <td>presentation layer</td>
                <td>this site, and the swarm UI — for a human to observe</td>
              </tr>
            </tbody>
          </table>
          <p>
            From there, the agents build what they need: a <code>Flow</code>{' '}
            is a DAG of steps (command, tool, LLM or subflow) with typed
            ports and a context — recursion via subflow.
          </p>
        </section>

        <section>
          <h2>Levels</h2>
          <table>
            <thead>
              <tr>
                <th>level</th>
                <th>noun</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>0</td>
                <td>
                  <code>SwarmManager</code> — the system
                </td>
              </tr>
              <tr>
                <td>1</td>
                <td>
                  <code>Swarm</code> — one tmux session, one run record, one
                  orchestrator
                </td>
              </tr>
              <tr>
                <td>2</td>
                <td>
                  <code>Agent</code>, <code>Orchestrator</code>,{' '}
                  <code>Role</code>, <code>Host</code>, <code>Loadout</code>
                </td>
              </tr>
              <tr>
                <td>3</td>
                <td>workflow — states, guarded transitions, back-edges</td>
              </tr>
              <tr>
                <td>4</td>
                <td>
                  <code>Flow</code> — the DAG inside one workflow state
                </td>
              </tr>
              <tr>
                <td>5</td>
                <td>
                  <code>Step</code> — command, tool, llm, subflow
                </td>
              </tr>
            </tbody>
          </table>
        </section>

        <section>
          <h2>Specified, checked</h2>
          <p>
            The specification validates and compiles with <code>ess</code>,
            synthesizes verification scenarios, and renders a browsable site
            from the same source — the same discipline this page follows: it
            says only what the spec and its README already say.
          </p>
        </section>
      </main>
    </Layout>
  );
}
