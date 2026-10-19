# Beaver

>  ⚠️ Experimental: This project is not production-ready, use at your own risk.

I built Beaver as I was in need of a solution, allowing me to securely and flexibly define server side AI assisted workflows automating some of the repeated work I do. Many of the existing solutions I found were either client side only, highly insecure, expensive proprietary SaaS or AI spaghetti code.

At the time Beaver allows you to define AI agents in YAML, which can be either triggered via CLI or through flows over a webhook. The focus is on autonomous agents that can call tools all on their own but without jeopardizing security in any way. There is a handful of embedded tools which can be extended with JavaScript or external MCP servers. The agents can use multiple LLM providers (currently Anthropic or OpenCode Zen).

## Features

- **Multi-Provider/-LLM** — Anthropic, Zen (more to come)
- **Pluggable Tools** — JavaScript (sandboxed) and MCP servers
- **Agents & Flows as YAML** — no custom DSL, just YAML
- **Ships as a container** — Helm chart included

## Quick start

Here's a few commands to play with Beaver. It's however recommended to install Beaver using the Helm Chart in [`chart/`](chart) and configure it with your own agents, flows, and tools.

```sh
# Chat with an agent
cargo run -p cli -- conversation <agent_name>

# Run the webhook server (serves /healthz and /hooks/{name} on :8080)
cargo run -p webhook
```

## Examples

There's a few examples in [`examples/`](examples) and [`chart/values.example.yaml`](chart/values.example.yaml). Here's a simple one, an agent that reviews a pull request and leaves comments on it:

### Agent definition

```yaml
metadata:
  name: reviewer
  display_name: Reviewer
  model:
    name: claude-fable-5
permissions:
  tools:
    - name: js_bitbucket_pr_get
    - name: js_bitbucket_pr_diff
    - name: js_bitbucket_pr_comment
prompt: |
  Review this pull request's diff and leave feedback as comments.
```

### Flow definition

A flow is a pipeline of steps triggered through the same `/hooks/{name}` endpoint. Flows live in `flows.path` (one `<name>.yaml` per flow, `metadata.name` must match the file name) and are re-read on every trigger, so adding, editing or removing a file needs no restart.

```yaml
version: 1
metadata:
  name: pr_review_flow
  description: Clone and review a PR
spec:
  arguments:
    url: { required: true, pattern: "^https://" }
    title: {}
  contexts:
    work:
      agent: coder
      chroot: { path: "runs/{{ Ctx.trace_id }}", create: true, delete: true }
  triggers:
    webhook:
      name: pr_review_flow
      token: "REPLACE_ME"
      arguments:
        url: "{{ Body.repository.links.html.href }}"
        title: "{{ Body.pullrequest.title }}"
  steps:
    - name: clone
      context: work
      type: tool
      arguments:
        tool: git_clone
        scopes: ["*"]
        arguments: { url: "{{ Arguments.url }}", path: "/repo" }
    - name: review
      context: work
      type: agent
      arguments:
        prompt: "Review the PR '{{ Arguments.title }}' in /repo. Clone said: {{ Contexts.work.last().content }}"
```

- **`version`** is required and must be `1`. Unknown fields are rejected.
- **Steps** run in order; the first failing step stops the flow. Step names must be identifiers (`^[A-Za-z_][A-Za-z0-9_]*$`) and unique. Every agent and tool step names a declared `context`; `loop`, `set` and `flow` steps have none.
- **`agent` steps** send the rendered `prompt` to the context's agent. **`tool` steps** call `tool` directly with the rendered `arguments` and the given `scopes`, without an LLM.
- **Contexts** are declared once under `spec.contexts`, keyed by an identifier name, each with one `agent` and an optional `chroot`. A context is a conversation plus a working directory: steps on the same context share both, so a later agent step continues the earlier conversation and a tool step's result is appended to it. Every declared context must be used by a step. All contexts are created when the run starts, before the first step, so an unknown agent or an invalid chroot fails the run before any work is done. This also happens for contexts whose steps all end up skipped.
- **`chroot.path`** is resolved against the global `context.chroot` and must stay inside it. `create` creates it if it is missing, `delete` removes it when the flow ends, whether it succeeded or not. Without `chroot` the global chroot is used. Chroot paths are rendered when the run starts, so they only see `Arguments`, `Ctx` and `Flow`.

#### Arguments

`spec.arguments` declares the only input of a flow: `<name>: { type, pattern, required }`. It is optional; a flow without it takes no arguments, and passing any fails the run. `type` is `string` (default), `number` (64-bit integer) or `float`; strings are trimmed and parsed for `number` and `float`. `pattern` is a regex a `string` argument must match somewhere (anchor it with `^…$` for a full match). `required` defaults to `false`; an optional argument that was not passed is simply absent. Arguments are checked before any step runs: an undeclared, missing required, unconvertible or non-matching argument fails the run. Names must be identifiers, and a `pattern` on a non-string or an invalid regex is rejected when the flow loads.

`triggers.webhook.arguments` maps the webhook payload to arguments, `<argument>: <template>`. These templates are the only ones that see `Body` (the payload) and `Query` (query parameters without `token`). Every key must be a declared argument and every required argument must be mapped. The rendered strings are taken literally, so a missing payload field renders `<NULL>` (use a `pattern` to catch that). If the mapped arguments are invalid, the hook answers `422` and the flow does not start. Unknown hook names and wrong tokens both get `401`.

#### Conditions and loops

- **`when`** on any step is a list of conditions; the step runs if **any** of them is true, and is skipped (logged) otherwise. A condition is `{ left, operator, right }`, `{ all: [...] }` or `{ any: [...] }`, nested freely. `left` and `right` are templates. `==`/`!=` compare strings, `<`, `<=`, `>`, `>=` compare numbers and `~=` matches `left` against the regex `right`. A non-number in a numeric comparison or an invalid regex fails the step. Empty lists and unknown operators are rejected when the flow loads.
- **`loop` steps** take `arguments: { steps, max, until }`. They run `steps` in order, then check `until` (same syntax as `when`), and stop once it is true or after `max` iterations (default `3`, at most `100`), logging a warning in the latter case. Step names must be unique across the whole flow, loops included. `when` on the loop gates the whole loop, `when` on an inner step is checked per iteration.

#### Variables

A **`set` step** writes typed variables, `arguments: { <variable>: { value, type, extract, default } }`. `value` is a template, `type` as for arguments. `extract` is an optional regex applied to the rendered value: capture group 1 if the pattern has a group, the whole match otherwise. If nothing matches or the result doesn't convert to `type`, a warning names the step and variable and `default` is used (`""`, `0` or `0.0` without one). All entries of one step render against the values from before the step. Variables are visible as `Variables.<variable>` right away: to later steps, later loop iterations, `when` and `until`.

```yaml
- name: refine
  type: loop
  arguments:
    max: 3
    until: [{ left: "{{ Variables.findings }}", operator: "==", right: "0" }]
    steps:
      - { name: fix, context: work, type: agent, arguments: { prompt: "Fix the findings in /repo." } }
      - { name: check, context: work, type: agent, arguments: { prompt: "Review /repo. End with 'Findings: <count>'." } }
      - name: count
        type: set
        arguments:
          findings: { value: "{{ Contexts.work.last().content }}", type: number, extract: "Findings: (\\d+)", default: 1 }
```

#### Calling flows

A **`flow` step** runs another flow synchronously, `arguments: { flow, arguments: { <argument>: <template> } }`. The called flow is loaded by name, its arguments are checked like any other run, and it runs with its own contexts, variables and chroot cleanup, sharing `Ctx`. When it finishes, its final variables are available as `Flows.<step>.<variable>`; when it fails, the step fails. Flow steps nest at most 8 levels deep, which also stops recursion. See [`examples/flows/pythagoras.yaml`](examples/flows/pythagoras.yaml), which calls [`calculate.yaml`](examples/flows/calculate.yaml) three times and combines the results.

Called flows share `Ctx.trace_id`: a child chroot like `runs/{{ Ctx.trace_id }}` with `delete: true` deletes the parent's directory of the same path when the child ends. Give child flows distinct chroot paths.

#### Templates

Prompts, chroot paths, condition sides, `set` values, flow step arguments and every string in tool `arguments` are [MiniJinja](https://docs.rs/minijinja) templates (no HTML escaping) with these values:

| Variable | Content |
| --- | --- |
| `Arguments.<argument>` | The typed arguments of the flow |
| `Variables.<variable>` | The variables written by `set` steps so far |
| `Contexts.<context>` | The messages of a context, `[{ type, agent, content, tool_calls, usage }]`, empty until its first step. `Contexts.<context>.last()` is the last one |
| `Flows.<step>.<variable>` | The final variables of the flow called by a `flow` step |
| `Ctx` | The request context (`trace_id`) |
| `Flow` | `{ name }` |

Webhook argument mappings instead see `Body`, `Query`, `Ctx` and `Flow`.

A missing value, at any depth, renders as `<NULL>` and logs a warning naming the path; the flow keeps running. `is defined`, `default(...)` and `{% if %}` still see it as undefined, so `{{ Arguments.title | default("untitled") }}` works as expected. `.last()` on an empty list is undefined too. Syntax errors fail the step.

## Helm Chart

The chart in [`chart/`](chart) deploys the webhook server as a Deployment + Service (with an optional Ingress and HPA). Your `configuration.yaml`, agents, and flows are supplied as Helm values and rendered into Kubernetes Secrets that get mounted into the pod; changing them triggers a rollout.

```sh
helm install beaver ./chart -f my-values.yaml
```

| Key | Description |
| --- | --- |
| `image.repository`, `image.tag` | Image to deploy. |
| `configuration` | Beaver's `configuration.yaml` |
| `agents` | Map of agent name → agent definition, same shape as the [agent definition](#agent-definition) above. |
| `flows` | Map of flow name → flow definition, same shape as the [flow definition](#flow-definition) above. |
| `env` | Extra environment variables for the container (values are templated). |
| `ingress.enabled`, `ingress.host` | Expose `/hooks` externally via an Ingress. |
| `webhook.resources`, `webhook.hpa`, `webhook.securityContext` | Pod resource requests/limits, autoscaling, and security context. |
| `extraObjects` | Escape hatch for arbitrary additional manifests (string or object, templated). |

[`chart/values.example.yaml`](chart/values.example.yaml) is a full working example — Anthropic as the inference provider, an Atlassian MCP server, Bitbucket/git JS tool config, the `reviewer` agent from the [Examples](#examples) above, and a `pr_review_flow` flow that checks out the pull request before reviewing it — copy it and adjust as needed.

## Security
The AI tools out there are powerful but many of them are scary dangerous. With Beaver I took a different attempt restricting what an agent can do by default. 

- **Explicit allow-listing** — an agent can only call tools listed in its own `permissions.tools`; nothing is available by default.
- **Chroot-confined** — the embedded tools resolve every path against a configured `chroot` and reject anything outside it.
- **Scoped JavaScript tools** — each JS tool declares the HTTP methods and URL patterns it's allowed to call in its `package.json` (see [`examples/tools/bitbucket_pr/package.json`](examples/tools/bitbucket_pr/package.json) or [`examples/tools/slack/package.json`](examples/tools/slack/package.json)); calls outside that scope are rejected.

## Architecture

A Rust workspace: `domains` (core logic) → `adapters` (inference, tools, config) → `app` (wiring), exposed via `platforms/cli` and `platforms/webhook`.

## Deployment

Docker images are published to `ghcr.io/pndrik/beaver` on tag push (multi-arch: amd64/arm64). A Helm chart is available in [`chart/`](chart).

## License

[Elastic License 2.0](LICENSE.md). Contributions require signing the [CLA](CLA.md).
