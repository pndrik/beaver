// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import SlackClient from './slack_client.js';

function getClient(configuration) {
  return new SlackClient({
    apiToken: configuration.api_key || "",
    baseUrl: configuration.base_url || 'https://slack.com',
  });
}

function sanitizeMessage(m) {
  return {
    ts: m.ts,
    thread_ts: m.thread_ts,
    user: m.user || m.username || m.bot_id,
    text: m.text,
    reply_count: m.reply_count,
    reactions: (m.reactions || []).map((r) => ({ name: r.name, count: r.count })),
  };
}

export { getClient, sanitizeMessage };
