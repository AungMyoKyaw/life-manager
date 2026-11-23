//! Native GPUI desktop application for Life Manager, backed directly by `life_core::Store`.
extern crate gpui_kit as gpui;

pub mod models;
pub mod panels;
pub mod panels_extra;
pub mod tokens;
mod views;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc};
use gpui_kit::{
    AppContext as _, Bounds, Context, Entity, WeakEntity, Window, WindowBounds, WindowOptions,
    component::{
        input::{InputEvent, InputState},
        theme::{Theme as KitTheme, ThemeMode as KitThemeMode},
    },
    px, size,
};
use models::{
    BackendHandle, RoutineStepRecord, ScheduleProposal, TaskFilter, TaskRecord, TaskSort,
    WorkspaceSnapshot, WorkspaceView,
};
use serde_json::{Value, json};
use std::path::PathBuf;
use tokens::{Palette, ThemeMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarMode {
    Day,
    ThreeDays,
    Week,
    Month,
    Agenda,
}

impl CalendarMode {
    pub const ALL: &'static [CalendarMode] = &[
        Self::Day,
        Self::ThreeDays,
        Self::Week,
        Self::Month,
        Self::Agenda,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Day => "Day",
            Self::ThreeDays => "3 Days",
            Self::Week => "Week",
            Self::Month => "Month",
            Self::Agenda => "Agenda",
        }
    }
}

/// Retained sidebar entity so scrolling the main workspace never re-renders the sidebar.
pub struct SidebarPane {
    pub app: WeakEntity<LifeManagerApp>,
    pub view: WorkspaceView,
    pub theme_mode: ThemeMode,
    pub sidebar_open: bool,
    pub inbox_count: usize,
}

/// Retained topbar entity so scrolling the main workspace never re-renders the topbar.
pub struct TopbarPane {
    pub app: WeakEntity<LifeManagerApp>,
    pub view: WorkspaceView,
    pub theme_mode: ThemeMode,
    pub inbox_count: usize,
}

pub struct LifeManagerApp {
    pub backend: BackendHandle,
    pub locked: bool,
    pub snapshot: WorkspaceSnapshot,
    pub view: WorkspaceView,
    pub theme_mode: ThemeMode,
    pub sidebar_open: bool,
    pub compact_density: bool,
    pub task_filter: TaskFilter,
    pub task_sort: TaskSort,
    pub calendar_mode: CalendarMode,
    pub calendar_offset_days: i64,
    pub selected_area_id: Option<String>,
    pub selected_project_id: Option<String>,
    pub selected_task_id: Option<String>,
    pub selected_note_id: Option<String>,
    pub selected_routine_id: Option<String>,
    pub routine_steps: Vec<RoutineStepRecord>,
    pub selected_focus_task_id: Option<String>,
    pub draft_priority: i64,
    pub schedule_proposal: Option<ScheduleProposal>,
    pub command_palette_open: bool,
    pub search_tasks: Vec<TaskRecord>,
    pub notice: Option<String>,
    pub error: Option<String>,
    pub last_backup_path: Option<String>,
    pub last_export_path: Option<String>,
    pub review_period: String,
    pub onboarding_dismissed: bool,
    pub onboarding_step: usize,
    pub quick_capture_open: bool,
    pub capture_preview: Option<Value>,
    pub bulk_selected_ids: Vec<String>,
    pub selected_task_timeline: Option<Value>,
    pub projects_tab: usize,
    pub goals_tab: usize,
    pub areas_tab: usize,
    pub settings_tab: usize,
    pub agents_tab: usize,
    pub active_guided_review_id: Option<String>,
    pub guided_review_kind: String,
    pub csv_preview: Option<Value>,
    pub ics_preview: Option<Value>,
    pub report_cache: Option<Value>,
    pub last_accessibility_audit: Option<Value>,
    // Retained child chrome views (cached by GPUI during main scroll frames)
    pub sidebar_pane: Entity<SidebarPane>,
    pub topbar_pane: Entity<TopbarPane>,
    // Pre-computed derived view slices so `Render::render` never sorts/clones during scroll frames
    pub cached_filtered_tasks: Vec<TaskRecord>,
    pub cached_today_priority_tasks: Vec<TaskRecord>,
    pub cached_open_count: usize,
    pub cached_urgent_count: usize,
    pub cached_today_due_count: usize,
    pub cached_inbox_count: usize,
    pub cached_completed_habits: usize,
    // Input states
    pub quick_task_input: Entity<InputState>,
    pub capture_parse_input: Entity<InputState>,
    pub note_title_input: Entity<InputState>,
    pub note_body_input: Entity<InputState>,
    pub project_name_input: Entity<InputState>,
    pub milestone_title_input: Entity<InputState>,
    pub goal_title_input: Entity<InputState>,
    pub key_result_title_input: Entity<InputState>,
    pub habit_title_input: Entity<InputState>,
    pub routine_title_input: Entity<InputState>,
    pub routine_step_input: Entity<InputState>,
    pub event_title_input: Entity<InputState>,
    pub protected_title_input: Entity<InputState>,
    pub area_name_input: Entity<InputState>,
    pub career_role_input: Entity<InputState>,
    pub learning_title_input: Entity<InputState>,
    pub finance_title_input: Entity<InputState>,
    pub interview_title_input: Entity<InputState>,
    pub savings_title_input: Entity<InputState>,
    pub cert_title_input: Entity<InputState>,
    pub rest_title_input: Entity<InputState>,
    pub admin_title_input: Entity<InputState>,
    pub review_title_input: Entity<InputState>,
    pub review_body_input: Entity<InputState>,
    pub guided_answer_input: Entity<InputState>,
    pub automation_title_input: Entity<InputState>,
    pub template_name_input: Entity<InputState>,
    pub remote_client_input: Entity<InputState>,
    pub csv_input: Entity<InputState>,
    pub attachment_path_input: Entity<InputState>,
    pub palette_input: Entity<InputState>,
    pub passphrase_input: Entity<InputState>,
}

impl LifeManagerApp {
    pub fn new(db_path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let weak_self = cx.weak_entity();
        let sidebar_pane = cx.new(|_| SidebarPane {
            app: weak_self.clone(),
            view: WorkspaceView::Today,
            theme_mode: ThemeMode::Light,
            sidebar_open: true,
            inbox_count: 0,
        });
        let topbar_pane = cx.new(|_| TopbarPane {
            app: weak_self,
            view: WorkspaceView::Today,
            theme_mode: ThemeMode::Light,
            inbox_count: 0,
        });

        let quick_task_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Capture a task, thought, or commitment...")
        });
        let capture_parse_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Natural capture: e.g. Ship v16 release tomorrow urgent #gpui")
        });
        let note_title_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Note title..."));
        let note_body_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Write markdown note, idea, or backlink...")
        });
        let project_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("New project name..."));
        let milestone_title_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("New project milestone title..."));
        let goal_title_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("New goal outcome..."));
        let key_result_title_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Add measurable key result (OKR)...")
        });
        let habit_title_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("New daily or weekly habit..."));
        let routine_title_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("New routine title..."));
        let routine_step_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Add a routine step..."));
        let event_title_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("New calendar commitment..."));
        let protected_title_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Protected block title (e.g. Deep Work)...")
        });
        let area_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("New life area name..."));
        let career_role_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Employer — Role (e.g. Acme — Staff Engineer)...")
        });
        let learning_title_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("New learning track or course..."));
        let finance_title_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("New financial reminder or commitment...")
        });
        let interview_title_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Interview title (e.g. Acme Architecture Round)...")
        });
        let savings_title_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Savings goal title (e.g. Emergency Reserve)...")
        });
        let cert_title_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Certification title (e.g. AWS SA Pro)...")
        });
        let rest_title_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Rest / sleep intention (e.g. Screens off 22:30)...")
        });
        let admin_title_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Personal admin item (e.g. Passport renewal)...")
        });
        let review_title_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Reflection title..."));
        let review_body_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("What moved forward? What needs adjustment?")
        });
        let guided_answer_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Write your guided reflection response...")
        });
        let automation_title_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("New local automation rule title...")
        });
        let template_name_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("New reusable task template title...")
        });
        let remote_client_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Remote MCP client name (e.g. Claude Desktop)...")
        });
        let csv_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Paste CSV rows (title,priority,due_date)...")
        });
        let attachment_path_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Local file path to attach (content-hashed)...")
        });
        let palette_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search tasks, projects, notes, or jump to...")
        });
        let passphrase_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Database passphrase (min 8 chars)...")
                .masked(true)
        });

        cx.subscribe_in(
            &capture_parse_input,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let raw = state.read(cx).value().trim().to_string();
                    if raw.is_empty() {
                        this.capture_preview = None;
                    } else if let Ok(parsed) =
                        this.backend.call("capture.parse", &json!({ "text": raw }))
                    {
                        this.capture_preview = Some(parsed);
                    }
                    cx.notify();
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &quick_task_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.submit_quick_task(window, cx);
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &palette_input,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let query = state.read(cx).value().to_string();
                    this.run_palette_search(&query);
                    cx.notify();
                }
            },
        )
        .detach();

        let backend = BackendHandle::new(db_path);
        let mut app = Self {
            backend,
            locked: false,
            snapshot: WorkspaceSnapshot::default(),
            view: WorkspaceView::Today,
            theme_mode: ThemeMode::Light,
            sidebar_open: true,
            compact_density: false,
            task_filter: TaskFilter::Open,
            task_sort: TaskSort::Priority,
            calendar_mode: CalendarMode::Week,
            calendar_offset_days: 0,
            selected_area_id: None,
            selected_project_id: None,
            selected_task_id: None,
            selected_note_id: None,
            selected_routine_id: None,
            routine_steps: Vec::new(),
            selected_focus_task_id: None,
            draft_priority: 2,
            schedule_proposal: None,
            command_palette_open: false,
            search_tasks: Vec::new(),
            notice: None,
            error: None,
            last_backup_path: None,
            last_export_path: None,
            review_period: "daily".to_string(),
            onboarding_dismissed: false,
            onboarding_step: 0,
            quick_capture_open: false,
            capture_preview: None,
            bulk_selected_ids: Vec::new(),
            selected_task_timeline: None,
            projects_tab: 0,
            goals_tab: 0,
            areas_tab: 0,
            settings_tab: 0,
            agents_tab: 0,
            active_guided_review_id: None,
            guided_review_kind: "morning".to_string(),
            csv_preview: None,
            ics_preview: None,
            report_cache: None,
            last_accessibility_audit: None,
            sidebar_pane,
            topbar_pane,
            cached_filtered_tasks: Vec::new(),
            cached_today_priority_tasks: Vec::new(),
            cached_open_count: 0,
            cached_urgent_count: 0,
            cached_today_due_count: 0,
            cached_inbox_count: 0,
            cached_completed_habits: 0,
            quick_task_input,
            capture_parse_input,
            note_title_input,
            note_body_input,
            project_name_input,
            milestone_title_input,
            goal_title_input,
            key_result_title_input,
            habit_title_input,
            routine_title_input,
            routine_step_input,
            event_title_input,
            protected_title_input,
            area_name_input,
            career_role_input,
            learning_title_input,
            finance_title_input,
            interview_title_input,
            savings_title_input,
            cert_title_input,
            rest_title_input,
            admin_title_input,
            review_title_input,
            review_body_input,
            guided_answer_input,
            automation_title_input,
            template_name_input,
            remote_client_input,
            csv_input,
            attachment_path_input,
            palette_input,
            passphrase_input,
        };
        app.reload();
        app.sync_chrome(cx);

        // 1-second background ticker that ONLY triggers a re-render when an active focus session is running
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
                let keep_going = this
                    .update(cx, |app, cx| {
                        if app.has_active_focus() {
                            cx.notify();
                        }
                    })
                    .is_ok();
                if !keep_going {
                    break;
                }
            }
        })
        .detach();

        app
    }

    pub fn has_active_focus(&self) -> bool {
        self.snapshot.focus.iter().any(|f| f.status == "active")
    }

    pub fn active_focus_elapsed_text(&self, started_at: &str, fallback_secs: i64) -> String {
        let secs = DateTime::parse_from_rfc3339(started_at)
            .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_seconds().max(0))
            .unwrap_or(fallback_secs.max(0));
        format!("{:02}m {:02}s", secs / 60, secs % 60)
    }

    pub fn palette(&self) -> Palette {
        Palette::for_mode(self.theme_mode)
    }

    pub fn sync_chrome(&self, cx: &mut Context<Self>) {
        let view = self.view;
        let theme_mode = self.theme_mode;
        let sidebar_open = self.sidebar_open;
        let inbox_count = self.cached_inbox_count;

        self.sidebar_pane.update(cx, |pane, cx| {
            if pane.view != view
                || pane.theme_mode != theme_mode
                || pane.sidebar_open != sidebar_open
                || pane.inbox_count != inbox_count
            {
                pane.view = view;
                pane.theme_mode = theme_mode;
                pane.sidebar_open = sidebar_open;
                pane.inbox_count = inbox_count;
                cx.notify();
            }
        });
        self.topbar_pane.update(cx, |pane, cx| {
            if pane.view != view || pane.theme_mode != theme_mode || pane.inbox_count != inbox_count
            {
                pane.view = view;
                pane.theme_mode = theme_mode;
                pane.inbox_count = inbox_count;
                cx.notify();
            }
        });
    }

    pub fn reload(&mut self) {
        match self.backend.snapshot() {
            Ok(snapshot) => {
                self.locked = false;
                self.snapshot = snapshot;
                self.error = None;
                if let Some(routine_id) = &self.selected_routine_id
                    && let Ok(val) = self
                        .backend
                        .call("routines.steps", &json!({ "routine_id": routine_id }))
                {
                    self.routine_steps = serde_json::from_value(val).unwrap_or_default();
                }
                self.recompute_derived();
            }
            Err(err) => {
                let msg = err.to_string();
                if msg.to_lowercase().contains("passphrase")
                    || msg.to_lowercase().contains("encrypted")
                {
                    self.locked = true;
                }
                self.error = Some(msg);
            }
        }
    }

    pub fn recompute_derived(&mut self) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let mut open: Vec<TaskRecord> = self
            .snapshot
            .tasks
            .iter()
            .filter(|t| t.status != "done" && t.status != "cancelled")
            .cloned()
            .collect();
        self.cached_open_count = open.len();
        self.cached_urgent_count = open.iter().filter(|t| t.priority <= 2).count();
        self.cached_today_due_count = open
            .iter()
            .filter(|t| {
                t.due_date
                    .as_deref()
                    .is_some_and(|d| d.get(..10).unwrap_or(d) <= today.as_str())
                    || t.planned_date
                        .as_deref()
                        .is_some_and(|d| d.get(..10).unwrap_or(d) == today.as_str())
            })
            .count();
        self.cached_inbox_count = self
            .snapshot
            .tasks
            .iter()
            .filter(|t| t.status == "inbox")
            .count();
        self.cached_completed_habits = self
            .snapshot
            .habits
            .iter()
            .filter(|h| h.today_value.unwrap_or(0) > 0)
            .count();

        open.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.title.cmp(&b.title))
        });
        open.truncate(6);
        self.cached_today_priority_tasks = open;
        self.cached_filtered_tasks = self.compute_filtered_tasks();
    }

    pub fn exec(
        &mut self,
        action: &str,
        payload: Value,
        notice: impl Into<String>,
    ) -> Option<Value> {
        match self.backend.call(action, &payload) {
            Ok(output) => {
                self.reload();
                self.notice = Some(notice.into());
                self.error = None;
                Some(output)
            }
            Err(err) => {
                self.error = Some(err.to_string());
                None
            }
        }
    }

    pub fn switch_view(&mut self, next: WorkspaceView, cx: &mut Context<Self>) {
        self.view = next;
        self.command_palette_open = false;
        match next {
            WorkspaceView::Inbox => {
                self.task_filter = TaskFilter::Inbox;
                self.cached_filtered_tasks = self.compute_filtered_tasks();
            }
            WorkspaceView::Tasks => {
                self.task_filter = TaskFilter::Open;
                self.cached_filtered_tasks = self.compute_filtered_tasks();
            }
            _ => {}
        }
        self.sync_chrome(cx);
        cx.notify();
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        self.sync_chrome(cx);
        cx.notify();
    }

    pub fn set_task_filter(&mut self, filter: TaskFilter, cx: &mut Context<Self>) {
        self.task_filter = filter;
        self.cached_filtered_tasks = self.compute_filtered_tasks();
        cx.notify();
    }

    pub fn cycle_task_sort(&mut self, cx: &mut Context<Self>) {
        self.task_sort = self.task_sort.next();
        self.cached_filtered_tasks = self.compute_filtered_tasks();
        cx.notify();
    }

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        self.theme_mode = match self.theme_mode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
        let kit_mode = match self.theme_mode {
            ThemeMode::Light => KitThemeMode::Light,
            ThemeMode::Dark => KitThemeMode::Dark,
        };
        KitTheme::change(kit_mode, None, cx);
        self.sync_chrome(cx);
        cx.notify();
    }

    pub fn submit_quick_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.quick_task_input.read(cx).value().trim().to_string();
        if title.is_empty() {
            return;
        }
        let status = if self.view == WorkspaceView::Inbox {
            "inbox"
        } else {
            "todo"
        };
        if self
            .exec(
                "tasks.create",
                json!({
                    "title": title,
                    "status": status,
                    "priority": self.draft_priority,
                    "project_id": self.selected_project_id,
                    "area_id": self.selected_area_id,
                }),
                "Task captured",
            )
            .is_some()
        {
            self.quick_task_input.update(cx, |state, cx| {
                state.set_value("", window, cx);
            });
            self.sync_chrome(cx);
        }
        cx.notify();
    }

    pub fn toggle_task_complete(&mut self, task: &TaskRecord, cx: &mut Context<Self>) {
        if task.status == "done" {
            self.exec(
                "tasks.update",
                json!({ "id": task.id, "status": "todo" }),
                "Task reopened",
            );
        } else {
            self.exec("tasks.complete", json!({ "id": task.id }), "Task completed");
        }
        self.sync_chrome(cx);
        cx.notify();
    }

    pub fn delete_task(&mut self, task_id: &str, cx: &mut Context<Self>) {
        self.exec("tasks.delete", json!({ "id": task_id }), "Task archived");
        if self.selected_task_id.as_deref() == Some(task_id) {
            self.selected_task_id = None;
        }
        self.sync_chrome(cx);
        cx.notify();
    }

    pub fn open_tasks(&self) -> Vec<&TaskRecord> {
        self.snapshot
            .tasks
            .iter()
            .filter(|t| t.status != "done" && t.status != "cancelled")
            .collect()
    }

    pub fn inbox_tasks(&self) -> Vec<&TaskRecord> {
        self.snapshot
            .tasks
            .iter()
            .filter(|t| t.status == "inbox")
            .collect()
    }

    pub fn filtered_tasks(&self) -> &[TaskRecord] {
        &self.cached_filtered_tasks
    }

    fn compute_filtered_tasks(&self) -> Vec<TaskRecord> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let mut list: Vec<TaskRecord> = self
            .snapshot
            .tasks
            .iter()
            .filter(|t| match self.task_filter {
                TaskFilter::Inbox => t.status == "inbox",
                TaskFilter::Open => t.status != "done" && t.status != "cancelled",
                TaskFilter::Today => {
                    t.status != "done"
                        && t.status != "cancelled"
                        && (t
                            .due_date
                            .as_deref()
                            .is_some_and(|d| d.get(..10).unwrap_or(d) <= today.as_str())
                            || t.planned_date
                                .as_deref()
                                .is_some_and(|d| d.get(..10).unwrap_or(d) == today.as_str()))
                }
                TaskFilter::All => true,
                TaskFilter::Completed => t.status == "done",
            })
            .filter(|t| {
                self.selected_project_id
                    .as_ref()
                    .is_none_or(|pid| t.project_id.as_ref() == Some(pid))
            })
            .filter(|t| {
                self.selected_area_id
                    .as_ref()
                    .is_none_or(|aid| t.area_id.as_ref() == Some(aid))
            })
            .cloned()
            .collect();

        list.sort_by(|a, b| match self.task_sort {
            TaskSort::Priority => a
                .priority
                .cmp(&b.priority)
                .then_with(|| a.title.cmp(&b.title)),
            TaskSort::DueDate => a
                .due_date
                .as_deref()
                .unwrap_or("9999-12-31")
                .cmp(b.due_date.as_deref().unwrap_or("9999-12-31")),
            TaskSort::Name => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        });
        list
    }

    pub fn calendar_reference_date(&self) -> NaiveDate {
        Local::now().date_naive() + Duration::days(self.calendar_offset_days)
    }

    pub fn calendar_visible_days(&self) -> Vec<NaiveDate> {
        let reference = self.calendar_reference_date();
        match self.calendar_mode {
            CalendarMode::Day | CalendarMode::Agenda => vec![reference],
            CalendarMode::ThreeDays => (0..3).map(|i| reference + Duration::days(i)).collect(),
            CalendarMode::Week | CalendarMode::Month => {
                let weekday = reference.weekday().num_days_from_monday() as i64;
                let monday = reference - Duration::days(weekday);
                (0..7).map(|i| monday + Duration::days(i)).collect()
            }
        }
    }

    pub fn shift_calendar(&mut self, direction: i64, cx: &mut Context<Self>) {
        let step = match self.calendar_mode {
            CalendarMode::Day | CalendarMode::Agenda => 1,
            CalendarMode::ThreeDays => 3,
            CalendarMode::Week => 7,
            CalendarMode::Month => 28,
        };
        self.calendar_offset_days += direction * step;
        cx.notify();
    }

    pub fn run_palette_search(&mut self, query: &str) {
        let q = query.trim();
        if q.is_empty() {
            self.search_tasks.clear();
            return;
        }
        if let Ok(val) = self.backend.call("search", &json!({ "query": q }))
            && let Some(tasks) = val.get("tasks")
        {
            self.search_tasks = serde_json::from_value(tasks.clone()).unwrap_or_default();
        }
    }

    pub fn unlock_with_passphrase(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pass = self.passphrase_input.read(cx).value().to_string();
        if pass.is_empty() {
            self.error = Some("Enter your database passphrase".into());
            cx.notify();
            return;
        }
        self.backend.set_passphrase(Some(pass));
        self.reload();
        if !self.locked {
            self.passphrase_input.update(cx, |s, cx| {
                s.set_value("", window, cx);
            });
            self.notice = Some("Database unlocked".into());
            self.sync_chrome(cx);
        }
        cx.notify();
    }
}

pub fn default_window_options() -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: gpui_kit::point(px(80.0), px(60.0)),
            size: size(px(tokens::CANVAS_WIDTH), px(tokens::CANVAS_HEIGHT)),
        })),
        ..WindowOptions::default()
    }
}

pub fn run() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(|cx| {
            gpui_kit::init(cx);
            let db_path = life_core::default_db_path();
            gpui_kit::open_window(default_window_options(), cx, move |window, cx| {
                cx.new(|cx| LifeManagerApp::new(db_path, window, cx))
            })
            .expect("failed to open Life Manager GPUI window");
        });
}
