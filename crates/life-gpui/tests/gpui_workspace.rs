//! Headless GPUI and `life-core` integration tests for `life-gpui`.
extern crate gpui_kit as gpui;

use gpui_kit::{AppContext as _, TestAppContext, WindowOptions, test::TestWindowExt as _};
use life_gpui::{
    LifeManagerApp,
    models::{BackendHandle, TaskFilter, WorkspaceView},
    tokens::{DARK, LIGHT, Palette, ThemeMode},
};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn design_tokens_match_figcraft_master() {
    assert_eq!(LIGHT.canvas, 0xF6F7F5);
    assert_eq!(LIGHT.accent, 0x266C56);
    assert_eq!(DARK.canvas, 0x112124);
    assert_eq!(DARK.accent, 0x419677);
    assert_eq!(Palette::for_mode(ThemeMode::Light).accent, 0x266C56);
    assert_eq!(Palette::for_mode(ThemeMode::Dark).accent, 0x419677);
}

#[test]
fn backend_handle_executes_core_domain_workflow() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("life-gpui-test.sqlite");
    let backend = BackendHandle::new(db_path);

    let created = backend
        .call(
            "tasks.create",
            &json!({
                "title": "Migrate UI to GPUI",
                "status": "todo",
                "priority": 1,
                "estimate_minutes": 45
            }),
        )
        .expect("create task");
    let task_id = created["id"].as_str().expect("task id").to_string();

    let snap = backend.snapshot().expect("snapshot");
    assert_eq!(snap.tasks.len(), 1);
    assert_eq!(snap.tasks[0].title, "Migrate UI to GPUI");
    assert_eq!(snap.stats.open_tasks, 1);

    backend
        .call("tasks.complete", &json!({ "id": task_id }))
        .expect("complete task");
    let snap = backend.snapshot().expect("snapshot after complete");
    assert_eq!(snap.stats.completed_tasks, 1);
}

#[gpui_kit::test]
fn headless_gpui_workspace_renders_and_mutates_store(cx: &mut TestAppContext) {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("headless-gpui.sqlite");

    cx.update(|cx| {
        gpui_kit::init(cx);
    });

    let (window, app) = cx
        .update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| LifeManagerApp::new(db_path.clone(), window, cx))
            })
        })
        .expect("open headless GPUI window");

    // Verify initial state and create a task through the GPUI view state
    app.update(cx, |this, cx| {
        assert!(!this.locked);
        assert_eq!(this.view, WorkspaceView::Today);
        this.exec(
            "tasks.create",
            json!({
                "title": "Verify GPUI window rendering · မြန်မာစာ အလုပ်စာရင်း စမ်းသပ်မှု",
                "status": "inbox",
                "priority": 1
            }),
            "Created in test",
        );
        assert_eq!(this.snapshot.tasks.len(), 1);
        assert_eq!(this.inbox_tasks().len(), 1);

        // Cycle through all 18 workspace views and render each frame in the headless window
        for target_view in WorkspaceView::ALL {
            this.switch_view(*target_view, cx);
            assert_eq!(this.view, *target_view);
        }

        // Exercise Planning, Goals/OKRs, Library templates/lenses, Agents proposals, and Life Domains
        let goal = this
            .exec(
                "goals.create",
                json!({ "title": "Ship public GPUI release", "target": 100, "progress": 0 }),
                "Goal created",
            )
            .expect("create goal");
        let gid = goal["id"].as_str().expect("goal id").to_string();
        this.exec(
            "key_results.create",
            json!({
                "goal_id": gid,
                "title": "All 18 views tested",
                "baseline": 0.0,
                "target": 100.0,
                "unit": "%"
            }),
            "KR created",
        );
        assert_eq!(this.snapshot.key_results.len(), 1);

        this.exec(
            "protected.create",
            json!({
                "title": "Deep work block",
                "weekday": 0,
                "start_time": "09:00",
                "end_time": "11:00",
                "kind": "protected"
            }),
            "Protected block created",
        );
        assert_eq!(this.snapshot.protected_blocks.len(), 1);

        this.exec(
            "templates.create",
            json!({
                "name": "Daily standup prep",
                "title": "Prepare standup summary",
                "priority": 2,
                "estimate_minutes": 15
            }),
            "Template created",
        );
        assert_eq!(this.snapshot.templates.len(), 1);

        this.exec(
            "saved_views.create",
            json!({
                "name": "P1 Queue",
                "entity": "tasks",
                "query": { "priority": 1 }
            }),
            "Saved view created",
        );
        assert_eq!(this.snapshot.saved_views.len(), 1);

        this.exec(
            "agents.proposals.create",
            json!({
                "actor": "mcp-test",
                "action": "tasks.create",
                "payload": { "title": "Proposed by MCP agent", "priority": 2 }
            }),
            "Proposal staged",
        );
        assert_eq!(this.snapshot.agent_proposals.len(), 1);

        this.exec(
            "career.create",
            json!({ "employer": "Acme", "role": "Principal Engineer", "stage": "applied" }),
            "Career added",
        );
        assert_eq!(this.snapshot.career.len(), 1);

        this.exec(
            "wellbeing.log",
            json!({ "energy": 5, "mood": 5, "sleep_hours": 8.0 }),
            "Wellbeing logged",
        );
        assert_eq!(this.snapshot.wellbeing.len(), 1);

        // Verify filter and theme toggling
        this.switch_view(WorkspaceView::Inbox, cx);
        assert_eq!(this.task_filter, TaskFilter::Inbox);
        assert_eq!(this.filtered_tasks().len(), 1);

        // Start and stop a focus session
        this.exec("focus.start", json!({}), "Focus started");
        assert_eq!(this.snapshot.focus.len(), 1);
        let focus_id = this.snapshot.focus[0].id.clone();
        this.exec("focus.stop", json!({ "id": focus_id }), "Focus stopped");

        // Toggle task completion
        let task = this.snapshot.tasks[0].clone();
        this.toggle_task_complete(&task, cx);
        assert_eq!(this.snapshot.stats.completed_tasks, 1);

        this.toggle_theme(cx);
        assert_eq!(this.theme_mode, ThemeMode::Dark);
    });

    for target_view in WorkspaceView::ALL {
        cx.update_window(window, |_, window, cx| {
            app.update(cx, |this, cx| {
                this.switch_view(*target_view, cx);
            });
            window.render_frame(cx);
        })
        .expect("render headless frame for view");
    }
}

#[gpui_kit::test]
fn encrypted_workspace_locks_and_unlocks_in_gpui(cx: &mut TestAppContext) {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("encrypted-gpui.sqlite");

    // Create and encrypt the database with a passphrase
    {
        let mut store = life_core::Store::open(&db_path).expect("open initial store");
        store
            .call(
                "tasks.create",
                &json!({ "title": "Protected task", "status": "todo", "priority": 1 }),
            )
            .expect("seed task");
        store
            .call(
                "encryption.enable",
                &json!({ "mode": "passphrase", "passphrase": "correct-horse-battery" }),
            )
            .expect("encrypt database");
    }

    cx.update(|cx| {
        gpui_kit::init(cx);
    });

    let (window, app) = cx
        .update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
                cx.new(|cx| LifeManagerApp::new(db_path.clone(), window, cx))
            })
        })
        .expect("open locked GPUI window");

    // Without a passphrase, the app starts on the locked screen
    app.update(cx, |this, _cx| {
        assert!(this.locked);
        assert!(this.snapshot.tasks.is_empty());
    });

    // Supply passphrase and unlock
    cx.update_window(window, |_, window, cx| {
        app.update(cx, |this, cx| {
            this.passphrase_input.update(cx, |input, cx| {
                input.set_value("correct-horse-battery", window, cx);
            });
            this.unlock_with_passphrase(window, cx);
            assert!(!this.locked);
            assert_eq!(this.snapshot.tasks.len(), 1);
            assert_eq!(this.snapshot.tasks[0].title, "Protected task");
        });
        window.render_frame(cx);
    })
    .expect("unlock and render frame");
}
