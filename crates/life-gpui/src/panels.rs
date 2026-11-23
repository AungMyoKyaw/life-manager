//! Full schema-v16 FigCraft extension panels for `LifeManagerApp`.
use crate::{
    LifeManagerApp,
    models::{WorkspaceView, format_short_date},
    tokens::Palette,
};
use chrono::Local;
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _,
    assets::IconName,
    component::{Icon, h_flex, input::Input, v_flex},
    div,
    prelude::FluentBuilder as _,
    px,
};
use serde_json::{Value, json};

fn panel_icon(name: IconName, size: f32, color: gpui_kit::Rgba) -> AnyElement {
    Icon::new(name)
        .size(px(size))
        .text_color(color)
        .into_any_element()
}

fn chip_btn(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    active: bool,
    pal: Palette,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id.into())
        .cursor_pointer()
        .px(px(12.0))
        .py(px(6.0))
        .rounded(px(8.0))
        .bg(if active {
            pal.accent()
        } else {
            pal.surface_raised()
        })
        .text_color(if active {
            pal.text_on_accent()
        } else {
            pal.text_secondary()
        })
        .text_size(px(11.0))
        .font_weight(FontWeight::BOLD)
        .child(label.into())
}

fn action_pill(
    id: impl Into<SharedString>,
    icon: Option<IconName>,
    label: impl Into<SharedString>,
    primary: bool,
    pal: Palette,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let fg = if primary {
        pal.text_on_accent()
    } else {
        pal.text_secondary()
    };
    h_flex()
        .id(id.into())
        .cursor_pointer()
        .px(px(12.0))
        .py(px(7.0))
        .gap(px(6.0))
        .rounded(px(8.0))
        .bg(if primary {
            pal.accent()
        } else {
            pal.surface_raised()
        })
        .border_1()
        .border_color(if primary { pal.accent() } else { pal.border() })
        .items_center()
        .text_size(px(11.0))
        .font_weight(FontWeight::BOLD)
        .text_color(fg)
        .when_some(icon, |el, ic| el.child(panel_icon(ic, 14.0, fg)))
        .child(label.into())
}

impl LifeManagerApp {
    pub fn render_subtab_bar(
        &self,
        prefix: &str,
        tabs: &[&'static str],
        active_idx: usize,
        pal: Palette,
        on_select: impl Fn(&mut Self, usize, &mut Context<Self>) + Copy + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut bar = h_flex()
            .p(px(4.0))
            .gap(px(4.0))
            .rounded(px(10.0))
            .bg(pal.surface())
            .border_1()
            .border_color(pal.border())
            .flex_wrap();
        for (idx, label) in tabs.iter().enumerate() {
            bar = bar.child(
                chip_btn(
                    SharedString::from(format!("{prefix}-tab-{idx}")),
                    *label,
                    active_idx == idx,
                    pal,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    on_select(this, idx, cx);
                })),
            );
        }
        bar.into_any_element()
    }

    /// First-run interactive onboarding banner (when workspace is empty or explicitly opened).
    pub fn render_onboarding_card(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let step = self.onboarding_step.min(2);
        let step_titles = [
            "1. Seed your Life Areas",
            "2. Configure weekly availability",
            "3. Capture your first priority",
        ];

        self.surface_card(pal)
            .border_color(pal.accent())
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .text_color(pal.accent_text())
                                    .child("FIRST-RUN SETUP · STEP-BY-STEP WORKSPACE WIZARD"),
                            )
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(step_titles[step]),
                            ),
                    )
                    .child(
                        action_pill("dismiss-onboarding-btn", None, "Dismiss", false, pal)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.onboarding_dismissed = true;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(8.0))
                    .flex_wrap()
                    .when(step == 0, |el| {
                        el.child(
                            action_pill(
                                "onboard-seed-areas",
                                Some(IconName::Layers),
                                "Seed Engineering, Career, Health & Finance areas",
                                true,
                                pal,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                for name in ["Engineering", "Career", "Health", "Finance"] {
                                    let _ =
                                        this.backend.call("areas.create", &json!({ "name": name }));
                                }
                                this.reload();
                                this.onboarding_step = 1;
                                this.notice = Some("Default life areas configured".into());
                                cx.notify();
                            })),
                        )
                    })
                    .when(step == 1, |el| {
                        el.child(
                            action_pill(
                                "onboard-set-availability",
                                Some(IconName::CalendarClock),
                                "Set Mon–Fri 09:00–17:00 availability",
                                true,
                                pal,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                let rules: Vec<Value> = (0..5)
                                    .map(|wd| {
                                        json!({
                                            "weekday": wd,
                                            "start_time": "09:00",
                                            "end_time": "17:00",
                                            "kind": "available"
                                        })
                                    })
                                    .collect();
                                if this
                                    .exec(
                                        "availability.replace",
                                        json!({ "rules": rules }),
                                        "Weekly availability saved",
                                    )
                                    .is_some()
                                {
                                    this.onboarding_step = 2;
                                }
                                cx.notify();
                            })),
                        )
                    })
                    .when(step == 2, |el| {
                        el.child(
                            action_pill(
                                "onboard-first-task",
                                Some(IconName::CircleCheck),
                                "Create first P1 task & finish setup",
                                true,
                                pal,
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.exec(
                                    "tasks.create",
                                    json!({
                                        "title": "Plan today's top three priorities",
                                        "priority": 1,
                                        "status": "todo",
                                        "planned_date": Local::now().format("%Y-%m-%d").to_string()
                                    }),
                                    "Onboarding complete",
                                );
                                this.onboarding_dismissed = true;
                                cx.notify();
                            })),
                        )
                    }),
            )
            .into_any_element()
    }

    /// `Cmd+N` Natural-Language Quick Capture Drawer using `capture.parse`.
    pub fn render_quick_capture_drawer(
        &mut self,
        pal: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let preview_summary = self.capture_preview.as_ref().map(|p| {
            let title = p["title"].as_str().unwrap_or("");
            let prio = p["priority"].as_i64().unwrap_or(3);
            let due = p["due_date"].as_str().unwrap_or("No date");
            format!("Parsed → \"{title}\" · P{prio} · Due: {due}")
        });

        self.surface_card(pal)
            .id("quick-capture-drawer")
            .w_full()
            .border_color(pal.accent())
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .text_color(pal.accent_text())
                            .child("QUICK CAPTURE (CMD+N · NATURAL-LANGUAGE PARSER)"),
                    )
                    .child(
                        action_pill("quick-capture-close", None, "Close", false, pal).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.quick_capture_open = false;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(10.0))
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&self.capture_parse_input)),
                    )
                    .child(
                        action_pill(
                            "quick-capture-commit",
                            Some(IconName::Plus),
                            "Save parsed task",
                            true,
                            pal,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            let raw = this.capture_parse_input.read(cx).value().trim().to_string();
                            if raw.is_empty() {
                                return;
                            }
                            let parsed = this
                                .backend
                                .call("capture.parse", &json!({ "text": raw }))
                                .unwrap_or_else(
                                    |_| json!({ "title": raw, "priority": 3, "status": "inbox" }),
                                );
                            if this
                                .exec("tasks.create", parsed, "Captured & parsed task saved")
                                .is_some()
                            {
                                this.capture_preview = None;
                                this.quick_capture_open = false;
                                this.capture_parse_input.update(cx, |s, cx| {
                                    s.set_value("", window, cx);
                                });
                            }
                            cx.notify();
                        })),
                    ),
            )
            .when_some(preview_summary, |el, summary| {
                el.child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(pal.accent_text())
                        .child(summary),
                )
            })
            .into_any_element()
    }

    /// Remaining schema-v16 Personal Life-Area Trackers (Interviews, Savings, Certifications, Rest, Administration).
    pub fn render_areas_v16_trackers(
        &mut self,
        pal: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut out = v_flex().w_full().min_w_0().gap(px(16.0));
        match self.areas_tab {
            1 => {
                // Career & Interviews
                let mut rows = v_flex().w_full().min_w_0().gap(px(8.0));
                for iv in &self.snapshot.interviews {
                    let iid = iv.id.clone();
                    rows = rows.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .p(px(12.0))
                            .rounded(px(9.0))
                            .bg(pal.surface_raised())
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::BOLD)
                                            .truncate()
                                            .child(iv.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child(format!(
                                                "{} · {}",
                                                format_short_date(Some(&iv.starts_at)),
                                                iv.status
                                            )),
                                    ),
                            )
                            .child(
                                action_pill(
                                    SharedString::from(format!("del-iv-{iid}")),
                                    None,
                                    "Archive",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "interviews.delete",
                                            json!({ "id": iid }),
                                            "Interview archived",
                                        );
                                        cx.notify();
                                    },
                                )),
                            ),
                    );
                }
                out = out.child(
                    self.surface_card(pal)
                        .child(
                            div()
                                .text_size(px(17.0))
                                .font_weight(FontWeight::BOLD)
                                .child("Interview scheduler"),
                        )
                        .child(
                            h_flex()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(Input::new(&self.interview_title_input)),
                                )
                                .child(
                                    action_pill(
                                        "add-interview-btn",
                                        Some(IconName::Plus),
                                        "Schedule interview",
                                        true,
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        |this, _, window, cx| {
                                            let title = this
                                                .interview_title_input
                                                .read(cx)
                                                .value()
                                                .trim()
                                                .to_string();
                                            if title.is_empty() {
                                                return;
                                            }
                                            let start = format!(
                                                "{}T10:00:00",
                                                Local::now().format("%Y-%m-%d")
                                            );
                                            if this
                                                .exec(
                                                    "interviews.create",
                                                    json!({
                                                        "title": title,
                                                        "starts_at": start,
                                                        "kind": "interview"
                                                    }),
                                                    "Interview scheduled",
                                                )
                                                .is_some()
                                            {
                                                this.interview_title_input.update(cx, |s, cx| {
                                                    s.set_value("", window, cx);
                                                });
                                            }
                                            cx.notify();
                                        },
                                    )),
                                ),
                        )
                        .child(rows),
                );
            }
            2 => {
                // Certifications Tracker
                let mut rows = v_flex().w_full().min_w_0().gap(px(8.0));
                for cert in &self.snapshot.certifications {
                    let cid = cert.id.clone();
                    let next_hours = cert.study_hours_done + 1.0;
                    rows = rows.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .p(px(12.0))
                            .rounded(px(9.0))
                            .bg(pal.surface_raised())
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::BOLD)
                                            .truncate()
                                            .child(cert.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child(format!(
                                                "{:.1}/{:.1}h studied · {}",
                                                cert.study_hours_done,
                                                cert.study_hours_target,
                                                cert.status
                                            )),
                                    ),
                            )
                            .child(
                                action_pill(
                                    SharedString::from(format!("cert-plus-{cid}")),
                                    Some(IconName::Plus),
                                    "+1h study",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "certifications.update",
                                            json!({ "id": cid, "study_hours_done": next_hours }),
                                            "Certification study hours updated",
                                        );
                                        cx.notify();
                                    },
                                )),
                            ),
                    );
                }
                out = out.child(
                    self.surface_card(pal)
                        .child(
                            div()
                                .text_size(px(17.0))
                                .font_weight(FontWeight::BOLD)
                                .child("Certification tracker"),
                        )
                        .child(
                            h_flex()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(Input::new(&self.cert_title_input)),
                                )
                                .child(
                                    action_pill(
                                        "add-cert-btn",
                                        Some(IconName::Plus),
                                        "Add certification",
                                        true,
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        |this, _, window, cx| {
                                            let title = this
                                                .cert_title_input
                                                .read(cx)
                                                .value()
                                                .trim()
                                                .to_string();
                                            if title.is_empty() {
                                                return;
                                            }
                                            if this
                                                .exec(
                                                    "certifications.create",
                                                    json!({
                                                        "title": title,
                                                        "study_hours_target": 40.0
                                                    }),
                                                    "Certification added",
                                                )
                                                .is_some()
                                            {
                                                this.cert_title_input.update(cx, |s, cx| {
                                                    s.set_value("", window, cx);
                                                });
                                            }
                                            cx.notify();
                                        },
                                    )),
                                ),
                        )
                        .child(rows),
                );
            }
            3 => {
                // Savings Goals
                let mut rows = v_flex().w_full().min_w_0().gap(px(8.0));
                for sg in &self.snapshot.savings_goals {
                    let sid = sg.id.clone();
                    let next_saved = sg.saved_minor + 10_000;
                    rows = rows.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .p(px(12.0))
                            .rounded(px(9.0))
                            .bg(pal.surface_raised())
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::BOLD)
                                            .truncate()
                                            .child(sg.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child(format!(
                                                "{} {:.2} / {:.2}",
                                                sg.currency,
                                                sg.saved_minor as f64 / 100.0,
                                                sg.target_minor as f64 / 100.0
                                            )),
                                    ),
                            )
                            .child(
                                action_pill(
                                    SharedString::from(format!("save-add-{sid}")),
                                    Some(IconName::Plus),
                                    "+100",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "savings_goals.update",
                                            json!({ "id": sid, "saved_minor": next_saved }),
                                            "Savings goal updated",
                                        );
                                        cx.notify();
                                    },
                                )),
                            ),
                    );
                }
                out = out.child(
                    self.surface_card(pal)
                        .child(
                            div()
                                .text_size(px(17.0))
                                .font_weight(FontWeight::BOLD)
                                .child("Savings goals"),
                        )
                        .child(
                            h_flex()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(Input::new(&self.savings_title_input)),
                                )
                                .child(
                                    action_pill(
                                        "add-savings-btn",
                                        Some(IconName::Plus),
                                        "New savings goal",
                                        true,
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        |this, _, window, cx| {
                                            let title = this
                                                .savings_title_input
                                                .read(cx)
                                                .value()
                                                .trim()
                                                .to_string();
                                            if title.is_empty() {
                                                return;
                                            }
                                            if this
                                                .exec(
                                                    "savings_goals.create",
                                                    json!({
                                                        "title": title,
                                                        "target_minor": 100_000,
                                                        "saved_minor": 0,
                                                        "currency": "USD"
                                                    }),
                                                    "Savings goal created",
                                                )
                                                .is_some()
                                            {
                                                this.savings_title_input.update(cx, |s, cx| {
                                                    s.set_value("", window, cx);
                                                });
                                            }
                                            cx.notify();
                                        },
                                    )),
                                ),
                        )
                        .child(rows),
                );
            }
            4 => {
                // Rest Intentions & Personal Administration
                let mut rest_rows = v_flex().w_full().min_w_0().gap(px(8.0));
                for r in &self.snapshot.rest_intentions {
                    let rid = r.id.clone();
                    rest_rows = rest_rows.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .p(px(10.0))
                            .rounded(px(8.0))
                            .bg(pal.surface_raised())
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(r.title.clone()),
                            )
                            .child(
                                action_pill(
                                    SharedString::from(format!("del-rest-{rid}")),
                                    None,
                                    "Archive",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "rest_intentions.delete",
                                            json!({ "id": rid }),
                                            "Rest intention archived",
                                        );
                                        cx.notify();
                                    },
                                )),
                            ),
                    );
                }

                let mut admin_rows = v_flex().w_full().min_w_0().gap(px(8.0));
                for a in &self.snapshot.administration {
                    let aid = a.id.clone();
                    let is_done = a.status == "done";
                    admin_rows = admin_rows.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .p(px(10.0))
                            .rounded(px(8.0))
                            .bg(pal.surface_raised())
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("{} ({})", a.title, a.status)),
                            )
                            .child(
                                action_pill(
                                    SharedString::from(format!("done-admin-{aid}")),
                                    None,
                                    if is_done { "Done" } else { "Complete" },
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if !is_done {
                                            this.exec(
                                                "administration.update",
                                                json!({ "id": aid, "status": "done" }),
                                                "Administration item completed",
                                            );
                                            cx.notify();
                                        }
                                    },
                                )),
                            ),
                    );
                }

                out = out.child(
                    h_flex()
                        .items_start()
                        .gap(px(16.0))
                        .w_full()
                        .min_w_0()
                        .child(
                            self.surface_card(pal)
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .text_size(px(17.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child("Sleep & rest intentions"),
                                )
                                .child(
                                    h_flex()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .child(Input::new(&self.rest_title_input)),
                                        )
                                        .child(
                                            action_pill(
                                                "add-rest-btn",
                                                Some(IconName::Plus),
                                                "Add intention",
                                                true,
                                                pal,
                                            )
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let title = this.rest_title_input.read(cx).value().trim().to_string();
                                                if title.is_empty() {
                                                    return;
                                                }
                                                if this
                                                    .exec(
                                                        "rest_intentions.create",
                                                        json!({ "title": title, "kind": "sleep" }),
                                                        "Rest intention added",
                                                    )
                                                    .is_some()
                                                {
                                                    this.rest_title_input.update(cx, |s, cx| {
                                                        s.set_value("", window, cx);
                                                    });
                                                }
                                                cx.notify();
                                            })),
                                        ),
                                )
                                .child(rest_rows),
                        )
                        .child(
                            self.surface_card(pal)
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .text_size(px(17.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child("Administration tracker"),
                                )
                                .child(
                                    h_flex()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .child(Input::new(&self.admin_title_input)),
                                        )
                                        .child(
                                            action_pill(
                                                "add-admin-btn",
                                                Some(IconName::Plus),
                                                "Add item",
                                                true,
                                                pal,
                                            )
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                let title = this.admin_title_input.read(cx).value().trim().to_string();
                                                if title.is_empty() {
                                                    return;
                                                }
                                                if this
                                                    .exec(
                                                        "administration.create",
                                                        json!({ "title": title, "kind": "paperwork" }),
                                                        "Administration item added",
                                                    )
                                                    .is_some()
                                                {
                                                    this.admin_title_input.update(cx, |s, cx| {
                                                        s.set_value("", window, cx);
                                                    });
                                                }
                                                cx.notify();
                                            })),
                                        ),
                                )
                                .child(admin_rows),
                        ),
                );
            }
            _ => {}
        }
        out.into_any_element()
    }

    /// Extended Settings Tabs: Shortcuts, Reminders, Remote MCP Clients, Integrations, CSV Import, Diagnostics & Accessibility Audit.
    pub fn render_settings_v16_tab(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        match self.settings_tab {
            1 => {
                // Security, System Diagnostics & Accessibility Audit
                let status = &self.snapshot.system_status;
                let schema_ver = status["schema_version"].as_i64().unwrap_or(16);
                let integrity = status["integrity"].as_str().unwrap_or("ok");
                let journal = status["journal_mode"].as_str().unwrap_or("wal");
                let bytes = status["database_bytes"].as_i64().unwrap_or(0);

                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(16.0))
                    .child(
                        self.surface_card(pal)
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("System status, diagnostics & about"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(pal.text_secondary())
                                    .child(format!(
                                        "Life Manager v{} (Native GPUI) · Schema v{} · Integrity: {} · Journal: {} · DB Size: {} KB",
                                        env!("CARGO_PKG_VERSION"),
                                        schema_ver,
                                        integrity,
                                        journal,
                                        bytes / 1024
                                    )),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .flex_wrap()
                                    .child(
                                        action_pill(
                                            "settings-verify-backup",
                                            Some(IconName::ShieldCheck),
                                            "Verify latest backup",
                                            false,
                                            pal,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            if let Some(path) = this.last_backup_path.clone() {
                                                this.exec(
                                                    "backup.verify",
                                                    json!({ "path": path }),
                                                    "Backup integrity verified",
                                                );
                                            } else {
                                                this.notice = Some("Create a backup first to verify".into());
                                            }
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        action_pill(
                                            "settings-a11y-audit",
                                            Some(IconName::Sparkles),
                                            "Run accessibility audit",
                                            true,
                                            pal,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            if let Ok(res) = this.backend.call("accessibility.audit", &json!({})) {
                                                this.last_accessibility_audit = Some(res);
                                                this.notice = Some("Accessibility inspection completed".into());
                                            }
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        action_pill(
                                            "settings-run-onboarding",
                                            Some(IconName::Layers),
                                            "Re-open onboarding wizard",
                                            false,
                                            pal,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.onboarding_dismissed = false;
                                            this.onboarding_step = 0;
                                            this.switch_view(WorkspaceView::Today, cx);
                                        })),
                                    ),
                            )
                            .when_some(self.last_accessibility_audit.clone(), |el, audit| {
                                el.child(
                                    div()
                                        .p(px(10.0))
                                        .rounded(px(8.0))
                                        .bg(pal.surface_raised())
                                        .text_size(px(11.0))
                                        .child(format!("Accessibility audit: {}", audit)),
                                )
                            }),
                    )
                    .child(self.render_settings_v16_extras(pal, cx))
                    .into_any_element()
            }
            2 => {
                // Keyboard Shortcuts & Reminders
                let mut rows = v_flex().w_full().min_w_0().gap(px(6.0));
                for sc in &self.snapshot.shortcuts {
                    rows = rows.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .p(px(10.0))
                            .rounded(px(8.0))
                            .bg(pal.surface_raised())
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(sc.label.clone()),
                            )
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(3.0))
                                    .rounded(px(5.0))
                                    .bg(pal.accent_muted())
                                    .text_color(pal.accent_text())
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(sc.keys.clone()),
                            ),
                    );
                }

                self.surface_card(pal)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Keyboard shortcuts"),
                            )
                            .child(
                                action_pill(
                                    "shortcuts-reset-btn",
                                    None,
                                    "Reset defaults",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.exec(
                                            "shortcuts.reset",
                                            json!({}),
                                            "Shortcuts reset to defaults",
                                        );
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(rows)
                    .into_any_element()
            }
            3 => {
                // Remote MCP Clients & Honest Integration Boundaries
                let mut clients = v_flex().w_full().min_w_0().gap(px(8.0));
                for c in &self.snapshot.remote_clients {
                    let cid = c.id.clone();
                    let revoked = c.revoked_at.is_some();
                    clients = clients.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .p(px(10.0))
                            .rounded(px(8.0))
                            .bg(pal.surface_raised())
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!(
                                        "{} ({})",
                                        c.name,
                                        if revoked { "Revoked" } else { "Active token" }
                                    )),
                            )
                            .when(!revoked, |el| {
                                el.child(
                                    action_pill(
                                        SharedString::from(format!("revoke-client-{cid}")),
                                        None,
                                        "Revoke",
                                        false,
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                                "remote_clients.revoke",
                                                json!({ "id": cid }),
                                                "Remote MCP client token revoked",
                                            );
                                            cx.notify();
                                        },
                                    )),
                                )
                            }),
                    );
                }

                let mut providers = v_flex().w_full().min_w_0().gap(px(8.0));
                for ig in &self.snapshot.integrations {
                    let prov = ig.provider.clone();
                    providers = providers.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .p(px(10.0))
                            .rounded(px(8.0))
                            .bg(pal.surface_raised())
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::BOLD)
                                            .child(ig.provider.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child(format!("Status: {}", ig.status)),
                                    ),
                            )
                            .child(
                                action_pill(
                                    SharedString::from(format!("check-ig-{prov}")),
                                    None,
                                    "Check credentials",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if let Ok(res) = this.backend.call(
                                            "integrations.configure",
                                            &json!({ "provider": prov }),
                                        ) {
                                            let configured =
                                                res["configured"].as_bool().unwrap_or(false);
                                            this.notice = Some(if configured {
                                                format!("{prov} environment credentials present")
                                            } else {
                                                format!("{prov}: offline / missing OAuth env keys")
                                            });
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                    );
                }

                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(16.0))
                    .child(
                        self.surface_card(pal)
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Remote MCP clients (bearer-hashed tokens)"),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .child(Input::new(&self.remote_client_input)),
                                    )
                                    .child(
                                        action_pill(
                                            "create-remote-client-btn",
                                            Some(IconName::Plus),
                                            "Issue token",
                                            true,
                                            pal,
                                        )
                                        .on_click(cx.listener(
                                            |this, _, window, cx| {
                                                let name = this
                                                    .remote_client_input
                                                    .read(cx)
                                                    .value()
                                                    .trim()
                                                    .to_string();
                                                if name.is_empty() {
                                                    return;
                                                }
                                                let token = format!(
                                                    "lm_tok_{}",
                                                    Local::now().timestamp_micros()
                                                );
                                                if this
                                                    .exec(
                                                        "remote_clients.create",
                                                        json!({ "name": name, "token": format!("{token}00000000") }),
                                                        "Remote MCP client token created",
                                                    )
                                                    .is_some()
                                                {
                                                    this.remote_client_input.update(cx, |s, cx| {
                                                        s.set_value("", window, cx);
                                                    });
                                                }
                                                cx.notify();
                                            },
                                        )),
                                    ),
                            )
                            .child(clients),
                    )
                    .child(
                        self.surface_card(pal)
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("External integrations (honest connection boundaries)"),
                            )
                            .child(providers),
                    )
                    .into_any_element()
            }
            4 => {
                // CSV & ICS Import Preview / Apply
                let preview_info = self.csv_preview.as_ref().map(|p| {
                    let valid = p["valid"].as_array().map_or(0, Vec::len);
                    let dup = p["duplicates"].as_array().map_or(0, Vec::len);
                    let inv = p["invalid"].as_array().map_or(0, Vec::len);
                    format!("CSV Preview → {valid} valid · {dup} duplicates · {inv} invalid")
                });

                self.surface_card(pal)
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("CSV & ICS safe import preview"),
                    )
                    .child(Input::new(&self.csv_input))
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .child(
                                action_pill(
                                    "csv-preview-btn",
                                    Some(IconName::Search),
                                    "Preview CSV",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        let content = this.csv_input.read(cx).value().to_string();
                                        if content.trim().is_empty() {
                                            return;
                                        }
                                        match this.backend.call(
                                            "csv.import.preview",
                                            &json!({ "content": content }),
                                        ) {
                                            Ok(res) => {
                                                this.csv_preview = Some(res);
                                                this.notice =
                                                    Some("CSV import preview ready".into());
                                            }
                                            Err(e) => this.error = Some(e.to_string()),
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                action_pill(
                                    "csv-apply-btn",
                                    Some(IconName::Check),
                                    "Apply valid rows",
                                    true,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        let content = this.csv_input.read(cx).value().to_string();
                                        if content.trim().is_empty() {
                                            return;
                                        }
                                        if this
                                            .exec(
                                                "csv.import.apply",
                                                json!({
                                                    "content": content,
                                                    "confirm": true,
                                                    "allow_invalid": true
                                                }),
                                                "CSV rows imported",
                                            )
                                            .is_some()
                                        {
                                            this.csv_preview = None;
                                            this.csv_input.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                action_pill(
                                    "export-ics-btn",
                                    Some(IconName::Download),
                                    "Export Calendar ICS",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        if let Ok(res) =
                                            this.backend.call("calendar.export_ics", &json!({}))
                                            && let Some(ics) = res["ics"].as_str()
                                        {
                                            let out = this
                                                .backend
                                                .db_path
                                                .parent()
                                                .unwrap_or_else(|| std::path::Path::new("/tmp"))
                                                .join("life-calendar.ics");
                                            if std::fs::write(&out, ics).is_ok() {
                                                this.notice = Some(format!(
                                                    "Exported ICS to {}",
                                                    out.display()
                                                ));
                                            }
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .when_some(preview_info, |el, info| {
                        el.child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(pal.accent_text())
                                .child(info),
                        )
                    })
                    .into_any_element()
            }
            _ => div().into_any_element(),
        }
    }

    /// Focus Presets, Manual Entry, Interruption Logging & Focus Trends panel.
    pub fn render_focus_v16_panel(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let mut preset_pills = h_flex().gap(px(8.0)).flex_wrap();
        for preset in &self.snapshot.focus_presets {
            let pid = preset.id.clone();
            preset_pills = preset_pills.child(
                action_pill(
                    SharedString::from(format!("start-preset-{pid}")),
                    Some(IconName::Play),
                    format!("{} ({}m)", preset.name, preset.focus_minutes),
                    false,
                    pal,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.exec(
                        "focus.start",
                        json!({
                            "mode": "pomodoro",
                            "preset_id": pid,
                            "task_id": this.selected_focus_task_id
                        }),
                        "Pomodoro preset started",
                    );
                    cx.notify();
                })),
            );
        }

        let active_id = self
            .snapshot
            .focus
            .iter()
            .find(|f| f.status == "active" || f.status == "paused")
            .map(|f| f.id.clone());

        self.surface_card(pal)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Pomodoro presets, manual entry & interruption log"),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .child(
                                action_pill(
                                    "create-pomodoro-preset",
                                    Some(IconName::Plus),
                                    "+ 50/10 Deep Preset",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.exec(
                                        "focus.presets.create",
                                        json!({
                                            "name": "Deep 50/10",
                                            "focus_minutes": 50,
                                            "break_minutes": 10,
                                            "cycles": 3
                                        }),
                                        "Pomodoro preset created",
                                    );
                                    cx.notify();
                                })),
                            )
                            .child(
                                action_pill(
                                    "log-manual-focus-25",
                                    Some(IconName::Clock3),
                                    "Log manual 25m",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.exec(
                                        "focus.manual",
                                        json!({ "minutes": 25, "task_id": this.selected_focus_task_id }),
                                        "Manual 25m focus session logged",
                                    );
                                    cx.notify();
                                })),
                            )
                            .when_some(active_id, |el, fid| {
                                el.child(
                                    action_pill(
                                        "interrupt-focus-btn",
                                        Some(IconName::CircleAlert),
                                        "Log interruption",
                                        false,
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                                "focus.interrupt",
                                                json!({ "id": fid, "reason": "Context switch" }),
                                                "Interruption recorded",
                                            );
                                            cx.notify();
                                        },
                                    )),
                                )
                            }),
                    ),
            )
            .when(!self.snapshot.focus_presets.is_empty(), |el| {
                el.child(preset_pills)
            })
            .into_any_element()
    }

    /// Guided Review Journeys (Morning, Evening, Weekly, Monthly) inside Reviews.
    pub fn render_guided_review_panel(
        &mut self,
        pal: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut kind_pills = h_flex().gap(px(6.0));
        for k in ["morning", "evening", "weekly", "monthly"] {
            let active = self.guided_review_kind == k;
            let k_str = k.to_string();
            kind_pills = kind_pills.child(
                chip_btn(
                    SharedString::from(format!("guide-kind-{k}")),
                    k.to_uppercase(),
                    active,
                    pal,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.guided_review_kind = k_str.clone();
                    cx.notify();
                })),
            );
        }

        let active_draft = self
            .active_guided_review_id
            .as_ref()
            .and_then(|id| self.snapshot.guided_reviews.iter().find(|r| &r.id == id))
            .cloned();

        self.surface_card(pal)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Guided review journey"),
                    )
                    .child(kind_pills),
            )
            .child(
                h_flex()
                    .gap(px(8.0))
                    .items_center()
                    .child(
                        action_pill(
                            "start-guided-review-btn",
                            Some(IconName::NotebookPen),
                            "Start / resume guided draft",
                            false,
                            pal,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            let kind = this.guided_review_kind.clone();
                            if let Some(res) = this.exec(
                                "review_guides.start",
                                json!({ "kind": kind }),
                                "Guided review draft ready",
                            ) {
                                this.active_guided_review_id = res["id"].as_str().map(String::from);
                            }
                            cx.notify();
                        })),
                    )
                    .when_some(active_draft, |el, draft| {
                        let did = draft.id.clone();
                        let prompt_key = match draft.kind.as_str() {
                            "morning" => "intention",
                            "evening" => "wins",
                            "weekly" => "wins",
                            _ => "progress",
                        };
                        el.child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Input::new(&self.guided_answer_input)),
                        )
                        .child(
                            action_pill(
                                "complete-guided-review-btn",
                                Some(IconName::Check),
                                "Complete review",
                                true,
                                pal,
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    let ans = this
                                        .guided_answer_input
                                        .read(cx)
                                        .value()
                                        .trim()
                                        .to_string();
                                    if ans.is_empty() {
                                        return;
                                    }
                                    let _ = this.backend.call(
                                        "review_guides.update",
                                        &json!({ "id": did, "answers": { prompt_key: ans } }),
                                    );
                                    if this
                                        .exec(
                                            "review_guides.complete",
                                            json!({ "id": did }),
                                            "Guided review completed & archived",
                                        )
                                        .is_some()
                                    {
                                        this.active_guided_review_id = None;
                                        this.guided_answer_input.update(cx, |s, cx| {
                                            s.set_value("", window, cx);
                                        });
                                    }
                                    cx.notify();
                                },
                            )),
                        )
                    }),
            )
            .into_any_element()
    }

    /// Agent Actor Permissions panel inside Agents view.
    pub fn render_agent_permissions_panel(
        &mut self,
        pal: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut rows = v_flex().w_full().min_w_0().gap(px(8.0));
        if self.snapshot.agent_permissions.is_empty() {
            rows = rows.child(
                div()
                    .text_size(px(12.0))
                    .text_color(pal.text_muted())
                    .child("No actor permission rules set (all actors unrestricted by default)."),
            );
        } else {
            for perm in &self.snapshot.agent_permissions {
                let actor = perm.actor.clone();
                let pat = perm.action_pattern.clone();
                rows = rows.child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .p(px(10.0))
                        .rounded(px(8.0))
                        .bg(pal.surface_raised())
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::BOLD)
                                .child(format!(
                                    "{} → {} ({})",
                                    perm.actor,
                                    perm.action_pattern,
                                    if perm.allowed != 0 { "ALLOW" } else { "DENY" }
                                )),
                        )
                        .child(
                            action_pill(
                                SharedString::from(format!("del-perm-{actor}-{pat}")),
                                None,
                                "Remove",
                                false,
                                pal,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.exec(
                                        "agents.permissions.delete",
                                        json!({ "target_actor": actor, "action_pattern": pat }),
                                        "Permission rule removed",
                                    );
                                    cx.notify();
                                },
                            )),
                        ),
                );
            }
        }

        self.surface_card(pal)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Per-actor agent permissions"),
                    )
                    .child(
                        action_pill(
                            "add-readonly-mcp-rule",
                            Some(IconName::ShieldCheck),
                            "Restrict mcp-assistant to tasks.*",
                            false,
                            pal,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.exec(
                                "agents.permissions.set",
                                json!({
                                    "target_actor": "mcp-assistant",
                                    "action_pattern": "tasks.*",
                                    "allowed": true
                                }),
                                "Agent permission rule saved",
                            );
                            cx.notify();
                        })),
                    ),
            )
            .child(rows)
            .into_any_element()
    }

    /// Project Templates, Blockers & Health panel inside Projects.
    pub fn render_projects_v16_panel(
        &mut self,
        pal: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut tpls = v_flex().w_full().min_w_0().gap(px(8.0));
        if self.snapshot.project_templates.is_empty() {
            tpls = tpls.child(
                div()
                    .text_size(px(12.0))
                    .text_color(pal.text_muted())
                    .child("No captured project templates yet. Capture any active project as a reusable template."),
            );
        } else {
            for pt in &self.snapshot.project_templates {
                let ptid = pt.id.clone();
                let pt_arch = pt.id.clone();
                tpls = tpls.child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .p(px(10.0))
                        .rounded(px(8.0))
                        .bg(pal.surface_raised())
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::BOLD)
                                .child(pt.name.clone()),
                        )
                        .child(
                            h_flex()
                                .gap(px(6.0))
                                .child(
                                    action_pill(
                                        SharedString::from(format!("apply-pt-{ptid}")),
                                        Some(IconName::Plus),
                                        "Instantiate project",
                                        true,
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                                "projects.template.apply",
                                                json!({ "id": ptid }),
                                                "Project created from template",
                                            );
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    action_pill(
                                        SharedString::from(format!("arch-pt-{pt_arch}")),
                                        None,
                                        "Archive",
                                        false,
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                                "projects.template.archive",
                                                json!({ "id": pt_arch }),
                                                "Project template archived",
                                            );
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                );
            }
        }

        let first_proj_id = self.snapshot.projects.first().map(|p| p.id.clone());
        self.surface_card(pal)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Project templates & blocker resolution"),
                    )
                    .when_some(first_proj_id, |el, pid| {
                        let pid_cap = pid.clone();
                        let pid_blk = pid.clone();
                        el.child(
                            h_flex()
                                .gap(px(8.0))
                                .child(
                                    action_pill(
                                        "capture-proj-tpl-btn",
                                        Some(IconName::BookOpen),
                                        "Capture first project as template",
                                        false,
                                        pal,
                                    )
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.exec(
                                            "projects.template.capture",
                                            json!({ "project_id": pid_cap, "name": "Captured project blueprint" }),
                                            "Project blueprint captured",
                                        );
                                        cx.notify();
                                    })),
                                )
                                .child(
                                    action_pill(
                                        "check-proj-blockers-btn",
                                        Some(IconName::CircleAlert),
                                        "Check blockers",
                                        false,
                                        pal,
                                    )
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Ok(res) = this.backend.call("projects.blockers", &json!({ "project_id": pid_blk })) {
                                            let cnt = res["count"].as_i64().unwrap_or(0);
                                            this.notice = Some(format!("Project dependency audit: {cnt} active blockers"));
                                        }
                                        cx.notify();
                                    })),
                                ),
                        )
                    }),
            )
            .child(tpls)
            .into_any_element()
    }

    /// Goal Cycles (Annual & Quarterly) and Monthly Review panel inside Goals.
    pub fn render_goals_v16_panel(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let mut cycles = v_flex().w_full().min_w_0().gap(px(8.0));
        for cy in &self.snapshot.goal_cycles {
            cycles = cycles.child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .p(px(10.0))
                    .rounded(px(8.0))
                    .bg(pal.surface_raised())
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(format!("{} ({})", cy.title, cy.period_key)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child(format!(
                                        "{} → {} · {}",
                                        cy.starts_at, cy.ends_at, cy.status
                                    )),
                            ),
                    ),
            );
        }

        self.surface_card(pal)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Annual & quarterly goal cycles"),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .child(
                                action_pill(
                                    "create-quarter-cycle-btn",
                                    Some(IconName::Plus),
                                    "Start current quarter cycle",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.exec(
                                            "goal_cycles.create",
                                            json!({ "kind": "quarter" }),
                                            "Quarterly goal cycle initialized",
                                        );
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                action_pill(
                                    "goals-monthly-review-btn",
                                    Some(IconName::NotebookPen),
                                    "Monthly goal review",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Ok(res) =
                                        this.backend.call("goals.monthly_review", &json!({}))
                                    {
                                        let cnt = res["count"].as_i64().unwrap_or(0);
                                        this.notice = Some(format!(
                                            "Monthly goal review computed across {cnt} active goals"
                                        ));
                                    }
                                    cx.notify();
                                })),
                            ),
                    ),
            )
            .child(cycles)
            .into_any_element()
    }

    /// Calendar Sources & Conflict Resolution panel inside Calendar.
    pub fn render_calendar_v16_panel(
        &mut self,
        pal: Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut sources = v_flex().w_full().min_w_0().gap(px(8.0));
        for src in &self.snapshot.calendar_sources {
            let sid = src.id.clone();
            sources = sources.child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .p(px(10.0))
                    .rounded(px(8.0))
                    .bg(pal.surface_raised())
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::BOLD)
                            .child(format!("{} ({} · {})", src.name, src.kind, src.timezone)),
                    )
                    .child(
                        action_pill(
                            SharedString::from(format!("del-calsrc-{sid}")),
                            None,
                            "Remove",
                            false,
                            pal,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.exec(
                                "calendar.sources.delete",
                                json!({ "id": sid }),
                                "Calendar source removed",
                            );
                            cx.notify();
                        })),
                    ),
            );
        }

        self.surface_card(pal)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Calendar sources & conflict resolution"),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .child(
                                action_pill(
                                    "check-cal-conflicts-btn",
                                    Some(IconName::CircleAlert),
                                    "Check schedule conflicts",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    let day = Local::now().format("%Y-%m-%d").to_string();
                                    if let Ok(res) = this.backend.call(
                                        "calendar.conflicts",
                                        &json!({
                                            "start_at": format!("{day}T09:00:00"),
                                            "end_at": format!("{day}T17:00:00")
                                        }),
                                    ) {
                                        let cnt = res.as_array().map_or(0, Vec::len);
                                        this.notice = Some(format!("Conflict resolution scan: {cnt} overlapping events today"));
                                    }
                                    cx.notify();
                                })),
                            )
                            .child(
                                action_pill(
                                    "add-local-calsrc-btn",
                                    Some(IconName::Plus),
                                    "Add local calendar source",
                                    false,
                                    pal,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.exec(
                                        "calendar.sources.create",
                                        json!({
                                            "name": "Work Calendar",
                                            "kind": "local",
                                            "timezone": "Asia/Singapore"
                                        }),
                                        "Calendar source added",
                                    );
                                    cx.notify();
                                })),
                            ),
                    ),
            )
            .child(sources)
            .into_any_element()
    }

    /// Notes Backlinks, Managed Attachments & Saved Searches panel inside Notes.
    pub fn render_notes_v16_panel(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let mut searches = h_flex().gap(px(8.0)).flex_wrap();
        for ss in &self.snapshot.saved_searches {
            let ssid = ss.id.clone();
            searches = searches.child(
                action_pill(
                    SharedString::from(format!("run-ss-{ssid}")),
                    Some(IconName::Search),
                    ss.name.clone(),
                    false,
                    pal,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Ok(res) = this
                        .backend
                        .call("saved_searches.run", &json!({ "id": ssid }))
                    {
                        let cnt = res["results"].as_array().map_or(0, Vec::len);
                        this.notice = Some(format!("Saved search matched {cnt} records"));
                    }
                    cx.notify();
                })),
            );
        }

        self.surface_card(pal)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Saved searches, backlinks & attached files"),
                    )
                    .child(
                        action_pill(
                            "create-saved-search-btn",
                            Some(IconName::Plus),
                            "Save 'urgent' search",
                            false,
                            pal,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.exec(
                                "saved_searches.create",
                                json!({ "name": "Urgent items", "query": "urgent" }),
                                "Saved search created",
                            );
                            cx.notify();
                        })),
                    ),
            )
            .when(!self.snapshot.saved_searches.is_empty(), |el| {
                el.child(searches)
            })
            .child(
                h_flex()
                    .gap(px(8.0))
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(&self.attachment_path_input)),
                    )
                    .child(
                        action_pill(
                            "attach-file-btn",
                            Some(IconName::Plus),
                            "Attach local file",
                            false,
                            pal,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            let path = this
                                .attachment_path_input
                                .read(cx)
                                .value()
                                .trim()
                                .to_string();
                            if path.is_empty() {
                                return;
                            }
                            if let Some(note) = this.snapshot.notes.first() {
                                let nid = note.id.clone();
                                if this
                                    .exec(
                                        "attachments.create",
                                        json!({
                                            "entity_kind": "notes",
                                            "entity_id": nid,
                                            "source_path": path
                                        }),
                                        "File attached & content-hashed",
                                    )
                                    .is_some()
                                {
                                    this.attachment_path_input.update(cx, |s, cx| {
                                        s.set_value("", window, cx);
                                    });
                                }
                            } else {
                                this.notice =
                                    Some("Create a note first before attaching files".into());
                            }
                            cx.notify();
                        })),
                    ),
            )
            .into_any_element()
    }
}
