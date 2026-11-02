# Tools

Tools are what agents use to act: read files, clone repositories, run scripts, call APIs. They come from three providers, each enabled in the [configuration](configuration.md#toolsproviders): embedded tools built into Beaver, JavaScript tool packages, and remote MCP servers.

An agent only sees the tools its `spec.tools.allow` rules match, see [agent tools](agents.md#tools). Run `cargo run -p cli -- tools list` to see every loaded tool.

## Tool names

| Provider | Name | Example |
|---|---|---|
| Embedded | The tool's own name | `file_read`, `git_clone`, `javascript_run`, `subagent_invoke` |
| JavaScript | `js_` + the tool's key in `package.json` | `js_slack_search` |
| MCP | `mcp_` + server name + `_` + the server's tool name | `mcp_atlassian_getJiraIssue` |

A rule `^js_slack_.*$` allows every tool of the `slack` package as long as its tool keys start with `slack_`. `^mcp_atlassian_.*$` allows every tool of the server `atlassian`.

## Embedded tools

Paths are absolute paths in the conversation's chroot, where `/` is its root. The file and git tools use the `scopes` of the allow rule as path prefixes: with `scopes: [/repo]` the tool only works on `/repo` and everything below it. A scope covers whole path components, so `/repo` doesn't cover `/repository`. Paths are checked as given and again after resolving symlinks. Without scopes the whole chroot is allowed. Scopes must be absolute paths; anything else, like `*`, matches nothing.

| Tool | Description | Scopes |
|---|---|---|
| `file_read` | Reads a file, optionally from line `start` to line `end`. | Path prefixes |
| `file_write` | Writes `content` to a file, replacing it, appending (`append`), or replacing `length` lines from line `start`. Line edits need a current `file_read` of the file. | Path prefixes |
| `file_delete` | Deletes a file or, recursively, a directory. A symlink is removed, not its target. | Path prefixes |
| `file_mkdir` | Creates a directory with missing parents. | Path prefixes |
| `file_find` | Finds files and directories below `path`: name regex `pattern`, `file_type`, `sort_by` `name`/`size`/`modified`, `descending`, `recursive` (default `true`), `max_results` (default `1000`), `exclude` (default `[".git"]`). | Path prefixes |
| `file_search` | Searches files up to 10 MB for lines matching a regex `pattern`, with `recursive`, `max_results` (default `1000`) and `exclude`. | Path prefixes |
| `git_clone` | Clones `url` into `path`, which must not exist yet. | Path prefixes |
| `git_pull` | Fetches and fast-forwards the current branch. | Path prefixes |
| `git_checkout` | Checks out `branch`, creating it from a remote-tracking branch or `HEAD` if needed. | Path prefixes |
| `git_commit` | Stages all changes and commits them with `message`. | Path prefixes |
| `git_push` | Pushes the current branch and sets its upstream. | Path prefixes |
| `git_diff` | Unstaged changes, or the diff between revisions `from` and `to`. At most 5000 lines. | Path prefixes |
| `git_log` | Commits newest first, from `revision` (default `HEAD`), optionally for one `file`, with `skip` and `limit` (default `20`, at most `200`). | Path prefixes |
| `git_show` | One commit with message, changed files and diff, optionally for one `file`. At most 1000 lines. | Path prefixes |
| `git_branches` | Local branches, optionally with remote-tracking ones (`include_remote`). | Path prefixes |
| `git_branch_delete` | Deletes a local branch and, with `delete_remote`, the one on the remote. | Path prefixes |
| `javascript_run` | Runs JavaScript on chroot files, see [below](#javascript_run). | `web:<regex>` |
| `subagent_list` | Lists the agents this agent may invoke. | `_list` and agent names |
| `subagent_invoke` | Starts a conversation with another agent. | `_invoke` and agent names |

`git_clone`, `git_pull`, `git_commit`, `git_push`, and `git_branch_delete` with `delete_remote` take the repository's `url` as an argument. It has to be `https://`, `ssh://` or `user@host:path`, and it needs a matching entry in [`tools.configuration.git.repositories`](configuration.md#toolsconfiguration), which provides the credentials and the commit author. Git needs the `disk` file system. See [security](security.md#git) for how repositories are contained.

`subagent_leave` is not listed: it's offered to an agent while it talks to a subagent. See [subagents](agents.md#subagents).

## `javascript_run`

`javascript_run` runs JavaScript the agent writes, so it can filter and aggregate data in code instead of reading it into the conversation. It runs in [Boa](https://boajs.dev), an engine without Node.js APIs. Pass exactly one of:

- `code`: runs as the body of an async function, so `await` and `return` work at the top level. Modules load with `await import("/lib/x.js")`.
- `path`: runs a `.js` file of the chroot as an ES module, so static `import` works. The default export is the result; a function is called and awaited. Boa can't parse `export default async () => …`, use `export default async function () { … }`.

The result is every `console.*` call as a line, followed by the value: strings as they are, nothing for `undefined`, JSON otherwise. A throw or rejection fails the tool with the console output so far. Scripts see the chroot as `/`:

| Global | Description |
|---|---|
| `fs.readFile(path)` | Content of a file as a string. |
| `fs.readDir(path)` | Entry names, directories end with `/`. Symlinks are not followed. |
| `fs.writeFile(path, content)`, `fs.appendFile(path, content)` | Replace or append to a file. The parent directory must exist. |
| `fetch(url, options)` | Only defined if the allow rule has `web:` scopes. |

Imports only resolve to files in the chroot. Absolute specifiers start at its root, relative ones at the importing module, and packages like `"lodash"` can't be imported. Paths that escape the chroot, through `..` or symlinks, throw.

Web access is off by default. Each `web:<regex>` scope of the allow rule allows any method on URLs the regex matches. The regex is used as written, so anchor it yourself. Redirects are checked against the same regexes. Other scopes are ignored, and an invalid regex fails the tool.

```yaml
spec:
  tools:
    allow:
      - name: javascript_run
        scopes: ['web:^https://api\.github\.com/']
```

A script stops after [`tools.configuration.javascript.loop_iteration_limit`](configuration.md#toolsconfiguration) loop iterations or at `recursion_limit` nested calls. There is no timeout: a script blocked in a single long operation, like a slow `fetch`, runs until it finishes. See [`examples/agents/analyst.yaml`](../examples/agents/analyst.yaml).

## JavaScript tools

A JavaScript tool package is a directory below [`tools.providers.javascript.path`](configuration.md#toolsproviders) with a `package.json`. Its `config.beaver.tools` declares the tools. The packages are loaded at start; a directory without a valid `package.json` is skipped.

```text
tools/
└── weather/
    ├── package.json
    ├── weather_current.js
    └── lib/
        └── client.js
```

```json
{
  "name": "weather",
  "version": "0.0.1",
  "config": {
    "beaver": {
      "tools": {
        "weather_current": {
          "main": "weather_current.js",
          "description": "Returns the current weather for a city.",
          "scopes": {
            "fetch": [
              { "url": "^https://api\\.weather\\.example/v1/current\\?.*$", "methods": ["GET"] }
            ]
          },
          "parameters": {
            "type": "object",
            "required": ["city"],
            "properties": {
              "city": { "type": "string", "description": "The city name." }
            }
          }
        }
      }
    }
  }
}
```

| Key | Default | Description |
|---|---|---|
| `name` | required | Package name. Its configuration is `tools.configuration.js_<name>`. |
| `version` | required | Package version. |
| `config.beaver.tools.<tool>` | required | One entry per tool. The agent sees it as `js_<tool>`. Tool keys must be unique across all packages. |
| `<tool>.main` | required | The module that exports `main`, relative to the package directory. |
| `<tool>.description` | required | Description shown to the model. |
| `<tool>.parameters` | required | JSON schema of the tool's input. Calls with invalid arguments fail before the tool runs. |
| `<tool>.scopes.fetch[]` | `[]` | `{ url, methods }`: URLs the tool may fetch. Without `methods` any method is allowed. |
| `<tool>.scopes.filesystem[]` | `[]` | `{ path, operations }`: paths in the agent's chroot the tool may `read` and/or `write` with `fs`. |

```json
"scopes": {
  "fetch": [
    { "url": "^https://api\\.example\\.com/.*$", "methods": ["GET", "POST"] }
  ],
  "filesystem": [
    { "path": "/reports", "operations": ["read", "write"] }
  ]
}
```

A `url` or `path` starting with `^` is a regex, otherwise a literal. A literal path also covers everything below it. Without `fetch` scopes the tool has no `fetch`, and without `filesystem` scopes it has no `fs`. `fs` offers the same functions as in [`javascript_run`](#javascript_run), with paths in the calling agent's chroot. `javascript_run` itself may read and write the whole chroot.

The module exports a function `main(input, configuration, permissions)` that returns a string, which becomes the tool result. Any other return value fails the call.

```js
// weather/weather_current.js
async function main(input, configuration, permissions) {
  const url = `https://api.weather.example/v1/current?city=${encodeURIComponent(input.city)}&units=${configuration.units}`;
  const response = await fetch(url, { headers: { Authorization: `Bearer ${configuration.api_key}` } });
  if (!response.ok) {
    throw new Error(`Weather API answered ${response.status}`);
  }
  return JSON.stringify(await response.json());
}

export { main };
```

- `input` is the call's arguments.
- `configuration` is the map `tools.configuration.js_<package name>`, with string values. It's empty if the key doesn't exist.
- `permissions` is the allow rule that permitted the call, `{ name, scopes }`. The tool can use the scopes for its own checks.

A tool can only import modules from its own package directory. `console.*` output is logged at `trace`. There are no loop or recursion limits for package tools. See [`examples/tools/`](../examples/tools) for two complete packages, `bitbucket_pr` and `slack`.

## MCP servers

Beaver connects to MCP servers over Streamable HTTP. Configure them under [`tools.providers.mcp.servers`](configuration.md#toolsproviders), with optional basic, bearer or header credentials ([example](configuration.md#mcp-servers)).

At start Beaver connects to each server and loads its tool list. It first offers protocol version `2026-07-28` through `server/discover` and falls back to `initialize` with the newest older version the server supports. `protocol_version` pins one version and skips the probe. A server that can't be reached or fails is skipped with a warning, and its tools are missing until the next restart. Every tool call opens its own session.

The tool result is the text content of the server's answer, followed by its structured content as JSON. Other content types, like images, are dropped.

## Output redirection

Tool outputs land in the conversation and use up the context window. With `spec.tools.redirect_output: true` in an agent, every provider tool it's offered (embedded, JavaScript and MCP) gets an optional `harness_redirect_to_file` argument. The full output is written to that absolute path in the chroot, replacing the file, and the tool message only shows the name, all arguments and a receipt:

```text
Output written to '/out/prs.json' (48213 bytes, 912 lines)
```

The parent directory must exist, and the path must not contain `..` or lead outside the chroot through a symlink. A failing tool writes nothing. A tool argument of the same name is replaced. Subagent tools are not redirected, and flow `tool` steps drop the argument and write nothing. Without `redirect_output`, using the argument fails.
