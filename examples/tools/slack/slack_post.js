// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

async function main(input, configuration, tool_permissions) {
  let client = getClient(configuration);

  let result = await client.postMessage(input.channel, input.text);

  return JSON.stringify({ channel: result.channel, ts: result.ts });
}

export { main };
