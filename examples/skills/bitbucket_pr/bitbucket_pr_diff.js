// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

async function main(input, configuration, skill_permissions) {
  let client = getClient(configuration, input.workspace);

  let diff = await client.showPullRequestsDiff(input.repository, input.id);
  if (typeof diff !== 'string') {
    throw new Error('Diff is not a string');
  }
  if (diff.split('\n').length > 1000) {
    throw new Error('Diff is too large to process');
  }

  return diff;
}

export { main };
