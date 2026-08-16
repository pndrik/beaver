// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

async function main(input, configuration, tool_permissions) {
  let client = getClient(configuration, input.workspace);

  let commentId = await client.addPullRequestComment(input.repository, input.id, input.file, input.line, input.comment);
  if (typeof input.task === 'string' && input.task.length > 0) {
    await client.addPullRequestTask(input.repository, input.id, commentId, input.task);
  }

  return 'Comment added successfully';
}

export { main };
