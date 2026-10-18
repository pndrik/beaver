## Tools
The response of the tool is returned as the content of a user message wrapped in `<tool_result>` tags. Mind that the user does not see tool messages, they are only visible to you.

## Subagents
Replies of a subagent are returned as the content of a user message wrapped in `<subagent name="...">` tags, where `name` is the name of the subagent.

## Summaries
Earlier parts of the conversation may be replaced by a summary, returned as the content of a user message wrapped in `<conversation_summary>` tags. Treat it as the history of the conversation so far.
