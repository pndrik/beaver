# Agents

An agent is a model, a system prompt and a set of tools it may call. You define it in one YAML file, and the CLI, flows and other agents (as subagents) can all use it. Each agent is a file `<agents.path>/<name>.yaml`, and `metadata.name` must be exactly `<name>` and names are case-sensitive.

## Reference

| Key | Default | Description |
|---|---|---|
| `version` | required | Must be `1`. |
| `metadata.name` | required | The agent's name, equal to the file name without `.yaml`. |
| `metadata.display_name` | required | Name shown in CLI replies (`[<display_name>]: …`) and by `subagent_list`. |
| `metadata.description` | required | Shown by `agents list` and `subagent_list`. Tells a parent agent what this agent is for. |
| `spec.inference.model` | required | Model id, e.g. `claude-sonnet-5-5`. See [inference providers](configuration.md#inference-providers-and-models). |
| `spec.inference.caching.default` | `none` | Prompt cache level for all messages: `none`, `5min`, `1h` or `24h`. Required when `caching` is set. |
| `spec.inference.caching.system` | `caching.default` | Cache level for the system prompt and the tool definitions. |
| `spec.inference.options.max_tokens` | `4096` | Maximum output tokens per reply. For models that use a fixed thinking budget (older Claude and Qwen models), the budget is added if `max_tokens` is not larger than it. |
| `spec.inference.options.temperature` | `-1` | Sampling temperature. A negative value uses the provider default. Ignored by models that reject sampling parameters (Claude Opus 4.7 and newer, Sonnet 5 and newer, Haiku 5.5, Fable). |
| `spec.inference.options.nucleus` | `-1` | Nucleus sampling (`top_p`). Negative uses the provider default. Ignored by the same models as `temperature`. |
| `spec.inference.options.reasoning_effort` | `medium` | `low`, `medium`, `high` or `max`. Models without effort support get a thinking budget of 1024, 8000 or 24000 tokens instead. Ignored by models without reasoning. |
| `spec.inference.options.stop_sequences` | `[]` | Strings that end a reply. Not sent to models on the OpenAI Responses API (`gpt-*` and most `grok-*`). |
| `spec.inference.compression.enabled` | `false` | Summarize the conversation when it gets long. See [Compression](#compression). |
| `spec.inference.compression.model` | `spec.inference.model` | Model of the summarizer. |
| `spec.inference.compression.prompt` | built-in | System prompt of the summarizer. |
| `spec.inference.compression.tokens` | — | Compress once the context reaches this many tokens. |
| `spec.inference.compression.messages` | — | Compress once the context has this many messages. |
| `spec.tools.allow` | `[]` | Rules for the tools the agent may call. Nothing is allowed by default. |
| `spec.tools.allow[].name` | required | Tool name: a literal or a `^...$` regex. See [Tools](#tools). |
| `spec.tools.allow[].scopes` | `[]` | Scopes passed to the tool. Their meaning depends on the tool. |
| `spec.tools.deny` | `[]` | Rules for tools the agent may never call, even if an allow rule matches. |
| `spec.tools.deny[].name` | required | Tool name: a literal or a `^...$` regex. |
| `spec.tools.redirect_output` | `false` | Offer the `harness_redirect_to_file` argument. See [output redirection](tools.md#output-redirection). |
| `spec.prompt` | required | The system prompt. Must not be blank. Beaver puts a short description of how tool results, subagents and summaries appear in front of it. |

## Validation

The whole file is checked when it's loaded, and an invalid agent is never started. `agents list` fails if any agent is invalid.

- Unknown keys (including the old flat layout and `tools.permissions`), missing required keys, wrong types and unknown model ids fail to parse with `Failed to parse agent file <path>: …`.
- A file whose `metadata.name` differs from its file name fails with `Agent name mismatch: expected 'coder', found 'Coder'`.
- Everything else fails with `agent_invalid`, naming the location: a `version` other than `1`, an empty name, a blank prompt, an empty rule name, a pattern without the closing `$`, an invalid regex, a literal containing regex syntax like `git_.*`, or an empty scope.

```text
Agent 'coder' is invalid: spec.tools.deny[1]: name '^git_' must end with '$'
```

## Tools

An agent can call nothing until you allow it. You list the tools it may call under `spec.tools.allow` and, optionally, the ones it must never call under `spec.tools.deny`:

```yaml
spec:
  tools:
    allow:
      - name: file_read            # exactly this tool
      - name: ^git_.*$             # every tool whose name starts with git_
        scopes: [/repo]            # ... but only on /repo
    deny:
      - name: git_push             # never, even though ^git_.*$ matches
```

With these rules the agent can call `file_read` anywhere in its chroot and every git tool on `/repo` except `git_push`. It can't call `file_write`, because no allow rule matches it.

### Names: literals and patterns

A rule `name` is either a literal or a pattern:

| Rule | Matches | Doesn't match |
|---|---|---|
| `file_read` | `file_read` | `file_reader`, `my_file_read` |
| `^file_.*$` | `file_read`, `file_write`, `file_find` | `profile_read` |
| `^file_(read\|find)$` | `file_read`, `file_find` | `file_write` |
| `^git_.*\|evil$` | `git_clone`, `evil` | `git` |

A literal matches only the tool with exactly that name, and it must not contain regex characters (`$*+?()[]{}|\^`): `git_.*` without `^` and `$` is rejected. A pattern starts with `^` and ends with `$` and has to match the whole tool name.

### Which rule wins

A tool is offered and can be called if no deny rule matches it and an allow rule does. Deny always wins. If several allow rules match, the first one in file order gives the call its `scopes`:

```yaml
spec:
  tools:
    allow:
      - name: file_write
        scopes: [/out]             # file_write may only write below /out
      - name: ^file_.*$            # all other file tools work on the whole chroot
```

Swapped around, `^file_.*$` would match `file_write` first, and `file_write` could write anywhere. A call to a denied or unlisted tool fails with `tool_permission_denied` and logs a `call_denied` audit warning; the agent sees the error as the tool result.

### Tool names per provider

Tool names depend on the provider ([tool names](tools.md#tool-names)), so you can allow a whole package or MCP server with one pattern:

```yaml
spec:
  tools:
    allow:
      - name: javascript_run       # embedded tools keep their name
      - name: js_slack_search      # one tool of the JavaScript package "slack"
      - name: ^js_bitbucket_pr_.*$ # every tool of the package "bitbucket_pr"
      - name: ^mcp_atlassian_.*$   # every tool of the MCP server "atlassian"
    deny:
      - name: ^mcp_atlassian_delete.*$
```

- Embedded tools: their own name, e.g. `file_read`, `git_clone`, `javascript_run`.
- JavaScript tools: `js_<tool>`, e.g. `js_slack_search`.
- MCP tools: `mcp_<server>_<tool>`, e.g. `mcp_atlassian_getJiraIssue`.

Run `cargo run -p cli -- tools list` to see the exact names.

### Scopes

`scopes` mean something different for each kind of tool ([embedded tools](tools.md#embedded-tools)):

```yaml
spec:
  tools:
    allow:
      - name: ^file_.*$
        scopes: [/docs, /out]                       # path prefixes in the chroot
      - name: javascript_run
        scopes: ['web:^https://api\.github\.com/']  # URLs fetch may call
      - name: ^subagent_.*$
        scopes: [_list, _invoke, researcher]        # see Subagents below
      - name: js_slack_post
        scopes: [channel:C0123]                     # passed to the tool as permissions.scopes
```

Without `scopes`, file and git tools work on the whole chroot, `javascript_run` has no `fetch`, and subagent tools can do nothing.

## Compression

Long conversations fill the model's context window. With compression, Beaver replaces the older part of the conversation with a summary once it gets too long:

```yaml
spec:
  inference:
    model: claude-sonnet-5-5
    compression:
      enabled: true
      tokens: 100000     # compress once the context reaches 100k tokens
      messages: 200      # ... or 200 messages, whichever comes first
```

Beaver checks before every inference call:

- `tokens` is compared with the context size of the agent's latest reply, its input plus output tokens.
- `messages` is compared with the number of messages since the latest summary.

Set at least one of them; with neither, nothing is compressed. A conversation with fewer than two messages is never compressed.

By default the agent's own model writes the summary with a built-in prompt. To save cost or control what's kept, set your own model and prompt:

```yaml
spec:
  inference:
    model: claude-opus-5-5
    compression:
      enabled: true
      tokens: 80000
      model: claude-haiku-5-5   # a cheaper model writes the summary
      prompt: |
        Summarize the work so far as bullet points: the goal, every decision,
        the files changed and the open next steps. Reply with the summary only.
```

There is no separate compression agent file. The summarizer is the same agent without tools and without compression, with `compression.model` and `compression.prompt` if they are set.

What happens to the conversation, with `messages: 6`:

```text
before                          after
[system]  You are …             [system]  You are …
[user]    Fix the tests         [user]    Fix the tests          ┐
[agent]   calls file_read       [agent]   calls file_read        │ kept, but no longer
[tool]    …                     [tool]    …                      │ sent to the model
[agent]   calls file_write      [agent]   calls file_write       │
[tool]    …                     [tool]    …                      ┘
[user]    Now run them          [summary] - Goal: fix the tests …
                                [user]    Now run them
```

The summarizer gets everything since the latest summary except the newest message. The summary is inserted before the newest message, and from then on only the summary and what follows it are sent to the model. The earlier messages stay in the conversation, so `/dump` in the CLI and `Contexts` in flows still show them. If summarizing fails, the inference call fails.

## Subagents

An agent can hand work to other agents. Allow it the subagent tools and name the agents it may talk to in the scopes:

```yaml
# in agents/lead.yaml
spec:
  tools:
    allow:
      - name: ^subagent_.*$
        scopes: [_list, _invoke, researcher, writer]
```

| Scope | Allows |
|---|---|
| `_list` | Calling `subagent_list`, which shows the agents named in the scopes with name, display name and description. |
| `_invoke` | Calling `subagent_invoke` at all. |
| `researcher`, `writer` | Which agents `subagent_list` shows and `subagent_invoke` may start. |

A conversation with a subagent goes like this:

```text
[user]        What changed in our API last month?
[Lea]         calls subagent_list
[tool]        Available subagents: Name: researcher …
[Lea]         calls subagent_invoke(name: researcher, prompt: "List the API changes in /docs since …")
[Remy]        The changelog lists three changes: …        ← the subagent's reply
[Lea]         Which of them break existing clients?     ← goes to the subagent, not the user
[Remy]        Only the second one: …
[Lea]         calls subagent_leave
[Lea]         Three things changed last month; one of them breaks clients: …
```

While a subagent conversation is open, the parent talks to the subagent instead of its user, and `subagent_list` and `subagent_invoke` are replaced by `subagent_leave`. Each subagent reply goes to the parent, and each parent reply goes back to the subagent, for up to 50 rounds, until the parent calls `subagent_leave`.

The subagent shares the parent's chroot but has its own model, prompt and tool rules. It can only use what its own file allows:

```yaml
# in agents/researcher.yaml
spec:
  tools:
    allow:
      - name: file_read
        scopes: [/docs]   # the researcher only reads /docs, whatever lead may do
```

## Output redirection

With `spec.tools.redirect_output: true` an agent can write any provider tool's output to a file instead of the conversation. See [output redirection](tools.md#output-redirection).

## Examples

### A simple agent

```yaml
# agents/assistant.yaml
version: 1
metadata:
  name: assistant
  display_name: Ada
  description: Answers questions in short sentences.
spec:
  inference:
    model: claude-sonnet-5-5
  prompt: |
    You answer questions in short, plain sentences. Say so if you don't know the answer.
```

### An agent with tools

```yaml
# agents/maintainer.yaml
version: 1
metadata:
  name: maintainer
  display_name: Max
  description: Works on the repository in /repo and reports to Jira.
spec:
  inference:
    model: claude-opus-5-5
    options:
      max_tokens: 16000
      reasoning_effort: high
  tools:
    allow:
      # Read and write only below /repo.
      - name: ^file_.*$
        scopes: [/repo]
      # Every git tool, but see the deny rule below.
      - name: ^git_.*$
        scopes: [/repo]
      # One JavaScript tool, by its literal name.
      - name: js_slack_post
      # Every tool of the MCP server "atlassian".
      - name: ^mcp_atlassian_.*$
      # Scripts may call the GitHub API.
      - name: javascript_run
        scopes: ['web:^https://api\.github\.com/']
    deny:
      - name: git_push
      - name: git_branch_delete
  prompt: |
    You maintain the repository checked out in /repo. Commit your changes, but never push.
    Report what you did in the Jira issue you were given and post a summary to Slack.
```

The deny rules win over `^git_.*$`, so the agent can commit but not push or delete branches.

### Compression with the built-in summarizer

```yaml
# agents/archivist.yaml
version: 1
metadata:
  name: archivist
  display_name: Archie
  description: Works through large document collections.
spec:
  inference:
    model: claude-sonnet-5-5
    compression:
      enabled: true
      tokens: 100000
  tools:
    allow:
      - name: file_find
      - name: file_read
  prompt: |
    You read the documents below /docs and answer questions about them.
```

### Compression with a custom summarizer

```yaml
# agents/investigator.yaml
version: 1
metadata:
  name: investigator
  display_name: Ivy
  description: Investigates incidents from log files.
spec:
  inference:
    model: claude-opus-5-5
    caching:
      system: 1h
      default: 5min
    compression:
      enabled: true
      model: claude-haiku-5-5
      tokens: 80000
      messages: 200
      prompt: |
        Summarize the investigation so far as bullet points: the incident, every finding
        with the file and line it came from, the hypotheses ruled out and the open questions.
        Keep error messages verbatim. Reply with the summary only.
  tools:
    allow:
      - name: ^file_.*$
        scopes: [/logs]
  prompt: |
    You investigate incidents. The logs are below /logs.
```

The summary is written by `claude-haiku-5-5` with the given prompt; the investigation itself keeps using `claude-opus-5-5`.

### Subagents

```yaml
# agents/lead.yaml
version: 1
metadata:
  name: lead
  display_name: Lea
  description: Talks to the user and delegates research to the researcher.
spec:
  inference:
    model: claude-fable-5-1
  tools:
    allow:
      - name: ^subagent_.*$
        scopes: [_list, _invoke, researcher]
  prompt: |
    You talk to the user. Delegate research to a subagent: list the available ones, invoke
    the right one with a self-contained task, and leave the conversation when you have the answer.
```

```yaml
# agents/researcher.yaml
version: 1
metadata:
  name: researcher
  display_name: Remy
  description: Researches questions in the documents below /docs and cites its sources.
spec:
  inference:
    model: claude-sonnet-5-5
  tools:
    allow:
      - name: file_find
        scopes: [/docs]
      - name: file_search
        scopes: [/docs]
      - name: file_read
        scopes: [/docs]
  prompt: |
    You answer the question you are given from the documents below /docs and cite the files you used.
```

`lead` can list and invoke only `researcher`. `researcher` can only read `/docs`, whatever `lead` is allowed.
