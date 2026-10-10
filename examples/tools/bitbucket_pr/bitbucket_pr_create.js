// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

async function main(input, configuration, tool_permissions) {
  let client = getClient(configuration, input.workspace);

  let defaultReviewers = await client.listEffectiveDefaultReviewers(input.repository);
  let author = await client.getCurrentUser();
  let reviewers = [...new Set(defaultReviewers.map((user) => user.uuid))]
    .filter((uuid) => uuid !== author.uuid);

  let pullRequest = await client.createPullRequest(input.repository, {
    title: input.title,
    description: input.description || '',
    sourceBranch: input.source_branch,
    destinationBranch: input.destination_branch,
    reviewers: reviewers,
  });

  return JSON.stringify(pullRequest);
}

export { main };
