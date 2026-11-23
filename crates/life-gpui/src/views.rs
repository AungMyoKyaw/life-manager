//! FigCraft-faithful GPUI views for Life Manager with retained Sidebar/Topbar entities,
//! proper Taffy flex-shrink/wrap constraints (`min_w_0`, `whitespace_normal`, `truncate`),
//! full typography weight hierarchy (`FontWeight`), and 120Hz native macOS scrolling.
use crate::{
    CalendarMode, LifeManagerApp, SidebarPane, TopbarPane,
    models::{
        ProposedSlot, ScheduleProposal, TaskFilter, TaskRecord, WorkspaceView, format_clock,
        format_duration, format_short_date, priority_label,
    },
    tokens::{self, Palette, ThemeMode},
};
use chrono::{Datelike, Duration, Local, NaiveDate};
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    assets::IconName,
    component::{
        Icon,
        button::{Button, ButtonVariants as _},
        h_flex,
        input::Input,
        scroll::ScrollableElement as _,
        v_flex,
    },
    div,
    prelude::FluentBuilder as _,
    px, relative, rgb,
};
use serde_json::json;

fn lucide(name: IconName, size_px: f32, color: gpui_kit::Rgba) -> Icon {
    Icon::new(name).size(px(size_px)).text_color(color)
}

fn primary_action(
    id: impl Into<SharedString>,
    icon: Option<IconName>,
    label: impl Into<SharedString>,
    pal: Palette,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let mut el = h_flex()
        .id(id.into())
        .cursor_pointer()
        .flex_shrink_0()
        .min_h(px(38.0))
        .px(px(15.0))
        .py(px(9.0))
        .gap(px(8.0))
        .rounded(px(8.0))
        .bg(pal.accent())
        .hover(|s| s.bg(pal.accent_hover()))
        .text_color(pal.text_on_accent())
        .text_size(px(12.0))
        .font_weight(FontWeight::BOLD);
    if let Some(ic) = icon {
        el = el.child(lucide(ic, 15.0, pal.text_on_accent()));
    }
    el.child(div().whitespace_nowrap().child(label.into()))
}

fn soft_action(
    id: impl Into<SharedString>,
    icon: Option<IconName>,
    label: impl Into<SharedString>,
    pal: Palette,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let mut el = h_flex()
        .id(id.into())
        .cursor_pointer()
        .flex_shrink_0()
        .min_h(px(36.0))
        .px(px(13.0))
        .py(px(7.0))
        .gap(px(7.0))
        .rounded(px(8.0))
        .bg(pal.surface())
        .border_1()
        .border_color(pal.border())
        .hover(|s| s.bg(pal.surface_raised()))
        .text_color(pal.text_secondary())
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD);
    if let Some(ic) = icon {
        el = el.child(lucide(ic, 14.0, pal.text_secondary()));
    }
    let lbl: SharedString = label.into();
    if !lbl.is_empty() {
        el = el.child(div().whitespace_nowrap().child(lbl));
    }
    el
}

impl Render for SidebarPane {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let pal = Palette::for_mode(self.theme_mode);
        let sidebar_width = if self.sidebar_open {
            tokens::SIDEBAR_WIDTH
        } else {
            74.0
        };
        let app_weak = self.app.clone();

        let brand_header = h_flex()
            .h(px(68.0))
            .flex_shrink_0()
            .px(px(18.0))
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .id("brand-home")
                    .cursor_pointer()
                    .gap(px(11.0))
                    .items_center()
                    .on_click({
                        let app_weak = app_weak.clone();
                        move |_, _, cx| {
                            if let Some(app) = app_weak.upgrade() {
                                app.update(cx, |this, cx| {
                                    this.switch_view(WorkspaceView::Today, cx);
                                });
                            }
                        }
                    })
                    .child(
                        div()
                            .w(px(30.0))
                            .h(px(30.0))
                            .flex_shrink_0()
                            .rounded(px(8.0))
                            .bg(pal.surface_selected())
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(lucide(IconName::Sparkles, 16.0, pal.accent_quiet())),
                    )
                    .when(self.sidebar_open, |el| {
                        el.child(
                            div()
                                .text_size(px(17.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(0xF1F8F3))
                                .whitespace_nowrap()
                                .child("Life Manager"),
                        )
                    }),
            )
            .child(
                div()
                    .id("sidebar-collapse-btn")
                    .cursor_pointer()
                    .w(px(28.0))
                    .h(px(28.0))
                    .flex_shrink_0()
                    .rounded(px(6.0))
                    .hover(|s| s.bg(pal.surface_inverse_raised()))
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_click({
                        let app_weak = app_weak.clone();
                        move |_, _, cx| {
                            if let Some(app) = app_weak.upgrade() {
                                app.update(cx, |this, cx| {
                                    this.toggle_sidebar(cx);
                                });
                            }
                        }
                    })
                    .child(lucide(
                        if self.sidebar_open {
                            IconName::PanelLeftClose
                        } else {
                            IconName::PanelLeftOpen
                        },
                        16.0,
                        pal.icon_inverse(),
                    )),
            );

        // Compact 32px row height so all 18 views fit comfortably in a 900px window without
        // clipping Today/Inbox/Tasks at the top, while still supporting scroll on smaller windows.
        let mut nav = v_flex()
            .id("sidebar-nav")
            .flex_1()
            .min_h_0()
            .px(px(12.0))
            .py(px(4.0))
            .gap(px(10.0))
            .overflow_y_scrollbar()
            .scroll_bounce(false);

        for (group_label, items) in WorkspaceView::GROUPS {
            let mut group = v_flex().gap(px(1.0));
            if self.sidebar_open {
                group = group.child(
                    div()
                        .px(px(11.0))
                        .pb(px(4.0))
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0x7FA597))
                        .whitespace_nowrap()
                        .child(*group_label),
                );
            } else {
                group = group.child(div().h(px(1.0)).mx(px(6.0)).my(px(4.0)).bg(rgb(0x315048)));
            }
            for item in *items {
                let active = self.view == *item;
                let item_copy = *item;
                let app_weak = app_weak.clone();
                let icon_color = if active {
                    pal.icon_inverse_active()
                } else {
                    pal.icon_inverse()
                };

                group = group.child(
                    h_flex()
                        .id(item_copy.nav_id())
                        .cursor_pointer()
                        .h(px(32.0))
                        .px(px(11.0))
                        .gap(px(12.0))
                        .rounded(px(7.0))
                        .bg(if active {
                            pal.surface_selected()
                        } else {
                            pal.surface_inverse()
                        })
                        .hover(|s| {
                            s.bg(if active {
                                pal.surface_selected()
                            } else {
                                rgb(0x213E37)
                            })
                        })
                        .text_color(if active {
                            pal.text_inverse()
                        } else {
                            rgb(0xBED4C9)
                        })
                        .text_size(px(13.0))
                        .font_weight(if active {
                            FontWeight::BOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .items_center()
                        .justify_between()
                        .on_click(move |_, _, cx| {
                            if let Some(app) = app_weak.upgrade() {
                                app.update(cx, |this, cx| {
                                    this.switch_view(item_copy, cx);
                                });
                            }
                        })
                        .child(
                            h_flex()
                                .gap(px(12.0))
                                .items_center()
                                .min_w_0()
                                .child(lucide(item_copy.icon(), 16.0, icon_color))
                                .when(self.sidebar_open, |el| {
                                    el.child(div().whitespace_nowrap().child(item_copy.label()))
                                }),
                        )
                        .when(
                            self.sidebar_open
                                && item_copy == WorkspaceView::Inbox
                                && self.inbox_count > 0,
                            |el| {
                                el.child(
                                    div()
                                        .px(px(6.0))
                                        .py(px(1.0))
                                        .rounded(px(9.0))
                                        .bg(rgb(0x436F5C))
                                        .text_color(rgb(0xEAF5EE))
                                        .text_size(px(10.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child(format!("{}", self.inbox_count)),
                                )
                            },
                        ),
                );
            }
            nav = nav.child(group);
        }

        let footer = v_flex()
            .flex_shrink_0()
            .px(px(18.0))
            .pt(px(8.0))
            .pb(px(16.0))
            .gap(px(12.0))
            .child(div().h(px(1.0)).w_full().bg(rgb(0x496358)))
            .child(
                h_flex()
                    .gap(px(10.0))
                    .items_center()
                    .child(
                        div()
                            .w(px(28.0))
                            .h(px(28.0))
                            .flex_shrink_0()
                            .rounded(px(999.0))
                            .bg(rgb(0xBD7455))
                            .text_color(rgb(0xFFFFFF))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::BOLD)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("L"),
                    )
                    .when(self.sidebar_open, |el| {
                        el.child(
                            v_flex()
                                .gap(px(1.0))
                                .min_w_0()
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(0xE6F1EB))
                                        .whitespace_nowrap()
                                        .child("Personal workspace"),
                                )
                                .child(
                                    h_flex()
                                        .gap(px(5.0))
                                        .items_center()
                                        .child(
                                            div()
                                                .w(px(7.0))
                                                .h(px(7.0))
                                                .rounded(px(999.0))
                                                .bg(rgb(0x3FA47D)),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(10.0))
                                                .text_color(rgb(0xA0B9AE))
                                                .whitespace_nowrap()
                                                .child("Local · Private"),
                                        ),
                                ),
                        )
                    }),
            );

        v_flex()
            .id("primary-sidebar")
            .w(px(sidebar_width))
            .h_full()
            .flex_shrink_0()
            .overflow_hidden()
            .bg(pal.surface_inverse())
            .text_color(pal.text_inverse())
            .child(brand_header)
            .child(nav)
            .child(footer)
    }
}

impl Render for TopbarPane {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let pal = Palette::for_mode(self.theme_mode);
        let app_weak = self.app.clone();

        h_flex()
            .id("workspace-topbar")
            .w_full()
            .h(px(70.0))
            .flex_shrink_0()
            .px(px(41.0))
            .bg(pal.canvas())
            .border_b_1()
            .border_color(pal.border())
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap(px(12.0))
                    .items_center()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(pal.text_muted())
                    .child("WORKSPACE")
                    .child(
                        div()
                            .font_weight(FontWeight::NORMAL)
                            .text_color(pal.text_faint())
                            .child("/"),
                    )
                    .child(
                        div()
                            .id("active-view-breadcrumb")
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .text_color(pal.accent_text())
                            .child(self.view.label().to_uppercase()),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(12.0))
                    .items_center()
                    .child(
                        h_flex()
                            .id("topbar-search-trigger")
                            .cursor_pointer()
                            .w(px(247.0))
                            .h(px(38.0))
                            .px(px(12.0))
                            .gap(px(10.0))
                            .rounded(px(8.0))
                            .bg(pal.surface())
                            .border_1()
                            .border_color(pal.border())
                            .hover(|s| s.border_color(pal.border_strong()))
                            .items_center()
                            .justify_between()
                            .on_click({
                                let app_weak = app_weak.clone();
                                move |_, _, cx| {
                                    if let Some(app) = app_weak.upgrade() {
                                        app.update(cx, |this, cx| {
                                            this.command_palette_open = !this.command_palette_open;
                                            cx.notify();
                                        });
                                    }
                                }
                            })
                            .child(
                                h_flex()
                                    .gap(px(10.0))
                                    .items_center()
                                    .child(lucide(IconName::Search, 15.0, pal.text_faint()))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(pal.text_faint())
                                            .child("Search anything..."),
                                    ),
                            )
                            .child(
                                div()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(pal.surface_raised())
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(pal.text_muted())
                                    .child("⌘ K"),
                            ),
                    )
                    .child(
                        div()
                            .id("topbar-command-btn")
                            .cursor_pointer()
                            .w(px(36.0))
                            .h(px(36.0))
                            .rounded(px(8.0))
                            .hover(|s| s.bg(pal.surface_raised()))
                            .flex()
                            .items_center()
                            .justify_center()
                            .on_click({
                                let app_weak = app_weak.clone();
                                move |_, _, cx| {
                                    if let Some(app) = app_weak.upgrade() {
                                        app.update(cx, |this, cx| {
                                            this.command_palette_open = !this.command_palette_open;
                                            cx.notify();
                                        });
                                    }
                                }
                            })
                            .child(lucide(IconName::Command, 18.0, pal.text_muted())),
                    )
                    .child(
                        div()
                            .id("topbar-theme-btn")
                            .cursor_pointer()
                            .w(px(36.0))
                            .h(px(36.0))
                            .rounded(px(8.0))
                            .hover(|s| s.bg(pal.surface_raised()))
                            .flex()
                            .items_center()
                            .justify_center()
                            .on_click({
                                let app_weak = app_weak.clone();
                                move |_, _, cx| {
                                    if let Some(app) = app_weak.upgrade() {
                                        app.update(cx, |this, cx| {
                                            this.toggle_theme(cx);
                                        });
                                    }
                                }
                            })
                            .child(lucide(
                                match self.theme_mode {
                                    ThemeMode::Light => IconName::Moon,
                                    ThemeMode::Dark => IconName::Sun,
                                },
                                18.0,
                                pal.text_muted(),
                            )),
                    )
                    .child(
                        div()
                            .id("topbar-inbox-btn")
                            .cursor_pointer()
                            .relative()
                            .w(px(36.0))
                            .h(px(36.0))
                            .rounded(px(8.0))
                            .hover(|s| s.bg(pal.surface_raised()))
                            .flex()
                            .items_center()
                            .justify_center()
                            .on_click({
                                let app_weak = app_weak.clone();
                                move |_, _, cx| {
                                    if let Some(app) = app_weak.upgrade() {
                                        app.update(cx, |this, cx| {
                                            this.switch_view(WorkspaceView::Inbox, cx);
                                        });
                                    }
                                }
                            })
                            .child(lucide(IconName::Bell, 18.0, pal.text_muted()))
                            .when(self.inbox_count > 0, |el| {
                                el.child(
                                    div()
                                        .w(px(7.0))
                                        .h(px(7.0))
                                        .rounded(px(999.0))
                                        .bg(pal.warning())
                                        .absolute()
                                        .top(px(7.0))
                                        .right(px(7.0)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .w(px(29.0))
                            .h(px(29.0))
                            .rounded(px(999.0))
                            .bg(rgb(0xF3E5DA))
                            .text_color(rgb(0xB56E4C))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::BOLD)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("L"),
                    ),
            )
    }
}

impl LifeManagerApp {
    pub(crate) fn surface_card(&self, pal: Palette) -> gpui_kit::Div {
        v_flex()
            .min_w_0()
            .bg(pal.surface())
            .border_1()
            .border_color(pal.border())
            .rounded(px(tokens::RADIUS_LG))
            .p(px(if self.compact_density { 16.0 } else { 22.0 }))
            .gap(px(14.0))
    }

    fn metric_card(
        &self,
        pal: Palette,
        label: &'static str,
        primary_val: String,
        secondary_val: Option<String>,
        hint: &'static str,
    ) -> AnyElement {
        v_flex()
            .flex_1()
            .min_w_0()
            .min_h(px(if self.compact_density { 105.0 } else { 132.0 }))
            .px(px(24.0))
            .py(px(20.0))
            .bg(pal.surface())
            .border_1()
            .border_color(pal.border())
            .rounded(px(tokens::RADIUS_LG))
            .justify_between()
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .text_color(pal.text_muted())
                    .child(label),
            )
            .child(
                h_flex()
                    .gap(px(6.0))
                    .items_baseline()
                    .child(
                        div()
                            .text_size(px(if self.compact_density { 28.0 } else { 35.0 }))
                            .font_weight(FontWeight::BOLD)
                            .text_color(pal.text())
                            .child(primary_val),
                    )
                    .when_some(secondary_val, |el, sec| {
                        el.child(
                            div()
                                .text_size(px(21.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(pal.text_faint())
                                .child(sec),
                        )
                    }),
            )
            .child(
                h_flex()
                    .gap(px(4.0))
                    .items_center()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(pal.accent_text())
                    .child(div().truncate().child(hint))
                    .child(lucide(IconName::ArrowUpRight, 13.0, pal.accent_text())),
            )
            .into_any_element()
    }

    fn render_unlock_screen(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .id("unlock-screen")
            .size_full()
            .bg(pal.canvas())
            .text_color(pal.text())
            .items_center()
            .justify_center()
            .p(px(32.0))
            .child(
                self.surface_card(pal)
                    .w(px(440.0))
                    .child(
                        div()
                            .text_size(px(tokens::TEXT_EYEBROW))
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .text_color(pal.accent_text())
                            .child("DATABASE PROTECTION"),
                    )
                    .child(
                        div()
                            .text_size(px(tokens::TEXT_TITLE))
                            .font_weight(FontWeight::BOLD)
                            .child("Unlock your encrypted workspace"),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .whitespace_normal()
                            .text_size(px(tokens::TEXT_BODY))
                            .text_color(pal.text_muted())
                            .child("Enter your SQLCipher passphrase to open Life Manager."),
                    )
                    .when_some(self.error.clone(), |el, err| {
                        el.child(
                            div()
                                .w_full()
                                .min_w_0()
                                .whitespace_normal()
                                .p(px(10.0))
                                .rounded(px(tokens::RADIUS_SM))
                                .bg(pal.danger_muted())
                                .text_color(pal.danger())
                                .text_size(px(tokens::TEXT_LABEL))
                                .child(err),
                        )
                    })
                    .child(Input::new(&self.passphrase_input))
                    .child(
                        primary_action(
                            "unlock-submit",
                            Some(IconName::ShieldCheck),
                            "Unlock workspace",
                            pal,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.unlock_with_passphrase(window, cx);
                        })),
                    ),
            )
            .into_any_element()
    }

    fn render_mint_capture_strip(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let p = self.draft_priority;
        h_flex()
            .w_full()
            .min_w_0()
            .min_h(px(47.0))
            .px(px(14.0))
            .py(px(6.0))
            .gap(px(10.0))
            .rounded(px(9.0))
            .bg(pal.accent_muted())
            .items_center()
            .child(lucide(IconName::Plus, 17.0, pal.accent_text()))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&self.quick_task_input)),
            )
            .child(
                div()
                    .id("cycle-draft-priority")
                    .cursor_pointer()
                    .flex_shrink_0()
                    .px(px(10.0))
                    .py(px(5.0))
                    .rounded(px(6.0))
                    .bg(pal.surface())
                    .text_color(pal.accent_text())
                    .text_size(px(11.0))
                    .font_weight(FontWeight::BOLD)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.draft_priority = if this.draft_priority >= 4 {
                            1
                        } else {
                            this.draft_priority + 1
                        };
                        cx.notify();
                    }))
                    .child(format!("P{p} · {}", priority_label(p))),
            )
            .child(
                h_flex()
                    .id("quick-task-submit")
                    .cursor_pointer()
                    .flex_shrink_0()
                    .px(px(10.0))
                    .py(px(6.0))
                    .gap(px(4.0))
                    .rounded(px(6.0))
                    .hover(|s| s.bg(pal.accent_quiet()))
                    .text_color(pal.accent_text())
                    .text_size(px(11.0))
                    .font_weight(FontWeight::BOLD)
                    .items_center()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.submit_quick_task(window, cx);
                    }))
                    .child("Add")
                    .child(lucide(IconName::ArrowRight, 14.0, pal.accent_text())),
            )
            .into_any_element()
    }

    fn render_hairline_task_row(
        &self,
        pal: Palette,
        task: &TaskRecord,
        id_prefix: &str,
        show_when_column: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let task_for_toggle = task.clone();
        let task_id_for_select = task.id.clone();
        let is_done = task.status == "done";
        let is_selected = self.selected_task_id.as_deref() == Some(&task.id);
        let subtitle = format!(
            "{}  ·  {}  ·  {} min",
            task.project_name
                .as_deref()
                .or(task.area_name.as_deref())
                .unwrap_or("Personal"),
            task.status.replace('_', " "),
            task.estimate_minutes
        );

        h_flex()
            .id(SharedString::from(format!("{id_prefix}-row-{}", task.id)))
            .w_full()
            .min_w_0()
            .min_h(px(if self.compact_density { 50.0 } else { 61.0 }))
            .px(px(8.0))
            .py(px(6.0))
            .border_b_1()
            .border_color(pal.border())
            .bg(if is_selected {
                pal.surface_raised()
            } else {
                pal.surface()
            })
            .hover(|s| s.bg(pal.surface_raised()))
            .justify_between()
            .items_center()
            .gap(px(14.0))
            .child(
                h_flex()
                    .gap(px(14.0))
                    .items_center()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .id(SharedString::from(format!("{id_prefix}-check-{}", task.id)))
                            .cursor_pointer()
                            .w(px(20.0))
                            .h(px(20.0))
                            .flex_shrink_0()
                            .rounded(px(999.0))
                            .border_1()
                            .border_color(if is_done { pal.accent() } else { rgb(0xB6C9BD) })
                            .bg(if is_done { pal.accent() } else { pal.surface() })
                            .hover(|s| s.bg(pal.accent()).border_color(pal.accent()))
                            .flex()
                            .items_center()
                            .justify_center()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_task_complete(&task_for_toggle, cx);
                            }))
                            .when(is_done, |el| {
                                el.child(lucide(IconName::Check, 12.0, pal.text_on_accent()))
                            }),
                    )
                    .child(
                        v_flex()
                            .id(SharedString::from(format!("{id_prefix}-open-{}", task.id)))
                            .cursor_pointer()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .gap(px(3.0))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected_task_id = Some(task_id_for_select.clone());
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(if is_done {
                                        pal.text_faint()
                                    } else {
                                        pal.text()
                                    })
                                    .child(task.title.clone()),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child(subtitle),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(12.0))
                    .items_center()
                    .flex_shrink_0()
                    .when(show_when_column, |el| {
                        el.child(
                            div()
                                .w(px(88.0))
                                .text_size(px(11.0))
                                .text_color(pal.text_muted())
                                .whitespace_nowrap()
                                .child(format_short_date(task.due_date.as_deref())),
                        )
                    })
                    .child(
                        div()
                            .px(px(9.0))
                            .py(px(4.0))
                            .rounded(px(6.0))
                            .bg(if task.priority == 1 {
                                pal.warning_muted()
                            } else {
                                pal.surface_raised()
                            })
                            .text_color(if task.priority == 1 {
                                pal.warning_text()
                            } else {
                                pal.text_muted()
                            })
                            .text_size(px(10.0))
                            .font_weight(FontWeight::BOLD)
                            .child(format!("P{}", task.priority)),
                    ),
            )
            .into_any_element()
    }

    fn render_today(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let open_count = self.cached_open_count;
        let urgent_count = self.cached_urgent_count;

        let metric_row = h_flex()
            .gap(px(15.0))
            .w_full()
            .min_w_0()
            .child(self.metric_card(
                pal,
                "OPEN PRIORITIES",
                format!("{urgent_count:02}"),
                Some(format!("/ {open_count:02}")),
                "Know what deserves attention",
            ))
            .child(self.metric_card(
                pal,
                "FOCUS RECORDED",
                format_duration(self.snapshot.stats.focus_seconds),
                None,
                "Intentional time invested",
            ))
            .child(self.metric_card(
                pal,
                "COMPLETED TASKS",
                format!("{:02}", self.snapshot.stats.completed_tasks),
                None,
                "Progress, one step at a time",
            ));

        let mut priority_rows = v_flex().w_full().min_w_0().min_h(px(240.0));
        if self.cached_today_priority_tasks.is_empty() {
            priority_rows = priority_rows
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .child(lucide(IconName::CircleCheck, 28.0, pal.accent_text()))
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(pal.text())
                        .child("A little room to breathe."),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(pal.text_muted())
                        .child("No outstanding tasks right now."),
                );
        } else {
            for task in &self.cached_today_priority_tasks {
                priority_rows = priority_rows
                    .child(self.render_hairline_task_row(pal, task, "today", true, cx));
            }
        }

        let actions_card = self
            .surface_card(pal)
            .flex_1()
            .min_w_0()
            .min_h(px(445.0))
            .justify_between()
            .child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(14.0))
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_start()
                            .child(
                                v_flex()
                                    .min_w_0()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .text_size(px(17.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(pal.text())
                                            .child("Your next meaningful actions"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child("Selected for impact, not just urgency"),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .id("today-all-tasks")
                                    .cursor_pointer()
                                    .flex_shrink_0()
                                    .gap(px(4.0))
                                    .items_center()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.accent_text())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.switch_view(WorkspaceView::Tasks, cx);
                                    }))
                                    .child("All tasks")
                                    .child(lucide(IconName::ArrowUpRight, 14.0, pal.accent_text())),
                            ),
                    )
                    .child(priority_rows),
            )
            .child(self.render_mint_capture_strip(pal, cx));

        let mut agenda_items = v_flex().w_full().min_w_0().gap(px(10.0));
        if self.snapshot.events.is_empty() {
            for (clock, title, sub, is_muted) in [
                (
                    "09:00",
                    "Make room for focus",
                    "Your schedule is open",
                    false,
                ),
                (
                    "13:00",
                    "Choose your next step",
                    "Schedule an intentional block",
                    true,
                ),
            ] {
                agenda_items = agenda_items.child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .gap(px(11.0))
                        .py(px(6.0))
                        .items_start()
                        .child(
                            div()
                                .w(px(42.0))
                                .flex_shrink_0()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(pal.text_muted())
                                .child(clock),
                        )
                        .child(
                            div()
                                .w(px(3.0))
                                .h(px(33.0))
                                .flex_shrink_0()
                                .rounded(px(4.0))
                                .bg(if is_muted {
                                    pal.warning()
                                } else {
                                    pal.accent()
                                }),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(pal.text())
                                        .truncate()
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(pal.text_muted())
                                        .truncate()
                                        .child(sub),
                                ),
                        ),
                );
            }
        } else {
            for ev in self.snapshot.events.iter().take(4) {
                agenda_items = agenda_items.child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .gap(px(11.0))
                        .py(px(6.0))
                        .items_start()
                        .child(
                            div()
                                .w(px(42.0))
                                .flex_shrink_0()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(pal.text_muted())
                                .child(format_clock(&ev.start_at)),
                        )
                        .child(
                            div()
                                .w(px(3.0))
                                .h(px(33.0))
                                .flex_shrink_0()
                                .rounded(px(4.0))
                                .bg(pal.accent()),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(pal.text())
                                        .truncate()
                                        .child(ev.title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(pal.text_muted())
                                        .truncate()
                                        .child(if ev.kind == "block" {
                                            "Protected work"
                                        } else {
                                            "Calendar event"
                                        }),
                                ),
                        ),
                );
            }
        }

        let agenda_card = self
            .surface_card(pal)
            .min_h(px(207.0))
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_start()
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.text())
                                    .child("The shape of your day"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child("Time blocks and commitments"),
                            ),
                    )
                    .child(
                        div()
                            .id("today-open-calendar")
                            .cursor_pointer()
                            .p(px(4.0))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.switch_view(WorkspaceView::Calendar, cx);
                            }))
                            .child(lucide(IconName::ArrowUpRight, 16.0, pal.text_muted())),
                    ),
            )
            .child(agenda_items);

        let total_habits = self.snapshot.habits.len();
        let completed_habits = self.cached_completed_habits;
        let habit_ratio = if total_habits > 0 {
            (completed_habits as f32 / total_habits as f32).clamp(0.06, 1.0)
        } else {
            0.08
        };
        let rhythm_card = self
            .surface_card(pal)
            .min_h(px(208.0))
            .justify_between()
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_start()
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.text())
                                    .child("Keep the rhythm"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child("Sustainable effort is productive effort."),
                            ),
                    )
                    .child(lucide(IconName::Sparkles, 18.0, pal.accent_text())),
            )
            .child(
                h_flex()
                    .w_full()
                    .justify_between()
                    .items_end()
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .text_color(pal.text_muted())
                                    .child("ACTIVE HABITS"),
                            )
                            .child(
                                h_flex()
                                    .gap(px(6.0))
                                    .items_baseline()
                                    .child(
                                        div()
                                            .text_size(px(29.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(pal.text())
                                            .child(format!("{total_habits}")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(pal.text_muted())
                                            .child("routines"),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .id("today-open-habits")
                            .cursor_pointer()
                            .flex_shrink_0()
                            .px(px(11.0))
                            .py(px(7.0))
                            .gap(px(6.0))
                            .rounded(px(7.0))
                            .bg(pal.accent_muted())
                            .text_color(pal.accent_text())
                            .text_size(px(11.0))
                            .font_weight(FontWeight::BOLD)
                            .items_center()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.switch_view(WorkspaceView::Habits, cx);
                            }))
                            .child("Review habits")
                            .child(lucide(IconName::ArrowRight, 14.0, pal.accent_text())),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .h(px(7.0))
                    .rounded(px(8.0))
                    .bg(pal.surface_raised())
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w(relative(habit_ratio))
                            .rounded(px(8.0))
                            .bg(pal.accent()),
                    ),
            );

        let mut projects_row = h_flex().gap(px(15.0)).w_full().min_w_0();
        for project in self.snapshot.projects.iter().take(3) {
            let pid = project.id.clone();
            let open_for_proj = self
                .snapshot
                .tasks
                .iter()
                .filter(|t| t.project_id.as_deref() == Some(&project.id) && t.status != "done")
                .count();
            projects_row = projects_row.child(
                v_flex()
                    .id(SharedString::from(format!("today-proj-{pid}")))
                    .cursor_pointer()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .min_h(px(166.0))
                    .p(px(20.0))
                    .rounded(px(13.0))
                    .bg(pal.surface())
                    .border_1()
                    .border_color(pal.border())
                    .hover(|s| s.border_color(pal.border_strong()))
                    .justify_between()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_project_id = Some(pid.clone());
                        this.switch_view(WorkspaceView::Tasks, cx);
                    }))
                    .child(
                        v_flex()
                            .w_full()
                            .min_w_0()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .w(px(34.0))
                                    .h(px(34.0))
                                    .rounded(px(9.0))
                                    .bg(pal.accent_muted())
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(lucide(IconName::FolderKanban, 18.0, pal.accent_text())),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(16.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.text())
                                    .child(project.name.clone()),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child(if project.description.is_empty() {
                                        "An active commitment".to_string()
                                    } else {
                                        project.description.clone()
                                    }),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .pt(px(11.0))
                            .border_t_1()
                            .border_color(pal.border())
                            .justify_between()
                            .items_center()
                            .child(
                                h_flex()
                                    .gap(px(6.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .w(px(6.0))
                                            .h(px(6.0))
                                            .rounded(px(999.0))
                                            .bg(pal.accent()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(pal.accent_text())
                                            .child(format!("{open_for_proj} open tasks")),
                                    ),
                            )
                            .child(lucide(IconName::ArrowUpRight, 14.0, pal.accent_text())),
                    ),
            );
        }

        v_flex()
            .gap(px(20.0))
            .w_full()
            .min_w_0()
            .child(metric_row)
            .child(
                h_flex()
                    .items_start()
                    .gap(px(16.0))
                    .w_full()
                    .min_w_0()
                    .child(actions_card)
                    .child(
                        v_flex()
                            .w(px(350.0))
                            .flex_shrink_0()
                            .gap(px(15.0))
                            .child(agenda_card)
                            .child(rhythm_card),
                    ),
            )
            .child(
                v_flex()
                    .pt(px(12.0))
                    .gap(px(16.0))
                    .w_full()
                    .min_w_0()
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .text_size(px(17.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(pal.text())
                                            .child("Projects in motion"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child("Things you're steadily bringing to life"),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .id("today-all-projects")
                                    .cursor_pointer()
                                    .gap(px(4.0))
                                    .items_center()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.accent_text())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.switch_view(WorkspaceView::Projects, cx);
                                    }))
                                    .child("View all projects")
                                    .child(lucide(IconName::ArrowUpRight, 14.0, pal.accent_text())),
                            ),
                    )
                    .child(projects_row),
            )
            .into_any_element()
    }

    fn render_tasks_or_inbox(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let filters = if self.view == WorkspaceView::Inbox {
            &[TaskFilter::Inbox, TaskFilter::All, TaskFilter::Today][..]
        } else {
            &[
                TaskFilter::Open,
                TaskFilter::Today,
                TaskFilter::All,
                TaskFilter::Completed,
            ][..]
        };

        let mut pill_tabs = h_flex()
            .p(px(4.0))
            .gap(px(3.0))
            .rounded(px(10.0))
            .bg(pal.surface())
            .border_1()
            .border_color(pal.border());

        for f in filters {
            let f_copy = *f;
            let chosen = self.task_filter == f_copy;
            let label = if f_copy == TaskFilter::Inbox && self.cached_inbox_count > 0 {
                format!("Inbox {}", self.cached_inbox_count)
            } else {
                f_copy.label().to_string()
            };
            pill_tabs = pill_tabs.child(
                div()
                    .id(SharedString::from(format!(
                        "task-filter-{}",
                        f_copy.label()
                    )))
                    .cursor_pointer()
                    .px(px(12.0))
                    .py(px(7.0))
                    .rounded(px(7.0))
                    .bg(if chosen {
                        pal.accent_muted()
                    } else {
                        pal.surface()
                    })
                    .text_color(if chosen {
                        pal.accent_text()
                    } else {
                        pal.text_muted()
                    })
                    .text_size(px(11.0))
                    .font_weight(FontWeight::BOLD)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_task_filter(f_copy, cx);
                    }))
                    .child(label),
            );
        }

        let sort_label = format!("Sort: {}", self.task_sort.label());
        let toolbar = h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .child(pill_tabs)
            .child(
                h_flex()
                    .gap(px(9.0))
                    .items_center()
                    .when(self.selected_project_id.is_some(), |el| {
                        el.child(
                            soft_action("clear-project-filter", None, "All projects", pal)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.selected_project_id = None;
                                    this.recompute_derived();
                                    cx.notify();
                                })),
                        )
                    })
                    .child(
                        soft_action("task-sort-btn", Some(IconName::Layers), sort_label, pal)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cycle_task_sort(cx);
                            })),
                    )
                    .child(
                        primary_action("toolbar-new-task", Some(IconName::Plus), "Add task", pal)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.submit_quick_task(window, cx);
                            })),
                    ),
            );

        let filtered_len = self.cached_filtered_tasks.len();
        let table_header = h_flex()
            .w_full()
            .px(px(42.0))
            .pb(px(10.0))
            .border_b_1()
            .border_color(pal.border())
            .justify_between()
            .text_size(px(10.0))
            .font_weight(FontWeight::BOLD)
            .text_color(pal.text_muted())
            .child("ITEM")
            .child(
                h_flex()
                    .gap(px(28.0))
                    .child(div().w(px(88.0)).child("WHEN"))
                    .child("PRIORITY"),
            );

        let mut list = v_flex().w_full().min_w_0();
        if self.cached_filtered_tasks.is_empty() {
            list = list.child(
                v_flex()
                    .min_h(px(240.0))
                    .items_center()
                    .justify_center()
                    .gap(px(10.0))
                    .child(lucide(IconName::Inbox, 34.0, pal.accent_text()))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(pal.text())
                            .child(if self.task_filter == TaskFilter::Inbox {
                                "Your inbox is clear."
                            } else {
                                "Nothing here yet."
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(pal.text_muted())
                            .child("Give yourself room for the work that matters."),
                    ),
            );
        } else {
            for task in &self.cached_filtered_tasks {
                list = list.child(self.render_hairline_task_row(pal, task, "tasks", true, cx));
            }
        }

        let p1_count = self
            .snapshot
            .tasks
            .iter()
            .filter(|t| t.status != "done" && t.priority == 1)
            .count();
        let done_count = self.snapshot.stats.completed_tasks;
        let today_due_count = self.cached_today_due_count;

        let lens_card = self
            .surface_card(pal)
            .child(
                v_flex()
                    .gap(px(3.0))
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Task lens"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(pal.text_muted())
                            .child("Make large lists feel smaller"),
                    ),
            )
            .child(
                v_flex()
                    .gap(px(2.0))
                    .child(
                        h_flex()
                            .id("lens-urgent")
                            .cursor_pointer()
                            .py(px(10.0))
                            .justify_between()
                            .items_center()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_task_filter(TaskFilter::Open, cx);
                            }))
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .w(px(9.0))
                                            .h(px(9.0))
                                            .rounded(px(999.0))
                                            .bg(pal.warning()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(pal.text_secondary())
                                            .child("Urgent priorities"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.accent_text())
                                    .child(format!("{p1_count:02}")),
                            ),
                    )
                    .child(
                        h_flex()
                            .id("lens-completed")
                            .cursor_pointer()
                            .py(px(10.0))
                            .justify_between()
                            .items_center()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_task_filter(TaskFilter::Completed, cx);
                            }))
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .w(px(9.0))
                                            .h(px(9.0))
                                            .rounded(px(999.0))
                                            .bg(pal.accent()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(pal.text_secondary())
                                            .child("Completed"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.accent_text())
                                    .child(format!("{done_count:02}")),
                            ),
                    )
                    .child(
                        h_flex()
                            .id("lens-today")
                            .cursor_pointer()
                            .py(px(10.0))
                            .justify_between()
                            .items_center()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_task_filter(TaskFilter::Today, cx);
                            }))
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .w(px(9.0))
                                            .h(px(9.0))
                                            .rounded(px(999.0))
                                            .bg(pal.text_faint()),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(pal.text_secondary())
                                            .child("Due today"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.accent_text())
                                    .child(format!("{today_due_count:02}")),
                            ),
                    ),
            );

        let selected_task = self
            .selected_task_id
            .as_ref()
            .and_then(|id| self.snapshot.tasks.iter().find(|t| &t.id == id))
            .cloned();

        let inspector_card =
            self.surface_card(pal)
                .child(if let Some(task) = selected_task {
                    let task_for_complete = task.clone();
                    let task_id_for_del = task.id.clone();
                    let tid_plan = task.id.clone();
                    let tid_snooze = task.id.clone();
                    let tid_focus = task.id.clone();
                    let tid_tpl = task.title.clone();
                    let desc_tpl = task.description.clone();
                    let prio_tpl = task.priority;
                    let today_str = Local::now().format("%Y-%m-%d").to_string();
                    let tomorrow_str = (Local::now().date_naive() + Duration::days(1))
                        .format("%Y-%m-%d")
                        .to_string();

                    let mut prio_pills = h_flex().gap(px(6.0));
                    for p in 1..=4 {
                        let tid_p = task.id.clone();
                        let active_p = task.priority == p;
                        prio_pills = prio_pills.child(
                            div()
                                .id(SharedString::from(format!("insp-prio-{p}")))
                                .cursor_pointer()
                                .px(px(8.0))
                                .py(px(4.0))
                                .rounded(px(6.0))
                                .bg(if active_p {
                                    pal.accent()
                                } else {
                                    pal.surface_raised()
                                })
                                .text_color(if active_p {
                                    pal.text_on_accent()
                                } else {
                                    pal.text_secondary()
                                })
                                .text_size(px(10.0))
                                .font_weight(FontWeight::BOLD)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.exec(
                                        "tasks.update",
                                        json!({ "id": tid_p, "priority": p }),
                                        format!("Priority updated to P{p}"),
                                    );
                                    cx.notify();
                                }))
                                .child(format!("P{p}")),
                        );
                    }

                    v_flex()
                        .w_full()
                        .min_w_0()
                        .gap(px(12.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(FontWeight::EXTRA_BOLD)
                                .text_color(pal.accent_text())
                                .child(format!("TASK DETAILS · REV #{}", task.revision)),
                        )
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .whitespace_normal()
                                .text_size(px(17.0))
                                .font_weight(FontWeight::BOLD)
                                .child(task.title.clone()),
                        )
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .whitespace_normal()
                                .text_size(px(12.0))
                                .text_color(pal.text_muted())
                                .child(if task.description.is_empty() {
                                    "No detailed description yet.".to_string()
                                } else {
                                    task.description.clone()
                                }),
                        )
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(pal.text_secondary())
                                        .child(format!(
                                            "{} · {}",
                                            priority_label(task.priority),
                                            task.status
                                        )),
                                )
                                .child(prio_pills),
                        )
                        .child(
                            h_flex()
                                .gap(px(6.0))
                                .flex_wrap()
                                .child(
                                    soft_action(
                                        "inspector-plan-today",
                                        Some(IconName::CalendarDays),
                                        "Plan today",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                            "tasks.update",
                                            json!({ "id": tid_plan, "planned_date": today_str }),
                                            "Planned for today",
                                        );
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    soft_action(
                                        "inspector-snooze-1d",
                                        Some(IconName::Clock3),
                                        "Snooze +1d",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                                "tasks.snooze",
                                                json!({ "id": tid_snooze, "date": tomorrow_str }),
                                                "Task snoozed to tomorrow",
                                            );
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    soft_action(
                                        "inspector-focus-task",
                                        Some(IconName::Timer),
                                        "Focus",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.selected_focus_task_id = Some(tid_focus.clone());
                                            this.switch_view(WorkspaceView::Focus, cx);
                                        },
                                    )),
                                )
                                .child(
                                    soft_action(
                                        "inspector-save-template",
                                        Some(IconName::BookOpen),
                                        "Save template",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                                "templates.create",
                                                json!({
                                                    "name": tid_tpl,
                                                    "title": tid_tpl,
                                                    "description": desc_tpl,
                                                    "priority": prio_tpl,
                                                    "estimate_minutes": 30
                                                }),
                                                "Saved to Library templates",
                                            );
                                            cx.notify();
                                        },
                                    )),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap(px(8.0))
                                .flex_wrap()
                                .child(
                                    primary_action(
                                        "inspector-toggle-complete",
                                        Some(IconName::Check),
                                        if task.status == "done" {
                                            "Reopen"
                                        } else {
                                            "Mark complete"
                                        },
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.toggle_task_complete(&task_for_complete, cx);
                                        },
                                    )),
                                )
                                .child(
                                    soft_action("inspector-archive", None, "Archive", pal)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.delete_task(&task_id_for_del, cx);
                                        })),
                                ),
                        )
                } else {
                    v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Saved views & details"),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .whitespace_normal()
                            .text_size(px(12.0))
                            .text_color(pal.text_muted())
                            .child(format!(
                                "{filtered_len} items in queue. Click any row to inspect details."
                            )),
                    )
                });

        let shortcuts_card = self
            .surface_card(pal)
            .child(
                v_flex()
                    .gap(px(3.0))
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Useful shortcuts"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(pal.text_muted())
                            .child("Jump straight to the slice of work you need"),
                    ),
            )
            .child(
                v_flex()
                    .gap(px(4.0))
                    .child(
                        h_flex()
                            .id("shortcut-today-commitments")
                            .cursor_pointer()
                            .py(px(8.0))
                            .gap(px(8.0))
                            .items_center()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(pal.accent_text())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_task_filter(TaskFilter::Today, cx);
                            }))
                            .child(lucide(IconName::ArrowUpRight, 14.0, pal.accent_text()))
                            .child("Today's commitments"),
                    )
                    .child(
                        h_flex()
                            .id("shortcut-highest-priority")
                            .cursor_pointer()
                            .py(px(8.0))
                            .gap(px(8.0))
                            .items_center()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(pal.accent_text())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_task_filter(TaskFilter::Open, cx);
                            }))
                            .child(lucide(IconName::ArrowUpRight, 14.0, pal.accent_text()))
                            .child("Highest priority first"),
                    )
                    .child(
                        h_flex()
                            .id("shortcut-upcoming-deadlines")
                            .cursor_pointer()
                            .py(px(8.0))
                            .gap(px(8.0))
                            .items_center()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(pal.accent_text())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.switch_view(WorkspaceView::Calendar, cx);
                            }))
                            .child(lucide(IconName::ArrowUpRight, 14.0, pal.accent_text()))
                            .child("Upcoming deadlines"),
                    )
                    .child(
                        h_flex()
                            .id("shortcut-capture-inbox")
                            .cursor_pointer()
                            .py(px(8.0))
                            .gap(px(8.0))
                            .items_center()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(pal.accent_text())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.switch_view(WorkspaceView::Inbox, cx);
                            }))
                            .child(lucide(IconName::ArrowUpRight, 14.0, pal.accent_text()))
                            .child("Capture inbox"),
                    ),
            );

        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(toolbar)
            .child(
                h_flex()
                    .items_start()
                    .gap(px(15.0))
                    .w_full()
                    .min_w_0()
                    .child(
                        self.surface_card(pal)
                            .flex_1()
                            .min_w_0()
                            .min_h(px(520.0))
                            .justify_between()
                            .child(
                                v_flex()
                                    .w_full()
                                    .min_w_0()
                                    .gap(px(14.0))
                                    .child(
                                        v_flex()
                                            .gap(px(4.0))
                                            .child(
                                                div()
                                                    .text_size(px(17.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .child(if self.view == WorkspaceView::Inbox {
                                                        "Captured, not forgotten"
                                                    } else {
                                                        "Priority queue"
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(pal.text_muted())
                                                    .child(format!(
                                                        "{filtered_len} items · Select one to see the details"
                                                    )),
                                            ),
                                    )
                                    .child(table_header)
                                    .child(list),
                            )
                            .child(self.render_mint_capture_strip(pal, cx)),
                    )
                    .child(
                        v_flex()
                            .w(px(310.0))
                            .flex_shrink_0()
                            .gap(px(15.0))
                            .child(lens_card)
                            .child(inspector_card)
                            .child(shortcuts_card),
                    ),
            )
            .child(self.render_tasks_v16_panel(pal, cx))
            .into_any_element()
    }

    fn render_calendar_or_planning(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let visible_days = self.calendar_visible_days();
        let ref_date = self.calendar_reference_date();
        let period_title = match self.calendar_mode {
            CalendarMode::Month => ref_date.format("%B %Y").to_string(),
            CalendarMode::ThreeDays | CalendarMode::Week => format!(
                "{} – {}",
                visible_days.first().unwrap().format("%b %-d"),
                visible_days.last().unwrap().format("%b %-d, %Y")
            ),
            CalendarMode::Day | CalendarMode::Agenda => {
                ref_date.format("%A, %B %-d, %Y").to_string()
            }
        };

        let mut mode_tabs = h_flex()
            .p(px(4.0))
            .gap(px(3.0))
            .rounded(px(10.0))
            .bg(pal.surface())
            .border_1()
            .border_color(pal.border());
        for mode in CalendarMode::ALL {
            let m = *mode;
            let active = self.calendar_mode == m;
            mode_tabs = mode_tabs.child(
                div()
                    .id(SharedString::from(format!("cal-mode-{}", m.label())))
                    .cursor_pointer()
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(7.0))
                    .bg(if active {
                        pal.accent_muted()
                    } else {
                        pal.surface()
                    })
                    .text_color(if active {
                        pal.accent_text()
                    } else {
                        pal.text_muted()
                    })
                    .text_size(px(11.0))
                    .font_weight(FontWeight::BOLD)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.calendar_mode = m;
                        cx.notify();
                    }))
                    .child(m.label()),
            );
        }

        let calendar_toolbar = h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap(px(9.0))
                    .items_center()
                    .child(
                        soft_action("cal-prev", Some(IconName::ChevronLeft), "", pal).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.shift_calendar(-1, cx);
                            }),
                        ),
                    )
                    .child(
                        soft_action("cal-today", None, "Today", pal).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.calendar_offset_days = 0;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        soft_action("cal-next", Some(IconName::ChevronRight), "", pal).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.shift_calendar(1, cx);
                            }),
                        ),
                    )
                    .child(
                        div()
                            .pl(px(8.0))
                            .text_size(px(19.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(pal.text())
                            .child(period_title),
                    ),
            )
            .child(mode_tabs);

        let grid_or_agenda = if self.calendar_mode == CalendarMode::Agenda {
            let mut list = v_flex().w_full().min_w_0().gap(px(8.0));
            if self.snapshot.events.is_empty() {
                list = list.child(
                    v_flex()
                        .min_h(px(220.0))
                        .items_center()
                        .justify_center()
                        .gap(px(8.0))
                        .child(lucide(IconName::CalendarDays, 32.0, pal.accent_text()))
                        .child(
                            div()
                                .text_size(px(18.0))
                                .font_weight(FontWeight::BOLD)
                                .child("Your calendar is open."),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(pal.text_muted())
                                .child("Make space for your next meaningful commitment."),
                        ),
                );
            } else {
                for ev in &self.snapshot.events {
                    let ev_id = ev.id.clone();
                    list = list.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .py(px(12.0))
                            .px(px(10.0))
                            .border_b_1()
                            .border_color(pal.border())
                            .justify_between()
                            .items_center()
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(14.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .w(px(96.0))
                                            .flex_shrink_0()
                                            .text_size(px(12.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(pal.text_muted())
                                            .child(format!(
                                                "{} · {}",
                                                format_short_date(Some(&ev.start_at)),
                                                format_clock(&ev.start_at)
                                            )),
                                    )
                                    .child(
                                        div()
                                            .w(px(4.0))
                                            .h(px(36.0))
                                            .flex_shrink_0()
                                            .rounded(px(4.0))
                                            .bg(pal.accent()),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .text_size(px(13.0))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .truncate()
                                                    .child(ev.title.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(pal.text_muted())
                                                    .child(if ev.kind == "block" {
                                                        "Protected task block"
                                                    } else {
                                                        "Calendar event"
                                                    }),
                                            ),
                                    ),
                            )
                            .child(
                                soft_action(
                                    SharedString::from(format!("del-event-{ev_id}")),
                                    None,
                                    "Remove",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "events.delete",
                                            json!({ "id": ev_id }),
                                            "Event removed",
                                        );
                                        cx.notify();
                                    },
                                )),
                            ),
                    );
                }
            }
            self.surface_card(pal)
                .flex_1()
                .min_w_0()
                .child(list)
                .into_any_element()
        } else if self.calendar_mode == CalendarMode::Month {
            let today_naive = Local::now().date_naive();
            let first_of_month =
                NaiveDate::from_ymd_opt(ref_date.year(), ref_date.month(), 1).unwrap_or(ref_date);
            let start_offset = first_of_month.weekday().num_days_from_monday() as i64;
            let grid_start = first_of_month - Duration::days(start_offset);

            let mut weekday_header = h_flex()
                .w_full()
                .py(px(10.0))
                .border_b_1()
                .border_color(pal.border());
            for wd in ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"] {
                weekday_header = weekday_header.child(
                    div()
                        .flex_1()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(pal.text_muted())
                        .flex()
                        .justify_center()
                        .child(wd),
                );
            }

            let mut weeks = v_flex().w_full();
            for week_idx in 0..5 {
                let mut week_row = h_flex().w_full().min_w_0();
                for day_idx in 0..7 {
                    let cell_date = grid_start + Duration::days(week_idx * 7 + day_idx);
                    let day_key = cell_date.format("%Y-%m-%d").to_string();
                    let is_current_month = cell_date.month() == ref_date.month();
                    let is_today = cell_date == today_naive;

                    let mut cell = v_flex()
                        .flex_1()
                        .min_w_0()
                        .h(px(96.0))
                        .p(px(8.0))
                        .gap(px(4.0))
                        .border_r_1()
                        .border_b_1()
                        .border_color(pal.border())
                        .bg(if is_current_month {
                            pal.surface()
                        } else {
                            pal.surface_raised()
                        })
                        .overflow_hidden()
                        .child(
                            div()
                                .w(px(24.0))
                                .h(px(24.0))
                                .rounded(px(999.0))
                                .bg(if is_today {
                                    pal.accent()
                                } else {
                                    pal.surface()
                                })
                                .text_color(if is_today {
                                    pal.text_on_accent()
                                } else if is_current_month {
                                    pal.text()
                                } else {
                                    pal.text_faint()
                                })
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(cell_date.format("%-d").to_string()),
                        );

                    for ev in self
                        .snapshot
                        .events
                        .iter()
                        .filter(|e| e.start_at.starts_with(&day_key))
                        .take(2)
                    {
                        cell = cell.child(
                            div()
                                .w_full()
                                .min_w_0()
                                .px(px(5.0))
                                .py(px(2.0))
                                .rounded(px(4.0))
                                .bg(pal.accent_muted())
                                .text_color(pal.accent_text())
                                .text_size(px(9.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .child(ev.title.clone()),
                        );
                    }
                    week_row = week_row.child(cell);
                }
                weeks = weeks.child(week_row);
            }

            v_flex()
                .flex_1()
                .min_w_0()
                .bg(pal.surface())
                .border_1()
                .border_color(pal.border())
                .rounded(px(13.0))
                .overflow_hidden()
                .child(weekday_header)
                .child(weeks)
                .into_any_element()
        } else {
            let today_naive = Local::now().date_naive();
            let mut header_row = h_flex()
                .w_full()
                .min_h(px(64.0))
                .border_b_1()
                .border_color(pal.border())
                .child(div().w(px(64.0)).flex_shrink_0());

            for day in &visible_days {
                let is_today = *day == today_naive;
                header_row = header_row.child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .border_l_1()
                        .border_color(pal.border())
                        .items_center()
                        .justify_center()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(pal.text_faint())
                                .child(day.format("%a").to_string().to_uppercase()),
                        )
                        .child(
                            div()
                                .text_size(px(18.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(if is_today {
                                    pal.accent_text()
                                } else {
                                    pal.text()
                                })
                                .child(day.format("%-d").to_string()),
                        ),
                );
            }

            let mut hours_col = v_flex().w_full().min_w_0();
            for hour in 8..=17 {
                let hour_prefix = format!("{hour:02}:");
                let mut row = h_flex()
                    .w_full()
                    .min_w_0()
                    .min_h(px(58.0))
                    .border_b_1()
                    .border_color(pal.border())
                    .child(
                        div()
                            .w(px(64.0))
                            .flex_shrink_0()
                            .p(px(8.0))
                            .text_size(px(10.0))
                            .text_color(pal.text_faint())
                            .child(format!("{hour:02}:00")),
                    );

                for day in &visible_days {
                    let day_key = day.format("%Y-%m-%d").to_string();
                    let mut cell = v_flex()
                        .flex_1()
                        .min_w_0()
                        .min_h(px(58.0))
                        .p(px(4.0))
                        .gap(px(4.0))
                        .border_l_1()
                        .border_color(pal.border());

                    for ev in self.snapshot.events.iter().filter(|e| {
                        e.start_at.starts_with(&day_key)
                            && e.start_at.get(11..14) == Some(hour_prefix.as_str())
                    }) {
                        cell = cell.child(
                            v_flex()
                                .w_full()
                                .min_w_0()
                                .p(px(6.0))
                                .rounded(px(6.0))
                                .bg(if ev.kind == "block" {
                                    pal.info()
                                } else {
                                    pal.accent_muted()
                                })
                                .text_color(pal.accent_text())
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .font_weight(FontWeight::BOLD)
                                        .truncate()
                                        .child(ev.title.clone()),
                                )
                                .child(div().text_size(px(9.0)).truncate().child(format!(
                                    "{} – {}",
                                    format_clock(&ev.start_at),
                                    format_clock(&ev.end_at)
                                ))),
                        );
                    }
                    row = row.child(cell);
                }
                hours_col = hours_col.child(row);
            }

            v_flex()
                .flex_1()
                .min_w_0()
                .bg(pal.surface())
                .border_1()
                .border_color(pal.border())
                .rounded(px(13.0))
                .overflow_hidden()
                .child(header_row)
                .child(hours_col)
                .into_any_element()
        };

        let proposal_panel = self
            .surface_card(pal)
            .w(px(330.0))
            .flex_shrink_0()
            .child(
                div()
                    .text_size(px(17.0))
                    .font_weight(FontWeight::BOLD)
                    .child("Schedule proposal"),
            )
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .whitespace_normal()
                    .text_size(px(12.0))
                    .text_color(pal.text_muted())
                    .child(
                        "Review deterministic candidate blocks before they land on your calendar.",
                    ),
            )
            .child(
                h_flex()
                    .gap(px(8.0))
                    .flex_wrap()
                    .child(
                        soft_action(
                            "propose-schedule-btn",
                            Some(IconName::CalendarClock),
                            "Propose today",
                            pal,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let day = Local::now().format("%Y-%m-%d").to_string();
                            if let Ok(val) = this
                                .backend
                                .call("schedule.propose", &json!({ "date": day }))
                            {
                                let slots: Vec<ProposedSlot> = val
                                    .get("slots")
                                    .or_else(|| val.get("candidates"))
                                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                                    .unwrap_or_default();
                                this.schedule_proposal =
                                    Some(ScheduleProposal { date: day, slots });
                                this.notice = Some("Schedule proposal generated".into());
                            }
                            cx.notify();
                        })),
                    )
                    .when_some(self.schedule_proposal.clone(), |el, prop| {
                        if prop.slots.is_empty() {
                            el
                        } else {
                            el.child(
                                primary_action(
                                    "apply-schedule-btn",
                                    Some(IconName::Check),
                                    "Apply blocks",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "schedule.apply",
                                            json!({ "slots": prop.slots }),
                                            "Schedule blocks applied",
                                        );
                                        this.schedule_proposal = None;
                                        cx.notify();
                                    },
                                )),
                            )
                        }
                    }),
            )
            .when_some(self.schedule_proposal.clone(), |el, prop| {
                let mut col = v_flex().w_full().min_w_0().gap(px(6.0));
                if prop.slots.is_empty() {
                    col = col.child(
                        div()
                            .w_full()
                            .min_w_0()
                            .whitespace_normal()
                            .text_size(px(11.0))
                            .text_color(pal.text_muted())
                            .child("No open candidate tasks to schedule for today."),
                    );
                } else {
                    for slot in prop.slots {
                        col = col.child(
                            div()
                                .w_full()
                                .min_w_0()
                                .p(px(8.0))
                                .rounded(px(6.0))
                                .bg(pal.accent_muted())
                                .text_color(pal.accent_text())
                                .text_size(px(11.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .child(format!(
                                    "{} – {} · {}",
                                    format_clock(&slot.start_at),
                                    format_clock(&slot.end_at),
                                    slot.title
                                )),
                        );
                    }
                }
                el.child(col)
            });

        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(calendar_toolbar)
            .child(
                self.surface_card(pal).child(
                    h_flex()
                        .gap(px(10.0))
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Input::new(&self.event_title_input)),
                        )
                        .child(
                            primary_action(
                                "create-event-btn",
                                Some(IconName::Plus),
                                "Protect 09:00–10:00 block",
                                pal,
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    let title =
                                        this.event_title_input.read(cx).value().trim().to_string();
                                    if title.is_empty() {
                                        return;
                                    }
                                    let day = this
                                        .calendar_reference_date()
                                        .format("%Y-%m-%d")
                                        .to_string();
                                    if this
                                        .exec(
                                            "events.create",
                                            json!({
                                                "title": title,
                                                "start_at": format!("{day}T09:00:00"),
                                                "end_at": format!("{day}T10:00:00"),
                                                "kind": "block",
                                            }),
                                            "Calendar block created",
                                        )
                                        .is_some()
                                    {
                                        this.event_title_input.update(cx, |s, cx| {
                                            s.set_value("", window, cx);
                                        });
                                    }
                                    cx.notify();
                                },
                            )),
                        ),
                ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(16.0))
                    .w_full()
                    .min_w_0()
                    .child(grid_or_agenda)
                    .child(proposal_panel),
            )
            .child(self.render_calendar_v16_panel(pal, cx))
            .into_any_element()
    }

    fn render_projects(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let phases = [
            ("active", "IN MOTION", pal.info()),
            ("paused", "ON HOLD", pal.warning_muted()),
            ("completed", "COMPLETED", pal.accent_muted()),
        ];
        let mut board = h_flex().items_start().gap(px(16.0)).w_full().min_w_0();
        for (phase, title, dot_bg) in phases {
            let phase_projects: Vec<_> = self
                .snapshot
                .projects
                .iter()
                .filter(|p| p.status == phase)
                .collect();
            let mut col = v_flex()
                .flex_1()
                .min_w_0()
                .min_h(px(460.0))
                .p(px(17.0))
                .gap(px(12.0))
                .rounded(px(14.0))
                .bg(pal.surface_sunken())
                .child(
                    h_flex()
                        .pb(px(6.0))
                        .justify_between()
                        .items_center()
                        .child(
                            h_flex()
                                .gap(px(8.0))
                                .items_center()
                                .child(div().w(px(10.0)).h(px(10.0)).rounded(px(999.0)).bg(dot_bg))
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(pal.text_secondary())
                                        .child(title),
                                ),
                        )
                        .child(
                            div()
                                .px(px(7.0))
                                .py(px(2.0))
                                .rounded(px(8.0))
                                .bg(pal.surface())
                                .text_size(px(11.0))
                                .font_weight(FontWeight::BOLD)
                                .text_color(pal.accent_text())
                                .child(format!("{}", phase_projects.len())),
                        ),
                );

            for project in phase_projects {
                let pid = project.id.clone();
                let pid_for_tasks = project.id.clone();
                let next_phase = match phase {
                    "active" => "paused",
                    "paused" => "completed",
                    _ => "active",
                };
                let open_count = self
                    .snapshot
                    .tasks
                    .iter()
                    .filter(|t| t.project_id.as_deref() == Some(&project.id) && t.status != "done")
                    .count();
                col = col.child(
                    v_flex()
                        .w_full()
                        .min_w_0()
                        .overflow_hidden()
                        .p(px(20.0))
                        .min_h(px(176.0))
                        .rounded(px(11.0))
                        .bg(pal.surface())
                        .border_1()
                        .border_color(pal.border())
                        .justify_between()
                        .gap(px(10.0))
                        .child(
                            v_flex()
                                .w_full()
                                .min_w_0()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .font_weight(FontWeight::EXTRA_BOLD)
                                        .text_color(pal.accent_text())
                                        .child("PERSONAL PROJECT"),
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(18.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(pal.text())
                                        .child(project.name.clone()),
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .min_w_0()
                                        .whitespace_normal()
                                        .text_size(px(11.0))
                                        .text_color(pal.text_muted())
                                        .child(if project.description.is_empty() {
                                            "A project worth making time for.".to_string()
                                        } else {
                                            project.description.clone()
                                        }),
                                ),
                        )
                        .child(
                            h_flex()
                                .w_full()
                                .pt(px(12.0))
                                .border_t_1()
                                .border_color(pal.border())
                                .justify_between()
                                .items_center()
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(pal.accent_text())
                                        .child(format!("{open_count} open")),
                                )
                                .child(
                                    h_flex()
                                        .gap(px(8.0))
                                        .child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "move-project-{pid}"
                                                )))
                                                .cursor_pointer()
                                                .px(px(8.0))
                                                .py(px(4.0))
                                                .rounded(px(6.0))
                                                .bg(pal.surface_raised())
                                                .text_size(px(10.0))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(pal.text_secondary())
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.exec(
                                                        "projects.update",
                                                        json!({ "id": pid, "status": next_phase }),
                                                        "Project status updated",
                                                    );
                                                    cx.notify();
                                                }))
                                                .child(format!("→ {next_phase}")),
                                        )
                                        .child(
                                            h_flex()
                                                .id(SharedString::from(format!(
                                                    "view-proj-tasks-{pid_for_tasks}"
                                                )))
                                                .cursor_pointer()
                                                .gap(px(4.0))
                                                .items_center()
                                                .text_size(px(11.0))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(pal.accent_text())
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.selected_project_id =
                                                        Some(pid_for_tasks.clone());
                                                    this.switch_view(WorkspaceView::Tasks, cx);
                                                }))
                                                .child("Tasks")
                                                .child(lucide(
                                                    IconName::ArrowRight,
                                                    13.0,
                                                    pal.accent_text(),
                                                )),
                                        ),
                                ),
                        ),
                );
            }
            board = board.child(col);
        }

        let mut milestones_list = v_flex().w_full().min_w_0().gap(px(8.0));
        if self.snapshot.milestones.is_empty() {
            milestones_list = milestones_list.child(
                div()
                    .py(px(8.0))
                    .text_size(px(12.0))
                    .text_color(pal.text_muted())
                    .child("No upcoming project milestones yet."),
            );
        } else {
            for ms in &self.snapshot.milestones {
                let mid = ms.id.clone();
                let done = ms.status == "done" || ms.status == "completed";
                let proj_name = self
                    .snapshot
                    .projects
                    .iter()
                    .find(|p| p.id == ms.project_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Project".into());
                milestones_list = milestones_list.child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .p(px(12.0))
                        .rounded(px(9.0))
                        .bg(pal.surface_raised())
                        .justify_between()
                        .items_center()
                        .gap(px(12.0))
                        .child(
                            h_flex()
                                .flex_1()
                                .min_w_0()
                                .gap(px(10.0))
                                .items_center()
                                .child(
                                    div()
                                        .id(SharedString::from(format!("complete-milestone-{mid}")))
                                        .cursor_pointer()
                                        .w(px(20.0))
                                        .h(px(20.0))
                                        .flex_shrink_0()
                                        .rounded(px(999.0))
                                        .border_1()
                                        .border_color(if done {
                                            pal.accent()
                                        } else {
                                            pal.border_strong()
                                        })
                                        .bg(if done { pal.accent() } else { pal.surface() })
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if !done {
                                                this.exec(
                                                    "milestones.complete",
                                                    json!({ "id": mid }),
                                                    "Milestone completed",
                                                );
                                                cx.notify();
                                            }
                                        }))
                                        .when(done, |el| {
                                            el.child(lucide(
                                                IconName::Check,
                                                12.0,
                                                pal.text_on_accent(),
                                            ))
                                        }),
                                )
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .text_size(px(13.0))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .truncate()
                                                .child(ms.title.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.0))
                                                .text_color(pal.text_muted())
                                                .truncate()
                                                .child(proj_name),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(pal.accent_text())
                                .child(format_short_date(ms.due_date.as_deref())),
                        ),
                );
            }
        }

        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(
                self.surface_card(pal).child(
                    h_flex()
                        .gap(px(10.0))
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Input::new(&self.project_name_input)),
                        )
                        .child(
                            primary_action(
                                "create-project-btn",
                                Some(IconName::Plus),
                                "New project",
                                pal,
                            )
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    let name =
                                        this.project_name_input.read(cx).value().trim().to_string();
                                    if name.is_empty() {
                                        return;
                                    }
                                    if this
                                        .exec(
                                            "projects.create",
                                            json!({ "name": name, "status": "active" }),
                                            "Project created",
                                        )
                                        .is_some()
                                    {
                                        this.project_name_input.update(cx, |s, cx| {
                                            s.set_value("", window, cx);
                                        });
                                    }
                                    cx.notify();
                                },
                            )),
                        ),
                ),
            )
            .child(board)
            .child({
                let default_pid = self
                    .selected_project_id
                    .clone()
                    .or_else(|| self.snapshot.projects.first().map(|p| p.id.clone()));
                self.surface_card(pal)
                    .child(
                        v_flex()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Upcoming milestones"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child("Clear checkpoints across your active projects."),
                            ),
                    )
                    .when_some(default_pid, |el, pid| {
                        el.child(
                            h_flex()
                                .gap(px(10.0))
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(Input::new(&self.milestone_title_input)),
                                )
                                .child(
                                    soft_action(
                                        "create-milestone-btn",
                                        Some(IconName::Plus),
                                        "Add milestone",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, window, cx| {
                                            let title = this
                                                .milestone_title_input
                                                .read(cx)
                                                .value()
                                                .trim()
                                                .to_string();
                                            if title.is_empty() {
                                                return;
                                            }
                                            let due = (Local::now().date_naive()
                                                + Duration::days(7))
                                            .format("%Y-%m-%d")
                                            .to_string();
                                            if this
                                                .exec(
                                                    "milestones.create",
                                                    json!({
                                                        "project_id": pid,
                                                        "title": title,
                                                        "due_date": due
                                                    }),
                                                    "Milestone created",
                                                )
                                                .is_some()
                                            {
                                                this.milestone_title_input.update(cx, |s, cx| {
                                                    s.set_value("", window, cx);
                                                });
                                            }
                                            cx.notify();
                                        },
                                    )),
                                ),
                        )
                    })
                    .child(milestones_list)
            })
            .child(self.render_projects_v16_panel(pal, cx))
            .into_any_element()
    }

    fn render_goals(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let mut list = v_flex().w_full().min_w_0().gap(px(14.0));
        for goal in &self.snapshot.goals {
            let gid = goal.id.clone();
            let gid_del = goal.id.clone();
            let next_progress = (goal.progress + 10.0).min(goal.target.max(100.0));
            let pct = if goal.target > 0.0 {
                ((goal.progress / goal.target) * 100.0).round() as i64
            } else {
                0
            };
            let ratio = (pct as f32 / 100.0).clamp(0.04, 1.0);

            let mut kr_rows = v_flex().w_full().min_w_0().gap(px(6.0));
            for kr in self
                .snapshot
                .key_results
                .iter()
                .filter(|k| k.goal_id == goal.id)
            {
                let krid = kr.id.clone();
                let next_val = (kr.current + 10.0).min(kr.target);
                kr_rows = kr_rows.child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .px(px(12.0))
                        .py(px(8.0))
                        .rounded(px(8.0))
                        .bg(pal.surface_raised())
                        .justify_between()
                        .items_center()
                        .gap(px(10.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!(
                                    "KR: {} ({:.0}/{:.0} {})",
                                    kr.title, kr.current, kr.target, kr.unit
                                )),
                        )
                        .child(
                            soft_action(
                                SharedString::from(format!("kr-step-{krid}")),
                                Some(IconName::Plus),
                                "+10",
                                pal,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.exec(
                                        "key_results.update",
                                        json!({ "id": krid, "current": next_val }),
                                        "Key result updated",
                                    );
                                    cx.notify();
                                },
                            )),
                        ),
                );
            }

            list = list.child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .p(px(24.0))
                    .gap(px(14.0))
                    .rounded(px(13.0))
                    .bg(pal.surface())
                    .border_1()
                    .border_color(pal.border())
                    .child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .justify_between()
                            .items_center()
                            .gap(px(20.0))
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(18.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .w(px(56.0))
                                            .h(px(56.0))
                                            .flex_shrink_0()
                                            .rounded(px(13.0))
                                            .bg(pal.accent_muted())
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(lucide(
                                                IconName::Target,
                                                22.0,
                                                pal.accent_text(),
                                            )),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(4.0))
                                            .child(
                                                div()
                                                    .text_size(px(10.0))
                                                    .font_weight(FontWeight::EXTRA_BOLD)
                                                    .text_color(pal.accent_text())
                                                    .child("ACTIVE DIRECTION"),
                                            )
                                            .child(
                                                div()
                                                    .w_full()
                                                    .min_w_0()
                                                    .whitespace_normal()
                                                    .text_size(px(21.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(pal.text())
                                                    .child(goal.title.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(pal.text_muted())
                                                    .child("Progress toward a measurable outcome."),
                                            ),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .w(px(240.0))
                                    .flex_shrink_0()
                                    .items_end()
                                    .gap(px(10.0))
                                    .child(
                                        div()
                                            .text_size(px(34.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(pal.text())
                                            .child(format!("{pct}%")),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(7.0))
                                            .rounded(px(8.0))
                                            .bg(pal.surface_raised())
                                            .overflow_hidden()
                                            .child(
                                                div()
                                                    .h_full()
                                                    .w(relative(ratio))
                                                    .rounded(px(8.0))
                                                    .bg(pal.accent()),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .gap(px(8.0))
                                            .child(
                                                soft_action(
                                                    SharedString::from(format!("goal-step-{gid}")),
                                                    Some(IconName::ArrowUpRight),
                                                    "+10%",
                                                    pal,
                                                )
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.exec(
                                                        "goals.update",
                                                        json!({
                                                            "id": gid,
                                                            "progress": next_progress
                                                        }),
                                                        "Goal progress updated",
                                                    );
                                                    cx.notify();
                                                })),
                                            )
                                            .child(
                                                soft_action(
                                                    SharedString::from(format!(
                                                        "goal-del-{gid_del}"
                                                    )),
                                                    None,
                                                    "Archive",
                                                    pal,
                                                )
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.exec(
                                                        "goals.delete",
                                                        json!({ "id": gid_del }),
                                                        "Goal archived",
                                                    );
                                                    cx.notify();
                                                })),
                                            ),
                                    ),
                            ),
                    )
                    .child(kr_rows),
            );
        }

        let first_gid = self.snapshot.goals.first().map(|g| g.id.clone());

        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(
                self.surface_card(pal)
                    .child(
                        h_flex()
                            .gap(px(10.0))
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(&self.goal_title_input)),
                            )
                            .child(
                                primary_action(
                                    "create-goal-btn",
                                    Some(IconName::Plus),
                                    "New goal",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        let title = this
                                            .goal_title_input
                                            .read(cx)
                                            .value()
                                            .trim()
                                            .to_string();
                                        if title.is_empty() {
                                            return;
                                        }
                                        if this
                                            .exec(
                                                "goals.create",
                                                json!({
                                                    "title": title,
                                                    "target": 100,
                                                    "progress": 0
                                                }),
                                                "Goal created",
                                            )
                                            .is_some()
                                        {
                                            this.goal_title_input.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .when_some(first_gid, |el, gid| {
                        el.child(
                            h_flex()
                                .gap(px(10.0))
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(Input::new(&self.key_result_title_input)),
                                )
                                .child(
                                    soft_action(
                                        "create-kr-btn",
                                        Some(IconName::Target),
                                        "Add key result",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, window, cx| {
                                            let title = this
                                                .key_result_title_input
                                                .read(cx)
                                                .value()
                                                .trim()
                                                .to_string();
                                            if title.is_empty() {
                                                return;
                                            }
                                            if this
                                                .exec(
                                                    "key_results.create",
                                                    json!({
                                                        "goal_id": gid,
                                                        "title": title,
                                                        "baseline": 0.0,
                                                        "target": 100.0,
                                                        "unit": "%"
                                                    }),
                                                    "Key result created",
                                                )
                                                .is_some()
                                            {
                                                this.key_result_title_input.update(cx, |s, cx| {
                                                    s.set_value("", window, cx);
                                                });
                                            }
                                            cx.notify();
                                        },
                                    )),
                                ),
                        )
                    }),
            )
            .child(list)
            .child(self.render_goals_v16_panel(pal, cx))
            .into_any_element()
    }

    fn render_focus(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let active = self
            .snapshot
            .focus
            .iter()
            .find(|f| f.status == "active" || f.status == "paused")
            .cloned();

        let selected_task_label = self
            .selected_focus_task_id
            .as_ref()
            .and_then(|id| self.snapshot.tasks.iter().find(|t| &t.id == id))
            .map(|t| format!("Task: {}", t.title))
            .unwrap_or_else(|| "Focus without a task".to_string());

        // 3-column Focus Hero Stage with explicit `.min_w_0()` and `.whitespace_normal()`
        // on both side columns so multi-line headings and quotes wrap cleanly inside the card!
        let stage = h_flex()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .min_h(px(440.0))
            .p(px(36.0))
            .gap(px(28.0))
            .rounded(px(17.0))
            .bg(pal.accent_muted())
            .items_center()
            .justify_between()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(14.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .text_color(pal.accent_text())
                            .child("FOCUS RITUAL"),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .whitespace_normal()
                            .text_size(px(26.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(pal.text())
                            .child(if active.is_some() {
                                "Focus is in motion."
                            } else {
                                "Make space to do meaningful work."
                            }),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .whitespace_normal()
                            .text_size(px(13.0))
                            .text_color(pal.text_secondary())
                            .child("A little less switching. A lot more intention."),
                    ),
            )
            .child(
                v_flex()
                    .w(px(290.0))
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(16.0))
                    .child(
                        div()
                            .w(px(256.0))
                            .h(px(256.0))
                            .p(px(15.0))
                            .rounded(px(999.0))
                            .bg(pal.accent_quiet())
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                v_flex()
                                    .size_full()
                                    .rounded(px(999.0))
                                    .bg(pal.surface())
                                    .items_center()
                                    .justify_center()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(36.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(pal.text())
                                            .child(if let Some(ref f) = active {
                                                if f.status == "paused" {
                                                    "Paused".to_string()
                                                } else {
                                                    self.active_focus_elapsed_text(
                                                        &f.started_at,
                                                        f.duration_seconds,
                                                    )
                                                }
                                            } else {
                                                "25m 00s".to_string()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.0))
                                            .font_weight(FontWeight::EXTRA_BOLD)
                                            .text_color(pal.accent_text())
                                            .child(if active.is_some() {
                                                "SESSION IN PROGRESS"
                                            } else {
                                                "READY WHEN YOU ARE"
                                            }),
                                    ),
                            ),
                    )
                    .child(if let Some(f) = active {
                        let fid_pause = f.id.clone();
                        let fid_stop = f.id.clone();
                        let is_paused = f.status == "paused";
                        h_flex()
                            .gap(px(10.0))
                            .child(
                                soft_action(
                                    "focus-pause-resume-btn",
                                    Some(if is_paused {
                                        IconName::Play
                                    } else {
                                        IconName::Pause
                                    }),
                                    if is_paused { "Resume" } else { "Pause" },
                                    pal,
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    let act = if is_paused {
                                        "focus.resume"
                                    } else {
                                        "focus.pause"
                                    };
                                    this.exec(
                                        act,
                                        json!({ "id": fid_pause }),
                                        "Focus state updated",
                                    );
                                    cx.notify();
                                })),
                            )
                            .child(
                                primary_action(
                                    "focus-stop-btn",
                                    Some(IconName::Square),
                                    "End & record",
                                    pal,
                                )
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.exec(
                                        "focus.stop",
                                        json!({ "id": fid_stop }),
                                        "Focus session recorded",
                                    );
                                    cx.notify();
                                })),
                            )
                            .into_any_element()
                    } else {
                        v_flex()
                            .w_full()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .id("cycle-focus-task")
                                    .cursor_pointer()
                                    .max_w(px(260.0))
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .rounded(px(8.0))
                                    .bg(pal.surface())
                                    .border_1()
                                    .border_color(pal.border())
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(pal.text_secondary())
                                    .truncate()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let open_ids: Vec<String> = this
                                            .snapshot
                                            .tasks
                                            .iter()
                                            .filter(|t| {
                                                t.status != "done" && t.status != "cancelled"
                                            })
                                            .map(|t| t.id.clone())
                                            .collect();
                                        if open_ids.is_empty() {
                                            this.selected_focus_task_id = None;
                                        } else if let Some(curr) = &this.selected_focus_task_id {
                                            let pos = open_ids.iter().position(|x| x == curr);
                                            this.selected_focus_task_id = match pos {
                                                Some(i) if i + 1 < open_ids.len() => {
                                                    Some(open_ids[i + 1].clone())
                                                }
                                                _ => None,
                                            };
                                        } else {
                                            this.selected_focus_task_id = Some(open_ids[0].clone());
                                        }
                                        cx.notify();
                                    }))
                                    .child(selected_task_label),
                            )
                            .child(
                                primary_action(
                                    "focus-start-btn",
                                    Some(IconName::Play),
                                    "Start focus session",
                                    pal,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.exec(
                                        "focus.start",
                                        json!({ "task_id": this.selected_focus_task_id }),
                                        "Focus session started",
                                    );
                                    cx.notify();
                                })),
                            )
                            .into_any_element()
                    }),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .text_color(pal.accent_text())
                            .child("YOUR ATTENTION IS VALUABLE"),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .whitespace_normal()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(pal.text())
                            .child("One deliberate session is worth more than a calendar full of intentions."),
                    )
                    .child(
                        v_flex()
                            .gap(px(10.0))
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .items_center()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(pal.accent_text())
                                    .child(lucide(IconName::Clock3, 14.0, pal.accent_text()))
                                    .child(format!(
                                        "{} recorded",
                                        format_duration(self.snapshot.stats.focus_seconds)
                                    )),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .items_center()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(pal.accent_text())
                                    .child(lucide(IconName::CircleCheck, 14.0, pal.accent_text()))
                                    .child(format!(
                                        "{} tasks completed",
                                        self.snapshot.stats.completed_tasks
                                    )),
                            ),
                    ),
            );

        let mut history = v_flex().w_full().min_w_0();
        for session in self.snapshot.focus.iter().take(6) {
            history = history.child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .min_h(px(54.0))
                    .px(px(12.0))
                    .border_b_1()
                    .border_color(pal.border())
                    .justify_between()
                    .items_center()
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(12.0))
                            .items_center()
                            .child(lucide(IconName::Clock3, 16.0, pal.accent_text()))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .truncate()
                                    .child(
                                        session
                                            .task_title
                                            .clone()
                                            .unwrap_or_else(|| "Independent focus".into()),
                                    ),
                            ),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .gap(px(16.0))
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child(format_short_date(Some(&session.started_at))),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.accent_text())
                                    .child(format_duration(session.duration_seconds)),
                            ),
                    ),
            );
        }

        v_flex()
            .gap(px(22.0))
            .w_full()
            .min_w_0()
            .child(stage)
            .child(self.render_focus_v16_panel(pal, cx))
            .child(
                self.surface_card(pal)
                    .child(
                        v_flex()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Recent focus sessions"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child("Every focused interval deserves to count."),
                            ),
                    )
                    .child(history),
            )
            .into_any_element()
    }

    fn render_habits_or_routines(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        if self.view == WorkspaceView::Routines {
            let active_rid = self
                .selected_routine_id
                .clone()
                .or_else(|| self.snapshot.routines.first().map(|r| r.id.clone()));
            let active_routine_title = active_rid
                .as_ref()
                .and_then(|id| self.snapshot.routines.iter().find(|r| &r.id == id))
                .map(|r| r.title.clone())
                .unwrap_or_else(|| "Select a routine".into());

            let mut list = v_flex().w_full().min_w_0().gap(px(10.0));
            for r in &self.snapshot.routines {
                let rid = r.id.clone();
                let is_selected = active_rid.as_deref() == Some(rid.as_str());
                list = list.child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .p(px(16.0))
                        .rounded(px(11.0))
                        .bg(if is_selected {
                            pal.accent_muted()
                        } else {
                            pal.surface_raised()
                        })
                        .border_1()
                        .border_color(if is_selected {
                            pal.accent()
                        } else {
                            pal.border()
                        })
                        .justify_between()
                        .items_center()
                        .child(
                            h_flex()
                                .flex_1()
                                .min_w_0()
                                .gap(px(12.0))
                                .items_center()
                                .child(lucide(IconName::Repeat2, 18.0, pal.accent_text()))
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .text_size(px(14.0))
                                                .font_weight(FontWeight::BOLD)
                                                .truncate()
                                                .child(r.title.clone()),
                                        )
                                        .when(!r.description.is_empty(), |el| {
                                            el.child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(pal.text_muted())
                                                    .truncate()
                                                    .child(r.description.clone()),
                                            )
                                        }),
                                ),
                        )
                        .child(
                            soft_action(
                                SharedString::from(format!("select-routine-{rid}")),
                                None,
                                if is_selected {
                                    "Selected"
                                } else {
                                    "View steps"
                                },
                                pal,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.selected_routine_id = Some(rid.clone());
                                    this.reload();
                                    cx.notify();
                                },
                            )),
                        ),
                );
            }

            let mut steps_list = v_flex().w_full().min_w_0().gap(px(8.0));
            if self.routine_steps.is_empty() {
                steps_list = steps_list.child(
                    div()
                        .py(px(12.0))
                        .text_size(px(12.0))
                        .text_color(pal.text_muted())
                        .child("No steps recorded yet. Add the first step below."),
                );
            } else {
                for (idx, step) in self.routine_steps.iter().enumerate() {
                    steps_list = steps_list.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .p(px(12.0))
                            .rounded(px(9.0))
                            .bg(pal.surface_raised())
                            .justify_between()
                            .items_center()
                            .gap(px(12.0))
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(12.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .w(px(28.0))
                                            .h(px(28.0))
                                            .flex_shrink_0()
                                            .rounded(px(7.0))
                                            .bg(pal.accent_muted())
                                            .text_color(pal.accent_text())
                                            .text_size(px(11.0))
                                            .font_weight(FontWeight::EXTRA_BOLD)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(format!("{:02}", idx + 1)),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .whitespace_normal()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(step.title.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.accent_text())
                                    .child(format!("{}m", step.duration_minutes)),
                            ),
                    );
                }
            }

            let step_count = self.routine_steps.len();
            let rid_for_add = active_rid.clone();

            return v_flex()
                .gap(px(16.0))
                .w_full()
                .min_w_0()
                .child(
                    self.surface_card(pal).child(
                        h_flex()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(&self.routine_title_input)),
                            )
                            .child(
                                primary_action(
                                    "create-routine-btn",
                                    Some(IconName::Plus),
                                    "New routine",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        let title = this
                                            .routine_title_input
                                            .read(cx)
                                            .value()
                                            .trim()
                                            .to_string();
                                        if title.is_empty() {
                                            return;
                                        }
                                        if this
                                            .exec(
                                                "routines.create",
                                                json!({ "title": title }),
                                                "Routine created",
                                            )
                                            .is_some()
                                        {
                                            this.routine_title_input.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                    ),
                )
                .child(
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
                                        .child("Repeatable sequences"),
                                )
                                .child(list),
                        )
                        .child(
                            self.surface_card(pal)
                                .flex_1()
                                .min_w_0()
                                .child(
                                    v_flex()
                                        .gap(px(3.0))
                                        .child(
                                            div()
                                                .text_size(px(10.0))
                                                .font_weight(FontWeight::EXTRA_BOLD)
                                                .text_color(pal.accent_text())
                                                .child("ORDERED ROUTINE STEPS"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(17.0))
                                                .font_weight(FontWeight::BOLD)
                                                .child(active_routine_title),
                                        ),
                                )
                                .child(steps_list)
                                .when_some(rid_for_add, |el, rid| {
                                    el.child(
                                        h_flex()
                                            .gap(px(8.0))
                                            .items_center()
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .child(Input::new(&self.routine_step_input)),
                                            )
                                            .child(
                                                primary_action(
                                                    "add-routine-step-btn",
                                                    Some(IconName::Plus),
                                                    "Add step",
                                                    pal,
                                                )
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        let step_title = this
                                                            .routine_step_input
                                                            .read(cx)
                                                            .value()
                                                            .trim()
                                                            .to_string();
                                                        if step_title.is_empty() {
                                                            return;
                                                        }
                                                        if this
                                                            .exec(
                                                                "routines.step.add",
                                                                json!({
                                                                    "routine_id": rid,
                                                                    "title": step_title,
                                                                    "position": step_count + 1,
                                                                    "duration_minutes": 5
                                                                }),
                                                                "Routine step added",
                                                            )
                                                            .is_some()
                                                        {
                                                            this.routine_step_input.update(
                                                                cx,
                                                                |s, cx| {
                                                                    s.set_value("", window, cx);
                                                                },
                                                            );
                                                        }
                                                        cx.notify();
                                                    },
                                                )),
                                            ),
                                    )
                                }),
                        ),
                )
                .into_any_element();
        }

        let weekdays = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];
        let today_weekday_idx = Local::now().date_naive().weekday().num_days_from_monday() as usize;
        let today = Local::now().format("%Y-%m-%d").to_string();

        let mut header = h_flex()
            .w_full()
            .px(px(16.0))
            .py(px(10.0))
            .border_b_1()
            .border_color(pal.border())
            .justify_between()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(pal.text_muted())
                    .child("HABIT"),
            );
        for wd in weekdays {
            header = header.child(
                div()
                    .w(px(48.0))
                    .flex_shrink_0()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(pal.text_muted())
                    .flex()
                    .justify_center()
                    .child(wd),
            );
        }

        let mut rows = v_flex().w_full().min_w_0();
        for habit in &self.snapshot.habits {
            let hid = habit.id.clone();
            let logged = habit.today_value.unwrap_or(0) > 0;
            let mut row = h_flex()
                .w_full()
                .min_w_0()
                .min_h(px(76.0))
                .px(px(16.0))
                .border_b_1()
                .border_color(pal.border())
                .justify_between()
                .items_center()
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(3.0))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .child(habit.title.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(pal.text_muted())
                                .truncate()
                                .child(format!(
                                    "{} · {} {} · {} total",
                                    habit.frequency, habit.target, habit.unit, habit.completions
                                )),
                        ),
                );

            for idx in 0..7 {
                let is_today_col = idx == today_weekday_idx;
                let filled = is_today_col && logged;
                let hid_click = hid.clone();
                let day_copy = today.clone();
                row = row.child(
                    div()
                        .w(px(48.0))
                        .flex_shrink_0()
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .id(SharedString::from(format!("habit-cell-{hid}-{idx}")))
                                .cursor_pointer()
                                .w(px(34.0))
                                .h(px(34.0))
                                .rounded(px(999.0))
                                .border_1()
                                .border_color(if filled {
                                    pal.accent()
                                } else {
                                    pal.border_strong()
                                })
                                .bg(if filled { pal.accent() } else { pal.surface() })
                                .flex()
                                .items_center()
                                .justify_center()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.exec(
                                        "habits.log",
                                        json!({
                                            "habit_id": hid_click,
                                            "day": day_copy,
                                            "value": if logged { 0 } else { 1 }
                                        }),
                                        "Habit updated",
                                    );
                                    cx.notify();
                                }))
                                .child(lucide(
                                    if filled {
                                        IconName::Check
                                    } else {
                                        IconName::Circle
                                    },
                                    16.0,
                                    if filled {
                                        pal.text_on_accent()
                                    } else {
                                        pal.text_faint()
                                    },
                                )),
                        ),
                );
            }
            rows = rows.child(row);
        }

        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(
                self.surface_card(pal).child(
                    h_flex()
                        .gap(px(10.0))
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Input::new(&self.habit_title_input)),
                        )
                        .child(
                            primary_action(
                                "create-habit-btn",
                                Some(IconName::Plus),
                                "New habit",
                                pal,
                            )
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    let title =
                                        this.habit_title_input.read(cx).value().trim().to_string();
                                    if title.is_empty() {
                                        return;
                                    }
                                    if this
                                        .exec(
                                            "habits.create",
                                            json!({
                                                "title": title,
                                                "frequency": "daily",
                                                "target": 1,
                                                "unit": "times"
                                            }),
                                            "Habit created",
                                        )
                                        .is_some()
                                    {
                                        this.habit_title_input.update(cx, |s, cx| {
                                            s.set_value("", window, cx);
                                        });
                                    }
                                    cx.notify();
                                },
                            )),
                        ),
                ),
            )
            .child(
                self.surface_card(pal)
                    .child(
                        v_flex()
                            .gap(px(3.0))
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Your routines"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child("Consistency without an arbitrary scoreboard."),
                            ),
                    )
                    .child(header)
                    .child(rows),
            )
            .into_any_element()
    }

    fn render_planning(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let tomorrow = (Local::now().date_naive() + Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        let overdue_ids: Vec<String> = self
            .snapshot
            .overdue_decisions
            .iter()
            .map(|d| d.task_id.clone())
            .collect();

        let mut overdue_list = v_flex().w_full().min_w_0().gap(px(8.0));
        if self.snapshot.overdue_decisions.is_empty() {
            overdue_list = overdue_list.child(
                div()
                    .py(px(12.0))
                    .text_size(px(12.0))
                    .text_color(pal.text_muted())
                    .child("No overdue tasks require triage. Your horizon is clear."),
            );
        } else {
            for item in &self.snapshot.overdue_decisions {
                let tid_today = item.task_id.clone();
                let tid_snooze = item.task_id.clone();
                let tid_block = item.task_id.clone();
                let today_cp = today.clone();
                let tomorrow_cp = tomorrow.clone();
                overdue_list = overdue_list.child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .p(px(12.0))
                        .rounded(px(9.0))
                        .bg(pal.surface_raised())
                        .justify_between()
                        .items_center()
                        .gap(px(12.0))
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::BOLD)
                                        .truncate()
                                        .child(item.title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(pal.warning_text())
                                        .child(format!(
                                            "P{} · {}d overdue (due {})",
                                            item.priority,
                                            item.days_overdue,
                                            format_short_date(item.due_date.as_deref())
                                        )),
                                ),
                        )
                        .child(
                            h_flex()
                                .flex_shrink_0()
                                .gap(px(6.0))
                                .child(
                                    soft_action(
                                        SharedString::from(format!("ovd-today-{tid_today}")),
                                        None,
                                        "Plan today",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                                "overdue.decide",
                                                json!({
                                                    "id": tid_today,
                                                    "decision": "reschedule",
                                                    "date": today_cp
                                                }),
                                                "Rescheduled for today",
                                            );
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    soft_action(
                                        SharedString::from(format!("ovd-snooze-{tid_snooze}")),
                                        None,
                                        "Snooze +1d",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                                "overdue.decide",
                                                json!({
                                                    "id": tid_snooze,
                                                    "decision": "snooze",
                                                    "date": tomorrow_cp
                                                }),
                                                "Snoozed to tomorrow",
                                            );
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    soft_action(
                                        SharedString::from(format!("ovd-block-{tid_block}")),
                                        None,
                                        "Block",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                                "overdue.decide",
                                                json!({
                                                    "id": tid_block,
                                                    "decision": "block"
                                                }),
                                                "Task marked blocked",
                                            );
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                );
            }
        }

        let weekdays = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];
        let mut protected_list = v_flex().w_full().min_w_0().gap(px(8.0));
        if self.snapshot.protected_blocks.is_empty() {
            protected_list = protected_list.child(
                div()
                    .py(px(8.0))
                    .text_size(px(12.0))
                    .text_color(pal.text_muted())
                    .child(
                        "No recurring protected blocks yet. Protect your deep work window below.",
                    ),
            );
        } else {
            for blk in &self.snapshot.protected_blocks {
                let bid = blk.id.clone();
                let wd = weekdays
                    .get(blk.weekday.clamp(0, 6) as usize)
                    .copied()
                    .unwrap_or("MON");
                protected_list = protected_list.child(
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
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::BOLD)
                                        .truncate()
                                        .child(blk.title.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(pal.accent_text())
                                        .child(format!(
                                            "{wd} · {} – {} ({})",
                                            blk.start_time, blk.end_time, blk.kind
                                        )),
                                ),
                        )
                        .child(
                            soft_action(
                                SharedString::from(format!("del-prot-{bid}")),
                                None,
                                "Remove",
                                pal,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.exec(
                                        "protected.delete",
                                        json!({ "id": bid }),
                                        "Protected block removed",
                                    );
                                    cx.notify();
                                },
                            )),
                        ),
                );
            }
        }

        let mut budget_rows = v_flex().w_full().min_w_0().gap(px(8.0));
        for area in &self.snapshot.areas {
            let aid_5 = area.id.clone();
            let aid_10 = area.id.clone();
            let current_budget = self
                .snapshot
                .time_budgets
                .iter()
                .find(|b| {
                    b.scope_kind == "area"
                        && b.scope_id.as_deref() == Some(area.id.as_str())
                        && b.period == "week"
                })
                .map(|b| b.minutes)
                .unwrap_or(0);
            let planned_mins: i64 = self
                .snapshot
                .tasks
                .iter()
                .filter(|t| {
                    t.area_id.as_deref() == Some(area.id.as_str())
                        && t.status != "done"
                        && t.status != "cancelled"
                })
                .map(|t| t.estimate_minutes)
                .sum();
            budget_rows = budget_rows.child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .p(px(12.0))
                    .rounded(px(9.0))
                    .bg(pal.surface_raised())
                    .justify_between()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::BOLD)
                                    .truncate()
                                    .child(area.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child(format!(
                                        "Planned: {} · Weekly budget: {}",
                                        format_duration(planned_mins * 60),
                                        if current_budget > 0 {
                                            format_duration(current_budget * 60)
                                        } else {
                                            "Unbudgeted".into()
                                        }
                                    )),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(px(6.0))
                            .child(
                                soft_action(
                                    SharedString::from(format!("budget-5h-{aid_5}")),
                                    None,
                                    "5h/wk",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "time_budgets.set",
                                            json!({
                                                "scope_kind": "area",
                                                "scope_id": aid_5,
                                                "period": "week",
                                                "minutes": 300
                                            }),
                                            "Weekly area budget set to 5h",
                                        );
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                soft_action(
                                    SharedString::from(format!("budget-10h-{aid_10}")),
                                    None,
                                    "10h/wk",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "time_budgets.set",
                                            json!({
                                                "scope_kind": "area",
                                                "scope_id": aid_10,
                                                "period": "week",
                                                "minutes": 600
                                            }),
                                            "Weekly area budget set to 10h",
                                        );
                                        cx.notify();
                                    },
                                )),
                            ),
                    ),
            );
        }

        let today_carry = today.clone();
        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(
                self.surface_card(pal)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .gap(px(3.0))
                                    .child(
                                        div()
                                            .text_size(px(17.0))
                                            .font_weight(FontWeight::BOLD)
                                            .child("Overdue triage & unfinished carryover"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child("Make explicit decisions on past-due work before committing to new blocks."),
                                    ),
                            )
                            .when(!overdue_ids.is_empty(), |el| {
                                el.child(
                                    primary_action(
                                        "carryover-all-btn",
                                        Some(IconName::CalendarClock),
                                        "Carry over all to today",
                                        pal,
                                    )
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.exec(
                                            "planning.carryover.apply",
                                            json!({
                                                "ids": overdue_ids,
                                                "target_date": today_carry
                                            }),
                                            "All overdue tasks carried over to today",
                                        );
                                        cx.notify();
                                    })),
                                )
                            }),
                    )
                    .child(overdue_list),
            )
            .child(
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
                                    .child("Protected time windows"),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .child(Input::new(&self.protected_title_input)),
                                    )
                                    .child(
                                        primary_action(
                                            "add-protected-block-btn",
                                            Some(IconName::Plus),
                                            "Protect 09:00–11:00",
                                            pal,
                                        )
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            let title = this
                                                .protected_title_input
                                                .read(cx)
                                                .value()
                                                .trim()
                                                .to_string();
                                            if title.is_empty() {
                                                return;
                                            }
                                            let wd = Local::now()
                                                .date_naive()
                                                .weekday()
                                                .num_days_from_monday()
                                                as i64;
                                            if this
                                                .exec(
                                                    "protected.create",
                                                    json!({
                                                        "title": title,
                                                        "weekday": wd,
                                                        "start_time": "09:00",
                                                        "end_time": "11:00",
                                                        "kind": "protected"
                                                    }),
                                                    "Protected window added",
                                                )
                                                .is_some()
                                            {
                                                this.protected_title_input.update(
                                                    cx,
                                                    |s, cx| {
                                                        s.set_value("", window, cx);
                                                    },
                                                );
                                            }
                                            cx.notify();
                                        })),
                                    ),
                            )
                            .child(protected_list),
                    )
                    .child(
                        self.surface_card(pal)
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Weekly area time budgets"),
                            )
                            .child(budget_rows),
                    ),
            )
            .child(self.render_planning_v16_panel(pal, cx))
            .into_any_element()
    }

    fn render_notes_or_reviews(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        if self.view == WorkspaceView::Insights {
            return self.render_insights(pal, cx);
        }
        if self.view == WorkspaceView::Reviews {
            let metric_row = h_flex()
                .gap(px(15.0))
                .w_full()
                .min_w_0()
                .child(self.metric_card(
                    pal,
                    "TASKS COMPLETED",
                    format!("{}", self.snapshot.stats.completed_tasks),
                    None,
                    "Actions brought to completion",
                ))
                .child(self.metric_card(
                    pal,
                    "FOCUS RECORDED",
                    format_duration(self.snapshot.stats.focus_seconds),
                    None,
                    "Time invested with intention",
                ))
                .child(self.metric_card(
                    pal,
                    "GOALS IN MOTION",
                    format!(
                        "{}",
                        self.snapshot
                            .goals
                            .iter()
                            .filter(|g| g.status == "active")
                            .count()
                    ),
                    None,
                    "Active directions to pursue",
                ));

            let mut period_pills = h_flex().gap(px(6.0));
            for p in ["daily", "weekly", "monthly"] {
                let active_p = self.review_period == p;
                let p_str = p.to_string();
                period_pills = period_pills.child(
                    div()
                        .id(SharedString::from(format!("rev-period-{p}")))
                        .cursor_pointer()
                        .px(px(10.0))
                        .py(px(5.0))
                        .rounded(px(7.0))
                        .bg(if active_p {
                            pal.accent()
                        } else {
                            pal.surface_raised()
                        })
                        .text_color(if active_p {
                            pal.text_on_accent()
                        } else {
                            pal.text_secondary()
                        })
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.review_period = p_str.clone();
                            cx.notify();
                        }))
                        .child(p.to_uppercase()),
                );
            }

            let mut list = v_flex().w_full().min_w_0().gap(px(10.0));
            for rev in &self.snapshot.reviews {
                list = list.child(
                    v_flex()
                        .w_full()
                        .min_w_0()
                        .p(px(16.0))
                        .rounded(px(11.0))
                        .bg(pal.surface_raised())
                        .gap(px(6.0))
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .whitespace_normal()
                                .text_size(px(14.0))
                                .font_weight(FontWeight::BOLD)
                                .child(format!("{} ({} · {})", rev.title, rev.period, rev.day)),
                        )
                        .child(
                            div()
                                .w_full()
                                .min_w_0()
                                .whitespace_normal()
                                .text_size(px(12.0))
                                .text_color(pal.text_muted())
                                .child(rev.content.clone()),
                        ),
                );
            }
            return v_flex()
                .gap(px(16.0))
                .w_full()
                .min_w_0()
                .child(metric_row)
                .child(self.render_guided_review_panel(pal, cx))
                .child(
                    self.surface_card(pal)
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    div()
                                        .text_size(px(15.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child("Guided reflection"),
                                )
                                .child(period_pills),
                        )
                        .child(Input::new(&self.review_title_input))
                        .child(Input::new(&self.review_body_input))
                        .child(
                            primary_action(
                                "create-review-btn",
                                Some(IconName::NotebookPen),
                                "Record reflection",
                                pal,
                            )
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    let title =
                                        this.review_title_input.read(cx).value().trim().to_string();
                                    let content =
                                        this.review_body_input.read(cx).value().trim().to_string();
                                    if title.is_empty() {
                                        return;
                                    }
                                    let day = Local::now().format("%Y-%m-%d").to_string();
                                    let period = this.review_period.clone();
                                    if this
                                        .exec(
                                            "reviews.create",
                                            json!({
                                                "title": title,
                                                "period": period,
                                                "day": day,
                                                "content": content
                                            }),
                                            "Reflection saved",
                                        )
                                        .is_some()
                                    {
                                        this.review_title_input.update(cx, |s, cx| {
                                            s.set_value("", window, cx);
                                        });
                                        this.review_body_input.update(cx, |s, cx| {
                                            s.set_value("", window, cx);
                                        });
                                    }
                                    cx.notify();
                                },
                            )),
                        ),
                )
                .child(self.surface_card(pal).child(list))
                .into_any_element();
        }

        let mut notes = h_flex().w_full().min_w_0().gap(px(15.0)).flex_wrap();
        for note in &self.snapshot.notes {
            let nid = note.id.clone();
            let nid_edit = note.id.clone();
            let edit_title = note.title.clone();
            let edit_body = note.content.clone();
            let is_editing = self.selected_note_id.as_deref() == Some(note.id.as_str());
            notes = notes.child(
                v_flex()
                    .w(px(330.0))
                    .min_w_0()
                    .overflow_hidden()
                    .min_h(px(220.0))
                    .p(px(22.0))
                    .rounded(px(13.0))
                    .bg(pal.surface())
                    .border_1()
                    .border_color(if is_editing {
                        pal.accent()
                    } else {
                        pal.border()
                    })
                    .justify_between()
                    .child(
                        v_flex()
                            .w_full()
                            .min_w_0()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .w(px(38.0))
                                    .h(px(38.0))
                                    .rounded(px(9.0))
                                    .bg(pal.accent_muted())
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(lucide(IconName::NotebookPen, 19.0, pal.accent_text())),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .text_color(pal.accent_text())
                                    .child("PERSONAL NOTE"),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(19.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(note.title.clone()),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .text_size(px(12.0))
                                    .text_color(pal.text_muted())
                                    .child(note.content.clone()),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .pt(px(12.0))
                            .border_t_1()
                            .border_color(pal.border())
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(pal.accent_text())
                                    .child(format_short_date(Some(&note.updated_at))),
                            )
                            .child(
                                h_flex()
                                    .gap(px(12.0))
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("edit-note-{nid_edit}")))
                                            .cursor_pointer()
                                            .text_size(px(11.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(pal.accent_text())
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.selected_note_id = Some(nid_edit.clone());
                                                let t = edit_title.clone();
                                                let b = edit_body.clone();
                                                this.note_title_input.update(cx, |s, cx| {
                                                    s.set_value(t, window, cx);
                                                });
                                                this.note_body_input.update(cx, |s, cx| {
                                                    s.set_value(b, window, cx);
                                                });
                                                cx.notify();
                                            }))
                                            .child("Edit"),
                                    )
                                    .child(
                                        div()
                                            .id(SharedString::from(format!("del-note-{nid}")))
                                            .cursor_pointer()
                                            .text_size(px(11.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(pal.danger())
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                if this.selected_note_id.as_deref()
                                                    == Some(nid.as_str())
                                                {
                                                    this.selected_note_id = None;
                                                }
                                                this.exec(
                                                    "notes.delete",
                                                    json!({ "id": nid }),
                                                    "Note deleted",
                                                );
                                                cx.notify();
                                            }))
                                            .child("Delete"),
                                    ),
                            ),
                    ),
            );
        }

        let is_editing_note = self.selected_note_id.is_some();

        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(
                self.surface_card(pal)
                    .child(Input::new(&self.note_title_input))
                    .child(Input::new(&self.note_body_input))
                    .child(
                        h_flex()
                            .gap(px(10.0))
                            .child(
                                primary_action(
                                    "create-note-btn",
                                    Some(IconName::Plus),
                                    if is_editing_note {
                                        "Update note"
                                    } else {
                                        "Save note"
                                    },
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        let title = this
                                            .note_title_input
                                            .read(cx)
                                            .value()
                                            .trim()
                                            .to_string();
                                        let content = this
                                            .note_body_input
                                            .read(cx)
                                            .value()
                                            .trim()
                                            .to_string();
                                        if title.is_empty() {
                                            return;
                                        }
                                        let res = if let Some(nid) = this.selected_note_id.take() {
                                            this.exec(
                                                "notes.update",
                                                json!({
                                                    "id": nid,
                                                    "title": title,
                                                    "content": content
                                                }),
                                                "Note updated",
                                            )
                                        } else {
                                            this.exec(
                                                "notes.create",
                                                json!({ "title": title, "content": content }),
                                                "Note saved",
                                            )
                                        };
                                        if res.is_some() {
                                            this.note_title_input.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                            this.note_body_input.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .when(is_editing_note, |el| {
                                el.child(
                                    soft_action("cancel-note-edit-btn", None, "Cancel edit", pal)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.selected_note_id = None;
                                            this.note_title_input.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                            this.note_body_input.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                            cx.notify();
                                        })),
                                )
                            }),
                    ),
            )
            .child(notes)
            .child(self.render_notes_v16_panel(pal, cx))
            .into_any_element()
    }

    fn render_insights(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let total_tasks = self.snapshot.tasks.len().max(1);
        let done_tasks = self
            .snapshot
            .tasks
            .iter()
            .filter(|t| t.status == "done")
            .count();
        let completion_pct = ((done_tasks as f32 / total_tasks as f32) * 100.0).round() as i64;
        let planned_mins: i64 = self
            .snapshot
            .tasks
            .iter()
            .filter(|t| t.status != "cancelled")
            .map(|t| t.estimate_minutes)
            .sum();

        let metric_row = h_flex()
            .gap(px(15.0))
            .w_full()
            .min_w_0()
            .child(self.metric_card(
                pal,
                "COMPLETION RATIO",
                format!("{completion_pct}%"),
                Some(format!("({done_tasks}/{})", self.snapshot.tasks.len())),
                "Recorded completions vs total tasks",
            ))
            .child(self.metric_card(
                pal,
                "FOCUSED EFFORT",
                format_duration(self.snapshot.stats.focus_seconds),
                None,
                "Verified focus intervals in SQLite",
            ))
            .child(self.metric_card(
                pal,
                "PLANNED CAPACITY",
                format_duration(planned_mins * 60),
                None,
                "Estimated effort across commitments",
            ))
            .child(self.metric_card(
                pal,
                "HABIT CHECK-INS",
                format!(
                    "{}",
                    self.snapshot
                        .habits
                        .iter()
                        .map(|h| h.completions)
                        .sum::<i64>()
                ),
                None,
                "Cumulative rhythm completions",
            ));

        let mut area_bars = v_flex().w_full().min_w_0().gap(px(12.0));
        for area in &self.snapshot.areas {
            let area_total = self
                .snapshot
                .tasks
                .iter()
                .filter(|t| t.area_id.as_deref() == Some(area.id.as_str()))
                .count();
            let area_done = self
                .snapshot
                .tasks
                .iter()
                .filter(|t| t.area_id.as_deref() == Some(area.id.as_str()) && t.status == "done")
                .count();
            let area_mins: i64 = self
                .snapshot
                .tasks
                .iter()
                .filter(|t| t.area_id.as_deref() == Some(area.id.as_str()))
                .map(|t| t.estimate_minutes)
                .sum();
            let ratio = if total_tasks > 0 {
                (area_total as f32 / total_tasks as f32).clamp(0.06, 1.0)
            } else {
                0.06
            };
            area_bars = area_bars.child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(5.0))
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(area.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child(format!(
                                        "{area_done}/{area_total} done · {} estimated",
                                        format_duration(area_mins * 60)
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(8.0))
                            .rounded(px(8.0))
                            .bg(pal.surface_raised())
                            .overflow_hidden()
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(ratio))
                                    .rounded(px(8.0))
                                    .bg(pal.accent()),
                            ),
                    ),
            );
        }

        let mut goal_bars = v_flex().w_full().min_w_0().gap(px(12.0));
        for goal in &self.snapshot.goals {
            let pct = if goal.target > 0.0 {
                ((goal.progress / goal.target) * 100.0).round() as i64
            } else {
                0
            };
            let ratio = (pct as f32 / 100.0).clamp(0.05, 1.0);
            goal_bars = goal_bars.child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(5.0))
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(goal.title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.accent_text())
                                    .child(format!("{pct}%")),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(8.0))
                            .rounded(px(8.0))
                            .bg(pal.surface_raised())
                            .overflow_hidden()
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(ratio))
                                    .rounded(px(8.0))
                                    .bg(pal.accent()),
                            ),
                    ),
            );
        }

        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(metric_row)
            .child(
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
                                v_flex()
                                    .gap(px(3.0))
                                    .child(
                                        div()
                                            .text_size(px(17.0))
                                            .font_weight(FontWeight::BOLD)
                                            .child("Commitment distribution by life area"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child("Direct count of tasks and estimated effort per domain."),
                                    ),
                            )
                            .child(area_bars),
                    )
                    .child(
                        self.surface_card(pal)
                            .flex_1()
                            .min_w_0()
                            .child(
                                v_flex()
                                    .gap(px(3.0))
                                    .child(
                                        div()
                                            .text_size(px(17.0))
                                            .font_weight(FontWeight::BOLD)
                                            .child("Goal & outcome achievement"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child("Progress derived from measurable targets and key results."),
                                    ),
                            )
                            .child(goal_bars),
                    ),
            )
            .child(self.render_insights_v16_panel(pal, cx))
            .into_any_element()
    }

    fn render_agents_or_automations(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        if self.view == WorkspaceView::Automations {
            let mut rules = v_flex().w_full().min_w_0();
            if self.snapshot.automations.is_empty() {
                rules = rules.child(
                    v_flex()
                        .min_h(px(180.0))
                        .items_center()
                        .justify_center()
                        .gap(px(10.0))
                        .child(lucide(IconName::Zap, 34.0, pal.accent_text()))
                        .child(
                            div()
                                .text_size(px(18.0))
                                .font_weight(FontWeight::BOLD)
                                .child("Less repetition. More room to focus."),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(pal.text_muted())
                                .child("Create a local automation rule above to trigger tasks or reflections."),
                        ),
                );
            } else {
                for rule in &self.snapshot.automations {
                    let rid = rule.id.clone();
                    let rid_toggle = rule.id.clone();
                    let enabled = rule.enabled != 0;
                    rules = rules.child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .py(px(14.0))
                            .border_b_1()
                            .border_color(pal.border())
                            .justify_between()
                            .items_center()
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(14.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .w(px(38.0))
                                            .h(px(38.0))
                                            .rounded(px(10.0))
                                            .bg(pal.accent_muted())
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(lucide(IconName::Zap, 18.0, pal.accent_text())),
                                    )
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .text_size(px(14.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .truncate()
                                                    .child(rule.title.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(pal.text_muted())
                                                    .child(format!(
                                                        "{} trigger · {} · {}",
                                                        rule.trigger_kind,
                                                        rule.action,
                                                        if enabled {
                                                            "Enabled"
                                                        } else {
                                                            "Disabled"
                                                        }
                                                    )),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .child(
                                        soft_action(
                                            SharedString::from(format!("run-auto-{rid}")),
                                            Some(IconName::Play),
                                            "Run",
                                            pal,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.exec(
                                                    "automations.run",
                                                    json!({ "id": rid }),
                                                    "Automation executed",
                                                );
                                                cx.notify();
                                            }),
                                        ),
                                    )
                                    .child(
                                        soft_action(
                                            SharedString::from(format!("toggle-auto-{rid_toggle}")),
                                            None,
                                            if enabled { "Disable" } else { "Enable" },
                                            pal,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.exec(
                                                    "automations.update",
                                                    json!({
                                                        "id": rid_toggle,
                                                        "enabled": !enabled
                                                    }),
                                                    "Rule updated",
                                                );
                                                cx.notify();
                                            }),
                                        ),
                                    ),
                            ),
                    );
                }
            }
            return v_flex()
                .gap(px(16.0))
                .w_full()
                .min_w_0()
                .child(
                    self.surface_card(pal).child(
                        h_flex()
                            .gap(px(10.0))
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(&self.automation_title_input)),
                            )
                            .child(
                                primary_action(
                                    "create-automation-btn",
                                    Some(IconName::Plus),
                                    "New automation rule",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        let title = this
                                            .automation_title_input
                                            .read(cx)
                                            .value()
                                            .trim()
                                            .to_string();
                                        if title.is_empty() {
                                            return;
                                        }
                                        if this
                                            .exec(
                                                "automations.create",
                                                json!({
                                                    "title": title,
                                                    "trigger_kind": "manual",
                                                    "action": "tasks.create",
                                                    "arguments": {
                                                        "title": title,
                                                        "priority": 2,
                                                        "status": "todo"
                                                    }
                                                }),
                                                "Automation rule created",
                                            )
                                            .is_some()
                                        {
                                            this.automation_title_input.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                    ),
                )
                .child(self.surface_card(pal).w_full().child(rules))
                .child(self.render_automations_v16_panel(pal, cx))
                .into_any_element();
        }

        let mut proposals = v_flex().w_full().min_w_0().gap(px(8.0));
        let pending_proposals: Vec<_> = self
            .snapshot
            .agent_proposals
            .iter()
            .filter(|p| p.state == "pending")
            .collect();
        if pending_proposals.is_empty() {
            proposals = proposals.child(
                div()
                    .text_size(px(12.0))
                    .text_color(pal.text_muted())
                    .child("No pending agent proposals waiting for approval."),
            );
        } else {
            for prop in pending_proposals {
                let pid_app = prop.id.clone();
                let pid_rej = prop.id.clone();
                proposals = proposals.child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .p(px(12.0))
                        .rounded(px(9.0))
                        .bg(pal.surface_raised())
                        .justify_between()
                        .items_center()
                        .gap(px(10.0))
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::BOLD)
                                        .truncate()
                                        .child(format!("{} · {}", prop.actor, prop.action)),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(pal.text_muted())
                                        .truncate()
                                        .child(prop.payload_json.clone()),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap(px(6.0))
                                .child(
                                    primary_action(
                                        SharedString::from(format!("approve-prop-{pid_app}")),
                                        Some(IconName::Check),
                                        "Approve",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                            "agents.proposals.approve",
                                            json!({ "id": pid_app, "reason": "Approved in GPUI" }),
                                            "Agent proposal approved and applied",
                                        );
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(
                                    soft_action(
                                        SharedString::from(format!("reject-prop-{pid_rej}")),
                                        None,
                                        "Reject",
                                        pal,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            this.exec(
                                            "agents.proposals.reject",
                                            json!({ "id": pid_rej, "reason": "Rejected by user" }),
                                            "Agent proposal rejected",
                                        );
                                            cx.notify();
                                        },
                                    )),
                                ),
                        ),
                );
            }
        }

        let mut ledger = v_flex().w_full().min_w_0();
        for entry in self.snapshot.history.iter().take(10) {
            ledger = ledger.child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .min_h(px(64.0))
                    .gap(px(14.0))
                    .border_b_1()
                    .border_color(pal.border())
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(14.0))
                            .items_center()
                            .child(
                                div()
                                    .w(px(34.0))
                                    .h(px(34.0))
                                    .flex_shrink_0()
                                    .rounded(px(999.0))
                                    .bg(pal.accent_muted())
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(lucide(IconName::Bot, 17.0, pal.accent_text())),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .font_weight(FontWeight::BOLD)
                                            .truncate()
                                            .child(entry.action.replace('.', " · ")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child(format!(
                                                "{} · {}",
                                                entry.actor,
                                                format_short_date(Some(&entry.created_at))
                                            )),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(6.0))
                            .bg(pal.accent_muted())
                            .text_color(pal.accent_text())
                            .text_size(px(10.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Recorded"),
                    ),
            );
        }

        let mut changes = v_flex().w_full().min_w_0();
        for ch in self.snapshot.agent_changes.iter().take(8) {
            let cid = ch.id.clone();
            let can_undo = ch.undoable && ch.undone_at.is_none();
            changes = changes.child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .min_h(px(56.0))
                    .border_b_1()
                    .border_color(pal.border())
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .truncate()
                                    .child(ch.action.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .child(format!(
                                        "{} · {}",
                                        ch.actor,
                                        format_short_date(Some(&ch.created_at))
                                    )),
                            ),
                    )
                    .when(can_undo, |el| {
                        el.child(
                            soft_action(
                                SharedString::from(format!("undo-change-{cid}")),
                                Some(IconName::RotateCcw),
                                "Undo",
                                pal,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.exec(
                                        "agents.undo",
                                        json!({ "id": cid }),
                                        "Task change undone",
                                    );
                                    cx.notify();
                                },
                            )),
                        )
                    }),
            );
        }

        let terminal_card = self
            .surface_card(pal)
            .child(
                v_flex()
                    .gap(px(3.0))
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("CLI · shared source of truth"),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(pal.text_muted())
                            .child("Headless commands, with or without the app"),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .p(px(18.0))
                    .rounded(px(10.0))
                    .bg(rgb(0x152B2A))
                    .text_color(rgb(0xD5EADC))
                    .text_size(px(12.0))
                    .gap(px(4.0))
                    .child("$ life today --json")
                    .child(format!(
                        "{{ \"tasks\": {}, \"focus\": \"{}\" }}",
                        self.snapshot.tasks.len(),
                        format_duration(self.snapshot.stats.focus_seconds)
                    ))
                    .child("$ life mcp serve --stdio"),
            );

        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(
                self.surface_card(pal)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                v_flex()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .text_size(px(17.0))
                                            .font_weight(FontWeight::BOLD)
                                            .child("Reviewable agent proposals"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child("Agents can stage mutations for your explicit approval before they touch your workspace."),
                                    ),
                            )
                            .child(
                                soft_action(
                                    "simulate-agent-proposal-btn",
                                    Some(IconName::Bot),
                                    "Stage sample proposal",
                                    pal,
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.exec(
                                        "agents.proposals.create",
                                        json!({
                                            "actor": "mcp-assistant",
                                            "action": "tasks.create",
                                            "payload": {
                                                "title": "Prepare weekly synthesis from agent review",
                                                "priority": 2,
                                                "status": "todo"
                                            }
                                        }),
                                        "Sample agent proposal staged for review",
                                    );
                                    cx.notify();
                                })),
                            ),
                    )
                    .child(proposals),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(17.0))
                    .w_full()
                    .min_w_0()
                    .child(
                        self.surface_card(pal)
                            .flex_1()
                            .min_w_0()
                            .child(
                                v_flex()
                                    .gap(px(3.0))
                                    .child(
                                        div()
                                            .text_size(px(17.0))
                                            .font_weight(FontWeight::BOLD)
                                            .child("Agent activity"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(pal.text_muted())
                                            .child(
                                                "Every operation can be inspected. You stay in control.",
                                            ),
                                    ),
                            )
                            .child(ledger),
                    )
                    .child(
                        v_flex()
                            .w(px(360.0))
                            .flex_shrink_0()
                            .gap(px(15.0))
                            .child(terminal_card)
                            .child(
                                self.surface_card(pal)
                                    .child(
                                        div()
                                            .text_size(px(17.0))
                                            .font_weight(FontWeight::BOLD)
                                            .child("Task changes with guarded undo"),
                                    )
                                    .child(changes),
                            ),
                    ),
            )
            .child(self.render_agent_permissions_panel(pal, cx))
            .into_any_element()
    }

    fn render_settings(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let tab_bar = self.render_subtab_bar(
            "settings",
            &[
                "General & Encryption",
                "System, Diagnostics & Audit",
                "Shortcuts",
                "Remote MCP & Integrations",
                "CSV & ICS Imports",
            ],
            self.settings_tab,
            pal,
            |this, idx, cx| {
                this.settings_tab = idx;
                cx.notify();
            },
            cx,
        );
        if self.settings_tab != 0 {
            return v_flex()
                .gap(px(16.0))
                .w_full()
                .min_w_0()
                .child(self.render_settings_v16_tab(pal, cx))
                .into_any_element();
        }
        let enc = &self.snapshot.encryption;
        let is_encrypted = enc.protection == "passphrase";

        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(tab_bar)
            .child(
                self.surface_card(pal)
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Workspace preferences & storage"),
                    )
                    .child(
                        h_flex()
                            .gap(px(10.0))
                            .flex_wrap()
                            .child(
                                soft_action(
                                    "settings-toggle-theme",
                                    Some(match self.theme_mode {
                                        ThemeMode::Light => IconName::Moon,
                                        ThemeMode::Dark => IconName::Sun,
                                    }),
                                    match self.theme_mode {
                                        ThemeMode::Light => "Switch to Dark theme",
                                        ThemeMode::Dark => "Switch to Light theme",
                                    },
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.toggle_theme(cx);
                                    },
                                )),
                            )
                            .child(
                                soft_action(
                                    "settings-toggle-density",
                                    Some(IconName::Layers),
                                    if self.compact_density {
                                        "Density: Compact"
                                    } else {
                                        "Density: Comfortable"
                                    },
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.compact_density = !this.compact_density;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                primary_action(
                                    "settings-create-backup",
                                    Some(IconName::DatabaseBackup),
                                    "Create verified SQLite backup",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        if let Some(res) = this.exec(
                                            "backup.create",
                                            json!({}),
                                            "Verified SQLite backup created",
                                        ) {
                                            this.last_backup_path = res
                                                .get("path")
                                                .and_then(|v| v.as_str())
                                                .map(String::from);
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                soft_action(
                                    "settings-export-json",
                                    Some(IconName::Download),
                                    "Export portable JSON snapshot",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        if let Some(res) = this.exec(
                                            "export",
                                            json!({}),
                                            "Portable JSON snapshot exported",
                                        ) {
                                            let out_path = this
                                                .backend
                                                .db_path
                                                .parent()
                                                .unwrap_or_else(|| std::path::Path::new("/tmp"))
                                                .join("life-export.json");
                                            if let Ok(bytes) = serde_json::to_vec_pretty(&res)
                                                && std::fs::write(&out_path, bytes).is_ok()
                                            {
                                                this.last_export_path =
                                                    Some(out_path.display().to_string());
                                            }
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .when_some(self.last_backup_path.clone(), |el, path| {
                        el.child(
                            div()
                                .w_full()
                                .min_w_0()
                                .whitespace_normal()
                                .text_size(px(11.0))
                                .text_color(pal.accent_text())
                                .child(format!("Verified backup: {path}")),
                        )
                    })
                    .when_some(self.last_export_path.clone(), |el, path| {
                        el.child(
                            div()
                                .w_full()
                                .min_w_0()
                                .whitespace_normal()
                                .text_size(px(11.0))
                                .text_color(pal.accent_text())
                                .child(format!("Exported JSON: {path}")),
                        )
                    }),
            )
            .child(
                self.surface_card(pal)
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .text_color(pal.accent_text())
                            .child("DATABASE PROTECTION (SQLCIPHER + ARGON2ID)"),
                    )
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child(if is_encrypted {
                                "This database is encrypted at rest"
                            } else {
                                "This database is currently unencrypted"
                            }),
                    )
                    .child(Input::new(&self.passphrase_input))
                    .child(h_flex().gap(px(10.0)).child(if is_encrypted {
                        soft_action(
                            "settings-decrypt-db",
                            Some(IconName::ShieldCheck),
                            "Decrypt database",
                            pal,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            let pass = this.passphrase_input.read(cx).value().to_string();
                            if this
                                .exec(
                                    "encryption.disable",
                                    json!({ "confirm": true, "passphrase": pass }),
                                    "Database decrypted",
                                )
                                .is_some()
                            {
                                this.backend.set_passphrase(None);
                                this.passphrase_input.update(cx, |s, cx| {
                                    s.set_value("", window, cx);
                                });
                                this.reload();
                            }
                            cx.notify();
                        }))
                        .into_any_element()
                    } else {
                        primary_action(
                            "settings-encrypt-db",
                            Some(IconName::ShieldCheck),
                            "Encrypt database with passphrase",
                            pal,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            let pass = this.passphrase_input.read(cx).value().to_string();
                            if pass.len() < 8 {
                                this.error =
                                    Some("Use a passphrase of at least 8 characters".into());
                                cx.notify();
                                return;
                            }
                            if this
                                .exec(
                                    "encryption.enable",
                                    json!({ "mode": "passphrase", "passphrase": pass }),
                                    "Database encrypted with SQLCipher",
                                )
                                .is_some()
                            {
                                this.backend.set_passphrase(Some(pass));
                                this.passphrase_input.update(cx, |s, cx| {
                                    s.set_value("", window, cx);
                                });
                                this.reload();
                            }
                            cx.notify();
                        }))
                        .into_any_element()
                    })),
            )
            .child(
                self.surface_card(pal)
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Life areas"),
                    )
                    .child(
                        h_flex()
                            .gap(px(10.0))
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(&self.area_name_input)),
                            )
                            .child(
                                primary_action(
                                    "create-area-btn",
                                    Some(IconName::Plus),
                                    "Add area",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        let name = this
                                            .area_name_input
                                            .read(cx)
                                            .value()
                                            .trim()
                                            .to_string();
                                        if name.is_empty() {
                                            return;
                                        }
                                        if this
                                            .exec(
                                                "areas.create",
                                                json!({ "name": name }),
                                                "Area created",
                                            )
                                            .is_some()
                                        {
                                            this.area_name_input.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child({
                        let mut areas_list = v_flex().w_full().min_w_0().gap(px(8.0));
                        for area in &self.snapshot.areas {
                            let aid = area.id.clone();
                            areas_list = areas_list.child(
                                h_flex()
                                    .w_full()
                                    .min_w_0()
                                    .p(px(10.0))
                                    .rounded(px(8.0))
                                    .bg(pal.surface_raised())
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        h_flex()
                                            .gap(px(10.0))
                                            .items_center()
                                            .child(
                                                div()
                                                    .w(px(10.0))
                                                    .h(px(10.0))
                                                    .rounded(px(999.0))
                                                    .bg(pal.accent()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(13.0))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(area.name.clone()),
                                            ),
                                    )
                                    .child(
                                        soft_action(
                                            SharedString::from(format!("del-area-{aid}")),
                                            None,
                                            "Archive",
                                            pal,
                                        )
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.exec(
                                                    "areas.delete",
                                                    json!({ "id": aid }),
                                                    "Area archived",
                                                );
                                                cx.notify();
                                            }),
                                        ),
                                    ),
                            );
                        }
                        areas_list
                    }),
            )
            .child(
                self.surface_card(pal)
                    .child(
                        div()
                            .text_size(px(17.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Recover archived tasks"),
                    )
                    .child({
                        let mut archived_list = v_flex().w_full().min_w_0().gap(px(8.0));
                        if self.snapshot.archived_tasks.is_empty() {
                            archived_list = archived_list.child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(pal.text_muted())
                                    .child("No archived tasks waiting for recovery."),
                            );
                        } else {
                            for task in self.snapshot.archived_tasks.iter().take(12) {
                                let tid = task.id.clone();
                                archived_list = archived_list.child(
                                    h_flex()
                                        .w_full()
                                        .min_w_0()
                                        .p(px(10.0))
                                        .rounded(px(8.0))
                                        .bg(pal.surface_raised())
                                        .justify_between()
                                        .items_center()
                                        .gap(px(10.0))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .truncate()
                                                .text_size(px(13.0))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child(task.title.clone()),
                                        )
                                        .child(
                                            soft_action(
                                                SharedString::from(format!("restore-task-{tid}")),
                                                Some(IconName::RotateCcw),
                                                "Restore",
                                                pal,
                                            )
                                            .on_click(
                                                cx.listener(move |this, _, _, cx| {
                                                    this.exec(
                                                        "tasks.restore",
                                                        json!({ "id": tid }),
                                                        "Task restored",
                                                    );
                                                    cx.notify();
                                                }),
                                            ),
                                        ),
                                );
                            }
                        }
                        archived_list
                    }),
            )
            .into_any_element()
    }

    fn render_areas(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let tab_bar = self.render_subtab_bar(
            "areas",
            &[
                "Overview & Domains",
                "Career & Interviews",
                "Certifications",
                "Savings Goals",
                "Rest & Administration",
            ],
            self.areas_tab,
            pal,
            |this, idx, cx| {
                this.areas_tab = idx;
                cx.notify();
            },
            cx,
        );
        if self.areas_tab != 0 {
            return v_flex()
                .gap(px(16.0))
                .w_full()
                .min_w_0()
                .child(self.render_areas_v16_trackers(pal, cx))
                .into_any_element();
        }

        let mut grid = h_flex().w_full().min_w_0().gap(px(16.0)).flex_wrap();
        for area in &self.snapshot.areas {
            let aid = area.id.clone();
            let aid_del = area.id.clone();
            let proj_count = self
                .snapshot
                .projects
                .iter()
                .filter(|p| p.area_id.as_deref() == Some(area.id.as_str()))
                .count();
            let task_count = self
                .snapshot
                .tasks
                .iter()
                .filter(|t| {
                    t.area_id.as_deref() == Some(area.id.as_str())
                        && t.status != "done"
                        && t.status != "cancelled"
                })
                .count();
            grid = grid.child(
                self.surface_card(pal)
                    .w(px(340.0))
                    .min_h(px(172.0))
                    .justify_between()
                    .child(
                        v_flex()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .text_color(pal.accent_text())
                                    .child("DOMAIN OF RESPONSIBILITY"),
                            )
                            .child(
                                div()
                                    .text_size(px(20.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child(area.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(pal.text_muted())
                                    .child(format!(
                                        "{proj_count} active projects · {task_count} open tasks"
                                    )),
                            ),
                    )
                    .child(
                        h_flex()
                            .pt(px(10.0))
                            .border_t_1()
                            .border_color(pal.border())
                            .justify_between()
                            .items_center()
                            .child(
                                soft_action(
                                    SharedString::from(format!("area-open-projects-{aid}")),
                                    Some(IconName::FolderKanban),
                                    "View projects",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.switch_view(WorkspaceView::Projects, cx);
                                    },
                                )),
                            )
                            .child(
                                soft_action(
                                    SharedString::from(format!("area-card-archive-{aid_del}")),
                                    None,
                                    "Archive",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "areas.delete",
                                            json!({ "id": aid_del }),
                                            "Area archived",
                                        );
                                        cx.notify();
                                    },
                                )),
                            ),
                    ),
            );
        }

        v_flex()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            .child(tab_bar)
            .child(
                self.surface_card(pal).child(
                    h_flex()
                        .gap(px(10.0))
                        .items_center()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Input::new(&self.area_name_input)),
                        )
                        .child(
                            primary_action(
                                "areas-view-create-btn",
                                Some(IconName::Plus),
                                "New life area",
                                pal,
                            )
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    let name =
                                        this.area_name_input.read(cx).value().trim().to_string();
                                    if name.is_empty() {
                                        return;
                                    }
                                    if this
                                        .exec(
                                            "areas.create",
                                            json!({ "name": name }),
                                            "Area created",
                                        )
                                        .is_some()
                                    {
                                        this.area_name_input.update(cx, |s, cx| {
                                            s.set_value("", window, cx);
                                        });
                                    }
                                    cx.notify();
                                },
                            )),
                        ),
                ),
            )
            .child(grid)
            .child(
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
                                    .child("Career pipeline & learning tracks"),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .child(Input::new(&self.career_role_input)),
                                    )
                                    .child(
                                        soft_action(
                                            "add-career-btn",
                                            Some(IconName::Plus),
                                            "Add role",
                                            pal,
                                        )
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            let raw = this
                                                .career_role_input
                                                .read(cx)
                                                .value()
                                                .trim()
                                                .to_string();
                                            if raw.is_empty() {
                                                return;
                                            }
                                            let (employer, role) = raw
                                                .split_once("—")
                                                .or_else(|| raw.split_once("-"))
                                                .map(|(a, b)| (a.trim(), b.trim()))
                                                .unwrap_or(("Opportunity", raw.as_str()));
                                            if this
                                                .exec(
                                                    "career.create",
                                                    json!({
                                                        "employer": if employer.is_empty() { "Company" } else { employer },
                                                        "role": if role.is_empty() { raw.as_str() } else { role },
                                                        "stage": "applied"
                                                    }),
                                                    "Career opportunity recorded",
                                                )
                                                .is_some()
                                            {
                                                this.career_role_input.update(cx, |s, cx| {
                                                    s.set_value("", window, cx);
                                                });
                                            }
                                            cx.notify();
                                        })),
                                    ),
                            )
                            .child({
                                let mut c_list = v_flex().w_full().min_w_0().gap(px(6.0));
                                for item in self.snapshot.career.iter().take(4) {
                                    c_list = c_list.child(
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
                                                    .flex_1()
                                                    .min_w_0()
                                                    .truncate()
                                                    .text_size(px(12.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .child(format!(
                                                        "{} — {}",
                                                        item.employer, item.role
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(pal.accent_text())
                                                    .child(item.stage.to_uppercase()),
                                            ),
                                    );
                                }
                                c_list
                            })
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .child(Input::new(&self.learning_title_input)),
                                    )
                                    .child(
                                        soft_action(
                                            "add-learning-btn",
                                            Some(IconName::Plus),
                                            "Add track",
                                            pal,
                                        )
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            let title = this
                                                .learning_title_input
                                                .read(cx)
                                                .value()
                                                .trim()
                                                .to_string();
                                            if title.is_empty() {
                                                return;
                                            }
                                            if this
                                                .exec(
                                                    "learning.create",
                                                    json!({
                                                        "title": title,
                                                        "status": "active",
                                                        "target_hours": 20.0
                                                    }),
                                                    "Learning track added",
                                                )
                                                .is_some()
                                            {
                                                this.learning_title_input.update(cx, |s, cx| {
                                                    s.set_value("", window, cx);
                                                });
                                            }
                                            cx.notify();
                                        })),
                                    ),
                            )
                            .child({
                                let mut l_list = v_flex().w_full().min_w_0().gap(px(6.0));
                                for tr in self.snapshot.learning.iter().take(4) {
                                    let lid = tr.id.clone();
                                    l_list = l_list.child(
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
                                                    .flex_1()
                                                    .min_w_0()
                                                    .truncate()
                                                    .text_size(px(12.0))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(tr.title.clone()),
                                            )
                                            .child(
                                                soft_action(
                                                    SharedString::from(format!(
                                                        "log-learn-{lid}"
                                                    )),
                                                    None,
                                                    "+30m study",
                                                    pal,
                                                )
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.exec(
                                                        "learning.log",
                                                        json!({
                                                            "track_id": lid,
                                                            "minutes": 30
                                                        }),
                                                        "Logged 30m learning session",
                                                    );
                                                    cx.notify();
                                                })),
                                            ),
                                    );
                                }
                                l_list
                            }),
                    )
                    .child(
                        self.surface_card(pal)
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Finance commitments & wellbeing check-in"),
                            )
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .items_center()
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .child(Input::new(&self.finance_title_input)),
                                    )
                                    .child(
                                        soft_action(
                                            "add-finance-btn",
                                            Some(IconName::Plus),
                                            "Add reminder",
                                            pal,
                                        )
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            let title = this
                                                .finance_title_input
                                                .read(cx)
                                                .value()
                                                .trim()
                                                .to_string();
                                            if title.is_empty() {
                                                return;
                                            }
                                            if this
                                                .exec(
                                                    "finance.create",
                                                    json!({
                                                        "title": title,
                                                        "status": "pending",
                                                        "currency": "USD"
                                                    }),
                                                    "Finance commitment added",
                                                )
                                                .is_some()
                                            {
                                                this.finance_title_input.update(cx, |s, cx| {
                                                    s.set_value("", window, cx);
                                                });
                                            }
                                            cx.notify();
                                        })),
                                    ),
                            )
                            .child({
                                let mut f_list = v_flex().w_full().min_w_0().gap(px(6.0));
                                for fin in self.snapshot.finance.iter().take(4) {
                                    let fid = fin.id.clone();
                                    let is_paid = fin.status == "paid";
                                    f_list = f_list.child(
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
                                                    .flex_1()
                                                    .min_w_0()
                                                    .truncate()
                                                    .text_size(px(12.0))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(fin.title.clone()),
                                            )
                                            .child(
                                                soft_action(
                                                    SharedString::from(format!("pay-fin-{fid}")),
                                                    None,
                                                    if is_paid { "Paid" } else { "Mark paid" },
                                                    pal,
                                                )
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    if !is_paid {
                                                        this.exec(
                                                            "finance.update",
                                                            json!({ "id": fid, "status": "paid" }),
                                                            "Commitment marked paid",
                                                        );
                                                        cx.notify();
                                                    }
                                                })),
                                            ),
                                    );
                                }
                                f_list
                            })
                            .child(
                                h_flex()
                                    .justify_between()
                                    .items_center()
                                    .pt(px(8.0))
                                    .border_t_1()
                                    .border_color(pal.border())
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(if let Some(w) = self.snapshot.wellbeing.first()
                                            {
                                                format!(
                                                    "Latest check-in ({}): Energy {}/5 · Mood {}/5",
                                                    w.day, w.energy, w.mood
                                                )
                                            } else {
                                                "No wellbeing check-in recorded today.".into()
                                            }),
                                    )
                                    .child(
                                        primary_action(
                                            "log-wellbeing-btn",
                                            Some(IconName::Sparkles),
                                            "Log today (4/5)",
                                            pal,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.exec(
                                                "wellbeing.log",
                                                json!({
                                                    "energy": 4,
                                                    "mood": 4,
                                                    "sleep_hours": 7.5,
                                                    "note": "Steady energy and intentional focus"
                                                }),
                                                "Daily wellbeing check-in recorded",
                                            );
                                            cx.notify();
                                        })),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_library(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let mut templates = v_flex().w_full().min_w_0().gap(px(8.0));
        for tpl in &self.snapshot.templates {
            let tpl_id = tpl.id.clone();
            let tpl_del = tpl.id.clone();
            templates = templates.child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .p(px(12.0))
                    .rounded(px(9.0))
                    .bg(pal.surface_raised())
                    .justify_between()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::BOLD)
                                    .truncate()
                                    .child(tpl.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .truncate()
                                    .child(format!(
                                        "{} · P{} · {}m",
                                        tpl.title, tpl.priority, tpl.estimate_minutes
                                    )),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap(px(6.0))
                            .child(
                                primary_action(
                                    SharedString::from(format!("apply-db-tpl-{tpl_id}")),
                                    Some(IconName::Plus),
                                    "Apply",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "templates.apply",
                                            json!({ "id": tpl_id }),
                                            "Task created from template",
                                        );
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                soft_action(
                                    SharedString::from(format!("del-db-tpl-{tpl_del}")),
                                    None,
                                    "Archive",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.exec(
                                            "templates.delete",
                                            json!({ "id": tpl_del }),
                                            "Template archived",
                                        );
                                        cx.notify();
                                    },
                                )),
                            ),
                    ),
            );
        }

        let presets: [(&str, &str, i64); 3] = [
            (
                "Weekly architecture & system review",
                "Review open milestones, prune backlog, and set top 3 goals",
                2,
            ),
            (
                "Deep work sprint block",
                "Protect 90 minutes of uninterrupted execution on P1 commitment",
                1,
            ),
            (
                "Inbox zero triage pass",
                "Clarify captured items into actionable tasks or reference notes",
                3,
            ),
        ];

        for (idx, (name, desc, priority)) in presets.into_iter().enumerate() {
            let spawn_title = name.to_string();
            templates = templates.child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .p(px(12.0))
                    .rounded(px(9.0))
                    .bg(pal.surface_raised())
                    .justify_between()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(2.0))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::BOLD)
                                    .truncate()
                                    .child(name),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .truncate()
                                    .child(format!("{desc} (P{priority})")),
                            ),
                    )
                    .child(
                        soft_action(
                            SharedString::from(format!("spawn-tpl-{idx}")),
                            Some(IconName::Plus),
                            "Spawn task",
                            pal,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.exec(
                                "tasks.create",
                                json!({
                                    "title": spawn_title,
                                    "description": desc,
                                    "priority": priority,
                                    "status": "todo"
                                }),
                                "Task spawned from library template",
                            );
                            cx.notify();
                        })),
                    ),
            );
        }

        let mut saved_views = v_flex().w_full().min_w_0().gap(px(8.0));
        for sv in &self.snapshot.saved_views {
            let sv_id = sv.id.clone();
            saved_views = saved_views.child(
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
                                    .child(sv.name.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(pal.text_muted())
                                    .truncate()
                                    .child(format!("{} · {}", sv.entity, sv.query_json)),
                            ),
                    )
                    .child(
                        soft_action(
                            SharedString::from(format!("del-sv-{sv_id}")),
                            None,
                            "Remove",
                            pal,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.exec(
                                "saved_views.delete",
                                json!({ "id": sv_id }),
                                "Saved lens removed",
                            );
                            cx.notify();
                        })),
                    ),
            );
        }

        let lenses: [(&str, &str, TaskFilter); 4] = [
            (
                "Today's commitments",
                "TASKS · DUE OR PLANNED TODAY",
                TaskFilter::Today,
            ),
            (
                "Urgent & open execution queue",
                "TASKS · OPEN BY PRIORITY",
                TaskFilter::Open,
            ),
            (
                "Unprocessed inbox captures",
                "INBOX · TRIAGE QUEUE",
                TaskFilter::Inbox,
            ),
            (
                "Completed work archive",
                "TASKS · COMPLETED LOG",
                TaskFilter::Completed,
            ),
        ];

        for (idx, (name, scope, filter)) in lenses.into_iter().enumerate() {
            saved_views = saved_views.child(
                h_flex()
                    .id(SharedString::from(format!("library-lens-{idx}")))
                    .cursor_pointer()
                    .w_full()
                    .min_w_0()
                    .p(px(12.0))
                    .rounded(px(9.0))
                    .bg(pal.surface_raised())
                    .justify_between()
                    .items_center()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_task_filter(filter, cx);
                        this.switch_view(WorkspaceView::Tasks, cx);
                    }))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(pal.accent_text())
                            .child(scope),
                    ),
            );
        }

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
                            .child("Reusable task templates"),
                    )
                    .child(
                        h_flex()
                            .gap(px(8.0))
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(&self.template_name_input)),
                            )
                            .child(
                                primary_action(
                                    "create-library-tpl-btn",
                                    Some(IconName::Plus),
                                    "Save template",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| {
                                        let name = this
                                            .template_name_input
                                            .read(cx)
                                            .value()
                                            .trim()
                                            .to_string();
                                        if name.is_empty() {
                                            return;
                                        }
                                        if this
                                            .exec(
                                                "templates.create",
                                                json!({
                                                    "name": name,
                                                    "title": name,
                                                    "priority": 2,
                                                    "estimate_minutes": 30
                                                }),
                                                "Reusable template saved",
                                            )
                                            .is_some()
                                        {
                                            this.template_name_input.update(cx, |s, cx| {
                                                s.set_value("", window, cx);
                                            });
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(templates),
            )
            .child(
                self.surface_card(pal)
                    .flex_1()
                    .min_w_0()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_size(px(17.0))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Saved lenses & workspace catalog"),
                            )
                            .child(
                                soft_action(
                                    "save-p1-lens-btn",
                                    Some(IconName::Plus),
                                    "Save P1 lens",
                                    pal,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.exec(
                                            "saved_views.create",
                                            json!({
                                                "name": "High-priority P1 focus lens",
                                                "entity": "tasks",
                                                "query": { "priority": 1, "status": "todo" }
                                            }),
                                            "Saved P1 lens to workspace catalog",
                                        );
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(saved_views),
            )
            .into_any_element()
    }

    fn render_command_palette(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        let mut quick_nav = h_flex().gap(px(6.0)).flex_wrap();
        for dest in [
            WorkspaceView::Today,
            WorkspaceView::Inbox,
            WorkspaceView::Tasks,
            WorkspaceView::Calendar,
            WorkspaceView::Projects,
            WorkspaceView::Focus,
            WorkspaceView::Notes,
            WorkspaceView::Agents,
            WorkspaceView::Settings,
        ] {
            quick_nav = quick_nav.child(
                soft_action(
                    SharedString::from(format!("palette-go-{}", dest.label())),
                    Some(dest.icon()),
                    dest.label(),
                    pal,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.switch_view(dest, cx);
                })),
            );
        }

        let mut results = v_flex().w_full().min_w_0().gap(px(4.0));
        for task in self.search_tasks.iter().take(6) {
            let tid = task.id.clone();
            results = results.child(
                h_flex()
                    .id(SharedString::from(format!("palette-result-{tid}")))
                    .cursor_pointer()
                    .w_full()
                    .min_w_0()
                    .p(px(10.0))
                    .rounded(px(7.0))
                    .bg(pal.surface_raised())
                    .justify_between()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.selected_task_id = Some(tid.clone());
                        this.switch_view(WorkspaceView::Tasks, cx);
                    }))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(task.title.clone()),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .child(format!("P{}", task.priority)),
                    ),
            );
        }

        self.surface_card(pal)
            .id("command-palette-panel")
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
                            .child("COMMAND PALETTE & WORKSPACE SEARCH"),
                    )
                    .child(
                        soft_action("palette-close-btn", None, "Close", pal).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.command_palette_open = false;
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(Input::new(&self.palette_input))
            .child(quick_nav)
            .when(!self.search_tasks.is_empty(), |el| el.child(results))
            .into_any_element()
    }

    fn render_active_view(&mut self, pal: Palette, cx: &mut Context<Self>) -> AnyElement {
        match self.view {
            WorkspaceView::Today => self.render_today(pal, cx),
            WorkspaceView::Inbox | WorkspaceView::Tasks => self.render_tasks_or_inbox(pal, cx),
            WorkspaceView::Calendar => self.render_calendar_or_planning(pal, cx),
            WorkspaceView::Planning => self.render_planning(pal, cx),
            WorkspaceView::Projects => self.render_projects(pal, cx),
            WorkspaceView::Areas => self.render_areas(pal, cx),
            WorkspaceView::Library => self.render_library(pal, cx),
            WorkspaceView::Goals => self.render_goals(pal, cx),
            WorkspaceView::Focus => self.render_focus(pal, cx),
            WorkspaceView::Habits | WorkspaceView::Routines => {
                self.render_habits_or_routines(pal, cx)
            }
            WorkspaceView::Notes | WorkspaceView::Reviews | WorkspaceView::Insights => {
                self.render_notes_or_reviews(pal, cx)
            }
            WorkspaceView::Agents | WorkspaceView::Automations => {
                self.render_agents_or_automations(pal, cx)
            }
            WorkspaceView::Settings => self.render_settings(pal, cx),
        }
    }
}

impl Render for LifeManagerApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = self.palette();
        if self.locked {
            return self.render_unlock_screen(pal, cx);
        }

        let palette_drawer = if self.command_palette_open {
            Some(self.render_command_palette(pal, cx))
        } else {
            None
        };
        let quick_capture_drawer = if self.quick_capture_open {
            Some(self.render_quick_capture_drawer(pal, cx))
        } else {
            None
        };
        let show_onboarding = !self.onboarding_dismissed
            && self.view == WorkspaceView::Today
            && self.snapshot.areas.is_empty()
            && self.snapshot.tasks.is_empty();
        let onboarding_drawer = if show_onboarding {
            Some(self.render_onboarding_card(pal, cx))
        } else {
            None
        };
        let active_body = self.render_active_view(pal, cx);
        let nice_date = Local::now().format("%A, %B %-d, %Y").to_string();
        let chip_date = Local::now().format("%a, %b %-d").to_string();
        let subtitle = if self.view == WorkspaceView::Today {
            format!("{nice_date} · {}", self.view.subtitle())
        } else {
            self.view.subtitle().to_string()
        };

        h_flex()
            .id("life-manager-shell")
            .size_full()
            .overflow_hidden()
            .bg(pal.canvas())
            .text_color(pal.text())
            .on_key_down(
                cx.listener(|this, event: &gpui_kit::KeyDownEvent, _window, cx| {
                    let key = event.keystroke.key.as_str();
                    let cmd_or_ctrl =
                        event.keystroke.modifiers.platform || event.keystroke.modifiers.control;
                    if cmd_or_ctrl && key.eq_ignore_ascii_case("k") {
                        this.command_palette_open = !this.command_palette_open;
                        this.quick_capture_open = false;
                        cx.notify();
                    } else if cmd_or_ctrl && key.eq_ignore_ascii_case("n") {
                        this.quick_capture_open = !this.quick_capture_open;
                        this.command_palette_open = false;
                        cx.notify();
                    } else if key == "escape"
                        && (this.command_palette_open || this.quick_capture_open)
                    {
                        this.command_palette_open = false;
                        this.quick_capture_open = false;
                        cx.notify();
                    }
                }),
            )
            .child(self.sidebar_pane.clone())
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(self.topbar_pane.clone())
                    .child(
                        v_flex()
                            .id("workspace-main-scroll")
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .min_w_0()
                            .px(px(41.0))
                            .pt(px(28.0))
                            .pb(px(24.0))
                            .gap(px(22.0))
                            .overflow_y_scrollbar()
                            .scroll_bounce(false)
                            .when_some(self.error.clone(), |el, err| {
                                el.child(
                                    h_flex()
                                        .w_full()
                                        .min_w_0()
                                        .p(px(12.0))
                                        .rounded(px(9.0))
                                        .bg(pal.warning_muted())
                                        .text_color(pal.warning_text())
                                        .justify_between()
                                        .items_center()
                                        .child(
                                            h_flex()
                                                .flex_1()
                                                .min_w_0()
                                                .gap(px(8.0))
                                                .items_center()
                                                .child(lucide(
                                                    IconName::CircleAlert,
                                                    16.0,
                                                    pal.warning_text(),
                                                ))
                                                .child(
                                                    div()
                                                        .flex_1()
                                                        .min_w_0()
                                                        .whitespace_normal()
                                                        .child(err),
                                                ),
                                        )
                                        .child(
                                            Button::new("dismiss-error-btn")
                                                .ghost()
                                                .label("Dismiss")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.error = None;
                                                    cx.notify();
                                                })),
                                        ),
                                )
                            })
                            .when_some(self.notice.clone(), |el, msg| {
                                el.child(
                                    h_flex()
                                        .w_full()
                                        .min_w_0()
                                        .p(px(10.0))
                                        .gap(px(8.0))
                                        .items_center()
                                        .rounded(px(9.0))
                                        .bg(pal.accent_muted())
                                        .text_color(pal.accent_text())
                                        .text_size(px(12.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(lucide(
                                            IconName::CircleCheck,
                                            15.0,
                                            pal.accent_text(),
                                        ))
                                        .child(msg),
                                )
                            })
                            .when_some(palette_drawer, |el, drawer| el.child(drawer))
                            .when_some(quick_capture_drawer, |el, drawer| el.child(drawer))
                            .when_some(onboarding_drawer, |el, drawer| el.child(drawer))
                            .child(
                                h_flex()
                                    .w_full()
                                    .min_w_0()
                                    .justify_between()
                                    .items_start()
                                    .child(
                                        v_flex()
                                            .flex_1()
                                            .min_w_0()
                                            .gap(px(6.0))
                                            .child(
                                                div()
                                                    .text_size(px(10.0))
                                                    .font_weight(FontWeight::EXTRA_BOLD)
                                                    .text_color(pal.accent_text())
                                                    .child(self.view.eyebrow()),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(34.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(pal.text())
                                                    .child(self.view.heading()),
                                            )
                                            .child(
                                                div()
                                                    .w_full()
                                                    .min_w_0()
                                                    .whitespace_normal()
                                                    .text_size(px(13.0))
                                                    .text_color(pal.text_muted())
                                                    .child(subtitle),
                                            ),
                                    )
                                    .child(
                                        h_flex()
                                            .flex_shrink_0()
                                            .mt(px(18.0))
                                            .px(px(11.0))
                                            .py(px(7.0))
                                            .gap(px(8.0))
                                            .rounded(px(8.0))
                                            .bg(pal.surface())
                                            .border_1()
                                            .border_color(pal.border())
                                            .items_center()
                                            .text_size(px(11.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(pal.text_secondary())
                                            .child(lucide(
                                                IconName::CalendarDays,
                                                14.0,
                                                pal.accent_text(),
                                            ))
                                            .child(chip_date),
                                    ),
                            )
                            .child(active_body)
                            .child(
                                h_flex()
                                    .w_full()
                                    .mt(px(14.0))
                                    .pt(px(12.0))
                                    .border_t_1()
                                    .border_color(pal.border())
                                    .justify_between()
                                    .text_size(px(9.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(pal.text_faint())
                                    .child(format!(
                                        "LIFE MANAGER  ·  {}  ·  Made for intentional days",
                                        self.view.label().to_uppercase()
                                    ))
                                    .child("LOCAL  ·  PRIVATE  ·  AGENT-READY"),
                            ),
                    ),
            )
            .into_any_element()
    }
}
