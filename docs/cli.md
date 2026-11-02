# CLI

The CLI lets you talk to an agent in the terminal and check which agents and tools Beaver loads. It's the quickest way to try and debug an agent before you use it in a flow.

## Running it

From the repository:

```sh
cargo run -p cli -- <command>
```

Or build the binary once and run it directly:

```sh
cargo build --release -p cli
./target/release/cli <command>
```

The Docker image contains the same binary as `/cli`. The CLI reads the same [configuration](configuration.md) as the webhook server, so set `CONFIGURATION__PATH` or environment variables like `AGENTS__PATH` before you run it.

## Commands

| Command | Prints |
|---|---|
| `conversation <agent>` | Starts an interactive conversation with the agent `<agent>`. |
| `agents list` | One line per agent: `<name> (<display_name>) - <description>`. |
| `tools list` | One line per tool of every enabled provider: `<name> - <description>`. These are the names to use in allow rules. |
| anything else | The usage text. |

## Conversation

`conversation <agent>` loads the agent and prompts with `[You]: `. Each line you type is sent to the agent, and its replies are printed as `[<display_name>]: <text>`. While the agent talks to a [subagent](agents.md#subagents), both sides' messages are printed with their display names. Empty lines are ignored. End the conversation with Ctrl-D.

The conversation's chroot is the whole [file system](configuration.md#file-system), so `/` in tool paths is `file_system.disk.root`.

If an inference fails, the CLI prints `Error during inference: <message>` and you can continue. Tool errors don't stop the agent; it sees them as tool results.

Type `/dump` to print the whole conversation so far:

```text
--- Conversation Dump ---
> [You]: How many files are in /docs?
> [Ada] (1520/48 tokens) called: file_find({"path":"/docs"})
> [Tool]: Name: file_find
Arguments: {"path":"/docs"}
Result: …
> [Ada] (1890/12 tokens): There are 12 files in /docs.
--- End of Dump ---
```

Each line shows who wrote the message (`You`, `Tool`, `Summary` or the agent's display name), the agent's token usage as input/output tokens, and the content. Tool calls are shown with their JSON arguments. Messages that compression has replaced by a summary are still in the dump. The system prompt is not.

## Debugging

- **Check that agents load.** `agents list` fails with the parse or validation error of the first invalid agent, see [validation](agents.md#validation). An agent that doesn't load in the CLI won't load in a flow either.
- **Check the tools.** `tools list` shows every tool of every enabled provider. A JavaScript package that doesn't show up has an invalid `package.json`, and a missing MCP server logged a `Failed to start MCP session` warning at start.
- **Raise the log level.** `LOGGER__LEVEL=debug` logs every conversation message, including prompts, tool results and summaries, and the tool calls (`"channel":"audit"`). `LOGGER__LEVEL=trace` adds the console output of JavaScript tools and the negotiated MCP protocol versions. Logs are JSON lines on stdout, mixed with the conversation; `LOGGER__LEVEL=off` hides them.
- **Find your logs.** Everything the CLI logs carries the fixed trace id `cli` in `ctx.trace_id`.
- **Use a local configuration.** Point `CONFIGURATION__PATH` at a file for local work, see the [local development example](configuration.md#local-development). A bootstrap error prints `Failed to bootstrap application: <message>`.
- **Inspect a conversation.** `/dump` shows which tools the agent called, with which arguments, what came back and how many tokens each reply used.
- **Test a flow.** The CLI can't run flows. Start the webhook server locally and trigger the hook with `curl`:

  ```sh
  LOGGER__LEVEL=debug cargo run -p webhook
  curl -H 'Authorization: Bearer <token>' -H 'Content-Type: application/json' \
    -d '{"question": "What is a beaver?"}' localhost:8080/hooks/answer
  ```

  The server answers right away; the flow's progress and errors are in the server's log. Flows with only `tool` steps run without an inference provider, which helps when you test chroots and templates.

## Limitations

- There are no flags or options; arguments are matched by position.
- There is no command to run a flow.
- `tools list` shows all tools, not the ones a given agent is offered.
- A conversation is not saved; it ends with Ctrl-D, there is no exit command.
- An unknown agent, or an error in `agents list` or `tools list`, ends the CLI with a Rust panic message that includes the error.
