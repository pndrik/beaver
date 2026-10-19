// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

async function main(input, configuration, tool_permissions) {
  let client = getClient(configuration);

  let result = await client.searchMessages(input.query, {
    count: input.count ?? 20,
    sort: input.sort ?? 'timestamp',
    sortDir: input.sort_dir ?? 'desc',
  });
  if (!result.messages || !Array.isArray(result.messages.matches)) {
    throw new Error('Did not receive a valid response from Slack API when searching messages');
  }

  let matches = result.messages.matches.map(m => ({
    channel: m.channel && m.channel.id,
    channel_name: m.channel && m.channel.name,
    ts: m.ts,
    thread_ts: m.thread_ts,
    user: m.user || m.username,
    text: m.text,
    permalink: m.permalink,
  }));

  return JSON.stringify(matches);
}

export { main };
