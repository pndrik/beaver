# Flows

A flow is a fixed pipeline of steps: agent prompts, direct tool calls, loops, variables and calls to other flows. Use a flow for a repeatable automation where you know the order of the work, like "clone the repository, then review it". Use a single agent when the agent should decide what to do next.

Flows are triggered through the webhook server (`/hooks/{name}`) or called from other flows. Each flow is the file `<flows.path>/<name>.yaml`, and `metadata.name` must match `<name>`. Flows are read again on every trigger, so adding, editing or removing a file needs no restart.

## Reference

Names of arguments, contexts, steps and variables must be identifiers (`^[A-Za-z_][A-Za-z0-9_]*$`). Unknown keys are rejected, except directly on a step, where they are ignored.

| Key | Default | Description |
|---|---|---|
| `version` | required | Must be `1`. |
| `metadata.name` | required | The flow's name, equal to the file name without `.yaml`. |
| `metadata.display_name` | — | Display name. |
| `metadata.description` | `""` | Description. |
| `spec.arguments.<argument>` | `{}` | Declares an input of the flow. See [Arguments](#arguments). |
| `spec.arguments.<argument>.type` | `string` | `string`, `number` (64-bit integer) or `float`. |
| `spec.arguments.<argument>.regex` | — | The value must match this regex somewhere. Only for `string`. |
| `spec.arguments.<argument>.required` | `false` | Whether the argument must be passed. |
| `spec.contexts.<context>` | `{}` | Declares a conversation with a working directory. See [Contexts](#contexts). |
| `spec.contexts.<context>.agent` | required | Name of the agent of this context. |
| `spec.contexts.<context>.chroot` | whole file system | The context's working directory. |
| `spec.contexts.<context>.chroot.path` | required | Path in the [file system](configuration.md#file-system), a template. |
| `spec.contexts.<context>.chroot.create` | `false` | Create the directory, with missing parents, if it doesn't exist. |
| `spec.contexts.<context>.chroot.delete` | `false` | Delete the directory when the flow ends. |
| `spec.triggers.webhook` | — | Makes the flow callable through `/hooks/{name}`. See [Webhook](#webhook). |
| `spec.triggers.webhook.name` | required | The `{name}` in `/hooks/{name}`. |
| `spec.triggers.webhook.token` | required | The token callers must send. |
| `spec.triggers.webhook.arguments` | `{}` | `<argument>: <template>`, maps the request to the flow's arguments. |
| `spec.steps` | required | The steps, run in order. At least one. |
| `steps[].name` | — | Optional identifier, unique across the flow, loops included. Needed only to read a `flow` step's results as `Flows.<name>`. Logs and errors name unnamed steps by position, e.g. `steps[2]` or `steps[1].steps[0]` inside a loop. |
| `steps[].type` | required | `agent`, `tool`, `loop`, `set` or `flow`. |
| `steps[].context` | — | The context the step works in. Required for `agent` and `tool` steps, not allowed for the others. |
| `steps[].when` | — | List of conditions. The step runs only if at least one is true. See [Flow control](#flow-control). |
| `steps[].arguments` | required | Depends on `type`, see the rows below. |
| `agent`: `arguments.prompt` | required | Template sent to the context's agent as the user message. |
| `tool`: `arguments.tool` | required | Name of the tool to call directly, without an LLM. |
| `tool`: `arguments.arguments` | `{}` | The tool's arguments. Every string in it, at any depth, is a template. |
| `tool`: `arguments.scopes` | `[]` | Scopes for the call, as in an [allow rule](agents.md#tools). |
| `loop`: `arguments.steps` | required | The steps of one iteration. At least one. |
| `loop`: `arguments.max` | `3` | Maximum number of iterations, `1` to `100`. |
| `loop`: `arguments.until` | — | List of conditions, checked after each iteration. The loop stops once one is true. |
| `set`: `arguments.<variable>.value` | required | Template for the value. See [Variables](#variables). |
| `set`: `arguments.<variable>.type` | `string` | `string`, `number` or `float`. |
| `set`: `arguments.<variable>.extract` | — | Regex applied to the rendered value. |
| `set`: `arguments.<variable>.default` | `""`, `0` or `0.0` | Value used if extraction or conversion fails. |
| `flow`: `arguments.flow` | required | Name of the flow to call. See [Calling another flow](#calling-another-flow). |
| `flow`: `arguments.arguments` | `{}` | `<argument>: <template>` for the called flow. |
| Condition `{ left, operator, right }` | — | Compares two templates with `==`, `!=`, `<`, `<=`, `>`, `>=` or `~=`. |
| Condition `{ all: [...] }` | — | True if every condition in the list is true. |
| Condition `{ any: [...] }` | — | True if at least one condition in the list is true. |

## Steps

Steps run in order, and the first failing step stops the flow. Every step has the same shape:

```yaml
- name: <step_name>          # optional identifier, unique in the flow
  type: <type>               # agent, tool, loop, set or flow
  context: <context_name>    # only for agent and tool steps
  when:                      # optional, the step runs if any condition is true
    - { left: "<template>", operator: "==", right: "<template>" }
  arguments:                 # depends on the type, see the table below
    <key>: <value>
```

| Type | Does | `context` | `arguments` |
|---|---|---|---|
| `agent` | Sends a prompt to the context's agent and lets it work. | required | `prompt` |
| `tool` | Calls one tool directly, without an LLM. | required | `tool`, `arguments`, `scopes` |
| `loop` | Repeats its inner steps until a condition is true. | not allowed | `steps`, `max`, `until` |
| `set` | Writes variables. | not allowed | `<variable>: { value, type, extract, default }` |
| `flow` | Runs another flow and waits for it. | not allowed | `flow`, `arguments` |

One step of each type:

```yaml
- type: agent
  context: work
  arguments:
    prompt: "Review the code in /repo and list the problems you find."

- type: tool
  context: work
  arguments:
    tool: git_clone
    scopes: [/repo]
    arguments: { url: "{{ Arguments.url }}", path: "/repo" }

- type: loop
  arguments:
    max: 3
    until: [{ left: "{{ Variables.status }}", operator: "==", right: "ok" }]
    steps:
      - { name: attempt, type: agent, context: work, arguments: { prompt: "Try again. End with 'Status: ok' or 'Status: failed'." } }
      - { name: status_check, type: set, arguments: { status: { value: "{{ Contexts.work.last().content }}", extract: "Status: (\\w+)" } } }

- type: set
  arguments:
    summary: { value: "{{ Contexts.work.last().content }}" }

- name: square
  type: flow
  arguments:
    flow: calculate
    arguments: { instruction: "Square the number {{ Arguments.a }}" }
```

- An **`agent` step** renders `prompt`, sends it to the context's agent and lets the agent work until it replies without calling a tool. A failing tool call doesn't fail the step; the agent sees the error and continues. A failing inference does.
- A **`tool` step** calls `tool` with the rendered `arguments` and the given `scopes`, without an LLM. The flow author grants the scopes: the context agent's allow rules are not checked. The result is added to the context's conversation as a tool message, so a later `agent` step on the same context sees it. A failing tool fails the step.
- A **`loop` step** runs its steps repeatedly, see [Flow control](#flow-control).
- A **`set` step** writes variables, see [Variables](#variables).
- A **`flow` step** runs another flow, see [Calling another flow](#calling-another-flow).

## Examples

A flow with one agent step, triggered by a webhook:

```yaml
# flows/answer.yaml
version: 1
metadata:
  name: answer
  description: Answers a question.
spec:
  arguments:
    question: { required: true }
  contexts:
    main:
      agent: assistant
  triggers:
    webhook:
      name: answer
      token: "<your_flow_token>"
      arguments:
        question: "{{ Body.question }}"
  steps:
    - name: answer
      context: main
      type: agent
      arguments:
        prompt: "{{ Arguments.question }}"
```

A flow that clones a repository into a temporary directory and reviews it. `git_clone` needs a matching entry in [`tools.configuration.git.repositories`](configuration.md#toolsconfiguration), and the agent `reviewer` needs the `file_*` tools to read `/repo`. The `clone` step doesn't need them: tool steps aren't checked against the agent's rules.

```yaml
# flows/pr_review.yaml
version: 1
metadata:
  name: pr_review
  description: Clones the repository of a pull request and reviews it.
spec:
  arguments:
    url: { required: true, regex: "^https://" }
    title: {}
  contexts:
    work:
      agent: reviewer
      chroot: { path: "runs/{{ Ctx.trace_id }}", create: true, delete: true }
  triggers:
    webhook:
      name: pr_review
      token: "<your_flow_token>"
      arguments:
        url: "{{ Body.repository.links.html.href }}"
        title: "{{ Body.pullrequest.title }}"
  steps:
    - name: clone
      context: work
      type: tool
      arguments:
        tool: git_clone
        arguments: { url: "{{ Arguments.url }}", path: "/repo" }
    - name: review
      context: work
      type: agent
      arguments:
        prompt: "Review the pull request '{{ Arguments.title | default('untitled') }}' in /repo. Clone said: {{ Contexts.work.last().content }}"
```

More examples are in [`examples/flows/`](../examples/flows).

## Calling a flow

### Webhook

A flow with `triggers.webhook` answers on `/hooks/{name}` of the webhook server (`cargo run -p webhook`, port [`webhooks.port`](configuration.md#webhooks)). Any HTTP method works. The caller sends the token as `Authorization: Bearer <token>` or as the query parameter `?token=<token>`; the header wins if both are set.

```sh
curl -X POST \
  -H 'Authorization: Bearer <your_flow_token>' \
  -H 'Content-Type: application/json' \
  -d '{"question": "What is a beaver?"}' \
  localhost:8080/hooks/answer
```

The body is parsed by its `Content-Type`. `application/json` is parsed as JSON, and `application/x-www-form-urlencoded` as a form, where `a[b]=c` becomes `{ "a": { "b": "c" } }`. Any other body is ignored. A body that doesn't parse is answered with `400`.

`triggers.webhook.arguments` maps the request to the flow's arguments, `<argument>: <template>`. These templates are the only ones that see `Body` (the parsed body) and `Query` (the query parameters without `token`), besides `Ctx` and `Flow`. Every key must be a declared argument, and every required argument must be mapped. The rendered strings are taken literally, so a missing payload field renders as `<NULL>`; use a `regex` to catch that.

| Status | Body | When |
|---|---|---|
| `200` | `{"success":true,"message":"Inference started"}` | The arguments are valid and the flow started. |
| `400` | Error text | The body doesn't parse. |
| `401` | `{"success":false,"message":"Missing token"}` | No token was sent. |
| `401` | `{"success":false,"message":"Invalid token"}` | No flow has this hook name, or the token is wrong. |
| `422` | `{"success":false,"message":"Invalid arguments"}` | The mapped arguments are invalid. The flow doesn't start. |
| `500` | `{"success":false,"message":"Internal error"}` | The flows couldn't be read. |

The flow runs in the background after the `200`. Its result is not returned; a failure is only logged (`Hook failed`). Invalid flow files are skipped with a warning when a hook is resolved. Use each hook name only once: when two flows share one, only one of them is found.

### Calling another flow

A `flow` step runs another flow and waits for it, `arguments: { flow, arguments: { <argument>: <template> } }`. The called flow is loaded by name and doesn't need a webhook trigger. Its arguments are rendered as strings and checked like any other run. It runs with its own contexts, variables and chroot cleanup, and shares `Ctx` with the caller. When it finishes, its final variables are available as `Flows.<name>.<variable>`, where `<name>` is the step's name; an unnamed `flow` step stores nothing. When it fails, the step fails.

Flow steps nest at most 8 levels deep, which also stops endless recursion. See [`examples/flows/pythagoras.yaml`](../examples/flows/pythagoras.yaml), which calls [`calculate.yaml`](../examples/flows/calculate.yaml) three times and combines the results.

Called flows share `Ctx.trace_id`. A child chroot like `runs/{{ Ctx.trace_id }}` with `delete: true` deletes the parent's directory of the same path when the child ends. Give child flows distinct chroot paths.

## Arguments

`spec.arguments` declares the only input of a flow, `<argument>: { type, regex, required }`. It's optional; a flow without it takes no arguments, and passing any fails the run.

- `type` is `string` (default), `number` (64-bit integer) or `float`. Strings are trimmed and parsed for `number` and `float`.
- A `string` argument must match its `regex` somewhere in the value. Anchor it with `^…$` to match the whole value.
- `required` defaults to `false`. An optional argument that isn't passed is absent: `Arguments.<argument>` is undefined.

Arguments are checked before any step runs, and every problem is reported at once: an undeclared, missing required, unconvertible or non-matching argument fails the run with `Flow '<name>' got invalid arguments: …`. Through the webhook that's a `422`. When the flow loads, invalid names, a `regex` on a non-string argument and invalid regexes are rejected.

## Contexts

A context is a conversation with one agent plus a working directory, its chroot. Contexts are declared once under `spec.contexts`. Steps on the same context share both: a later `agent` step continues the earlier conversation, and a `tool` step's result is added to it. Every declared context must be used by at least one step.

All contexts are created when the run starts, before the first step. An unknown agent or an invalid chroot fails the run before any work is done, even for contexts whose steps end up skipped.

- **`chroot.path`** is a path in the [file system](configuration.md#file-system), relative (`runs/x`) or absolute (`/runs/x`), both resolved against its root. It must stay inside the root, also through symlinks. The path is rendered when the run starts, so it can only use `Arguments`, `Ctx` and `Flow`.
- **`create`** creates the directory with missing parents. Without it the directory must exist.
- **`delete`** removes the directory when the flow ends, whether it succeeded or not. Parent directories made by `create`, like `runs/`, are kept. The root itself can't be deleted.
- Without `chroot` the whole file system is the chroot.

## Variables

A `set` step writes typed variables, `arguments: { <variable>: { value, type, extract, default } }`.

- `value` is a template. `type` works as for arguments.
- `extract` is a regex applied to the rendered value. The result is capture group 1 if the pattern has a group, the whole match otherwise.
- If nothing matches or the result doesn't convert to `type`, a warning names the step and variable, and `default` is used: `""`, `0` or `0.0` without one.

All entries of one step are rendered against the values from before the step. Variables are visible as `Variables.<variable>` right away: to later steps, later loop iterations, `when` and `until`.

```yaml
- type: set
  arguments:
    findings: { value: "{{ Contexts.work.last().content }}", type: number, extract: "Findings: (\\d+)", default: 1 }
```

## Flow control

**`when`** on any step is a list of conditions. The step runs if any of them is true and is skipped otherwise (logged at `debug`). A condition is `{ left, operator, right }`, `{ all: [...] }` or `{ any: [...] }`, nested freely. `left` and `right` are templates.

| Operator | Compares |
|---|---|
| `==`, `!=` | The rendered strings. |
| `<`, `<=`, `>`, `>=` | Both sides as numbers. A side that isn't a number fails the step. |
| `~=` | Whether `left` matches the regex `right`. An invalid regex fails the step. |

Empty condition lists and unknown operators are rejected when the flow loads.

A **`loop` step** takes `arguments: { steps, max, until }`. It runs `steps` in order, then checks `until`, and stops once a condition is true or after `max` iterations. Stopping at `max` logs a warning. `when` on the loop gates the whole loop; `when` on an inner step is checked in every iteration.

```yaml
- type: loop
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

## Templates

Prompts, chroot paths, both sides of conditions, `set` values, `flow` step arguments and every string in `tool` arguments are [MiniJinja](https://docs.rs/minijinja) templates, without HTML escaping. They see these values:

| Value | Content |
|---|---|
| `Arguments.<argument>` | The typed arguments of the flow. |
| `Variables.<variable>` | The variables written by `set` steps so far. |
| `Contexts.<context>` | The messages of a context, `[{ type, agent, content, tool_calls, usage }]`, empty until its first step. `type` is `user`, `assistant`, `tool` or `summary`; `agent` is the agent's name on its own messages. |
| `Contexts.<context>.last()` | The last message of a context. |
| `Flows.<name>.<variable>` | The final variables of the flow called by the `flow` step named `<name>`. Unnamed `flow` steps aren't stored. |
| `Ctx` | The request context, `{ trace_id }`. |
| `Flow` | `{ name }` of the running flow. |

Webhook argument mappings see `Body`, `Query`, `Ctx` and `Flow` instead.

A missing value, at any depth, renders as `<NULL>` and logs a warning naming the path; the flow keeps running. `is defined`, `default(...)` and `{% if %}` still treat it as undefined, so `{{ Arguments.title | default("untitled") }}` works as expected. `.last()` on an empty list is undefined too. A template syntax error fails the step.
