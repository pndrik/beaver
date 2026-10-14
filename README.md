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
metadata:
  name: pr_review_flow
  description: Clone and review a PR
spec:
  triggers:
    webhook: { name: pr_review_flow, token: "REPLACE_ME" }
  steps:
    - name: clone
      context:
        name: work
        chroot: { path: "runs/{{ Ctx.trace_id }}", create_if_missing: true, delete: true }
      action:
        type: tool
        arguments:
          tool: git_clone
          scopes: ["*"]
          arguments: { url: "{{ Body.repository.links.html.href }}", path: "/repo" }
    - name: review
      context: { name: work }
      action:
        type: agent
        arguments:
          agent: coder
          prompt: "Review the PR in /repo. Clone said: {{ Steps.clone.output }}"
```

- **Steps** run in order; the first failing step stops the flow. Step names must be identifiers (`^[A-Za-z_][A-Za-z0-9_]*$`) and unique.
- **`agent` steps** send the rendered `prompt` to `agent`. **`tool` steps** call `tool` directly with the rendered `arguments` and the given `scopes`, without an LLM.
- **Contexts** are a conversation plus a working directory. Steps with the same `context.name` share both, so a later agent step continues the earlier conversation and a tool step's result is appended to it. Every context needs exactly one agent: at least one agent step, and all of its agent steps use the same agent. Tool steps run in that agent's conversation and chroot.
- **`chroot.path`** is resolved against the global `context.chroot` and must stay inside it. `create_if_missing` creates it, `delete` removes it when the flow ends, whether it succeeded or not. Without `chroot` the global chroot is used, and only the first step of a context decides its chroot.
- Unknown names and wrong tokens both get `401`.

Prompts, chroot paths and every string in tool `arguments` are [MiniJinja](https://docs.rs/minijinja) templates (no HTML escaping) with these values:

| Variable | Content |
| --- | --- |
| `Body` | The webhook payload |
| `Query` | Query parameters, without `token` |
| `Ctx` | The request context (`trace_id`) |
| `Flow` | `{ name }` |
| `Steps.<step>` | `{ output, context, chroot, messages: [{ type, agent, content }] }` for every step that already ran |

A missing value, at any depth, renders as `<NULL>` and logs a warning naming the path; the flow keeps running. `is defined`, `default(...)` and `{% if %}` still see it as undefined, so `{{ Body.title | default("untitled") }}` works as expected. Syntax errors fail the step.

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
- **Scoped JavaScript tools** — each JS tool declares the HTTP methods and URL patterns it's allowed to call in its `package.json` (see [`examples/tools/bitbucket_pr/package.json`](examples/tools/bitbucket_pr/package.json)); calls outside that scope are rejected.

## Architecture

A Rust workspace: `domains` (core logic) → `adapters` (inference, tools, config) → `app` (wiring), exposed via `platforms/cli` and `platforms/webhook`.

## Deployment

Docker images are published to `ghcr.io/pndrik/beaver` on tag push (multi-arch: amd64/arm64). A Helm chart is available in [`chart/`](chart).

## License

[Elastic License 2.0](LICENSE.md). Contributions require signing the [CLA](CLA.md).
