// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

async function main(input, configuration, tool_permissions) {
  let client = getClient(configuration, input.workspace);

  await client.updatePullRequest(input.repository, input.id, {
    title: input.title,
    description: input.description,
  });

  return 'Pull request updated successfully';
}

export { main };
