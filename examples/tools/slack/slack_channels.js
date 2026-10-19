// Copyright 2026 Patrick Hunziker
// Licensed under the Elastic License 2.0. See LICENSE.md in the project root.

import { getClient } from "./lib/shared.js";

const MAX_PAGES = 20;
const MAX_TOPIC = 100;

// Private channels are only listed if the token is a member.
function isMember(channel) {
  return channel.is_member === true || channel.is_private === true;
}

function matches(channel, query) {
  return [channel.name, channel.name_normalized, channel.topic?.value, channel.purpose?.value]
    .some((value) => typeof value === 'string' && value.toLowerCase().includes(query));
}

function summarize(channel) {
  const topic = channel.topic?.value || channel.purpose?.value || '';
  return {
    id: channel.id,
    name: channel.name,
    is_private: channel.is_private,
    members: channel.num_members,
    topic: topic.length > MAX_TOPIC ? `${topic.slice(0, MAX_TOPIC)}...` : topic,
  };
}

async function main(input, configuration, tool_permissions) {
  const query = String(input.query ?? '').trim().replace(/^#/, '').toLowerCase();
  const limit = input.limit ?? 50;
  if (typeof limit !== 'number' || limit < 1 || limit > 200) {
    throw new Error('Limit must be a number between 1 and 200');
  }

  const client = getClient(configuration);
  const types = input.include_private ? 'public_channel,private_channel' : 'public_channel';

  const found = [];
  let exact = false;
  let cursor;
  for (let page = 0; page < MAX_PAGES; page++) {
    const result = await client.listConversations({ types, cursor });
    if (!Array.isArray(result.channels)) {
      throw new Error('Did not receive a valid response from Slack API when listing channels');
    }

    for (const channel of result.channels) {
      if (isMember(channel) && (query.length === 0 || matches(channel, query))) {
        found.push(channel);
        exact ||= channel.name === query || channel.name_normalized === query;
      }
    }

    cursor = result.response_metadata?.next_cursor;
    // Keep looking for an exact match, it goes first.
    if (!cursor || (found.length >= limit && (query.length === 0 || exact))) {
      break;
    }
  }

  const isExact = (channel) => channel.name === query || channel.name_normalized === query;
  found.sort((a, b) => Number(isExact(b)) - Number(isExact(a)) || a.name.localeCompare(b.name));

  return JSON.stringify({
    channels: found.slice(0, limit).map(summarize),
    more: found.length > limit || Boolean(cursor),
  });
}

export { main };
