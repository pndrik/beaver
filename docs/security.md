# Security

Many AI tools give an agent broad access by default. Beaver does the opposite: an agent can do nothing until you allow it, and what it's allowed stays confined to its own directory. This page describes the security model layer by layer.

## Tools

An agent can only call tools that match one of its `spec.tools.allow` rules and none of its `spec.tools.deny` rules. Nothing is allowed by default. The rules are validated when the agent loads, and an invalid agent never starts. A denied call fails with `tool_permission_denied` and logs a `call_denied` audit warning; every permitted call is logged as an audit event too. Subagents are checked against their own rules, not their parent's. See [agent tools](agents.md#tools).

Flow `tool` steps are the exception: they call a tool directly with the scopes written in the flow, without checking the context agent's rules. Whoever can change a flow file decides what its tool steps may do.

## Files

All file access goes through one [file system](configuration.md#file-system), which can't leave `file_system.disk.root`. Each conversation is narrowed further to its chroot:

- Paths are resolved through symlinks and rejected if they lead outside the chroot. Paths with `..` are rejected.
- Deleting a symlink removes the link, not its target, and the chroot itself can't be deleted.
- A path that leads outside fails with the same error as a missing one, `does not exist or is not accessible`, so errors never tell whether something outside exists.
- File and git tools can be narrowed further with path scopes, which are checked on the resolved path as well.

Agent and flow definitions, JavaScript tools and the configuration are read from their own paths, not through the file system. Keep `agents.path`, `flows.path`, `tools.providers.javascript.path` and the configuration file outside `file_system.disk.root`, so agents can't read or change them.

## Git

The git tools only open repositories whose git directory, worktree and objects are inside the repository directory:

- No `gitdir:` files or `core.worktree` pointing elsewhere, no alternates, no `commondir`, and no config includes.
- System, global and XDG git configuration is ignored.
- Symlinks are checked out as regular files (`core.symlinks=false`).
- Only `https://`, `ssh://` and `user@host:path` URLs are accepted, and only for repositories listed in [`tools.configuration.git.repositories`](configuration.md#toolsconfiguration).
- SSH connections are only made to hosts whose key is listed in the repository's `known_hosts`.

## Scripts

[`javascript_run`](tools.md#javascript_run) runs in Boa, a JavaScript engine without Node.js APIs. Scripts only read, write and import files inside the chroot, can't import packages, and have no `fetch` unless the allow rule has `web:<regex>` scopes. Redirects are checked against the same patterns. Loops and recursion are limited, but there is no timeout.

## JavaScript tools

Each [JavaScript tool](tools.md#javascript-tools) declares in its `package.json` which URLs it may fetch and which paths it may read or write. Everything else is rejected, and without scopes a tool has neither `fetch` nor `fs`. File access is always confined to the calling agent's chroot. Tools only import modules from their own package directory. See [`examples/tools/bitbucket_pr/package.json`](../examples/tools/bitbucket_pr/package.json) and [`examples/tools/slack/package.json`](../examples/tools/slack/package.json).

## Webhooks

Every hook needs its flow's token, sent as `Authorization: Bearer <token>` or `?token=`. Tokens are compared in constant time. A wrong token and an unknown hook name get the same `401`, so callers can't probe for hook names. Use long random tokens, and prefer the header: query strings end up in proxy logs. See [webhook](flows.md#webhook).

## Logging

Tokens, `Authorization` headers, query strings and request bodies are never logged. Prompts and agent outputs are only logged at `debug`, variable values and JavaScript tool output only at `trace`. Keep production at `info` or above if prompts contain sensitive data. See [logging](configuration.md#logging).
