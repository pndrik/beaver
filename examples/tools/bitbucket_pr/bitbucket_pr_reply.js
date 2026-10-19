// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

async function main(input, configuration, tool_permissions) {
  let client = getClient(configuration, input.workspace);

  let commentId = await client.replyToPullRequestComment(input.repository, input.id, input.comment_id, input.comment);

  return JSON.stringify({ id: commentId });
}

export { main };
