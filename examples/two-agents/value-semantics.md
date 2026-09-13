# Evidence value compatibility

The original reader's conversions are part of its report contract. `swarm-check` reads evidence
into its own ordered value type; it does not change the workspace's `serde_json::Value` ordering.
This matters because the runtime hashes serialized request values for persisted idempotency keys.
The regression `ordered_evidence_does_not_change_runtime_json_map_serialization` checks both
behaviors with the same object in different key orders.

Integer membership and numeric equality are separate rules. Boolean values count as integers for
claimed attempts and census totals, but integral floats do not. Floats can still equal a transcript's
turn number for attribution. Arbitrary-precision integer values preserve unsigned runtime counters,
missing-attempt claims, ordering, and census sums without narrowing them to `i64`. Numerically equal
boolean/integer counter keys coalesce while keeping the first key's displayed spelling.

An optional reason uses Python truthiness: null, false, zero and empty strings/containers add no
suffix. Container values use recursive Python representation, including nested string quoting,
boolean/null words and source object order. Top-level strings remain unquoted. Event payloads
searched for ceiling sentences use the old JSON representation instead, including source member
order and ASCII escaping. The ordinary report JSON schema does not change.

Every call site in the two corrected classes is enumerated below. The tests materialize raw evidence
and compare the full parsed report and CLI exit against 81 captured baseline reports in
`tests/fixtures/value-semantics.json`; no baseline interpreter runs during Rust tests.

| Call site | Preserved rule | Regression group |
|---|---|---|
| `agent_of`: spend iteration join | Numeric equality accepts booleans and integral floats | `numeric_attribution_equality` |
| `three`: claimed attempts | Integer membership includes booleans and arbitrary integers, excludes floats | `integer_attempt_claims`, `counting_is_distinct_from_numeric_equality` |
| `three`: aggregation/sorting and missing-attempt text | Numeric keys coalesce and sort, preserving first spelling | `equivalent_integer_keys_keep_first_spelling` |
| `five`: census denial totals | Integer membership and unbounded integer summation | `census_integer_membership_and_unbounded_sum` |
| `five`: optional reason suffix | Truthiness of all JSON value categories | `denial_reason_truthiness` |
| `five`: reason text | Recursive representation, numeric formatting, quoting, object order | `recursive_reason_representation` |
| `one`: spawned role text | Recursive representation for non-string roles | `spawned_role_representation` |
| `two`: posted agent text | Recursive representation for non-string agent claims | `posted_agent_representation` |
| `two`: taken agent text | Recursive representation for non-string agent claims | `taken_agent_representation` |
| `five`: requested tool text | Recursive representation | `tool_name_representation` |
| `five`: denying decider text | Recursive representation | `decider_representation` |
| `five`: listed operations | Arrays yield values, objects yield keys, strings yield characters; nested values are represented recursively | `operation_iteration_and_representation` |
| `ceiling_refusals`: serialized event payload | The first matching sentence follows source member order | `event_json_preserves_source_member_order` |

The five original port-adversary tests remain unchanged. These corrections concern readable
inputs; the documented read-only SQLite and inaccessible-file behavior remains in force.

The final review added five more regressions, whose assertions are preserved. `unicode_parser.rs` compares complete
JSON reports, text bytes and both exit codes against 88 saved baseline scenarios. These inputs
exercise the same representation call sites above, with the following additional boundaries:

| Regression group | Cases | Boundary |
|---|---:|---|
| `unicode_printable_categories` | 10 | Letters, marks, numbers, punctuation and symbols remain printable, including combining marks and both quote choices |
| `unicode_other_and_separator_categories` | 15 | Unicode Other and Separator categories are escaped in nested representation |
| `ascii_space_stays_printable` | 1 | ASCII space is the printable separator exception |
| `legacy_nonfinite_constants` | 8 | NaN, Infinity, -Infinity and overflow exponents remain readable locally |
| `nested_unpaired_surrogates` | 9 | Nested values and object keys escape lone surrogates; literal backslashes and NULs remain distinct |
| `top_level_surrogates_distinguish_json_and_text` | 6 | JSON preserves escaped surrogates; text encoding fails before stdout for unencodable lone surrogates |
| `surrogateescape_text_bytes` | 3 | Top-level U+DC80 through U+DCFF use the baseline stdout surrogateescape policy |
| `valid_surrogate_pairs` | 3 | Valid escaped pairs decode to the corresponding scalar |
| `ordinary_malformed_grammar_remains_rejected` | 18 | Trailing data/commas, raw controls, invalid escapes and incomplete structures remain rejected |
| `sqlite_text_transport_boundaries` | 12 | SQLite name, actor, issuer and payload text distinguish literal NUL markers from escaped surrogate evidence |
| `deep_legacy_readable_evidence` | 3 | 150, 512 and 30,000 nested arrays preserve the measured baseline report |

The parser accepts the legacy nonfinite constants and lone escaped surrogates without changing
ordinary workspace serde behavior. A local lossless string representation carries unpaired
surrogates through report formatting. It escapes every literal input NUL, including SQLite
metadata and library error strings; report serialization decodes the representation explicitly.
Filesystem names and displayed paths enter as scalar strings. Text output reproduces the measured
UTF-8/surrogateescape stdout behavior; the error message for an unencodable surrogate is a concise
diagnostic rather than the original Python traceback.

Parsing, rendering, cloning and destruction use heap work lists. A separate unit test rejects
100,000-level incomplete arrays and objects without recursive stack growth. There is no fixed
128-level acceptance limit. The saved Python 3.14 baseline accepts 30,000 nested arrays but fails
at larger depths through environment-dependent stack exhaustion; the Rust reader does not emulate
that host stack threshold. These checks establish the stated accepted inputs and bounded call-stack
use, not parity for every possible memory-exhaustion boundary.
