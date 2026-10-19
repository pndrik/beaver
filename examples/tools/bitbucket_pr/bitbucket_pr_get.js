// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

async function main(input, configuration, tool_permissions) {
  let client = getClient(configuration, input.workspace);

  let pr = await client.getPullRequest(input.repository, input.id);
  if (typeof pr.title !== 'string' || typeof pr.description !== 'string') {
    throw new Error('Did not receive a valid response from Bitbucket API when fetching pull request');
  }

  let comments = await client.showAllPullRequestComments(input.repository, input.id);
  if (!Array.isArray(comments)) {
    throw new Error('Did not receive a valid response from Bitbucket API when fetching pull request comments');
  }

  let comments_sanitized = comments.map(c => {
    let comment = {
      id: c.id,
      content: c.content.raw,
      user: c.user.display_name,
    };

    if (c.parent && typeof c.parent.id === 'number') {
      comment.parent = c.parent.id;
    }

    if (c.inline && c.path && c.inline.to) {
      comment.line = c.inline.to;
      comment.file = c.path;
    }

    return comment;
  });

  return JSON.stringify(comments_sanitized);
}

export { main };
