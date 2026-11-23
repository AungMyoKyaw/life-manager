//! Typed workspace view state and `life-core` domain projections for `life-gpui`.
use gpui_kit::assets::IconName;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkspaceView {
    Today,
    Inbox,
    Tasks,
    Calendar,
    Projects,
    Goals,
    Planning,
    Areas,
    Focus,
    Habits,
    Routines,
    Notes,
    Reviews,
    Insights,
    Agents,
    Automations,
    Library,
    Settings,
}

impl WorkspaceView {
    pub const ALL: &'static [WorkspaceView] = &[
        Self::Today,
        Self::Inbox,
        Self::Tasks,
        Self::Calendar,
        Self::Projects,
        Self::Goals,
        Self::Planning,
        Self::Areas,
        Self::Focus,
        Self::Habits,
        Self::Routines,
        Self::Notes,
        Self::Reviews,
        Self::Insights,
        Self::Agents,
        Self::Automations,
        Self::Library,
        Self::Settings,
    ];

    pub const GROUPS: &'static [(&'static str, &'static [WorkspaceView])] = &[
        (
            "YOUR WORKSPACE",
            &[Self::Today, Self::Inbox, Self::Tasks, Self::Calendar],
        ),
        (
            "PLAN & TRACK",
            &[Self::Projects, Self::Goals, Self::Planning, Self::Areas],
        ),
        (
            "RHYTHM",
            &[
                Self::Focus,
                Self::Habits,
                Self::Routines,
                Self::Notes,
                Self::Reviews,
                Self::Insights,
            ],
        ),
        (
            "SYSTEM",
            &[
                Self::Agents,
                Self::Automations,
                Self::Library,
                Self::Settings,
            ],
        ),
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Inbox => "Inbox",
            Self::Tasks => "Tasks",
            Self::Calendar => "Calendar",
            Self::Projects => "Projects",
            Self::Goals => "Goals",
            Self::Planning => "Planning",
            Self::Areas => "Areas",
            Self::Focus => "Focus",
            Self::Habits => "Habits",
            Self::Routines => "Routines",
            Self::Notes => "Notes",
            Self::Reviews => "Reviews",
            Self::Insights => "Insights",
            Self::Agents => "Agents",
            Self::Automations => "Automations",
            Self::Library => "Library",
            Self::Settings => "Settings",
        }
    }

    pub const fn icon(self) -> IconName {
        match self {
            Self::Today => IconName::LayoutDashboard,
            Self::Inbox => IconName::Inbox,
            Self::Tasks => IconName::ListTodo,
            Self::Calendar => IconName::CalendarDays,
            Self::Projects => IconName::FolderKanban,
            Self::Goals => IconName::Target,
            Self::Planning => IconName::CalendarClock,
            Self::Areas => IconName::Layers,
            Self::Focus => IconName::Timer,
            Self::Habits => IconName::Repeat2,
            Self::Routines => IconName::Layers,
            Self::Notes => IconName::NotebookPen,
            Self::Reviews => IconName::ChartNoAxesCombined,
            Self::Insights => IconName::ChartNoAxesCombined,
            Self::Agents => IconName::Bot,
            Self::Automations => IconName::Zap,
            Self::Library => IconName::BookOpen,
            Self::Settings => IconName::Settings2,
        }
    }

    pub const fn nav_id(self) -> &'static str {
        match self {
            Self::Today => "nav-today",
            Self::Inbox => "nav-inbox",
            Self::Tasks => "nav-tasks",
            Self::Calendar => "nav-calendar",
            Self::Projects => "nav-projects",
            Self::Goals => "nav-goals",
            Self::Planning => "nav-planning",
            Self::Areas => "nav-areas",
            Self::Focus => "nav-focus",
            Self::Habits => "nav-habits",
            Self::Routines => "nav-routines",
            Self::Notes => "nav-notes",
            Self::Reviews => "nav-reviews",
            Self::Insights => "nav-insights",
            Self::Agents => "nav-agents",
            Self::Automations => "nav-automations",
            Self::Library => "nav-library",
            Self::Settings => "nav-settings",
        }
    }

    pub const fn eyebrow(self) -> &'static str {
        match self {
            Self::Today => "PERSONAL COMMAND CENTER",
            Self::Focus => "ONE THING AT A TIME",
            Self::Agents => "AUTOMATION & CONTROL",
            Self::Calendar => "TIME & COMMITMENTS",
            _ => "YOUR WORKSPACE",
        }
    }

    pub const fn heading(self) -> &'static str {
        match self {
            Self::Today => "Today, with intention.",
            other => other.label(),
        }
    }

    pub const fn subtitle(self) -> &'static str {
        match self {
            Self::Today => "See what matters, and leave space for the rest.",
            Self::Inbox => "Everything captured, nothing forgotten. Process when you're ready.",
            Self::Tasks => "Clear next actions, realistic priorities, meaningful progress.",
            Self::Calendar => "Protect the time you need to do your best work.",
            Self::Projects => "Your active commitments, all in one calm overview.",
            Self::Goals => "Keep your direction visible and your progress tangible.",
            Self::Planning => "Review your available time before committing to more.",
            Self::Areas => "Your commitments, gathered by the parts of life that matter.",
            Self::Focus => "Less context switching. More time for what matters.",
            Self::Habits => "Build consistency without chasing perfect streaks.",
            Self::Routines => "Group your intentions into a repeatable sequence.",
            Self::Notes => "A quiet place for ideas, learning and reflection.",
            Self::Reviews => "Look back with perspective. Move forward deliberately.",
            Self::Insights => "Measured effort, without arbitrary productivity scores.",
            Self::Agents => "Your agents can act. You can always see what changed.",
            Self::Automations => "Local rules with visible actions and activity history.",
            Self::Library => "Reusable structures, saved views, and portable data.",
            Self::Settings => "Make your workspace work the way you think.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskFilter {
    Inbox,
    Open,
    Today,
    All,
    Completed,
}

impl TaskFilter {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Open => "Open",
            Self::Today => "Today",
            Self::All => "All",
            Self::Completed => "Completed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskSort {
    Priority,
    DueDate,
    Name,
}

impl TaskSort {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Priority => "Priority",
            Self::DueDate => "Due date",
            Self::Name => "Name",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Priority => Self::DueDate,
            Self::DueDate => Self::Name,
            Self::Name => Self::Priority,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct TaskRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub status: String,
    #[serde(default = "default_priority")]
    pub priority: i64,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub area_id: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub planned_date: Option<String>,
    #[serde(default = "default_estimate")]
    pub estimate_minutes: i64,
    #[serde(default)]
    pub recurrence: Option<String>,
    #[serde(default)]
    pub revision: i64,
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default)]
    pub area_name: Option<String>,
}

fn default_priority() -> i64 {
    3
}

fn default_estimate() -> i64 {
    30
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ProjectRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub area_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct GoalRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub target: f64,
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub due_date: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct EventRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub start_at: String,
    #[serde(default)]
    pub end_at: String,
    #[serde(default)]
    pub all_day: i64,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub task_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct HabitRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub frequency: String,
    #[serde(default)]
    pub target: i64,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub completions: i64,
    #[serde(default)]
    pub today_value: Option<i64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct NoteRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct FocusRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub duration_seconds: i64,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub task_title: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct AreaRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub icon: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct StatsRecord {
    #[serde(default)]
    pub open_tasks: i64,
    #[serde(default)]
    pub completed_tasks: i64,
    #[serde(default)]
    pub active_projects: i64,
    #[serde(default)]
    pub focus_seconds: i64,
    #[serde(default)]
    pub habits: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct HistoryRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct RoutineRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct RoutineStepRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub routine_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub position: i64,
    #[serde(default)]
    pub duration_minutes: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ReviewRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub period: String,
    #[serde(default)]
    pub day: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct AutomationRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub trigger_kind: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub arguments: String,
    #[serde(default)]
    pub enabled: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct MilestoneRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct AgentChangeRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub entity_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub undoable: bool,
    #[serde(default)]
    pub undone_at: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct EncryptionStatus {
    #[serde(default)]
    pub protection: String,
    #[serde(default)]
    pub encrypted_on_disk: bool,
    #[serde(default)]
    pub cipher: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ProposedSlot {
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub start_at: String,
    #[serde(default)]
    pub end_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ScheduleProposal {
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub slots: Vec<ProposedSlot>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct TemplateRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_priority")]
    pub priority: i64,
    #[serde(default = "default_estimate")]
    pub estimate_minutes: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct SavedFilterRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub entity: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub query_json: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct KeyResultRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub goal_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub baseline: f64,
    #[serde(default)]
    pub target: f64,
    #[serde(default)]
    pub current: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct AgentProposalRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub payload_json: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub created_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ProtectedBlockRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub weekday: i64,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub end_time: String,
    #[serde(default)]
    pub kind: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct TimeBudgetRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub scope_kind: String,
    #[serde(default)]
    pub scope_id: Option<String>,
    #[serde(default)]
    pub period: String,
    #[serde(default)]
    pub minutes: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct OverdueDecisionRecord {
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default = "default_priority")]
    pub priority: i64,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub days_overdue: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct CareerRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub employer: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub stage: String,
    #[serde(default)]
    pub location: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct LearningRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub progress: f64,
    #[serde(default)]
    pub target_hours: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct FinanceRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub amount_minor: Option<i64>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct WellbeingRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub day: String,
    #[serde(default)]
    pub energy: i64,
    #[serde(default)]
    pub mood: i64,
    #[serde(default)]
    pub sleep_hours: Option<f64>,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct InterviewRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub starts_at: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct SavingsGoalRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub target_minor: i64,
    #[serde(default)]
    pub saved_minor: i64,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub due_date: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct CertificationRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub deadline_date: Option<String>,
    #[serde(default)]
    pub study_hours_target: f64,
    #[serde(default)]
    pub study_hours_done: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct RestIntentionRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub starts_at: String,
    #[serde(default)]
    pub ends_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct AdministrationRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub renewal_date: Option<String>,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct FocusPresetRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub focus_minutes: i64,
    #[serde(default)]
    pub break_minutes: i64,
    #[serde(default)]
    pub cycles: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct GoalCycleRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub period_key: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub starts_at: String,
    #[serde(default)]
    pub ends_at: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ProjectTemplateRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct CalendarSourceRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub read_only: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ShortcutRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub action: String,
    #[serde(default)]
    pub keys: String,
    #[serde(default)]
    pub label: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct IntegrationRecord {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub account_label: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct RemoteClientRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub revoked_at: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct AgentPermissionRecord {
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub action_pattern: String,
    #[serde(default)]
    pub allowed: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct GuidedReviewRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub day: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub answers_json: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct SavedSearchRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub query: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct AvailabilityRuleRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub weekday: i64,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub end_time: String,
    #[serde(default)]
    pub kind: String,
}

#[derive(Clone, Debug, Default)]
pub struct WorkspaceSnapshot {
    pub tasks: Vec<TaskRecord>,
    pub archived_tasks: Vec<TaskRecord>,
    pub projects: Vec<ProjectRecord>,
    pub goals: Vec<GoalRecord>,
    pub events: Vec<EventRecord>,
    pub habits: Vec<HabitRecord>,
    pub notes: Vec<NoteRecord>,
    pub focus: Vec<FocusRecord>,
    pub areas: Vec<AreaRecord>,
    pub stats: StatsRecord,
    pub history: Vec<HistoryRecord>,
    pub routines: Vec<RoutineRecord>,
    pub reviews: Vec<ReviewRecord>,
    pub automations: Vec<AutomationRecord>,
    pub milestones: Vec<MilestoneRecord>,
    pub agent_changes: Vec<AgentChangeRecord>,
    pub encryption: EncryptionStatus,
    pub templates: Vec<TemplateRecord>,
    pub saved_views: Vec<SavedFilterRecord>,
    pub key_results: Vec<KeyResultRecord>,
    pub agent_proposals: Vec<AgentProposalRecord>,
    pub protected_blocks: Vec<ProtectedBlockRecord>,
    pub time_budgets: Vec<TimeBudgetRecord>,
    pub overdue_decisions: Vec<OverdueDecisionRecord>,
    pub career: Vec<CareerRecord>,
    pub learning: Vec<LearningRecord>,
    pub finance: Vec<FinanceRecord>,
    pub wellbeing: Vec<WellbeingRecord>,
    pub interviews: Vec<InterviewRecord>,
    pub savings_goals: Vec<SavingsGoalRecord>,
    pub certifications: Vec<CertificationRecord>,
    pub rest_intentions: Vec<RestIntentionRecord>,
    pub administration: Vec<AdministrationRecord>,
    pub focus_presets: Vec<FocusPresetRecord>,
    pub goal_cycles: Vec<GoalCycleRecord>,
    pub project_templates: Vec<ProjectTemplateRecord>,
    pub calendar_sources: Vec<CalendarSourceRecord>,
    pub shortcuts: Vec<ShortcutRecord>,
    pub integrations: Vec<IntegrationRecord>,
    pub remote_clients: Vec<RemoteClientRecord>,
    pub agent_permissions: Vec<AgentPermissionRecord>,
    pub guided_reviews: Vec<GuidedReviewRecord>,
    pub saved_searches: Vec<SavedSearchRecord>,
    pub availability: Vec<AvailabilityRuleRecord>,
    pub system_status: Value,
    pub diagnostics: Value,
}

fn parse_vec<T: for<'de> Deserialize<'de>>(val: Value) -> Vec<T> {
    serde_json::from_value(val).unwrap_or_default()
}

fn parse_one<T: for<'de> Deserialize<'de> + Default>(val: Value) -> T {
    serde_json::from_value(val).unwrap_or_default()
}

pub fn load_snapshot(store: &mut life_core::Store) -> life_core::Result<WorkspaceSnapshot> {
    let empty = json!({});
    let overdue_val = store
        .call("overdue.decisions", &empty)
        .unwrap_or(Value::Null);
    let overdue_decisions = overdue_val
        .get("decisions")
        .cloned()
        .map(parse_vec)
        .unwrap_or_default();
    Ok(WorkspaceSnapshot {
        tasks: parse_vec(store.call("tasks.list", &empty)?),
        archived_tasks: parse_vec(store.call("tasks.archived", &empty)?),
        projects: parse_vec(store.call("projects.list", &empty)?),
        goals: parse_vec(store.call("goals.list", &empty)?),
        events: parse_vec(store.call("events.list", &empty)?),
        habits: parse_vec(store.call("habits.list", &empty)?),
        notes: parse_vec(store.call("notes.list", &empty)?),
        focus: parse_vec(store.call("focus.list", &empty)?),
        areas: parse_vec(store.call("areas.list", &empty)?),
        stats: parse_one(store.call("stats", &empty)?),
        history: parse_vec(store.call("history.list", &empty)?),
        routines: parse_vec(store.call("routines.list", &empty)?),
        reviews: parse_vec(store.call("reviews.list", &empty)?),
        automations: parse_vec(store.call("automations.list", &empty)?),
        milestones: parse_vec(store.call("milestones.list", &empty)?),
        agent_changes: parse_vec(store.call("agents.changes", &empty).unwrap_or(Value::Null)),
        encryption: parse_one(
            store
                .call("encryption.status", &empty)
                .unwrap_or(Value::Null),
        ),
        templates: parse_vec(store.call("templates.list", &empty).unwrap_or(Value::Null)),
        saved_views: parse_vec(
            store
                .call("saved_views.list", &empty)
                .unwrap_or(Value::Null),
        ),
        key_results: parse_vec(
            store
                .call("key_results.list", &empty)
                .unwrap_or(Value::Null),
        ),
        agent_proposals: parse_vec(
            store
                .call("agents.proposals.list", &empty)
                .unwrap_or(Value::Null),
        ),
        protected_blocks: parse_vec(store.call("protected.list", &empty).unwrap_or(Value::Null)),
        time_budgets: parse_vec(
            store
                .call("time_budgets.list", &empty)
                .unwrap_or(Value::Null),
        ),
        overdue_decisions,
        career: parse_vec(store.call("career.list", &empty).unwrap_or(Value::Null)),
        learning: parse_vec(store.call("learning.list", &empty).unwrap_or(Value::Null)),
        finance: parse_vec(store.call("finance.list", &empty).unwrap_or(Value::Null)),
        wellbeing: parse_vec(
            store
                .call("wellbeing.list", &json!({"limit": 14}))
                .unwrap_or(Value::Null),
        ),
        interviews: parse_vec(store.call("interviews.list", &empty).unwrap_or(Value::Null)),
        savings_goals: parse_vec(
            store
                .call("savings_goals.list", &empty)
                .unwrap_or(Value::Null),
        ),
        certifications: parse_vec(
            store
                .call("certifications.list", &empty)
                .unwrap_or(Value::Null),
        ),
        rest_intentions: parse_vec(
            store
                .call("rest_intentions.list", &empty)
                .unwrap_or(Value::Null),
        ),
        administration: parse_vec(
            store
                .call("administration.list", &empty)
                .unwrap_or(Value::Null),
        ),
        focus_presets: parse_vec(
            store
                .call("focus.presets.list", &empty)
                .unwrap_or(Value::Null),
        ),
        goal_cycles: parse_vec(
            store
                .call("goal_cycles.list", &empty)
                .unwrap_or(Value::Null),
        ),
        project_templates: parse_vec(
            store
                .call("projects.templates.list", &empty)
                .unwrap_or(Value::Null),
        ),
        calendar_sources: parse_vec(
            store
                .call("calendar.sources.list", &empty)
                .unwrap_or(Value::Null),
        ),
        shortcuts: parse_vec(store.call("shortcuts.list", &empty).unwrap_or(Value::Null)),
        integrations: parse_vec(
            store
                .call("integrations.list", &empty)
                .unwrap_or(Value::Null),
        ),
        remote_clients: parse_vec(
            store
                .call("remote_clients.list", &empty)
                .unwrap_or(Value::Null),
        ),
        agent_permissions: parse_vec(
            store
                .call("agents.permissions.list", &empty)
                .unwrap_or(Value::Null),
        ),
        guided_reviews: parse_vec(
            store
                .call("review_guides.list", &empty)
                .unwrap_or(Value::Null),
        ),
        saved_searches: parse_vec(
            store
                .call("saved_searches.list", &empty)
                .unwrap_or(Value::Null),
        ),
        availability: parse_vec(
            store
                .call("availability.list", &empty)
                .unwrap_or(Value::Null),
        ),
        system_status: store.call("system.status", &empty).unwrap_or(Value::Null),
        diagnostics: store
            .call("diagnostics.report", &empty)
            .unwrap_or(Value::Null),
    })
}

pub fn format_duration(seconds: i64) -> String {
    if seconds >= 3600 {
        format!("{}h {:02}m", seconds / 3600, (seconds % 3600) / 60)
    } else {
        format!("{}m", (seconds / 60).max(0))
    }
}

pub fn format_short_date(raw: Option<&str>) -> String {
    match raw {
        Some(s) if s.len() >= 10 => s[..10].to_string(),
        Some(s) if !s.is_empty() => s.to_string(),
        _ => "Anytime".to_string(),
    }
}

pub fn format_clock(raw: &str) -> String {
    if raw.len() >= 16 {
        raw[11..16].to_string()
    } else {
        raw.to_string()
    }
}

pub fn priority_label(priority: i64) -> &'static str {
    match priority {
        1 => "Urgent",
        2 => "High",
        3 => "Medium",
        4 => "Low",
        _ => "Normal",
    }
}

#[derive(Clone)]
pub struct BackendHandle {
    pub db_path: PathBuf,
    pub passphrase: Option<String>,
    conn_cache: Arc<Mutex<Option<life_core::Store>>>,
}

impl std::fmt::Debug for BackendHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackendHandle")
            .field("db_path", &self.db_path)
            .field("has_passphrase", &self.passphrase.is_some())
            .finish()
    }
}

impl BackendHandle {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            passphrase: None,
            conn_cache: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_passphrase(&mut self, passphrase: Option<String>) {
        self.passphrase = passphrase;
        self.invalidate();
    }

    pub fn invalidate(&self) {
        if let Ok(mut guard) = self.conn_cache.lock() {
            *guard = None;
        }
    }

    fn open_fresh_store(&self) -> life_core::Result<life_core::Store> {
        if let Some(pass) = &self.passphrase {
            life_core::Store::open_with_passphrase(&self.db_path, pass)
        } else {
            life_core::Store::open(&self.db_path)
        }
    }

    fn with_store<R>(
        &self,
        f: impl FnOnce(&mut life_core::Store) -> life_core::Result<R>,
    ) -> life_core::Result<R> {
        let mut guard = self
            .conn_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.is_none() {
            *guard = Some(self.open_fresh_store()?);
        }
        let store = guard.as_mut().expect("store initialized");
        f(store)
    }

    pub fn call(&self, action: &str, payload: &Value) -> life_core::Result<Value> {
        let invalidate_after = matches!(
            action,
            "encryption.enable" | "encryption.disable" | "backup.restore"
        );
        let res = self.with_store(|store| store.call(action, payload));
        if invalidate_after {
            self.invalidate();
        }
        res
    }

    pub fn snapshot(&self) -> life_core::Result<WorkspaceSnapshot> {
        self.with_store(load_snapshot)
    }
}
