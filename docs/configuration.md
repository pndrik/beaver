# Configuration

Beaver reads one configuration tree. It's made from built-in defaults, an optional YAML file, an optional `.env` file and environment variables. This page lists every key, explains how agents find their inference provider, and covers the file system and logging.

## Sources

Beaver merges four sources. When a key is set in more than one, the later one wins:

1. The built-in defaults in [`adapters/src/configuration/defaults.yaml`](../adapters/src/configuration/defaults.yaml).
2. The configuration file. Its path comes from the environment variable `CONFIGURATION__PATH`, default `~/.config/beaver/configuration.yaml`. The file is optional.
3. A `.env` file in the working directory, also optional. It's parsed as INI, so nested keys go in sections: `[logger]` followed by `level=debug`, or `[inference.providers.anthropic]` followed by `apikey=...`. Lines like `LOGGER__LEVEL=debug` in `.env` are not read as nested keys.
4. Environment variables. `__` separates the levels, so `LOGGER__LEVEL=debug` sets `logger.level` and `INFERENCE__PROVIDERS__ANTHROPIC__APIKEY` sets `inference.providers.anthropic.apikey`. Empty variables are ignored. Lists such as `tools.providers.mcp.servers` can't be set this way; put them in the file.

The sources are read again whenever Beaver looks up a value. Strings, numbers and booleans are then cached for `configuration.cache.ttl` seconds. Some things are set up once at start and need a restart to change: the file system, the logger, the listen address, the enabled providers, and the tools of the JavaScript and MCP providers. Agent and flow files are read from disk on every use. See [agents](agents.md) and [flows](flows.md).

## Reference

### `file_system`

| Key | Default | Description |
|---|---|---|
| `file_system.provider` | `disk` | `disk` stores files under `file_system.disk.root`. `memory` keeps them in memory: they're lost on exit, and git tools don't work. See [File system](#file-system). |
| `file_system.disk.root` | `/tmp` | The host directory that holds all chroots. It must exist, otherwise Beaver doesn't start. |

### `logger`

| Key | Default | Description |
|---|---|---|
| `logger.level` | `info` | `trace`, `debug`, `info`, `warn`, `error` or `off`. Applies to Beaver and its dependencies. Environment variable `LOGGER__LEVEL`. |
| `logger.provider` | `stdout` | Where logs go. `stdout` is the only provider. |

### `configuration`

| Key | Default | Description |
|---|---|---|
| `configuration.cache.ttl` | `60` | How many seconds strings, numbers and booleans are cached. Must not be negative. |

### `webhooks`

| Key | Default | Description |
|---|---|---|
| `webhooks.port` | `8080` | Port of the webhook server. |
| `webhooks.bind` | `0.0.0.0` | Address the webhook server binds to. |

### `flows`

| Key | Default | Description |
|---|---|---|
| `flows.provider` | `file` | Where flow definitions come from. `file` is the only provider. |
| `flows.path` | `""` | Directory with one `<name>.yaml` per flow. When empty, there are no flows and every hook answers `401`. |

### `agents`

| Key | Default | Description |
|---|---|---|
| `agents.provider` | `file` | Where agent definitions come from. `file` is the only provider. |
| `agents.path` | `""` | Directory with one `<name>.yaml` per agent. It must be set: with the empty default, loading an agent fails with `Failed to list directory`. |

### `inference.providers`

| Key | Default | Description |
|---|---|---|
| `inference.providers.anthropic.enabled` | `false` | Use the Anthropic API. |
| `inference.providers.anthropic.endpoint` | `https://api.anthropic.com/v1/` | Base URL of the Anthropic API. |
| `inference.providers.anthropic.apikey` | `""` | Anthropic API key. Environment variable `INFERENCE__PROVIDERS__ANTHROPIC__APIKEY`. |
| `inference.providers.openai.enabled` | `false` | Use the OpenAI API. |
| `inference.providers.openai.endpoint` | `https://api.openai.com/v1/` | Base URL of the OpenAI API, or of a proxy or OpenAI-compatible gateway. A missing trailing `/` is added. |
| `inference.providers.openai.apikey` | `""` | OpenAI API key. Environment variable `INFERENCE__PROVIDERS__OPENAI__APIKEY`. |
| `inference.providers.zen.enabled` | `false` | Use [OpenCode Zen](https://opencode.ai/zen). |
| `inference.providers.zen.endpoint` | `https://opencode.ai/zen/v1/` | Base URL of the Zen API. |
| `inference.providers.zen.apikey` | `""` | Zen API key. Environment variable `INFERENCE__PROVIDERS__ZEN__APIKEY`. |

### `tools.providers`

| Key | Default | Description |
|---|---|---|
| `tools.providers.embedded.enabled` | `true` | The built-in file, git, subagent and `javascript_run` tools. See [tools](tools.md#embedded-tools). |
| `tools.providers.javascript.enabled` | `false` | JavaScript tool packages. See [tools](tools.md#javascript-tools). |
| `tools.providers.javascript.path` | `""` | Directory with one sub-directory per package. Must be set when the provider is enabled, otherwise Beaver doesn't start. |
| `tools.providers.mcp.enabled` | `false` | Tools of remote MCP servers. See [tools](tools.md#mcp-servers). |
| `tools.providers.mcp.servers` | `[]` | List of MCP servers, one entry per server with the keys below. |
| `servers[].name` | required | Server name. It's part of the tool names: `mcp_<name>_<tool>`. |
| `servers[].url` | required | URL of the server's Streamable HTTP endpoint. |
| `servers[].description` | `""` | Free text. Beaver doesn't use it. |
| `servers[].credentials.basic.username`, `servers[].credentials.basic.password` | — | Sent as HTTP basic authentication. |
| `servers[].credentials.bearer` | — | Sent as `Authorization: Bearer <token>`. Can't be combined with `basic`. |
| `servers[].credentials.headers` | — | Map of extra HTTP headers, e.g. an API key header. |
| `servers[].protocol_version` | negotiated | Pins the MCP protocol version, e.g. `2025-06-18`, and skips the version probe. An unknown version fails the server. |

### `tools.configuration`

| Key | Default | Description |
|---|---|---|
| `tools.configuration.javascript.loop_iteration_limit` | `100000000` | `javascript_run` stops a script after this many loop iterations. |
| `tools.configuration.javascript.recursion_limit` | `512` | `javascript_run` stops a script at this depth of nested calls. |
| `tools.configuration.git.repositories` | `[]` | Repositories the git tools may use, one entry per repository with the keys below. A git tool that takes a `url` fails with `No configuration for repository` unless an entry matches. |
| `repositories[].url` | required | The repository URL, or a regex written `^...$` matching it. The first matching entry is used. |
| `repositories[].email` | required | Commit author email. |
| `repositories[].display_name` | `email` | Commit author name. |
| `repositories[].username` | user in the URL, else `git` | User name for authentication. |
| `repositories[].password` | — | Password or access token for HTTPS. |
| `repositories[].ssh_key` | — | Private key for SSH, the key file content in OpenSSH format. |
| `repositories[].known_hosts` | `[]` | Host keys in `known_hosts` format. SSH connections to a host without a matching key fail. |
| `tools.configuration.js_<package>` | — | Map of strings passed to every tool of the JavaScript package `<package>` as its `configuration`. See [tools](tools.md#javascript-tools). |
| `tools.configuration.js_bitbucket_pr.api_key` | `""` | API key of the example [`bitbucket_pr`](../examples/tools/bitbucket_pr) tools. |
| `tools.configuration.js_bitbucket_pr.base_url` | `https://api.bitbucket.org` | API base URL of the `bitbucket_pr` tools. |
| `tools.configuration.js_slack.api_key` | `""` | Token of the example [`slack`](../examples/tools/slack) tools. |
| `tools.configuration.js_slack.base_url` | `https://slack.com` | API base URL of the `slack` tools. |

## Inference providers and models

An agent names a model in `spec.inference.model`. The id must be one Beaver knows, listed in [`domains/src/inference/models/model.rs`](../domains/src/inference/models/model.rs); any other id fails the agent with `unknown model: <id>`.

On every inference call Beaver asks the enabled providers in a fixed order: Anthropic, OpenAI, then Zen. Each one lists its models through its `/models` endpoint, and the first provider whose list contains the model serves the call. If none does, the call fails with `No inference provider found for model`. A provider that is enabled but can't list its models (wrong key, no network) fails the call, so only enable providers you have a key for.

## File system

Everything agents and flows do with files goes through one file system: file and git tools, `javascript_run`, the `fs` object of JavaScript tools, output redirection and flow chroots. Each conversation only sees its chroot, where `/` is the chroot's root. The CLI and contexts without a `chroot` use the whole file system; flows can narrow it per context (see [contexts](flows.md#contexts)).

The `disk` provider can't leave `file_system.disk.root`. Paths with `..` are rejected, and so are symlinks that lead outside, absolute ones included. Symlinks it creates are stored as relative links. The `memory` provider is meant for tests.

Agent and flow definitions, JavaScript tool packages and the configuration are read from their own paths on the host, not through this file system.

## Logging

Beaver writes one JSON object per line to stdout. Data is in fields. Everything logged while handling a request, a CLI command or the start carries the context's attributes under `ctx`:

```json
{"channel":"flow","ctx":{"trace_id":"5b0c…"},"flow":"review","level":"INFO","message":"Flow started","timestamp":"2026-10-31T10:00:00.123456Z"}
{"channel":"access log","client_ip":"10.0.0.7","ctx":{"trace_id":"5b0c…"},"latency_ms":6.52,"level":"INFO","method":"POST","path":"/hooks/review","response_size":48,"status":200,"timestamp":"2026-10-31T10:00:00.130000Z","user_agent":"curl/8.4","version":"HTTP/1.1"}
```

The trace id is a new UUID per webhook request, `boot` while starting, and `cli` for the CLI. Tool calls and denied calls are logged with `"channel":"audit"`.

Tokens, `Authorization` headers, query strings and request bodies are never logged. Prompts and agent outputs are only logged at `debug`, variable values and the console output of JavaScript tools only at `trace`.

## Examples

Each example is a complete `configuration.yaml`. API keys can stay out of the file and come from environment variables instead, e.g. `INFERENCE__PROVIDERS__ANTHROPIC__APIKEY`.

### Anthropic (Claude)

```yaml
# configuration.yaml
inference:
  providers:
    anthropic:
      enabled: true
      apikey: "<your_anthropic_api_key>"
```

### OpenAI

```yaml
# configuration.yaml
inference:
  providers:
    openai:
      enabled: true
      apikey: "<your_openai_api_key>"
```

GPT models such as `gpt-5.5` use the Responses API, other models Chat Completions.

### OpenCode Zen

```yaml
# configuration.yaml
inference:
  providers:
    zen:
      enabled: true
      apikey: "<your_zen_api_key>"
```

When several enabled providers list a model, the first of `anthropic`, `openai`, `zen` is used: the direct APIs before the aggregator.

### MCP servers

```yaml
# configuration.yaml
tools:
  providers:
    mcp:
      enabled: true
      servers:
        - name: atlassian
          url: "https://mcp.atlassian.com/v1/mcp/preview"
          credentials:
            basic:
              username: "<username>"
              password: "<api_token>"
        - name: tracker
          url: "https://mcp.example.com/mcp"
          protocol_version: "2025-06-18"
          credentials:
            bearer: "<your_token>"
            headers:
              X-Tenant: "beaver"
```

The tools appear as `mcp_atlassian_<tool>` and `mcp_tracker_<tool>`.

### JavaScript tools with per-package configuration

```yaml
# configuration.yaml
tools:
  providers:
    javascript:
      enabled: true
      path: /etc/beaver/tools
  configuration:
    js_slack:
      api_key: "<your_slack_token>"
    js_weather:
      api_key: "<your_weather_api_key>"
      units: "metric"
```

The package `weather` in `/etc/beaver/tools/weather/` gets `{ "api_key": "…", "units": "metric" }` as its `configuration`.

### Git repositories with an SSH key

```yaml
# configuration.yaml
tools:
  configuration:
    git:
      repositories:
        - url: "^ssh://git@bitbucket\\.org/example/.*$"
          email: "beaver@example.com"
          display_name: "Beaver Bot"
          ssh_key: |
            -----BEGIN OPENSSH PRIVATE KEY-----
            <your_ssh_private_key>
            -----END OPENSSH PRIVATE KEY-----
          known_hosts:
            - "bitbucket.org ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIazEu89wgQZ4bqs3d63QSMzYVa0MuJ2e2gKTKqu+UUO"
        - url: "https://github.com/example/docs.git"
          email: "beaver@example.com"
          username: "beaver-bot"
          password: "<your_access_token>"
```

### Local development

Run from the repository root, with the examples as agents, flows and tools:

```yaml
# configuration.yaml
file_system:
  disk:
    root: /tmp/beaver
logger:
  level: debug
agents:
  path: ./examples/agents
flows:
  path: ./examples/flows
tools:
  providers:
    javascript:
      enabled: true
      path: ./examples/tools
inference:
  providers:
    anthropic:
      enabled: true
```

```sh
mkdir -p /tmp/beaver
export INFERENCE__PROVIDERS__ANTHROPIC__APIKEY="<your_anthropic_api_key>"
CONFIGURATION__PATH=./configuration.yaml cargo run -p cli -- agents list
```

Relative paths are resolved against the working directory.
