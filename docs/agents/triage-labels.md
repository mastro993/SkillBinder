# Triage labels

The vocabulary is the default Matt Pocock triage set: two category roles and five state roles. An
issue that has been triaged carries exactly one of each. This repository uses the strings as they
are written here; there is no local renaming table to keep in sync.

## Categories

| Label         | Meaning                      |
| ------------- | ---------------------------- |
| `bug`         | Something is broken          |
| `enhancement` | A new feature or improvement |

## States

| Label             | Meaning                                                                                                                         |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `needs-triage`    | A maintainer still has to evaluate it. This is where an unlabelled issue lands.                                                 |
| `needs-info`      | Waiting on the reporter. It returns to `needs-triage` once the reporter answers.                                                |
| `ready-for-agent` | Fully specified, with an agent brief attached, so an agent can take it without further questions.                               |
| `ready-for-human` | The same brief, plus the reason it cannot be delegated: a judgment call, external access, a design decision, or manual testing. |
| `wontfix`         | Will not be actioned. The issue closes with the reason recorded.                                                                |

See `issue-tracker.md` for where a work item lives and how a fix lands. The upstream source of this
vocabulary is `mattpocock/skills`, and it is explicitly per-repository overridable, so a rename here
means editing this file and the GitHub labels together, not one of them.

## Rules

- One category and one state. Two state labels on one issue is a conflict: flag it instead of
  picking one.
- Triage is human-invoked. An agent triages when asked, gathers context, verifies the claim, and
  recommends the labels; it does not reclasify issues on its own.
- Only issues the project did not create need triage. Work that came out of this repository's own
  planning is already specified.
- Every comment an agent posts during triage says so: start it with the AI-generated line the
  triage skill defines.

## Applying them

```sh
gh issue edit <number> --add-label "bug" --add-label "ready-for-agent"
gh issue edit <number> --remove-label "needs-triage"
gh issue close <number> --comment "<reason>"
```

External pull requests are triaged the same way: they either answer a report or become one.
