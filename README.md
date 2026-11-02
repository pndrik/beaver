# Beaver: self-hosted AI agents and LLM workflow automation

Beaver is a platform for autonomous AI agents and AI workflows. You define agents and multi-step flows in YAML and trigger them from the CLI or through webhooks, e.g. from GitHub, Bitbucket, Jira or Slack. It's written in Rust and runs as a Docker container or on Kubernetes with Helm.

I built Beaver because I needed a secure and flexible way to automate repeated work with AI on the server side. The solutions I found were client-side only, insecure, expensive proprietary SaaS, or AI spaghetti code. Beaver's agents call tools on their own, but only the tools you allow, and only inside their own sandbox.

## Features

- **Multi-Provider/-LLM** — Claude via the Anthropic API, plus GPT, Gemini, Qwen, Kimi, GLM and more via OpenCode Zen
- **Pluggable Tools** — built-in file, git and JavaScript tools, your own sandboxed JavaScript tools, and MCP (Model Context Protocol) servers
- **Agents & Flows as YAML** — no custom DSL, just YAML
- **Webhook triggers** — start an AI workflow from any service that can send an HTTP request
- **Secure by default** — allow-listed tools, chrooted file access, sandboxed scripts
- **Subagents and context compression** — agents delegate to other agents and summarize long conversations
- **Ships as a container** — Docker image (amd64/arm64) and Helm chart included

## Use cases

- Review pull requests automatically: clone the repository, read the diff and comment on the PR.
- Triage Jira issues or answer Slack messages with an agent that uses MCP or JavaScript tools.
- Run multi-step AI pipelines with loops, conditions and variables, e.g. "fix, review, repeat until clean".
- Analyse large files in code instead of the context window.

## Quick start

Create an agent, a flow that uses it, and a configuration in a directory `local/` of the repository.

```yaml
# local/agents/summarizer.yaml
version: 1
metadata:
  name: summarizer
  display_name: Sumi
  description: Summarizes text in three sentences.
spec:
  inference:
    model: claude-sonnet-5-5
  prompt: |
    Summarize the text you get in at most three short sentences.
```

```yaml
# local/flows/summarize.yaml
version: 1
metadata:
  name: summarize
spec:
  arguments:
    text: { required: true }
  contexts:
    main: { agent: summarizer }
  triggers:
    webhook:
      name: summarize
      token: "<your_flow_token>"
      arguments:
        text: "{{ Body.text }}"
  steps:
    - name: summarize
      context: main
      type: agent
      arguments:
        prompt: "{{ Arguments.text }}"
```

```yaml
# local/configuration.yaml
agents:
  path: ./local/agents
flows:
  path: ./local/flows
inference:
  providers:
    anthropic:
      enabled: true
```

Then, from the repository root:

```sh
export CONFIGURATION__PATH=./local/configuration.yaml
export INFERENCE__PROVIDERS__ANTHROPIC__APIKEY="<your_anthropic_api_key>"

# Chat with the agent (end with Ctrl-D)
cargo run -p cli -- conversation summarizer

# Run the webhook server (serves /healthz and /hooks/{name} on :8080),
# logging the conversation at debug level
LOGGER__LEVEL=debug cargo run -p webhook

# Trigger the flow from another terminal; the summary shows up in the server log
curl -H 'Authorization: Bearer <your_flow_token>' -H 'Content-Type: application/json' \
  -d '{"text": "Beavers are large rodents that build dams..."}' localhost:8080/hooks/summarize
```

More agents, flows and tools are in [`examples/`](examples). For a real deployment, install the [Helm chart](docs/helm.md) on your cluster.

## Documentation

- [Configuration](docs/configuration.md) — every setting, inference providers, file system, logging, example configurations
- [CLI](docs/cli.md) — commands, conversations and debugging agents locally
- [Agents](docs/agents.md) — the agent spec, tool rules, compression, subagents
- [Flows](docs/flows.md) — the flow spec, webhooks, arguments, contexts, variables, flow control, templates
- [Tools](docs/tools.md) — embedded tools, `javascript_run`, JavaScript tool packages, MCP servers, output redirection
- [Security](docs/security.md) — the security model
- [Deployment](docs/deployment.md) — Docker image and building from source
- [Helm chart](docs/helm.md) — install Beaver on Kubernetes and manage agents and flows as Helm values

## License

[Elastic License 2.0](LICENSE.md). Contributions require signing the [CLA](CLA.md).
