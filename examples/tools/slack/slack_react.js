// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

async function main(input, configuration, tool_permissions) {
  let client = getClient(configuration);

  let emoji = await client.addReaction(input.channel, input.timestamp, input.name);

  return `Reaction :${emoji}: added successfully`;
}

export { main };
