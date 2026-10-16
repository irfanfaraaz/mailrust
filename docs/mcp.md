# Use Mailrust from your AI

Mailrust has an MCP endpoint. MCP (Model Context Protocol) is the standard way for AI apps to find and use an app's actions. With it, an AI app can do everything the dashboard can: send email, manage contacts and tags, write and send campaigns, and read reports.

## What you need

- Your engine's public URL, for example `https://mail.example.com`. The endpoint is `https://mail.example.com/mcp`.
- A full-access API key (`mr_live_...`). Make one in the dashboard under API keys. A sending key works too, but it only gets the email tools.

## Claude Code

```bash
claude mcp add --transport http mailrust https://mail.example.com/mcp --header "Authorization: Bearer mr_live_..."
```

## Claude Desktop

Settings → Developer → Edit Config, then add:

```json
{
  "mcpServers": {
    "mailrust": {
      "type": "http",
      "url": "https://mail.example.com/mcp",
      "headers": { "Authorization": "Bearer mr_live_..." }
    }
  }
}
```

## Cursor

In `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "mailrust": {
      "url": "https://mail.example.com/mcp",
      "headers": { "Authorization": "Bearer mr_live_..." }
    }
  }
}
```

## Safety

- The AI can do everything the key can do. Treat the key like a password.
- Actions that cannot be undone (send a campaign, delete, cancel) need `confirm: true`. Without it, they change nothing and return a preview, for example how many readers a campaign will reach. These tools are marked destructive, so your AI app asks you before it runs them.
- There are no tools for API keys, team members or login. A person does those in the dashboard.

## Tools

Emails, contacts and imports, tags, campaigns, newsletter settings, sending domains, webhooks, suppressions and the account overview. Your AI app shows the full list with descriptions.
