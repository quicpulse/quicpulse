# Code Coverage

Reference notes for QuicPulse test coverage: how it is measured, where it stands,
what was done in each pass, and what is worth doing next.

## Contents

| File | What it is |
|---|---|
| `tarpaulin.txt` | Raw `cargo tarpaulin` output: per-file totals plus an explicit list of uncovered line numbers. |
| `README.md` | This document. |

## Current state

**55.47% line coverage — 11881 / 21419 lines** (2026-08-17)

Previous baseline was 52.07% (11145 / 21403), committed in `bb84748`. To diff against it:

```bash
git show bb84748:coverage/tarpaulin.txt
```

Test suite at time of measurement: **753 lib unit tests + 80 test binaries, all passing.**

## Regenerating the report

```bash
cargo tarpaulin --skip-clean --out Stdout > coverage/tarpaulin.txt 2>&1
```

Notes:

- Takes roughly 25–40 minutes — tarpaulin recompiles every test binary with
  instrumentation, and this crate has 80 of them.
- Do **not** add `--release`. Per `.claude/CLAUDE.md` this project uses dev builds
  and tests only; release builds are CI's job.
- Tarpaulin writes ANSI colour codes even when redirected to a file. Strip them so
  the committed report stays readable and diffable:
  ```bash
  python3 -c "import re,sys;p='coverage/tarpaulin.txt';s=open(p,errors='replace').read();open(p,'w').write(re.sub(r'\x1b\[[0-9;]*m','',s))"
  ```
- When a previous report exists, tarpaulin appends a per-file delta
  (e.g. `src/status.rs: 13/13 +30.77%`). Parsers must tolerate that trailing field.
- Tooling used for the current figures: `cargo-tarpaulin 0.37.2`, `rustc 1.97.1`.

## Measurement caveats

- **`#[cfg(test)]` blocks are excluded** from the denominator. Total coverable lines
  moved only 21403 → 21414 across a pass that added ~3300 lines of test code, so the
  percentages reflect production code only. The +11 came from the two production
  fixes below.
- **Small run-to-run noise exists.** `src/core.rs` moved 789 → 788 covered lines in a
  pass that never touched it. Treat single-line swings as instrumentation noise, not
  regressions.
- Because fixes can add production lines, a pass can raise covered-line count more
  than the percentage suggests. Read both numbers.
- **Deleting well-covered code lowers a file's percentage.** Removing the duplicated
  `form_urlencode` dropped `client/http.rs` from 50% to 49% while its uncovered count
  stayed at exactly 241. When judging a file, compare the **uncovered count**, not the
  ratio — the ratio moves when the denominator changes.

## Pass log

### 2026-08-17 — 52.07% → 55.47% (+3.40 pts, +736 lines)

Approach: ranked all 172 source files by uncovered line count, then targeted the
highest-yield **pure-logic** modules — ones testable without a network peer or live
server. Added inline `#[cfg(test)]` modules, matching the existing convention
(111 source files already used them).

**277 tests added across 13 files.** Lib tests 474 → 753.

| Tests | Gain | Before | After | Uncovered |
|---:|---:|---|---|---|
| 30 | +116 | 167/290 (57%) | 283/290 (**97%**) | 123 → 7 |
| 35 | +97 | 46/136 (33%) | 143/145 (**98%**) | 90 → 2 |
| 48 | +84 | 66/143 (46%) | 150/159 (**94%**) | 77 → 9 |
| 20 | +83 | 1/84 (1%) | 84/84 (**100%**) | 83 → 0 |
| 15 | +82 | 0/82 (0%) | 82/82 (**100%**) | 82 → 0 |
| 16 | +67 | 0/67 (0%) | 67/67 (**100%**) | 67 → 0 |
| 25 | +57 | 36/93 (38%) | 93/93 (**100%**) | 57 → 0 |
| 23 | +56 | 444/1894 (23%) | 500/1894 (26%) | 1450 → 1394 |
| 18 | +55 | 24/83 (28%) | 79/83 (**95%**) | 59 → 4 |
| 11 | +24 | 0/24 (0%) | 24/24 (**100%**) | 24 → 0 |
| 16 | +13 | 12/14 (85%) | 25/25 (**100%**) | 2 → 0 |
| — | +11 | 80/123 (65%) | 91/123 (73%) | 43 → 32 |
| 10 | +8 | 9/17 (52%) | 17/17 (**100%**) | 8 → 0 |
| 10 | +4 | 9/13 (69%) | 13/13 (**100%**) | 4 → 0 |

Files, in the same order: `output/formatters/colors.rs`, `openapi/schema_mapper.rs`,
`devexp/curl.rs`, `output/formatters/json.rs`, `output/formatters/xml.rs`,
`uploads/multipart.rs`, `request/builder.rs`, `pipeline/runner.rs`,
`output/codec/pretty.rs`, `uploads/chunked.rs`, `strings.rs`, `request/json.rs`,
`uploads/compress.rs`, `status.rs`.

Nine files went from partial or zero coverage to 100%. `request/json.rs` gained
incidentally because the `request/builder.rs` tests exercise `set_nested_value`.

Two files show a *negative* line delta from the `form_urlencode` consolidation and
are **not** regressions — their uncovered counts are unchanged, they simply have
fewer total lines:

| Delta | Before | After | Uncovered | File |
|---:|---|---|---|---|
| −9 | 241/482 (50%) | 232/473 (49%) | 241 → 241 | `client/http.rs` |
| −12 | 789/1348 (58%) | 777/1337 (58%) | 559 → 560 | `core.rs` |

(`core.rs` gaining one uncovered line is the run-to-run noise described above; nothing
in that file was touched beyond deleting the duplicate helper.)

## Bugs found by these tests

Writing tests against real behaviour (rather than asserting whatever the code
already did) surfaced three defects. All three are now fixed, each with regression
tests named below.

### Fixed — inverted integer ranges in OpenAPI magic values

`src/openapi/schema_mapper.rs`

For a format-less `integer`, the `(0, 1000)` defaults were passed to
`int_with_constraints` as *type clamps*. A schema with `minimum: -50, maximum: -10`
therefore produced `{random_int:0:-10}` — an inverted range. The `int32`/`int64`
arms pass genuine type bounds and were always correct, which is what confirmed
0/1000 were only ever meant as defaults.

A follow-up property test then caught a second case: `maximum: -1` with no
`minimum` yielded `{random_int:0:-1}`. `int_with_constraints` was restructured so a
defaulted bound can never cross an explicit one, a reversed schema is repaired, and
clamping to the format's representable range happens last.

Regression tests: `test_negative_integer_bounds_are_preserved`,
`test_generated_int_ranges_are_never_inverted`.

### Fixed — `--curl` emitted a redacted bearer token

`src/devexp/curl.rs`

`args.auth` is a `SecretString` whose `Display` impl renders `[REDACTED]`. The
Bearer arm used `format!("Authorization: Bearer {}", auth)`, so the exported command
contained `Bearer [REDACTED]` and could not run. The Basic and Digest arms call
`shell_escape(auth)`, which reaches the real value via `Deref` — that inconsistency
is what showed this was accidental rather than deliberate redaction. Fixed by using
`auth.as_str()`.

Regression test: `test_bearer_auth_becomes_an_authorization_header` (also asserts
the output contains no `REDACTED`).

### Fixed — `--curl` silently dropped query parameters

`src/devexp/curl.rs`, `src/strings.rs`

Query parameters are applied when the request is built (`request/builder.rs`,
`client/http.rs`) and were never folded into `processed.url`. The curl exporter's
item loop only handled `Header`, `EmptyHeader`, and `HeaderFile`, so a `q==value`
argument vanished from the exported command entirely — the command ran, but against
a different URL than the one QuicPulse itself requests.

Fixed by adding `build_url`, which appends `QueryParam` / `QueryParamFile` items
using the same encoder and the same `?` / `&` separator logic as the real request
path in `core.rs`.

That required a shared encoder. `form_urlencode` existed as two byte-identical
private copies (`core.rs:5` and `client/http.rs:51`); a third copy in the exporter
would have been the thing most likely to drift out of sync and silently reintroduce
this class of bug. Both copies were removed and the function now lives in
`strings.rs` as the single encoder for query strings and form bodies, so the
request sent, the URL displayed, and the exported command cannot disagree.

Verified end to end against a local echo server — QuicPulse and its own exported
curl command produce byte-identical request lines:

```
PATH /search?q=search+term&filter=a%26b   <- quicpulse
PATH /search?q=search+term&filter=a%26b   <- exported curl command
```

Regression tests: `test_query_params_are_appended_to_the_exported_url`,
`test_query_params_use_form_style_encoding`,
`test_query_params_merge_into_a_url_that_already_has_some`,
`test_url_with_query_params_is_shell_quoted`,
`test_exported_query_string_matches_the_real_request_url`, plus 10 unit tests
covering `form_urlencode` directly in `strings.rs`.

Note the form **body** encoder in `curl.rs` (`percent_encode`, using
`NON_ALPHANUMERIC`) is deliberately left alone — it emits `%20` for spaces where
the request path uses `+`. Both decode identically per the urlencoded spec, so this
is cosmetic, but it is the remaining spot where exporter and client encoders differ.

## Techniques that worked

Reusable notes for the next pass.

- **Ground assertions in observed behaviour first.** For quirky formatters, dump
  real output before writing expectations, then decide whether that output is
  correct. Writing the test blind and "fixing" it to match the code just enshrines
  bugs.
- **`reqwest::multipart::Form::into_stream()` is public.** Multipart tests can assert
  on real wire bytes — field names, `filename=`, content types, raw payloads — instead
  of only checking that a `Form` was constructed.
- **Drive CLI code through the real pipeline.** `Args::try_parse_from([...])` (needs
  `use clap::Parser`) followed by `cli::process::process_args` exercises the actual
  parse path and covers `cli/process.rs` for free.
- **Build serde config types from YAML fixtures.** `Workflow` has no `Default`;
  `serde_yaml::from_str` is the clean way to construct one. `WorkflowStep` and
  `StepAssertions` do derive `Default`.
- **Property tests earn their keep.** The second inverted-range bug was found by
  sweeping bound combinations, not by a hand-written case.
- **Watch out for library recursion limits.** `serde_json::from_str` caps nesting at
  128, so a depth-limit test deeper than that must build the value programmatically.
- **Round-trip assertions are strong and cheap.** Strip-ANSI-and-compare for
  colourizers, inflate-and-compare for compression, reparse-and-compare for JSON
  formatting: each proves content is preserved without pinning cosmetic details.
- **Verify exporters against a real listener.** For anything that claims to reproduce
  a request (`--curl`, codegen, HAR export), a local echo server proves fidelity in a
  way unit tests cannot: run the tool, run its own generated output, and diff the
  request lines the server actually received. This is what confirmed the query-param
  fix rather than merely asserting on a formatted string.
- **Duplicated helpers are a coverage smell.** `form_urlencode` existed as two
  identical private copies; each copy is separately untested and free to drift.
  Consolidating before adding a third caller both fixed the bug and removed the
  conditions for it to recur.
- Test private helpers directly from the inline module via `use super::*` — no need to
  widen visibility.

## Remaining gaps, by priority

9538 lines still uncovered. The cheap pure-logic wins are largely spent; what is
left needs test infrastructure.

**Best remaining ratio — likely still pure logic, no server needed:**

| Uncovered | Current | File |
|---:|---|---|
| 216 | 6/222 (2%) | `pipeline/sharing.rs` |
| 305 | 121/426 (28%) | `grpc/dynamic.rs` |
| 177 | 78/255 (30%) | `grpc/reflection.rs` |
| 178 | 27/205 (13%) | `auth/oauth2_flows.rs` |

**Biggest single prize — needs `wiremock` fixtures:**

| Uncovered | Current | File |
|---:|---|---|
| 1394 | 500/1894 (26%) | `pipeline/runner.rs` |
| 560 | 777/1337 (58%) | `core.rs` |
| 241 | 232/473 (49%) | `client/http.rs` |

`pipeline/runner.rs` is dominated by a single ~2900-line async `run` method
(line ~488 onward). Its pure parts — `validate`, `StepResult::passed`,
`format_workflow_results`, `format_workflow_results_json` — are already covered;
everything further requires executing steps against a mock server.

**Needs a test gRPC server (~1300 lines combined):**
`grpc/client.rs` (21/369), `grpc/mod.rs` (54/326), `grpc/interactive.rs` (0/195).

**Needs real network peers — lowest value per unit of effort:**
`client/http3.rs` (4/673), `websocket/stream.rs` (0/129),
`websocket/interactive.rs` (0/111).

**Small zero-coverage files, quick wins if a pass needs padding:**
`output/formatters/headers.rs` (0/19), `output/error.rs` (0/7),
`models/types.rs` (0/23), `downloads/status.rs` (0/13), `debug/mod.rs` (0/14).
