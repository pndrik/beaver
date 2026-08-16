// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

class BitbucketClient {
  constructor({
    username = '',
    apiToken,
    workspace,
    baseUrl = 'https://api.bitbucket.org'
  }) {
    if (!apiToken || typeof apiToken !== 'string') {
      throw new Error('API token is required');
    }
    if (!workspace || typeof workspace !== 'string') {
      throw new Error('Workspace is required');
    }
    if (typeof username !== 'string') {
      throw new Error('Username must be a string');
    }

    this.workspace = workspace;
    this.username = username;
    this.apiToken = apiToken;
    this.baseUrl = baseUrl;
  }

  getAuthHeader() {
    if (this.username.length > 0) {
      return 'Basic ' + btoa(`${this.username}:${this.apiToken}`);
    }
    return `Bearer ${this.apiToken}`;
  }

  async request(path, options = {}) {
    const response = await fetch(`${this.baseUrl}${path}`, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        Authorization: this.getAuthHeader(),
        ...(options.headers || {}),
      },
    });

    const contentType = response.headers.get('content-type') || '';
    const data = contentType.includes('application/json')
      ? await response.json()
      : await response.text();

    if (!(response.status >= 200 && response.status < 300)) {
      throw new Error(
        `Bitbucket API error ${response.status}: ${
          typeof data === 'string' ? data : JSON.stringify(data)
        }`
      );
    }

    return data;
  }

  listPullRequests(repository, state = 'OPEN') {
    if (typeof repository !== 'string' || repository.length === 0) {
      throw new Error('Repository is required');
    }
    if (!['OPEN', 'MERGED', 'DECLINED', 'SUPERSEDED'].includes(state)) {
      throw new Error('Invalid state. Must be one of OPEN, MERGED, DECLINED, SUPERSEDED');
    }

    return this.request(`/2.0/repositories/${this.workspace}/${repository}/pullrequests?state=${state}`);
  }

  getPullRequest(repository, id) {
    if (typeof repository !== 'string' || repository.length === 0) {
      throw new Error('Repository is required');
    }
    if (typeof id !== 'number' || id < 0) {
      throw new Error('Pull request ID is missing or invalid');
    }

    return this.request(`/2.0/repositories/${this.workspace}/${repository}/pullrequests/${id}`);
  }

  async createPullRequest(repository, { title, description = '', sourceBranch, destinationBranch }) {
    if (typeof repository !== 'string' || repository.length === 0) {
      throw new Error('Repository is required');
    }
    if (!title || typeof title !== 'string') {
      throw new Error('Title is required and must be a string');
    }
    if (typeof description !== 'string') {
      throw new Error('Description must be a string');
    }
    if (!sourceBranch || typeof sourceBranch !== 'string') {
      throw new Error('Source branch is required and must be a string');
    }
    if (!destinationBranch || typeof destinationBranch !== 'string') {
      throw new Error('Destination branch is required and must be a string');
    }

    const res = await this.request(`/2.0/repositories/${this.workspace}/${repository}/pullrequests`, {
      method: 'POST',
      body: JSON.stringify({
        title,
        description,
        source: { branch: { name: sourceBranch } },
        destination: { branch: { name: destinationBranch } },
      }),
    });

    if (!res || typeof res.id !== 'number') {
      throw new Error('Did not receive a valid response from Bitbucket API when creating pull request');
    }

    return res;
  }

  showPullRequestsDiff(repository, id) {
    if (typeof repository !== 'string' || repository.length === 0) {
      throw new Error('Repository is required');
    }
    if (typeof id !== 'number' || id < 0) {
      throw new Error('Pull request ID is missing or invalid');
    }

    return this.request(`/2.0/repositories/${this.workspace}/${repository}/pullrequests/${id}/diff`);
  }

  async addPullRequestComment(repository, id, file, line, comment) {
    if (typeof repository !== 'string' || repository.length === 0) {
      throw new Error('Repository is required');
    }
    if (typeof id !== 'number' || id < 0) {
      throw new Error('Pull request ID is missing or invalid');
    }
    if (!file || typeof file !== 'string') {
      throw new Error('File path is required and must be a string');
    }
    if (typeof line !== 'number' || line < 0) {
      throw new Error('Line number is missing or invalid');
    }
    if (!comment || typeof comment !== 'string') {
      throw new Error('Comment is required and must be a string');
    }

    let res = await this.request(`/2.0/repositories/${this.workspace}/${repository}/pullrequests/${id}/comments`, {
      method: 'POST',
      body: JSON.stringify({
        content: { raw: comment },
        inline: { path: file, to: line }
      }),
    });

    if (!res || typeof res.id !== 'number') {
      throw new Error('Did not receive a valid response from Bitbucket API when adding comment');
    }

    return res.id;
  }

  addPullRequestTask(repository, id, comment_id, task) {
    if (!repository) {
      throw new Error('Repository is required');
    }
    if (typeof id !== 'number' || id < 0) {
      throw new Error('Pull request ID is missing or invalid');
    }
    if (!task || typeof task !== 'string') {
      throw new Error('Task is required and must be a string');
    }

    return this.request(`/2.0/repositories/${this.workspace}/${repository}/pullrequests/${id}/tasks`, {
      method: 'POST',
      body: JSON.stringify({
        content: { raw: task },
        comment: { id: comment_id }
      })
    });
  }

  showAllPullRequestComments(repository, id) {
    if (typeof repository !== 'string' || repository.length === 0) {
      throw new Error('Repository is required');
    }
    if (typeof id !== 'number' || id < 0) {
      throw new Error('Pull request ID is missing or invalid');
    }

    return this.request(`/2.0/repositories/${this.workspace}/${repository}/pullrequests/${id}/comments`);
  }
}



export default BitbucketClient;
