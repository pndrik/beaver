// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient, sanitizeMessage } from "./lib/shared.js";

async function main(input, configuration, tool_permissions) {
  let client = getClient(configuration);

  let result = input.thread_ts
    ? await client.replies(input.channel, input.thread_ts, input.limit ?? 50)
    : await client.history(input.channel, {
      limit: input.limit ?? 20,
      oldest: input.oldest,
      latest: input.latest,
    });
  if (!Array.isArray(result.messages)) {
    throw new Error('Did not receive a valid response from Slack API when reading messages');
  }

  return JSON.stringify(result.messages.map(sanitizeMessage));
}

export { main };
