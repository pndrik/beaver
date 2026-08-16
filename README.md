# Beaver

>  ⚠️ Experimental: This project is not production-ready, use at your own risk.

I built Beaver as I was in need of a solution, allowing me to securely and flexibly define server side AI assisted workflows automating some of the repeated work I do. Many of the existing solutions I found were either client side only, highly insecure, expensive proprietary SaaS or AI spaghetti code.

At the time Beaver allows you to define AI agents in YAML, which can be either triggered via CLI or a webhook. The focus is on autonomous agents that can call tools all on their own but without jeopardizing security in any way. There is a handful of embedded which can be extended with JavaScript or external MCP servers. The agents can use multiple LLM providers (currently Anthropic or OpenCode Zen as desired).

## Features

- **Multi-Provider/-LLM** — Anthropic, Zen (more to come)
- **Pluggable Tools** — JavaScript (sandboxed) and MCP servers
- **Agents & Webhooks as YAML** — no custom DSL, just YAML
- **Ships as a container** — Helm chart included

## Quick start

Here's a few commands to play with Beaver. It's however recommended to install Beaver using the Helm Chart in [`chart/`](chart) and configure it with your own agents, webhooks, and tools.

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

### Webhook definition

```yaml
metadata:
  name: pr_review
  display_name: PR Review
  token: "REPLACE_WITH_YOUR_WEBHOOK_TOKEN"
handler:
  agent: reviewer
  prompt: |
    A new pull request was opened: {{ Body.pullrequest.id }}
```

`POST` the webhook payload to `/hooks/pr_review?token=<TOKEN>` and the reviewer agent runs.

## Helm Chart

The chart in [`chart/`](chart) deploys the webhook server as a Deployment + Service (with an optional Ingress and HPA). Your `configuration.yaml`, agents, and webhooks are supplied as Helm values and rendered into Kubernetes Secrets that get mounted into the pod; changing them triggers a rollout.

```sh
helm install beaver ./chart -f my-values.yaml
```

| Key | Description |
| --- | --- |
| `image.repository`, `image.tag` | Image to deploy. |
| `configuration` | Beaver's `configuration.yaml` |
| `agents` | Map of agent name → agent definition, same shape as the [agent definition](#agent-definition) above. |
| `webhooks` | Map of webhook name → webhook definition, same shape as the [webhook definition](#webhook-definition) above. |
| `env` | Extra environment variables for the container (values are templated). |
| `ingress.enabled`, `ingress.host` | Expose `/hooks` externally via an Ingress. |
| `webhook.resources`, `webhook.hpa`, `webhook.securityContext` | Pod resource requests/limits, autoscaling, and security context. |
| `extraObjects` | Escape hatch for arbitrary additional manifests (string or object, templated). |

[`chart/values.example.yaml`](chart/values.example.yaml) is a full working example — Anthropic as the inference provider, an Atlassian MCP server, Bitbucket/git JS tool config, and the `reviewer` agent + `pr_review` webhook from the [Examples](#examples) above — copy it and adjust as needed.

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
