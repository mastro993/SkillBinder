# 016 - Local diagnostics and logging

Status: accepted, 2026-09-21.

## Decision

Every warning, error, and command outcome in the Rust workspace is emitted through the `tracing`
facade and written to one local rotating file. `tracing` is a facade, so `core`, `db`, and
`platform` emit at the layer that owns the decision without depending on a logging crate and
without inverting the dependency direction.

One subscriber is installed by `apps/desktop` in the native startup, before application state
opens. It formats JSON lines and owns a panic hook that records a panic location and no payload.
The rotating writer lives in `crates/platform/src/rotating_log.rs`, the line redactor in
`crates/platform/src/redact.rs`, and the bounded queue that feeds the writer in
`crates/platform/src/log_sink.rs`. The queue is drained by a dedicated worker thread, so no emitter
performs file IO.

The log root is `AppPaths::logs()`, which is the application directory plus `logs`. The application
directory is `~/.skillbinder`, resolved from the home directory the OS reports, so the log root is
`~/.skillbinder/logs`. No path is built from a hard-coded home string.

The active file is `skillbinder.log`, and `skillbinder.log.1` through `skillbinder.log.4` hold the
older ones. The policy is five files of 5 MiB each, which is 5242880 bytes per file, and 14 days of
retention, which is 1209600 seconds. The active file rotates when the next line would exceed the
size cap or when it has reached the age cap. Aged backups are deleted when the sink opens and on
every rotation. A rotation failure degrades the sink to dropping lines and counting the drops,
rather than letting one file grow without bound.

Redaction runs in the sink worker, between the dequeue and the write, so no caller can skip it. The
rules strip the home directory prefix, in both its plain and its JSON escaped form, to `~`, replace
URL userinfo with `***`, replace the values of credential shaped query parameters with `***`, and
replace bearer token and authorization header values with `***`. The credential parameter names are
`access_token`, `api_key`, `apikey`, `client_secret`, `id_token`, `key`, `password`, `passwd`,
`refresh_token`, `secret`, and `token`, matched without case sensitivity.

The honest limit of that redaction is that it is shape based. It removes the shapes it knows and
cannot prove that a free-form message contains nothing sensitive, so a caller that formats a secret
into a message under an unrecognized name can still write it. Provenance is guaranteed, pattern
coverage is not.

Command outcomes are recorded once, in `commands::recorded`, which is the only seam a command
touches. A record carries the `diagnostic_id` the UI already shows on an error, the error
code, and whether the error is retryable. A record never carries the `AppError` message string.

Verbosity is one word in the `SKILLBINDER_LOG` environment variable, mapped to a level filter and
defaulting to `info`, which is the level the app ships at.

The design deliberately excludes telemetry, automatic crash upload, a log export command, a log
viewer, and a Settings surface for paths or levels. The UI and the UI thread do not log beyond
what crosses the command boundary. There is no second log directory, no portable log location, and
no log location inside the library repository, so a log file never enters a Git history.

## Alternatives

`tracing-appender` was rejected because it rotates by time only and therefore cannot produce five
files of 5 MiB each.

An owned typed event enum on application state was rejected. Its redaction guarantee is the
strongest of the shapes considered, because an event type can have no path or message field, but
`core` and `db` could not emit at all without a logging port on core traits, which would leave
rollback failures and JSON fallbacks invisible.

A boundary-only audit that logs inside `apps/desktop` was rejected. It is the smallest diff of the
shapes considered, and it fails the request, because domain warnings stay unauditable and it puts a
file writer in the shell crate whose job is transport mapping and composition.

A `~/.skillbinder/logs` root was rejected. It adds a second directory root next to the application
local data directory that the specification defined at the time, and it contradicted the rule
against paths built from a hard-coded home string. Superseded: the application directory is now
`~/.skillbinder` itself, so the log root is inside the single application root and the second-root
objection no longer applies. The path is still resolved from the home directory the OS reports, not
from a home string in the source.

The `env-filter` feature of `tracing-subscriber` was rejected. It pulls in regex machinery for a
filter that no user-facing surface exposes, when one level word covers the requirement.

## Consequences

Logging never fails a command and never blocks the UI thread. A full queue drops and counts lines
instead of making the emitter wait, and a failed rotation degrades the sink instead of taking the
process down or growing a file without bound.

Every emission site is a searchable call, and no crate needs a logging port. Command outcome
recording has one seam, so a new command cannot silently skip it as long as the wrapper calls the
seam, and one test compares the seam against `generate_handler!` to catch the one that does not.

The log file is a support artifact. The `diagnostic_id` on a user error dialog and the same string
inside the file are one grep apart, and the record omits the message text that could carry user
data. Redaction happens before the write, so a file that leaves the machine has been through the
same rules as one read in place.

Rotation is bounded. The steady state is at most five files and at most 25 MiB of log data per
machine, and the age deletion keeps the backup count down on a lightly used install.

## Reversal cost

Dropping local logging means deleting the sink, the redactor, the queue, and the install site,
removing the dependency pins, and reverting the command wrappers to direct handler bodies.
Emission call sites use the facade and compile either way, so they are not part of the reversal.

Switching the sink to another backend touches the install site and the sink crate only, because
callers depend on the facade and on the seam rather than on the file.

Changing the retention numbers means editing `RotationPolicy::default` and this ADR.
