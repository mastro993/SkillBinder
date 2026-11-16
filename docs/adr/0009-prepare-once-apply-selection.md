# An import plan is prepared once for every offered skill and applied to the ticked ones

The Import skills dialog (ADR 0008) shows each skill's replacements before Import, and they come from preparation: re-inspecting the chosen version and reading each directory where it would become active. So the dialog prepares one import plan for every skill it offers when it opens, and Import applies the plan to the ticked skills only. Unticking or ticking a skill, and acknowledging invalid metadata, need no new preparation: the plan holds invalid skills too, marked as such. Choosing another version of a skill prepares the plan again, because the content it would import changed; the ticks are kept.

Application checks the ticked skills against the plan: each must be in it, at most 500, and an invalid one only with the acknowledgement. The plan's other rules are unchanged: it expires after 300 seconds, it is refused when the library changed after review or a source changed, and applying a completed plan returns its stored result. When Import meets an expired plan, the dialog prepares it again with the same ticks and shows the result before the user presses Import again, so replacements are never applied unseen.

One preparation step serves both entry points. It runs a scan to completion, replacing a running one, and plans the offered skills from that scan: onboarding scans before any workspace exists, so it covers your agents' global skill directories alone. The separate first-boot preparation and its synchronous global scan are removed.

## Considered Options

- Preparing a plan for the ticked skills on every tick was rejected. Each preparation re-inspects every chosen version, which is slow for hundreds of skills and would make ticking lag.
- Preparing only when Import is pressed, and applying in the same call, was rejected. The replacements would be applied without the user seeing them, which ADR 0006 forbids.
- Computing replacements in every scan, so the dialog could read them from the found skills, was rejected. Every background scan would read every place a skill could become active, for a dialog that is rarely open.
