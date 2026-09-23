# Execution Policy

Execution policy is a machine-readable guide for effort, tools, delegation,
verification, and human decision points. Factory includes a generic default at
`config/execution-policy.yaml`; a local workspace may override it with
`records/execution-policy.yaml` or `AGENT_EXECUTION_POLICY`.

Each profile defines:

- `effort` and an abstract `model` role; runtime adapters map roles to available providers.
- `skills` and `tools` to load or allow for the task.
- `delegation` boundaries such as `none`, `bounded-readonly`, or `bounded-parallel`.
- `verification` evidence required before reporting completion.
- `human_gate` decisions that require the user's input.
- `allowed_actions` the profile may carry out autonomously.

The policy guides planning; it does not grant production, credential, destructive,
external, or scope-expansion authority. Those actions still follow the active
runtime's safety and approval rules.
