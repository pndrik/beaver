// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

async function main(input, configuration, skill_permissions) {
  let client = getClient(configuration, input.workspace);
  let pullRequests = await client.listPullRequests(input.repository, input.state);

  return JSON.stringify(pullRequests);
}

export { main };
