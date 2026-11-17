# Onboarding steps are an ordered list derived from saved facts

Onboarding becomes a short list of steps, and steps will be added and removed as the product changes. Today the current step is a stored cursor, the `OnboardingStep` name saved under the `onboarding` device setting, and the engine allows a move by comparing variant numbers. Adding, removing, or reordering a step changes that comparison and breaks saved rows. So the engine no longer stores where the user is. It keeps one ordered list of steps, and each step answers two questions from facts the engine already saves: whether it applies on this device, and whether it is done. The current step is the first one that applies and is not done; onboarding is finished when the Import step is done. The UI renders the list it receives, counts only the steps that apply, and moves Back through them in memory.

The facts are the ones the steps exist to settle: whether application data is writable, whether a usable Git was found or its step was skipped, whether the agent choice was saved, and whether onboarding finished. Each is saved by the action that settles it, so repeating an action is harmless and a restart resumes at the first unsettled step. A step that no longer applies, such as Git once it is installed, drops out of the list instead of showing as skipped.

Adding a step means adding a variant to the step enum, placing it in the list, and answering its two questions; the exhaustive matches fail to compile until it is. Removing one deletes its variant and leaves the other facts untouched.

## Considered Options

- Keeping a stored cursor and adding new variants was rejected. Every change of order needs a data migration, and a step that stops applying, such as Git once installed, would need its own skip rule.
- A registry of step objects behind a trait was rejected. The steps are known at compile time, and an enum with exhaustive matches makes a new step fail to compile until every place handles it.
- Storing a done flag per step was rejected. Most steps are done because of a real fact, such as Git being found, and a flag could disagree with it.

## Consequences

- The `onboarding` device setting is read once to carry developer data over: `Complete` becomes a finished onboarding with every detected agent chosen, any other value starts the new steps. No release has shipped (ADR 0004).
- The captured contract's four steps (`prerequisites`, `boundaries`, `syncChoice`, `ready`) are superseded: the source boundaries, sync choice, and Create library screens are removed, and the library is created without asking on the way into Import.
