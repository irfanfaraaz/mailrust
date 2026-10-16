//! The MCP tool table: one row per dashboard action. Names and descriptions are written for models;
//! the input schemas come from the OpenAPI document (see `schema.rs`).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Changes nothing.
    Read,
    /// Creates or changes something.
    Write,
    /// Cannot be undone: needs `confirm: true`, otherwise it only returns a preview.
    Destructive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    /// The OpenAPI JSON body: object fields become top-level arguments, an array becomes `items`.
    Json,
    /// `import_contacts`: CSV text, sent to the API as a multipart file upload.
    CsvUpload,
}

#[derive(Debug)]
pub struct ToolDef {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// Lower case, as in the OpenAPI document: "get", "post", "patch", "delete".
    pub method: &'static str,
    pub path: &'static str,
    pub kind: Kind,
    /// A sending key may call it (its REST route accepts sending keys).
    pub sending: bool,
    pub body: BodyKind,
    /// Destructive tools: what happens, finishing "This will ...".
    pub effect: &'static str,
    /// Destructive tools: a GET path that shows the target in the preview.
    pub preview_get: Option<&'static str>,
}

const fn t(
    name: &'static str,
    title: &'static str,
    method: &'static str,
    path: &'static str,
    kind: Kind,
    description: &'static str,
) -> ToolDef {
    ToolDef {
        name,
        title,
        description,
        method,
        path,
        kind,
        sending: false,
        body: BodyKind::Json,
        effect: "",
        preview_get: None,
    }
}

const fn d(
    name: &'static str,
    title: &'static str,
    method: &'static str,
    path: &'static str,
    description: &'static str,
    effect: &'static str,
    preview_get: Option<&'static str>,
) -> ToolDef {
    ToolDef {
        effect,
        preview_get,
        ..t(name, title, method, path, Kind::Destructive, description)
    }
}

use Kind::{Read, Write};

pub const TOOLS: &[ToolDef] = &[
    // Emails
    ToolDef {
        sending: true,
        ..t(
            "send_email",
            "Send an email",
            "post",
            "/emails",
            Write,
            "Send one transactional email, now or at `send_at`. `from` must use a verified domain (see list_domains). Returns the email id; check delivery with get_email.",
        )
    },
    ToolDef {
        sending: true,
        ..t(
            "send_email_batch",
            "Send up to 100 emails",
            "post",
            "/emails/batch",
            Write,
            "Send up to 100 emails in one call. Put them in `items`, each shaped like send_email. Either all are queued or none.",
        )
    },
    t(
        "list_emails",
        "List emails",
        "get",
        "/emails",
        Read,
        "List sent and scheduled emails, newest first. Filters and paging are in the input schema.",
    ),
    ToolDef {
        sending: true,
        ..t(
            "get_email",
            "Get an email",
            "get",
            "/emails/{id}",
            Read,
            "Get one email with its status and delivery events (delivered, opened, clicked, bounced).",
        )
    },
    d(
        "cancel_email",
        "Cancel a scheduled email",
        "post",
        "/emails/{id}/cancel",
        "Cancel an email that is still scheduled, so it is never sent.",
        "cancel this scheduled email, so it is never sent",
        Some("/emails/{id}"),
    ),
    // Contacts (the newsletter audience)
    t(
        "list_contacts",
        "List contacts",
        "get",
        "/contacts",
        Read,
        "List newsletter contacts. Search with `q` (email or name), filter by `status` or `tag` (a tag id). Up to 100 per page; pass `next_cursor` from the answer as `after` for the next page.",
    ),
    t(
        "get_contact",
        "Get a contact",
        "get",
        "/contacts/{id}",
        Read,
        "Get one contact with its fields and tags.",
    ),
    t(
        "create_contact",
        "Add a contact",
        "post",
        "/contacts",
        Write,
        "Add one contact. If the email already exists, it is updated instead. Tags are names; missing tags are created.",
    ),
    t(
        "update_contact",
        "Update a contact",
        "patch",
        "/contacts/{id}",
        Write,
        "Change a contact's name, fields, tags or status. Only the fields you send change.",
    ),
    d(
        "delete_contact",
        "Delete a contact",
        "delete",
        "/contacts/{id}",
        "Delete a contact and its history for good.",
        "delete this contact and its history for good",
        Some("/contacts/{id}"),
    ),
    t(
        "export_contacts",
        "Export contacts as CSV",
        "get",
        "/contacts/export",
        Read,
        "Get every contact as CSV text, with a header row.",
    ),
    ToolDef {
        body: BodyKind::CsvUpload,
        ..t(
            "import_contacts",
            "Import contacts from CSV",
            "post",
            "/contacts/imports",
            Write,
            "Import contacts from CSV text. The first row is the header and needs an `email` column. `consent` must be true: these people agreed to get your emails. The import runs in the background; follow it with get_import.",
        )
    },
    t(
        "list_imports",
        "List imports",
        "get",
        "/contacts/imports",
        Read,
        "List contact imports, newest first, with their progress.",
    ),
    t(
        "get_import",
        "Get an import",
        "get",
        "/contacts/imports/{id}",
        Read,
        "Get one import's progress: rows done, added, updated, skipped, and row errors.",
    ),
    // Tags
    t(
        "list_tags",
        "List tags",
        "get",
        "/tags",
        Read,
        "List tags with how many contacts each has.",
    ),
    t(
        "create_tag",
        "Create a tag",
        "post",
        "/tags",
        Write,
        "Create a tag.",
    ),
    t(
        "rename_tag",
        "Rename a tag",
        "patch",
        "/tags/{id}",
        Write,
        "Rename a tag.",
    ),
    d(
        "delete_tag",
        "Delete a tag",
        "delete",
        "/tags/{id}",
        "Delete a tag. The contacts stay; they just lose the tag.",
        "delete this tag (the contacts stay, they lose the tag)",
        None,
    ),
    // Campaigns (newsletter issues)
    t(
        "list_campaigns",
        "List campaigns",
        "get",
        "/campaigns",
        Read,
        "List newsletter campaigns (issues), newest first, with status and open and click counts.",
    ),
    t(
        "get_campaign",
        "Get a campaign",
        "get",
        "/campaigns/{id}",
        Read,
        "Get one campaign with its subject, content blocks and audience tags.",
    ),
    t(
        "create_campaign",
        "Create a campaign",
        "post",
        "/campaigns",
        Write,
        "Create a draft campaign. `blocks` is the editor document (JSON). `include_tags` and `exclude_tags` pick the audience; no include tags means every subscribed contact.",
    ),
    t(
        "update_campaign",
        "Update a campaign",
        "patch",
        "/campaigns/{id}",
        Write,
        "Change a draft campaign. Only the fields you send change.",
    ),
    d(
        "delete_campaign",
        "Delete a campaign",
        "delete",
        "/campaigns/{id}",
        "Delete a campaign for good.",
        "delete this campaign for good",
        Some("/campaigns/{id}"),
    ),
    t(
        "preview_campaign",
        "Preview a campaign",
        "post",
        "/campaigns/{id}/preview",
        Read,
        "Render a campaign without sending: HTML, text, size, pre-send checks, warnings and how many readers it would reach.",
    ),
    t(
        "send_test_campaign",
        "Send a test",
        "post",
        "/campaigns/{id}/test",
        Write,
        "Send a test copy of a campaign to a few addresses in `to`. Real readers get nothing.",
    ),
    d(
        "send_campaign",
        "Send a campaign",
        "post",
        "/campaigns/{id}/send",
        "Send a campaign to its audience now, or schedule it with `send_at` (ISO 8601). Call preview_campaign first and fix failed checks.",
        "send this campaign to every reader in its audience",
        Some("/campaigns/{id}"),
    ),
    t(
        "campaign_report",
        "Campaign report",
        "get",
        "/campaigns/{id}/report",
        Read,
        "Get a sent campaign's results: delivered, opens, clicks, bounces, unsubscribes, top links.",
    ),
    t(
        "pause_campaign",
        "Pause sending",
        "post",
        "/campaigns/{id}/pause",
        Write,
        "Pause a campaign that is sending. Resume it with resume_campaign.",
    ),
    t(
        "resume_campaign",
        "Resume sending",
        "post",
        "/campaigns/{id}/resume",
        Write,
        "Resume a paused campaign.",
    ),
    d(
        "cancel_campaign",
        "Cancel a campaign",
        "post",
        "/campaigns/{id}/cancel",
        "Stop a scheduled or sending campaign for good. Readers not reached yet get nothing.",
        "stop this campaign for good; readers not reached yet get nothing",
        Some("/campaigns/{id}"),
    ),
    // Newsletter settings
    t(
        "get_newsletter",
        "Get newsletter settings",
        "get",
        "/newsletter",
        Read,
        "Get the newsletter settings: name, sender, footer, archive and signup options.",
    ),
    t(
        "update_newsletter",
        "Update newsletter settings",
        "patch",
        "/newsletter",
        Write,
        "Change newsletter settings. Only the fields you send change.",
    ),
    // Sending domains
    t(
        "list_domains",
        "List domains",
        "get",
        "/domains",
        Read,
        "List sending domains and whether each is verified.",
    ),
    t(
        "get_domain",
        "Get a domain",
        "get",
        "/domains/{id}",
        Read,
        "Get one domain with the DNS records to add at the DNS provider.",
    ),
    t(
        "add_domain",
        "Add a domain",
        "post",
        "/domains",
        Write,
        "Add a sending domain. The answer lists DNS records to add; then call verify_domain.",
    ),
    t(
        "update_domain",
        "Update a domain",
        "patch",
        "/domains/{id}",
        Write,
        "Change a domain's settings. Only the fields you send change.",
    ),
    t(
        "verify_domain",
        "Verify a domain",
        "post",
        "/domains/{id}/verify",
        Write,
        "Check the domain's DNS records now and update its status.",
    ),
    d(
        "delete_domain",
        "Delete a domain",
        "delete",
        "/domains/{id}",
        "Remove a sending domain. Email from it stops working.",
        "remove this sending domain; email from it stops working",
        Some("/domains/{id}"),
    ),
    // Webhooks
    t(
        "list_webhooks",
        "List webhooks",
        "get",
        "/webhooks",
        Read,
        "List webhook endpoints and the events each gets.",
    ),
    t(
        "create_webhook",
        "Create a webhook",
        "post",
        "/webhooks",
        Write,
        "Add a webhook: a public https URL and the event types to send to it.",
    ),
    d(
        "delete_webhook",
        "Delete a webhook",
        "delete",
        "/webhooks/{id}",
        "Remove a webhook endpoint.",
        "remove this webhook endpoint; it gets no more events",
        None,
    ),
    // Suppressions
    t(
        "list_suppressions",
        "List suppressions",
        "get",
        "/suppressions",
        Read,
        "List addresses that never get email (bounces, complaints, manual). Search with `q`.",
    ),
    t(
        "add_suppression",
        "Suppress an address",
        "post",
        "/suppressions",
        Write,
        "Stop all email to an address.",
    ),
    d(
        "remove_suppression",
        "Unsuppress an address",
        "delete",
        "/suppressions/{address}",
        "Allow email to a suppressed address again. Only do this if the person asked for it.",
        "allow email to this address again",
        None,
    ),
    // Overview
    t(
        "stats_overview",
        "Account overview",
        "get",
        "/stats/overview",
        Read,
        "Get the account overview: email and contact counts, and recent sending numbers.",
    ),
];

/// Routes with no tool, on purpose. `/public/*` routes are skipped as a group.
pub const LEFT_OUT: &[(&str, &str, &str)] = &[
    (
        "post",
        "/api-keys",
        "a new key would print a secret into the chat log",
    ),
    ("get", "/api-keys", "API keys are managed by a person"),
    (
        "delete",
        "/api-keys/{id}",
        "API keys are managed by a person",
    ),
    ("post", "/auth/login", "browser login"),
    ("post", "/auth/verify", "browser login"),
    ("post", "/auth/logout", "browser session"),
    ("post", "/auth/logout-all", "browser session"),
    ("get", "/me", "needs a browser session, not a key"),
    ("get", "/team", "needs a browser session, not a key"),
    (
        "post",
        "/team/invites",
        "needs a browser session, not a key",
    ),
    (
        "delete",
        "/team/invites/{id}",
        "needs a browser session, not a key",
    ),
    (
        "patch",
        "/team/members/{user_id}",
        "needs a browser session, not a key",
    ),
    (
        "delete",
        "/team/members/{user_id}",
        "needs a browser session, not a key",
    ),
    (
        "post",
        "/invites/accept",
        "needs a browser session, not a key",
    ),
    ("get", "/health", "for load balancers, not people"),
    (
        "post",
        "/images",
        "needs a file; use image URLs in campaign blocks",
    ),
];
