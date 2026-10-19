// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

class SlackClient {
  constructor({
    apiToken,
    baseUrl = 'https://slack.com'
  }) {
    if (!apiToken || typeof apiToken !== 'string') {
      throw new Error('API token is required');
    }

    this.apiToken = apiToken;
    this.baseUrl = baseUrl;
  }

  async request(method, httpMethod = 'GET', params = {}) {
    const defined = Object.entries(params).filter(([, value]) => value !== undefined && value !== null);
    let url = `${this.baseUrl}/api/${method}`;
    const options = {
      method: httpMethod,
      headers: { Authorization: `Bearer ${this.apiToken}` },
    };

    if (httpMethod === 'GET') {
      url += '?' + defined
        .map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(String(value))}`)
        .join('&');
    } else {
      options.headers['Content-Type'] = 'application/json; charset=utf-8';
      options.body = JSON.stringify(Object.fromEntries(defined));
    }

    const response = await fetch(url, options);
    const contentType = response.headers.get('content-type') || '';
    const data = contentType.includes('application/json')
      ? await response.json()
      : await response.text();

    if (!(response.status >= 200 && response.status < 300) || typeof data !== 'object' || !data.ok) {
      const error = typeof data === 'object' ? data.error || 'unknown_error' : data;
      throw new Error(`Slack API error ${response.status}: ${error}`);
    }

    return data;
  }

  searchMessages(query, { count = 20, sort = 'timestamp', sortDir = 'desc' } = {}) {
    if (!query || typeof query !== 'string') {
      throw new Error('Query is required and must be a string');
    }
    if (typeof count !== 'number' || count < 1 || count > 100) {
      throw new Error('Count must be a number between 1 and 100');
    }
    if (!['timestamp', 'score'].includes(sort)) {
      throw new Error('Invalid sort. Must be one of timestamp, score');
    }
    if (!['desc', 'asc'].includes(sortDir)) {
      throw new Error('Invalid sort direction. Must be one of desc, asc');
    }

    return this.request('search.messages', 'GET', { query, count, sort, sort_dir: sortDir });
  }

  history(channel, { limit = 20, oldest, latest } = {}) {
    if (!channel || typeof channel !== 'string') {
      throw new Error('Channel is required and must be a string');
    }
    if (typeof limit !== 'number' || limit < 1 || limit > 200) {
      throw new Error('Limit must be a number between 1 and 200');
    }

    return this.request('conversations.history', 'GET', { channel, limit, oldest, latest });
  }

  listConversations({ types = 'public_channel', cursor, limit = 1000 } = {}) {
    if (typeof limit !== 'number' || limit < 1 || limit > 1000) {
      throw new Error('Limit must be a number between 1 and 1000');
    }

    return this.request('conversations.list', 'GET', {
      types,
      cursor,
      limit,
      exclude_archived: true,
    });
  }

  replies(channel, ts, limit = 50) {
    if (!channel || typeof channel !== 'string') {
      throw new Error('Channel is required and must be a string');
    }
    if (!ts || typeof ts !== 'string') {
      throw new Error('Thread timestamp is required and must be a string');
    }
    if (typeof limit !== 'number' || limit < 1 || limit > 200) {
      throw new Error('Limit must be a number between 1 and 200');
    }

    return this.request('conversations.replies', 'GET', { channel, ts, limit });
  }

  postMessage(channel, text, { threadTs, broadcast = false } = {}) {
    if (!channel || typeof channel !== 'string') {
      throw new Error('Channel is required and must be a string');
    }
    if (!text || typeof text !== 'string') {
      throw new Error('Text is required and must be a string');
    }
    if (threadTs !== undefined && (typeof threadTs !== 'string' || threadTs.length === 0)) {
      throw new Error('Thread timestamp must be a non-empty string');
    }

    return this.request('chat.postMessage', 'POST', {
      channel,
      text,
      thread_ts: threadTs,
      reply_broadcast: threadTs && broadcast ? true : undefined,
    });
  }

  async addReaction(channel, timestamp, name) {
    if (!channel || typeof channel !== 'string') {
      throw new Error('Channel is required and must be a string');
    }
    if (!timestamp || typeof timestamp !== 'string') {
      throw new Error('Message timestamp is required and must be a string');
    }
    if (!name || typeof name !== 'string') {
      throw new Error('Emoji name is required and must be a string');
    }

    const emoji = name.trim().replace(/^:+|:+$/g, '');
    if (emoji.length === 0) {
      throw new Error('Emoji name is required and must be a string');
    }

    try {
      await this.request('reactions.add', 'POST', { channel, timestamp, name: emoji });
    } catch (error) {
      if (!String(error.message).endsWith(': already_reacted')) {
        throw error;
      }
    }

    return emoji;
  }
}

export default SlackClient;
