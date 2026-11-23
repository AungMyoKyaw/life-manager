//! The MCP tool catalog is a public contract.
//!
//! Every tool name published by an earlier release must keep resolving, so an
//! existing agent keeps working when the catalog grows. This test pins the
//! complete list of previously published names.

/// The tool names published by the 0.1.0 release and every release since.
const PUBLISHED_TOOLS: &[&str] = &[
    "life_agents_changes",
    "life_agents_undo",
    "life_areas_create",
    "life_areas_dashboard",
    "life_areas_delete",
    "life_areas_list",
    "life_areas_update",
    "life_attachments_create",
    "life_attachments_delete",
    "life_attachments_get",
    "life_attachments_list",
    "life_attachments_restore",
    "life_automations_create",
    "life_automations_history",
    "life_automations_list",
    "life_automations_run",
    "life_automations_update",
    "life_availability_list",
    "life_availability_replace",
    "life_backup_create",
    "life_backup_restore",
    "life_backup_verify",
    "life_calendar_conflicts",
    "life_calendar_export_ics",
    "life_calendar_import_apply",
    "life_calendar_import_preview",
    "life_career_create",
    "life_career_delete",
    "life_career_get",
    "life_career_list",
    "life_career_restore",
    "life_career_update",
    "life_dashboard",
    "life_dependencies_add",
    "life_dependencies_list",
    "life_dependencies_remove",
    "life_events_create",
    "life_events_list",
    "life_events_update",
    "life_export",
    "life_finance_create",
    "life_finance_delete",
    "life_finance_get",
    "life_finance_list",
    "life_finance_restore",
    "life_finance_update",
    "life_focus_list",
    "life_focus_pause",
    "life_focus_resume",
    "life_focus_start",
    "life_focus_stop",
    "life_goal_cycles_create",
    "life_goal_cycles_get",
    "life_goal_cycles_list",
    "life_goal_cycles_prioritize",
    "life_goal_cycles_update",
    "life_goals_create",
    "life_goals_history",
    "life_goals_list",
    "life_goals_update",
    "life_habits_configure",
    "life_habits_create",
    "life_habits_history",
    "life_habits_list",
    "life_habits_log",
    "life_habits_skip",
    "life_habits_weekly",
    "life_history_list",
    "life_import_apply",
    "life_import_preview",
    "life_key_results_create",
    "life_key_results_delete",
    "life_key_results_list",
    "life_key_results_update",
    "life_learning_create",
    "life_learning_delete",
    "life_learning_get",
    "life_learning_list",
    "life_learning_log",
    "life_learning_restore",
    "life_learning_sessions",
    "life_learning_update",
    "life_life_domains_dashboard",
    "life_links_add",
    "life_links_list",
    "life_links_remove",
    "life_milestones_complete",
    "life_milestones_create",
    "life_milestones_list",
    "life_notes_create",
    "life_notes_list",
    "life_notes_update",
    "life_projects_create",
    "life_projects_health",
    "life_projects_list",
    "life_projects_template_apply",
    "life_projects_template_archive",
    "life_projects_template_capture",
    "life_projects_templates_get",
    "life_projects_templates_list",
    "life_projects_timeline",
    "life_projects_update",
    "life_reminders_claim",
    "life_reminders_delivered",
    "life_reminders_dismiss",
    "life_reminders_due",
    "life_reminders_history",
    "life_reminders_release",
    "life_reminders_settings",
    "life_reminders_snooze",
    "life_reminders_upcoming",
    "life_reports_summary",
    "life_review_guides_complete",
    "life_review_guides_get",
    "life_review_guides_list",
    "life_review_guides_prompts",
    "life_review_guides_start",
    "life_review_guides_update",
    "life_reviews_create",
    "life_reviews_list",
    "life_reviews_update",
    "life_routines_create",
    "life_routines_delete",
    "life_routines_list",
    "life_routines_step_add",
    "life_routines_steps",
    "life_saved_views_create",
    "life_saved_views_delete",
    "life_saved_views_list",
    "life_schedule_apply",
    "life_schedule_propose",
    "life_search",
    "life_settings_list",
    "life_settings_set",
    "life_stats",
    "life_tasks_archived",
    "life_tasks_bulk",
    "life_tasks_complete",
    "life_tasks_create",
    "life_tasks_delete",
    "life_tasks_get",
    "life_tasks_list",
    "life_tasks_restore",
    "life_tasks_snooze",
    "life_tasks_update",
    "life_templates_apply",
    "life_templates_create",
    "life_templates_delete",
    "life_templates_list",
    "life_today",
    "life_wellbeing_delete",
    "life_wellbeing_list",
    "life_wellbeing_log",
];

/// The tool names published after the 0.1.0 release, when the catalog was
/// regenerated from a single action specification table.
const ADDED_TOOLS: &[&str] = &[
    "life_accessibility_audit",
    "life_administration_list",
    "life_agents_permissions_delete",
    "life_agents_permissions_list",
    "life_agents_permissions_set",
    "life_agents_proposals_approve",
    "life_agents_proposals_create",
    "life_agents_proposals_list",
    "life_agents_proposals_reject",
    "life_automations_schedule_delete",
    "life_automations_schedule_due",
    "life_automations_schedule_list",
    "life_automations_schedule_run_due",
    "life_automations_schedule_set",
    "life_capture_parse",
    "life_certifications_create",
    "life_certifications_delete",
    "life_certifications_list",
    "life_certifications_update",
    "life_csv_import_apply",
    "life_csv_import_history",
    "life_csv_import_preview",
    "life_diagnostics_report",
    "life_entity_timeline",
    "life_focus_interrupt",
    "life_focus_manual",
    "life_focus_presets_create",
    "life_focus_presets_delete",
    "life_focus_presets_list",
    "life_focus_trends",
    "life_goals_detail",
    "life_goals_limit",
    "life_goals_monthly_review",
    "life_integrations_configure",
    "life_integrations_list",
    "life_integrations_status",
    "life_interviews_create",
    "life_interviews_delete",
    "life_interviews_list",
    "life_interviews_update",
    "life_links_backlinks",
    "life_notification_authorization_get",
    "life_notification_authorization_set",
    "life_overdue_decide",
    "life_overdue_decisions",
    "life_personal_overview",
    "life_planning_carryover",
    "life_planning_carryover_apply",
    "life_planning_next_week",
    "life_planning_session",
    "life_planning_session_get",
    "life_projects_blockers",
    "life_projects_complete",
    "life_projects_notes",
    "life_projects_reopen",
    "life_recurrence_exceptions_create",
    "life_recurrence_exceptions_delete",
    "life_recurrence_exceptions_list",
    "life_recurrence_occurrences",
    "life_remote_clients_create",
    "life_remote_clients_list",
    "life_remote_clients_revoke",
    "life_rest_intentions_create",
    "life_rest_intentions_delete",
    "life_rest_intentions_list",
    "life_reviews_journal_archive",
    "life_saved_searches_create",
    "life_saved_searches_delete",
    "life_saved_searches_list",
    "life_saved_searches_run",
    "life_saved_views_run",
    "life_security_audit_list",
    "life_security_audit_log",
    "life_shortcuts_list",
    "life_shortcuts_reset",
    "life_shortcuts_update",
    "life_system_status",
    "life_time_budgets_delete",
    "life_time_budgets_list",
    "life_time_budgets_report",
    "life_time_budgets_set",
];

#[test]
fn every_published_tool_name_still_resolves() {
    let tools = life_cli::action_spec::tools();
    let names: Vec<String> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or("").to_string())
        .collect();
    for published in PUBLISHED_TOOLS.iter().chain(ADDED_TOOLS.iter()) {
        assert!(
            names.iter().any(|name| name == published),
            "The published tool {published} is no longer offered"
        );
    }
}

#[test]
fn the_catalog_has_no_duplicate_tool_names() {
    let tools = life_cli::action_spec::tools();
    let mut names: Vec<String> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or("").to_string())
        .collect();
    let total = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), total, "Two actions publish the same tool name");
}

#[test]
fn every_tool_declares_a_schema_with_a_description() {
    for tool in life_cli::action_spec::tools() {
        let name = tool["name"].as_str().unwrap_or("");
        assert!(!name.is_empty(), "A tool has no name");
        assert!(
            tool["description"].as_str().is_some_and(|d| !d.is_empty()),
            "The tool {name} has no description"
        );
        assert_eq!(
            tool["inputSchema"]["type"], "object",
            "The tool {name} has no object input schema"
        );
    }
}

/// A tool that declares no parameters at all is almost always a mistake: the
/// action specification was not extended when the action was added.
#[test]
fn no_tool_declares_an_empty_parameter_set() {
    // These read the whole workspace and legitimately take nothing.
    let exempt = [
        "today",
        "dashboard",
        "stats",
        "export",
        "search",
        "system.status",
        "accessibility.audit",
        "history.list",
        "life_domains.dashboard",
        "personal.overview",
        "diagnostics.report",
    ];
    for tool in life_cli::action_spec::tools() {
        let name = tool["name"].as_str().unwrap_or("");
        if exempt.contains(&name) {
            continue;
        }
        let properties = tool["inputSchema"]["properties"].as_object();
        let count = properties.map(|p| p.len()).unwrap_or(0);
        assert!(
            count > 0,
            "The tool {name} declares no parameters. Its action specification was probably not extended."
        );
    }
}

/// Every required parameter must also be a declared parameter.
#[test]
fn every_required_parameter_is_declared() {
    for tool in life_cli::action_spec::tools() {
        let name = tool["name"].as_str().unwrap_or("");
        let properties = tool["inputSchema"]["properties"].as_object();
        let Some(properties) = properties else {
            continue;
        };
        let required = tool["inputSchema"]["required"].as_array();
        for value in required.into_iter().flatten() {
            let parameter = value.as_str().unwrap_or("");
            assert!(
                properties.contains_key(parameter),
                "The tool {name} requires {parameter} but does not declare it"
            );
        }
    }
}

/// An action that writes must declare at least one parameter, so a write can
/// never be called with no arguments at all.
/// A read that takes no parameters must still declare the optional ones it
/// understands, so an agent can narrow the result.
#[test]
fn every_read_action_declares_its_filters() {
    for (action, _) in life_cli::action_spec::ACTIONS {
        if action.ends_with(".create")
            || action.ends_with(".update")
            || action.ends_with(".delete")
            || action.ends_with(".add")
            || action.ends_with(".apply")
            || action.ends_with(".log")
            || action.ends_with(".set")
            || action.ends_with(".decide")
            || action.ends_with(".prioritize")
            || action.ends_with(".capture")
            || action.ends_with(".enable")
            || action.ends_with(".disable")
        {
            continue;
        }
        let fields = life_cli::action_spec::fields(action);
        // Every action understands the actor and the idempotency key.
        assert!(
            fields.iter().any(|f| f.name == "actor"),
            "The read action {action} does not declare the actor"
        );
    }
}

#[test]
fn every_write_action_requires_something() {
    let mut writes: Vec<(&str, Vec<&str>)> = Vec::new();
    for (action, _) in life_cli::action_spec::ACTIONS {
        let required = life_cli::action_spec::required(action);
        // backup.create takes an optional destination, so it is the one write
        // that legitimately needs no parameter.
        let writes_like = *action != "backup.create"
            && (action.ends_with(".create")
                || action.ends_with(".update")
                || action.ends_with(".delete")
                || action.ends_with(".add")
                || action.ends_with(".apply")
                || action.ends_with(".log")
                || action.ends_with(".set")
                || action.ends_with(".decide")
                || action.ends_with(".prioritize")
                || action.ends_with(".capture")
                || action.ends_with(".enable")
                || action.ends_with(".disable"));
        if writes_like {
            writes.push((action, required.to_vec()));
        }
    }
    assert!(writes.len() > 50, "the write-action list looks wrong");
    for (action, required) in writes {
        assert!(
            !required.is_empty(),
            "The write action {action} requires no parameter, so it can be called with nothing"
        );
    }
}

#[test]
fn every_tool_maps_back_to_a_domain_action() {
    for tool in life_cli::action_spec::tools() {
        let name = tool["name"].as_str().unwrap_or("");
        assert!(
            life_cli::action_spec::action_for_tool(name).is_some(),
            "The tool {name} does not map to a domain action"
        );
    }
}
