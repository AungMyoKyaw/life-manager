//! One explicit specification per domain action.
//!
//! The same table drives the MCP tool catalog, the tool-name to action
//! mapping and the required-parameter validation, so the agent surface can
//! never drift from the domain.

use serde_json::{Value, json};

/// A single parameter of an action.
#[derive(Clone, Copy)]
pub struct Field {
    pub name: &'static str,
    pub kind: &'static str,
    pub enums: Option<&'static [&'static str]>,
}

const fn field(name: &'static str) -> Field {
    Field {
        name,
        kind: "string",
        enums: None,
    }
}

const TASK_STATUS: &[&str] = &[
    "inbox",
    "todo",
    "in_progress",
    "blocked",
    "done",
    "cancelled",
];
const RECURRENCE: &[&str] = &["daily", "weekdays", "weekly", "monthly"];
const FOCUS_MODE: &[&str] = &["stopwatch", "pomodoro", "manual"];
const PROPOSAL_STATE: &[&str] = &["pending", "approved", "rejected"];
const SCHEDULE_KIND: &[&str] = &["daily", "weekly"];
const CALENDAR_SOURCE_KIND: &[&str] = &["local", "ics", "external"];
const SCOPE_KIND: &[&str] = &["area", "project", "global"];
const BUDGET_PERIOD: &[&str] = &["day", "week", "month"];
const FILTER_ENTITY: &[&str] = &["tasks", "projects", "notes", "events"];
const SEARCH_ENTITY: &[&str] = &["all", "tasks", "notes", "projects", "events", "goals"];
const EXCEPTION_ACTION: &[&str] = &["skip", "reschedule"];

/// The action catalog. Every entry is a domain action the store can run.
pub const ACTIONS: &[(&str, &str)] = &[
    ("today", "Get today's priorities and schedule"),
    ("dashboard", "Get summary of the life manager"),
    ("stats", "Get statistics"),
    ("search", "Search the local data"),
    ("export", "Export all local data"),
    ("areas.list", "List life areas"),
    ("areas.create", "Create a personal life area"),
    ("areas.update", "Rename or customize a personal life area"),
    (
        "areas.delete",
        "Archive a personal life area without deleting linked tasks",
    ),
    (
        "areas.dashboard",
        "Summary of life area tasks projects and goals",
    ),
    ("tasks.list", "List tasks"),
    ("tasks.get", "Get a task"),
    ("tasks.create", "Create a task"),
    ("tasks.update", "Update a task"),
    ("tasks.complete", "Complete a task"),
    ("tasks.delete", "Archive a task"),
    ("tasks.restore", "Recover an archived task"),
    (
        "tasks.archived",
        "List deleted tasks available for recovery",
    ),
    ("tasks.bulk", "Atomically update multiple tasks"),
    ("tasks.snooze", "Reschedule overdue task"),
    ("projects.list", "List projects"),
    ("projects.create", "Create a project"),
    ("projects.update", "Update a project"),
    (
        "projects.health",
        "Explainable project health and blocker report",
    ),
    ("projects.timeline", "Project tasks and milestone timeline"),
    ("projects.templates.list", "List reusable project templates"),
    (
        "projects.templates.get",
        "Inspect reusable project template",
    ),
    (
        "projects.template.capture",
        "Capture project structure as reusable template",
    ),
    (
        "projects.template.apply",
        "Instantiate project with tasks, milestones, and dependencies",
    ),
    (
        "projects.template.archive",
        "Archive reusable project template",
    ),
    ("goals.list", "List goals"),
    ("goals.create", "Create a goal"),
    ("goals.update", "Update goal progress"),
    ("goals.history", "Goal progress history"),
    (
        "goals.limit",
        "Active goal count against the configured limit",
    ),
    (
        "accessibility.audit",
        "Inspect focus, contrast, motion and keyboard reachability",
    ),
    ("key_results.list", "List measurable goal key results"),
    ("key_results.create", "Create measurable key result"),
    ("key_results.update", "Update key result and goal progress"),
    ("key_results.delete", "Delete key result"),
    ("goal_cycles.list", "List yearly or quarterly goal cycles"),
    (
        "goal_cycles.get",
        "Read planning cycle and ranked focus goals",
    ),
    (
        "goal_cycles.create",
        "Create yearly direction or quarterly priorities",
    ),
    (
        "goal_cycles.update",
        "Update goal cycle metadata and status",
    ),
    (
        "goal_cycles.prioritize",
        "Set up to three focus goals for a period",
    ),
    ("events.list", "List calendar events"),
    ("events.create", "Create an event"),
    ("events.update", "Update an event"),
    ("events.delete", "Delete an event"),
    ("calendar.conflicts", "Find calendar overlaps"),
    (
        "calendar.window",
        "Events in a window with source, all-day and time zone detail",
    ),
    (
        "calendar.export_ics",
        "Export local calendar in iCalendar format",
    ),
    (
        "calendar.import.preview",
        "Preview local iCalendar file VEVENTs, duplicates and unsupported items",
    ),
    (
        "calendar.import.apply",
        "Commit reviewed iCalendar events to the protected local calendar",
    ),
    (
        "calendar.sources.list",
        "List calendar sources and their read-only state",
    ),
    (
        "calendar.sources.create",
        "Create a local or imported calendar source",
    ),
    (
        "calendar.sources.update",
        "Rename, recolour or toggle a calendar source",
    ),
    ("calendar.sources.delete", "Remove a calendar source"),
    (
        "recurrence.exceptions.list",
        "List per-occurrence exceptions for a recurring task",
    ),
    (
        "recurrence.exceptions.create",
        "Skip or reschedule one occurrence of a recurring task",
    ),
    (
        "recurrence.exceptions.delete",
        "Remove a per-occurrence exception",
    ),
    (
        "recurrence.occurrences",
        "Expand a recurring task into concrete occurrence dates",
    ),
    ("schedule.propose", "Suggest a time-block schedule"),
    ("schedule.apply", "Apply a reviewed schedule atomically"),
    ("protected.list", "List weekly protected time blocks"),
    ("protected.create", "Protect a recurring time block"),
    ("protected.delete", "Remove a protected time block"),
    ("time_budgets.list", "List weekly time budgets"),
    (
        "time_budgets.set",
        "Set a weekly budget for an area, project or the workspace",
    ),
    ("time_budgets.delete", "Remove a time budget"),
    (
        "time_budgets.report",
        "Planned minutes per area against the weekly budget",
    ),
    (
        "planning.carryover",
        "Unfinished work eligible for carryover",
    ),
    (
        "planning.carryover.apply",
        "Move unfinished tasks into the next period",
    ),
    (
        "planning.buffers",
        "Proposal with transition buffers and travel time",
    ),
    ("planning.next_week", "Next week preparation summary"),
    ("habits.list", "List habits"),
    ("habits.create", "Create habit"),
    ("habits.update", "Update habit"),
    ("habits.delete", "Delete habit"),
    ("habits.log", "Log a habit"),
    ("habits.configure", "Set flexible cadence and rest days"),
    ("habits.skip", "Record a voluntary skip or rest day"),
    ("habits.weekly", "Progress against optional weekly quotas"),
    ("habits.history", "Habit history"),
    ("routines.list", "List routines"),
    ("routines.create", "Create a routine"),
    ("routines.update", "Update a routine"),
    ("routines.steps", "List steps"),
    ("routines.step.add", "Add a routine step"),
    ("routines.delete", "Archive routine"),
    ("milestones.list", "List milestones"),
    ("milestones.create", "Create milestone"),
    ("milestones.complete", "Complete milestone"),
    ("reviews.list", "List reviews"),
    ("reviews.create", "Create review"),
    ("reviews.update", "Update review"),
    ("review_guides.prompts", "Read guided review prompts"),
    (
        "review_guides.list",
        "List guided drafts and completed reviews",
    ),
    ("review_guides.get", "Read a guided review"),
    ("review_guides.start", "Start or resume a guided review"),
    ("review_guides.update", "Save reflection answers"),
    (
        "review_guides.complete",
        "Finish a guided review into canonical journal",
    ),
    ("notes.list", "List notes"),
    ("notes.create", "Create note"),
    ("notes.update", "Update note"),
    ("notes.delete", "Delete note"),
    ("focus.list", "List focus sessions"),
    ("focus.start", "Start focus session"),
    ("focus.stop", "End focus session"),
    ("focus.pause", "Pause focus"),
    ("focus.resume", "Resume focus"),
    (
        "focus.interrupt",
        "Record an interruption reason and pause the session",
    ),
    ("focus.manual", "Record a completed work session by hand"),
    (
        "focus.trends",
        "Recorded focus minutes against planned task estimates",
    ),
    ("focus.presets.list", "List focus presets"),
    ("focus.presets.create", "Create or update a focus preset"),
    ("focus.presets.delete", "Delete a focus preset"),
    ("dependencies.list", "List task dependencies"),
    ("dependencies.add", "Add dependency"),
    ("dependencies.remove", "Remove dependency"),
    ("links.list", "List related entity links and backlinks"),
    ("links.add", "Link two workspace entities"),
    ("links.remove", "Remove entity relation"),
    (
        "links.backlinks",
        "Everything linked to an entity in both directions",
    ),
    ("templates.list", "List task templates"),
    ("templates.create", "Create task template"),
    ("templates.delete", "Archive task template"),
    ("templates.apply", "Create task from a template"),
    ("saved_views.list", "List saved task filters"),
    ("saved_views.create", "Save a reusable filter"),
    ("saved_views.delete", "Delete a saved filter"),
    (
        "saved_views.run",
        "Run a saved filter and return the matching rows",
    ),
    ("saved_searches.list", "List saved searches"),
    ("saved_searches.create", "Save a reusable search"),
    ("saved_searches.delete", "Delete a saved search"),
    (
        "saved_searches.run",
        "Run a saved search across one entity or everything",
    ),
    ("availability.list", "Read weekly availability"),
    ("availability.replace", "Set reviewed weekly availability"),
    ("import.preview", "Preview portable data import"),
    ("import.apply", "Apply reviewed import transaction"),
    (
        "reports.summary",
        "Explainable planning and productivity report",
    ),
    (
        "reports.plan_vs_actual",
        "Planned effort against completed and recorded effort",
    ),
    (
        "reports.goal_achievement",
        "Goal progress and achievement from key results",
    ),
    (
        "reports.area_distribution",
        "Where recorded effort went, by life area",
    ),
    (
        "reports.journal_export",
        "Export private journal entries for a window",
    ),
    ("automations.list", "List automation rules"),
    ("automations.create", "Create automation"),
    ("automations.update", "Enable or disable automation"),
    ("automations.run", "Execute automation"),
    ("automations.history", "List automation runs"),
    (
        "automations.schedule.list",
        "List scheduled automation triggers",
    ),
    (
        "automations.schedule.set",
        "Schedule an automation for a daily or weekly time",
    ),
    (
        "automations.schedule.delete",
        "Remove a scheduled automation trigger",
    ),
    (
        "automations.schedule.due",
        "Scheduled triggers that are due right now",
    ),
    (
        "automations.schedule.run_due",
        "Run every due scheduled trigger once",
    ),
    (
        "agents.changes",
        "Inspect task change snapshots and undo eligibility",
    ),
    (
        "agents.undo",
        "Undo a task change when the record has not been modified since",
    ),
    ("agents.permissions.list", "List agent action permissions"),
    (
        "agents.permissions.set",
        "Allow or deny an action pattern for an agent actor",
    ),
    (
        "agents.permissions.delete",
        "Remove an agent action permission",
    ),
    (
        "agents.proposals.list",
        "List agent proposals awaiting review",
    ),
    (
        "agents.proposals.create",
        "Propose a change for human review",
    ),
    (
        "agents.proposals.approve",
        "Approve a proposal and apply it",
    ),
    ("agents.proposals.reject", "Reject a proposal with a reason"),
    ("settings.list", "List settings"),
    ("settings.set", "Set preference"),
    ("history.list", "Inspect agent and user activity"),
    ("backup.create", "Create and verify local backup"),
    (
        "backup.verify",
        "Validate a SQLite backup before restoration",
    ),
    (
        "backup.restore",
        "Restore a verified backup with explicit confirmation and safety snapshot",
    ),
    ("career.list", "List career applications"),
    ("career.get", "Get a job application"),
    ("career.create", "Track a job application"),
    (
        "career.update",
        "Update an application stage and next action",
    ),
    ("career.delete", "Archive a job application"),
    ("career.restore", "Restore an archived job application"),
    ("learning.list", "List learning tracks"),
    ("learning.get", "Get a learning track"),
    ("learning.create", "Create a learning track"),
    ("learning.update", "Update learning progress"),
    ("learning.delete", "Archive a learning track"),
    ("learning.restore", "Restore an archived learning track"),
    ("learning.log", "Record a study session"),
    ("learning.sessions", "List study sessions"),
    ("finance.list", "List lightweight financial reminders"),
    ("finance.get", "Get a financial reminder"),
    (
        "finance.create",
        "Create financial reminder with integer minor units",
    ),
    ("finance.update", "Update a financial reminder"),
    ("finance.delete", "Archive a financial reminder"),
    ("finance.restore", "Restore an archived financial reminder"),
    ("wellbeing.list", "List optional wellbeing check-ins"),
    ("wellbeing.log", "Record a private wellbeing check-in"),
    ("wellbeing.delete", "Delete a wellbeing check-in"),
    ("life_domains.dashboard", "Summarize personal commitments"),
    (
        "attachments.list",
        "List attachment metadata for a note, task, project or goal",
    ),
    (
        "attachments.create",
        "Attach base64-encoded file data to a record; 8 MiB limit",
    ),
    (
        "attachments.get",
        "Read checksum-verified attachment bytes as base64",
    ),
    ("attachments.delete", "Archive an attachment"),
    ("attachments.restore", "Restore an archived attachment"),
    ("reminders.settings", "Read opt-in reminder preferences"),
    ("reminders.upcoming", "List local upcoming reminders"),
    ("reminders.due", "List due reminders outside quiet hours"),
    (
        "reminders.claim",
        "Claim a due reminder with a short SQLite lease",
    ),
    ("reminders.delivered", "Record successful leased delivery"),
    ("reminders.release", "Release a reminder claim"),
    ("reminders.dismiss", "Dismiss a reminder"),
    ("reminders.snooze", "Snooze a reminder"),
    (
        "reminders.history",
        "Inspect delivery and dismissal history",
    ),
    (
        "interviews.list",
        "List scheduled interviews and preparation",
    ),
    ("interviews.create", "Schedule an interview"),
    ("interviews.update", "Update an interview"),
    ("interviews.delete", "Archive an interview"),
    ("savings_goals.list", "List savings goals"),
    ("savings_goals.create", "Create a savings goal"),
    ("savings_goals.update", "Update a savings goal"),
    ("savings_goals.delete", "Archive a savings goal"),
    ("certifications.list", "List certification deadlines"),
    ("certifications.create", "Create a certification deadline"),
    ("certifications.update", "Update a certification deadline"),
    ("certifications.delete", "Archive a certification deadline"),
    ("rest_intentions.list", "List sleep and rest intentions"),
    ("rest_intentions.create", "Record a sleep or rest intention"),
    ("rest_intentions.delete", "Archive a rest intention"),
    ("administration.list", "List personal administration items"),
    (
        "administration.create",
        "Create a personal administration item",
    ),
    (
        "administration.update",
        "Update a personal administration item",
    ),
    (
        "administration.delete",
        "Archive a personal administration item",
    ),
    ("personal.overview", "Summarize every personal life area"),
    (
        "entity.timeline",
        "Read the activity and change history of one record",
    ),
    (
        "overdue.decisions",
        "List overdue tasks with the decisions that apply",
    ),
    ("overdue.decide", "Apply an overdue decision to a task"),
    ("projects.blockers", "List blocked work and what blocks it"),
    ("projects.notes", "List the notes linked to a project"),
    (
        "projects.complete",
        "Complete a project and record the outcome",
    ),
    ("projects.reopen", "Reopen a paused or completed project"),
    (
        "goals.detail",
        "Read a goal with its key results and history",
    ),
    ("goals.monthly_review", "Read the monthly goal review"),
    (
        "reviews.journal_archive",
        "Read the private journal archive",
    ),
    ("planning.session", "Run or record a daily planning session"),
    (
        "planning.session.get",
        "Read a recorded daily planning session",
    ),
    ("capture.parse", "Parse natural language into a task draft"),
    (
        "system.status",
        "Read the live database health and schema version",
    ),
    (
        "diagnostics.report",
        "Read storage, table counts and the security audit",
    ),
    ("security.audit.log", "Record a security audit event"),
    ("security.audit.list", "Read the security audit log"),
    (
        "csv.import.preview",
        "Preview a CSV import without writing anything",
    ),
    ("csv.import.apply", "Apply a reviewed CSV import"),
    (
        "csv.import.history",
        "List the records imported from a source",
    ),
    ("shortcuts.list", "List the keyboard shortcuts"),
    ("shortcuts.update", "Bind a key combination to an action"),
    ("shortcuts.reset", "Restore the default keyboard shortcuts"),
    (
        "integrations.list",
        "List every integration and its real state",
    ),
    (
        "integrations.configure",
        "Report what an integration still needs",
    ),
    ("integrations.status", "Record the state of an integration"),
    ("remote_clients.list", "List the remote MCP clients"),
    ("remote_clients.create", "Create a remote MCP client token"),
    ("remote_clients.revoke", "Revoke a remote MCP client"),
    (
        "notification.authorization.set",
        "Record the notification authorization state",
    ),
    (
        "notification.authorization.get",
        "Read the notification authorization state",
    ),
    ("encryption.status", "Read how the database is protected"),
    (
        "encryption.enable",
        "Encrypt the database with a passphrase",
    ),
    (
        "encryption.disable",
        "Decrypt the database back to a plain file",
    ),
    (
        "encryption.verify_passphrase",
        "Check a passphrase without opening the database",
    ),
];

/// The MCP tool name for an action.
pub fn tool_name(action: &str) -> String {
    format!("life_{}", action.replace('.', "_"))
}

/// The domain action for an MCP tool name. The mapping is derived from the
/// catalog, so it is exact for every action including three-part names.
pub fn action_for_tool(tool: &str) -> Option<&'static str> {
    let name = tool.strip_prefix("life_").unwrap_or(tool);
    ACTIONS
        .iter()
        .find(|(action, _)| tool_name(action) == tool)
        .map(|(action, _)| *action)
        .or_else(|| {
            ACTIONS
                .iter()
                .find(|(action, _)| action.replace('.', "_") == name)
                .map(|(action, _)| *action)
        })
}

pub fn description(action: &str) -> &'static str {
    ACTIONS
        .iter()
        .find(|(name, _)| *name == action)
        .map(|(_, description)| *description)
        .unwrap_or("Run a domain action")
}

/// Required parameters for an action.
pub fn required(action: &str) -> &'static [&'static str] {
    match action {
        "agents.undo"
        | "areas.update"
        | "areas.delete"
        | "attachments.delete"
        | "attachments.get"
        | "attachments.restore"
        | "automations.run"
        | "automations.update"
        | "career.delete"
        | "career.get"
        | "career.restore"
        | "career.update"
        | "finance.delete"
        | "finance.get"
        | "finance.restore"
        | "finance.update"
        | "goals.update"
        | "key_results.delete"
        | "key_results.update"
        | "learning.delete"
        | "learning.get"
        | "learning.restore"
        | "learning.update"
        | "links.remove"
        | "milestones.complete"
        | "notes.update"
        | "projects.update"
        | "projects.templates.get"
        | "projects.template.apply"
        | "projects.template.archive"
        | "reviews.update"
        | "routines.delete"
        | "saved_views.delete"
        | "tasks.bulk"
        | "tasks.complete"
        | "tasks.delete"
        | "tasks.get"
        | "tasks.restore"
        | "tasks.snooze"
        | "tasks.update"
        | "templates.apply"
        | "templates.delete"
        | "wellbeing.delete" => &["id"],
        "tasks.create" | "goals.create" | "habits.create" | "notes.create" | "reviews.create"
        | "routines.create" => &["title"],
        "projects.create" | "areas.create" => &["name"],
        "career.create" => &["employer", "role"],
        "learning.create" | "finance.create" => &["title"],
        "learning.log" => &["track_id", "minutes"],
        "learning.sessions" => &["track_id"],
        "wellbeing.log" => &["energy", "mood"],
        "calendar.import.preview" => &["content"],
        "calendar.import.apply" => &["content", "confirm"],
        "attachments.list" => &["owner_kind", "owner_id"],
        "attachments.create" => &["owner_kind", "owner_id", "filename", "data_base64"],
        "templates.create" => &["name", "title"],
        "saved_views.create" => &["name", "query"],
        "saved_views.run" => &["id"],
        "saved_searches.create" => &["name", "query"],
        "saved_searches.delete" | "saved_searches.run" => &["id"],
        "key_results.create" => &["goal_id", "title"],
        "key_results.list" | "goals.history" => &["goal_id"],
        "links.list" | "links.backlinks" => &["kind", "id"],
        "links.add" => &["source_kind", "source_id", "target_kind", "target_id"],
        "availability.replace" => &["rules"],
        "import.preview" => &["items"],
        "import.apply" => &["items", "confirm"],
        "backup.verify" | "backup.restore" => &["path"],
        "events.create" => &["title", "start_at", "end_at"],
        "calendar.conflicts" => &["start_at", "end_at"],
        "calendar.window" => &["from", "to"],
        "calendar.sources.create" => &["name"],
        "calendar.sources.update" | "calendar.sources.delete" => &["id"],
        "recurrence.exceptions.list" | "recurrence.exceptions.delete" => {
            &["task_id", "occurrence_date"]
        }
        "recurrence.exceptions.create" => &["task_id", "occurrence_date"],
        "recurrence.occurrences" => &["task_id", "from", "to"],
        "habits.log" => &["habit_id"],
        "habits.configure" => &["id", "quota_period", "weekly_quota", "rest_days"],
        "habits.skip" => &["habit_id"],
        "reminders.claim" | "reminders.delivered" | "reminders.release" | "reminders.dismiss" => {
            &["key"]
        }
        "reminders.snooze" => &["key", "until"],
        "routines.steps" => &["routine_id"],
        "routines.step.add" => &["routine_id", "title"],
        "dependencies.add" | "dependencies.remove" => &["task_id", "blocks_id"],
        "search" => &["query"],
        "schedule.apply" => &["slots"],
        "settings.set" => &["key", "value"],
        "automations.create" => &["title", "action"],
        "automations.schedule.list" => &["rule_id"],
        "automations.schedule.set" => &["rule_id", "kind", "at_time"],
        "automations.schedule.delete" => &["id"],
        "agents.permissions.set" => &["target_actor", "action_pattern"],
        "agents.permissions.delete" => &["target_actor", "action_pattern"],
        "agents.proposals.list" => &["state"],
        "agents.proposals.create" => &["action"],
        "agents.proposals.approve" => &["id"],
        "agents.proposals.reject" => &["id", "reason"],
        "protected.create" => &["title", "weekday", "start_time", "end_time"],
        "protected.delete" => &["id"],
        "time_budgets.set" => &["scope_kind", "minutes"],
        "time_budgets.delete" => &["id"],
        "planning.carryover" => &["from", "to"],
        "planning.carryover.apply" => &["ids", "target_date"],
        "focus.start" => &["mode"],
        "focus.stop" | "focus.pause" | "focus.resume" | "focus.interrupt" => &["id"],
        "focus.manual" => &["minutes"],
        "focus.presets.create" => &["name", "focus_minutes"],
        "focus.presets.delete" => &["id"],
        "milestones.create" => &["project_id", "title"],
        "goal_cycles.get" | "goal_cycles.update" => &["id"],
        "goal_cycles.create" => &["kind"],
        "goal_cycles.prioritize" => &["id", "goal_ids"],
        "review_guides.start" => &["kind"],
        "review_guides.get" | "review_guides.complete" => &["id"],
        "review_guides.update" => &["id", "answers"],
        "projects.template.capture" => &["project_id"],
        "interviews.create" => &["title", "starts_at"],
        "interviews.update" | "interviews.delete" => &["id"],
        "savings_goals.create" => &["title"],
        "savings_goals.update" | "savings_goals.delete" => &["id"],
        "certifications.create" => &["title"],
        "certifications.update" | "certifications.delete" => &["id"],
        "rest_intentions.create" => &["title"],
        "rest_intentions.delete" => &["id"],
        "administration.create" => &["title"],
        "administration.update" | "administration.delete" => &["id"],
        "entity.timeline" => &["kind", "id"],
        "overdue.decide" => &["id"],
        "projects.notes" => &["project_id"],
        "projects.complete" | "projects.reopen" => &["id"],
        "goals.detail" => &["id"],
        "planning.session.get" => &["date"],
        "capture.parse" => &["text"],
        "security.audit.log" => &["event"],
        "csv.import.preview" => &["content"],
        "csv.import.apply" => &["content", "confirm"],
        "shortcuts.update" => &["action", "keys"],
        "integrations.configure" | "integrations.status" => &["provider"],
        "remote_clients.create" => &["name", "token"],
        "remote_clients.revoke" => &["id"],
        "notification.authorization.set" => &["status"],
        "habits.update" | "habits.delete" => &["id"],
        "events.update" | "events.delete" => &["id"],
        "routines.update" => &["id"],
        "notes.delete" => &["id"],
        "encryption.enable" => &["passphrase"],
        "encryption.disable" => &["confirm"],
        "encryption.verify_passphrase" => &["passphrase"],
        _ => &[],
    }
}

fn common_fields(action: &str) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = vec!["actor", "request_id"];
    if action.starts_with("tasks.") {
        names.extend_from_slice(&[
            "title",
            "description",
            "status",
            "priority",
            "project_id",
            "area_id",
            "goal_id",
            "parent_id",
            "due_date",
            "planned_date",
            "estimate_minutes",
            "recurrence",
            "tags",
            "expected_revision",
        ]);
    }
    if action.starts_with("projects.") {
        names.extend_from_slice(&["name", "description", "status", "area_id", "project_id"]);
    }
    if action.starts_with("goals.") {
        names.extend_from_slice(&[
            "title", "status", "target", "progress", "due_date", "area_id", "limit",
        ]);
    }
    if action.starts_with("events.") || action.starts_with("calendar.") {
        names.extend_from_slice(&[
            "title",
            "description",
            "start_at",
            "end_at",
            "all_day",
            "locked",
            "override_locked",
            "color",
            "kind",
            "task_id",
            "recurrence",
            "timezone_id",
            "from",
            "to",
            "weekday",
            "start_time",
            "end_time",
            "read_only",
            "name",
            "content",
            "confirm",
        ]);
    }
    if action.starts_with("habits.") {
        names.extend_from_slice(&[
            "title",
            "frequency",
            "target",
            "unit",
            "color",
            "habit_id",
            "day",
            "value",
            "quota_period",
            "weekly_quota",
            "rest_days",
            "status",
            "from",
            "to",
            "limit",
        ]);
    }
    if action.starts_with("notes.") || action.starts_with("links.") {
        names.extend_from_slice(&[
            "title",
            "content",
            "area_id",
            "kind",
            "id",
            "source_kind",
            "source_id",
            "target_kind",
            "target_id",
            "relation",
        ]);
    }
    if action.starts_with("focus.") {
        names.extend_from_slice(&[
            "id",
            "task_id",
            "mode",
            "preset_id",
            "reason",
            "minutes",
            "day",
            "name",
            "focus_minutes",
            "break_minutes",
            "cycles",
            "from",
            "to",
        ]);
    }
    if action.starts_with("reports.") {
        names.extend_from_slice(&["from", "to"]);
    }
    if action.starts_with("recurrence.") {
        names.extend_from_slice(&[
            "task_id",
            "occurrence_date",
            "action",
            "moved_to",
            "reason",
            "from",
            "to",
        ]);
    }
    if action.starts_with("planning.") || action.starts_with("time_budgets.") {
        names.extend_from_slice(&[
            "from",
            "to",
            "ids",
            "target_date",
            "buffer_minutes",
            "travel_minutes",
            "scope_kind",
            "scope_id",
            "period",
            "minutes",
            "week_start",
            "id",
        ]);
    }
    if action.starts_with("protected.") {
        names.extend_from_slice(&["title", "weekday", "start_time", "end_time", "kind", "id"]);
    }
    if action.starts_with("saved_views.") || action.starts_with("saved_searches.") {
        names.extend_from_slice(&["id", "name", "entity", "query"]);
    }
    if action.starts_with("automations.") {
        names.extend_from_slice(&[
            "id",
            "title",
            "action",
            "arguments",
            "trigger_kind",
            "enabled",
            "rule_id",
            "kind",
            "at_time",
            "weekday",
            "state",
        ]);
    }
    if action.starts_with("agents.") {
        names.extend_from_slice(&[
            "id",
            "actor",
            "target_actor",
            "action_pattern",
            "allowed",
            "action",
            "payload",
            "reason",
            "state",
        ]);
    }
    if action.starts_with("settings.") {
        names.extend_from_slice(&["key", "value"]);
    }
    if action.starts_with("career.") {
        names.extend_from_slice(&[
            "id",
            "employer",
            "role",
            "location",
            "stage",
            "next_action",
            "next_action_on",
            "interview_at",
            "job_url",
            "notes",
            "salary_min",
            "salary_max",
            "currency",
            "expected_revision",
        ]);
    }
    if action.starts_with("learning.") {
        names.extend_from_slice(&[
            "id",
            "title",
            "provider",
            "status",
            "progress",
            "target_hours",
            "due_date",
            "notes",
            "track_id",
            "minutes",
            "day",
            "note",
            "expected_revision",
        ]);
    }
    if action.starts_with("finance.") {
        names.extend_from_slice(&[
            "id",
            "title",
            "amount_minor",
            "currency",
            "due_date",
            "category",
            "status",
            "notes",
            "expected_revision",
        ]);
    }
    if action.starts_with("wellbeing.") {
        names.extend_from_slice(&["id", "day", "energy", "mood", "sleep_hours", "note"]);
    }
    if action.starts_with("attachments.") {
        names.extend_from_slice(&[
            "owner_kind",
            "owner_id",
            "filename",
            "mime_type",
            "data_base64",
            "id",
        ]);
    }
    if action.starts_with("reminders.") {
        names.extend_from_slice(&[
            "key",
            "until",
            "lookback_minutes",
            "forward_minutes",
            "limit",
        ]);
    }
    if action.starts_with("templates.") || action.starts_with("projects.template") {
        names.extend_from_slice(&[
            "id",
            "name",
            "title",
            "description",
            "priority",
            "estimate_minutes",
            "project_id",
            "area_id",
        ]);
    }
    if action.starts_with("routines.") || action.starts_with("milestones.") {
        names.extend_from_slice(&[
            "id",
            "title",
            "description",
            "routine_id",
            "project_id",
            "due_date",
            "duration_minutes",
            "position",
        ]);
    }
    if action.starts_with("reviews.") || action.starts_with("review_guides.") {
        names.extend_from_slice(&["id", "title", "content", "period", "day", "kind", "answers"]);
    }
    if action.starts_with("dependencies.") {
        names.extend_from_slice(&["task_id", "blocks_id"]);
    }
    if action.starts_with("availability.") {
        names.extend_from_slice(&["rules"]);
    }
    if action.starts_with("import.") {
        names.extend_from_slice(&["items", "confirm", "allow_invalid"]);
    }
    if action.starts_with("backup.") {
        names.extend_from_slice(&["path"]);
    }
    if action.starts_with("key_results.") {
        names.extend_from_slice(&[
            "id", "goal_id", "title", "unit", "baseline", "target", "current",
        ]);
    }
    if action.starts_with("goal_cycles.") {
        names.extend_from_slice(&[
            "id",
            "kind",
            "year",
            "quarter",
            "title",
            "description",
            "status",
            "goal_ids",
        ]);
    }
    if action.starts_with("areas.") {
        names.extend_from_slice(&["id", "name", "color", "icon"]);
    }
    if action.starts_with("schedule.") {
        names.extend_from_slice(&["date", "slots", "buffer_minutes", "now"]);
    }
    if action.starts_with("interviews.") {
        names.extend_from_slice(&[
            "id",
            "title",
            "kind",
            "starts_at",
            "ends_at",
            "location",
            "preparation",
            "status",
            "application_id",
        ]);
    }
    if action.starts_with("savings_goals.") {
        names.extend_from_slice(&[
            "id",
            "title",
            "target_minor",
            "saved_minor",
            "currency",
            "due_date",
            "note",
        ]);
    }
    if action.starts_with("certifications.") {
        names.extend_from_slice(&[
            "id",
            "title",
            "provider",
            "status",
            "exam_date",
            "deadline_date",
            "study_hours_target",
            "study_hours_done",
            "note",
        ]);
    }
    if action.starts_with("rest_intentions.") {
        names.extend_from_slice(&[
            "id",
            "title",
            "kind",
            "starts_at",
            "ends_at",
            "days",
            "note",
        ]);
    }
    if action.starts_with("administration.") {
        names.extend_from_slice(&[
            "id",
            "title",
            "kind",
            "renewal_date",
            "reminder_days",
            "amount_minor",
            "currency",
            "status",
            "note",
        ]);
    }
    if action.starts_with("overdue.") {
        names.extend_from_slice(&["id", "decision", "date", "priority", "scope"]);
    }
    if action.starts_with("csv.") {
        names.extend_from_slice(&["content", "confirm", "allow_invalid", "source"]);
    }
    if action.starts_with("shortcuts.") {
        names.extend_from_slice(&["action", "keys", "label"]);
    }
    if action.starts_with("integrations.") {
        names.extend_from_slice(&[
            "provider",
            "status",
            "account_label",
            "scopes",
            "last_sync_at",
        ]);
    }
    if action.starts_with("remote_clients.") {
        names.extend_from_slice(&["id", "name", "token", "scopes"]);
    }
    if action.starts_with("notification.") {
        names.extend_from_slice(&["bundle_id", "status"]);
    }
    if action.starts_with("security.") {
        names.extend_from_slice(&["event", "detail"]);
    }
    if action.starts_with("capture.") {
        names.extend_from_slice(&["text"]);
    }
    if action.starts_with("entity.") {
        names.extend_from_slice(&["kind", "id"]);
    }
    if action.starts_with("planning.session") {
        names.extend_from_slice(&["date", "priorities"]);
    }
    if action.starts_with("encryption.") {
        names.extend_from_slice(&["passphrase", "confirm"]);
    }
    names
}

/// The parameters of an action, with their types.
pub fn fields(action: &str) -> Vec<Field> {
    let mut out: Vec<Field> = Vec::new();
    for name in required(action) {
        out.push(field(name));
    }
    for name in common_fields(action) {
        if !out.iter().any(|f| f.name == name) {
            out.push(field(name));
        }
    }
    for entry in &mut out {
        entry.kind = match entry.name {
            "priority" | "estimate_minutes" | "expected_revision" | "limit" | "weekly_quota"
            | "lookback_minutes" | "forward_minutes" | "year" | "quarter" | "energy" | "mood"
            | "minutes" | "amount_minor" | "salary_min" | "salary_max" | "buffer_minutes"
            | "travel_minutes" | "weekday" | "focus_minutes" | "break_minutes" | "cycles"
            | "manual_minutes" => "integer",
            "target" | "progress" | "baseline" | "current" | "target_hours" | "sleep_hours" => {
                "number"
            }
            "enabled" | "confirm" | "archived" | "allow_invalid" | "include_archived"
            | "all_day" | "locked" | "override_locked" | "read_only" | "allowed" => "boolean",
            "slots" | "ids" | "items" | "rules" | "rest_days" | "goal_ids" | "tags" | "days"
            | "scopes" | "priorities" => "array",
            "arguments" | "fields" | "answers" | "query" | "payload" => "object",
            _ => "string",
        };
        entry.enums = match entry.name {
            "status" if action.starts_with("tasks.") => Some(TASK_STATUS),
            "recurrence" => Some(RECURRENCE),
            "mode" if action.starts_with("focus.") => Some(FOCUS_MODE),
            "state" if action.starts_with("agents.proposals") => Some(PROPOSAL_STATE),
            "kind" if action.starts_with("automations.schedule") => Some(SCHEDULE_KIND),
            "kind" if action.starts_with("calendar.sources") => Some(CALENDAR_SOURCE_KIND),
            "scope_kind" => Some(SCOPE_KIND),
            "period" if action.starts_with("time_budgets") => Some(BUDGET_PERIOD),
            "entity" if action.starts_with("saved_views") => Some(FILTER_ENTITY),
            "entity" if action.starts_with("saved_searches") => Some(SEARCH_ENTITY),
            "action" if action == "recurrence.exceptions.create" => Some(EXCEPTION_ACTION),
            _ => None,
        };
    }
    out
}

/// The MCP tool catalog, generated from the action spec table.
pub fn tools() -> Vec<Value> {
    let mut out = Vec::new();
    for (action, _) in ACTIONS {
        let mut properties = serde_json::Map::new();
        for entry in fields(action) {
            let mut spec = json!({"type": entry.kind});
            if let Some(values) = entry.enums {
                spec["enum"] = json!(values);
            }
            if entry.name == "rest_days" {
                spec["items"] = json!({"type":"integer","minimum":0,"maximum":6});
            }
            if entry.name == "ids" || entry.name == "goal_ids" {
                spec["items"] = json!({"type":"string"});
            }
            if entry.name == "items" {
                spec["items"] = json!({"type":"object","additionalProperties":true});
            }
            if entry.name == "rules" {
                spec["items"] = json!({"type":"object","properties":{"weekday":{"type":"integer"},"start_time":{"type":"string"},"end_time":{"type":"string"},"kind":{"type":"string"}},"required":["weekday","start_time","end_time"]});
            }
            if entry.name == "slots" {
                spec["items"] = json!({"type":"object","properties":{"task_id":{"type":"string"},"start_at":{"type":"string"},"end_at":{"type":"string"}},"required":["task_id","start_at","end_at"]});
            }
            if entry.name == "tags" {
                spec["items"] = json!({"type":"string"});
            }
            properties.insert(entry.name.to_string(), spec);
        }
        out.push(json!({
            "name": tool_name(action),
            "description": description(action),
            "inputSchema": {
                "type": "object",
                "properties": properties,
                "required": required(action),
                "additionalProperties": false,
            },
        }));
    }
    out
}

/// Validate the arguments of an action against its specification.
pub fn validate(action: &str, arguments: &Value) -> Result<(), String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "arguments must be an object".to_string())?;
    for name in required(action) {
        if !object.contains_key(*name) {
            return Err(format!("{action} requires the parameter {name}"));
        }
    }
    for entry in fields(action) {
        let Some(value) = object.get(entry.name) else {
            continue;
        };
        let ok = match entry.kind {
            "integer" => value.is_i64() || value.is_u64(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            _ => value.is_string(),
        };
        if !ok {
            return Err(format!(
                "{action} parameter {} must be {}",
                entry.name, entry.kind
            ));
        }
        if let Some(values) = entry.enums
            && let Some(text) = value.as_str()
            && !values.contains(&text)
        {
            return Err(format!(
                "{action} parameter {} must be one of {}",
                entry.name,
                values.join(", ")
            ));
        }
    }
    Ok(())
}
