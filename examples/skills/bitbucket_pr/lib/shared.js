// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import BitbucketClient from './bitbucket_client.js';

function getClient(configuration, workspace) {
  return new BitbucketClient({
    username: configuration.username || "",
    apiToken: configuration.api_key || "",
    workspace: workspace,
    baseUrl: configuration.base_url || 'https://api.bitbucket.org',
  });
}

export { getClient };
