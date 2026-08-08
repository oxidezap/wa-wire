# wa-wire-gate

Compare two recordings and say whether the candidate may ship.

```sh
wa-wire-gate --profile regression baseline.wawr candidate.wawr
```

Everything this does already existed as library code and was reachable only
from tests. That is the gap it closes: a container, a comparator and a set of
profiles that nobody can run is a design, not a tool.

## The three answers

`pass`, `fail`, and **`incomparable`** — which is not a pass. A gate that folded
"these were never comparable" into either of the other two would report a
conclusion drawn from a comparison that never happened, and that is precisely
what [RFC-010](../../DESIGN.md#rfc-010--recording-container) was specified to
prevent.

| Exit | Meaning |
| --- | --- |
| 0 | pass |
| 1 | at least one finding failed under the profile |
| 2 | the recordings may not be compared |
| 64 | the arguments were wrong |
| 66 | a recording could not be read |

## The two profiles

| | `interop` | `regression` |
| --- | --- | --- |
| The question | two engines, one input: do they mean the same thing? | one engine, two builds: did the newer one lose anything? |
| Direction | symmetric | the baseline is the reference |
| Differing frame bytes | two valid encodings | the encoder changed |
| Coverage lost | a limit on the adapter | a regression |
| Coverage gained | not a finding | an improvement, reported and passing |

## What it reports

The header describes both recordings, the verdict names the profile that
answered, and the findings are split into failures, improvements and everything
the profile tolerated but still saw.

Below that, what the payloads turned out to be:

```
content:
  conversation   baseline    6   candidate    6
  image          baseline    2   candidate    1   <- differs
  unreadable     baseline    0   candidate    1   <- differs
```

Per side rather than merged. A single total would hide a candidate that read
fewer messages than the baseline, which is the finding. The section is omitted
when neither side carries a plaintext, rather than printed empty.

Long lists are trimmed and say what they trimmed. A cap nobody is told about
reads as "that was all of them".

## Which dictionary

Frames are parsed against the token dictionary they were encoded with (D-082),
and this build has exactly one: the bundled table. A recording naming a
different one is refused rather than parsed with the wrong table and blamed on
an engine.

## Testing

```sh
cargo test -p wa-wire-gate
```

The library tests decide what the gate concludes; `tests/cli.rs` runs the real
binary and checks the exit codes a pipeline branches on.
