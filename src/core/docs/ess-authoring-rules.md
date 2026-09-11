# ess/1 authoring rules for swarm2 (ess 0.22.2, probed 2026-09-11)

Read this before writing a domain file. Every line is either from the `ess-specify:specify` skill,
from `../ess/examples/billing/domains/invoice.yaml`, or from a refusal observed on this machine.

## Layout and validation

- Legacy layout: `swarm2/system.yaml` + every `*.yaml` under `swarm2/` recursively. The header's
  `domains:` list and the files must agree in both directions; either half alone is refused.
- Validate a **directory**: `ess specify validate --path <dir>`. Expected: `swarm2 v1 — N file(s), valid`, exit 0.
- To validate one new domain before the others exist: copy `swarm2/` to a scratch dir with a
  session-unique name under `~/.cache/` (never `/tmp`, never a fixed name), edit the scratch
  `system.yaml` `domains:` list to the domains present there, validate there. Relay every refusal
  verbatim; do not stop at the first.
- A cross-domain `owns`/`references` target must exist in the scratch. If the target domain is not
  yours and not in the base, write the relation as a comment `# DEFERRED-RELATION: <yaml>` and
  say so in your report; the integrator adds it.

## Entities

- `identity`, at least one field, and `lifecycle` are all required.
- `identity.type` may be `Uuid`, `String` or `Integer` (all probed). Use `Uuid` for anything a
  command creates whose id the implementation assigns; `String` where an existing slug is the
  identity (agent role slug); `Integer` for a stable number (decisions).
- A state with no outgoing transition must be in `terminal:`; every non-terminal state needs one.
- Every transition must be taken by some command outcome `moves: <Entity>.<transition>` —
  otherwise `missing_causation`. Add a state only together with the outcome that reaches it.
- `invariants:` are `field op literal` (`amount >= 0`, `total.amount >= 0`). **Field-vs-field is
  refused** (`disk_target_gb >= disk_floor_gb` → ESS-TYPE-002, the right side reads as text).
- `relations:` — `owns` (far side has no meaning alone; `via` = the field on the **target** typed
  by the owner's identity), `references` (far side stands alone; `via` = the field on the
  **source**). Unknown target refused; two owners refused. Cross-domain targets are accepted.
  A cardinality or ownership you cannot read from the sources is `UNMAPPED:`, not a guess.

## Commands

- **Exactly one unconditional outcome** per command. All-conditional is refused
  (ESS-COMMAND-005 `non_exhaustive_branches`); two unconditional is refused (ESS-COMMAND-004).
- `when:` reads **input fields only** (ESS-COMMAND-003 `unobservable_fact`). Grammar: `a == B`,
  `a != B`, `a > 0`, `a >= 0`, `a < 0`, `{all: [p, q]}`, `{any: [p, q]}`. Enum literals are
  checked against declared variants. "A row exists" / "already stamped" is a lookup, not a guard —
  state it in prose beside the command and declare the error under `errors:`.
- Outcome verbs: `creates: <Entity>` (starts at `initial`; `instance:` names a field of an
  **emitted event** that carries the new id — String or Uuid both probed), `moves:
  <Entity>.<transition>` and `updates: <Entity>` (`instance:` names the **input** field carrying
  the id), `error: <Error>` (no subject; a refusal changes nothing), `wrong_state: true` + `error:`
  (only on a `moves:` command — the branch for every state the transition does not run from).
- `emits: [events]` + `payload: {Event: {field: input.x}}` — one line per event field whose value
  comes from the input; fields the implementation assigns (ids, resolved lookups) get no line.
- `sets: {field: input.x}` — what the entity holds afterwards; input fields only.
- `naming: {wire, display}` on every command.

## Events, views, errors, actors, types

- Events: `fields` only. An event may carry fields no input has (resolved by the implementation).
- Views: `source` one entity, `consistency: read_your_writes | eventual`, `filter` is **one
  equality** (`state == Posted`, `read_at == null`); no `&&`. `fields` must exist on the source
  (no joins — denormalise onto the entity if a view needs it, and say why). `order_by: [field desc]`
  requires the field in `fields`. `naming` on every view.
- Errors: `name`, `summary`, `fields`. An error may be declared and named by no outcome.
- Actors: `may: [commands]`; an actor with no `may` is legal and means "in the picture, permitted nothing".
- Types: `kind: newtype|struct|enum|union`; struct fields may nest structs and `List<T>`,
  `Map<String, String>`, `Optional<T>`. Primitives: `String Integer Decimal Boolean Uuid Timestamp
  Duration Bytes`. Enum type of an entity's lifecycle state is `<Entity>.State`.
- Cross-domain type references (`type: swarm2.config.Harness` from another domain) — allowed for
  enums/structs as far as probed; if refused, declare the type locally and report it.

## Provenance

- Every entity, field, command and view carries a comment saying which file or decision it was
  read from (`swarm/WORKING-RULES.md §8`, `decisions.md row 34`, `.agents/README.md`). Quote
  rather than cite line numbers (WORKING-RULES §6).
- What no source says is `# UNMAPPED: <what would settle it>` at the place it would go, and is
  listed in the report. Never invent a type, state or edge to make the file validate.
- No `/tmp`. Scratch under `~/.cache/swarm2-<yourname>-<pid>/`. Write only the one file you own.
  No commits.
