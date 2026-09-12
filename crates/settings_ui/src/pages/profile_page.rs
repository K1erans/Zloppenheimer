use chrono::{Datelike, Days, Local};
use gpui::{ScrollHandle, Subscription};
use ui::{ButtonLike, Tooltip, prelude::*};
use workspace::AppState;

use super::ai_page::text;
use crate::SettingsWindow;

#[derive(Clone, Copy, PartialEq)]
enum ActivityPeriod {
    Daily,
    Weekly,
    Cumulative,
}

pub(crate) struct ProfilePageState {
    period: ActivityPeriod,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

pub(crate) fn render_profile_page(
    settings_window: &mut SettingsWindow,
    _window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    if settings_window.profile_page_state.is_none() {
        let mut subscriptions = Vec::new();
        if let Some(store) = agent::ActivityStore::try_global(cx) {
            subscriptions.push(cx.observe(&store, |_, _, cx| cx.notify()));
        }
        let user_store = AppState::global(cx).user_store.clone();
        subscriptions.push(cx.observe(&user_store, |_, _, cx| cx.notify()));
        settings_window.profile_page_state = Some(ProfilePageState {
            period: ActivityPeriod::Daily,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        });
    }
    let Some(state) = settings_window.profile_page_state.as_ref() else {
        return gpui::Empty.into_any_element();
    };
    let activity = agent::ActivityStore::try_global(cx).map(|store| store.read(cx).snapshot());
    let user_store = AppState::global(cx).user_store.clone();
    let user_store = user_store.read(cx);
    let user = user_store.current_user();
    let name = user
        .as_ref()
        .and_then(|user| user.name.clone())
        .map(SharedString::from)
        .or_else(|| user.as_ref().map(|user| user.username.clone()))
        .unwrap_or_else(|| "Your profile".into());
    let username: SharedString = match user.as_ref() {
        Some(user) if user.username.starts_with('@') => user.username.clone(),
        Some(user) => format!("@{}", user.username).into(),
        None => "Not signed in".into(),
    };
    let initials: String = name
        .split_whitespace()
        .take(2)
        .filter_map(|part| part.chars().next())
        .flat_map(char::to_uppercase)
        .collect();
    let plan = user_store.plan().map(|plan| match plan {
        cloud_api_types::Plan::ZedFree => "Free",
        cloud_api_types::Plan::ZedPro => "Pro",
        cloud_api_types::Plan::ZedProTrial => "Pro trial",
        cloud_api_types::Plan::ZedBusiness => "Business",
        cloud_api_types::Plan::ZedVip => "VIP",
        cloud_api_types::Plan::ZedStudent => "Student",
    });
    let loaded = activity
        .as_ref()
        .is_some_and(|activity| !activity.loading && activity.error.is_none());
    let values = activity
        .as_ref()
        .map(|activity| {
            [
                compact_number(activity.lifetime_tokens),
                compact_number(activity.peak_tokens),
                format_chat_duration(activity.longest_chat_seconds),
                format_days(activity.current_streak),
                format_days(activity.longest_streak),
            ]
        })
        .unwrap_or_default();
    let today = Local::now().date_naive();
    let start = today.checked_sub_days(Days::new(364)).unwrap_or(today);
    let start = start
        .checked_sub_days(Days::new(u64::from(start.weekday().num_days_from_monday())))
        .unwrap_or(start);
    let mut counts = Vec::new();
    let mut cumulative = 0_u64;
    for index in 0..371 {
        let date = start.checked_add_days(Days::new(index)).unwrap_or(today);
        let daily = activity
            .as_ref()
            .and_then(|activity| activity.daily_tokens.get(&date))
            .copied()
            .unwrap_or(0);
        cumulative = cumulative.saturating_add(daily);
        let value = match state.period {
            ActivityPeriod::Daily => daily,
            ActivityPeriod::Cumulative => cumulative,
            ActivityPeriod::Weekly => {
                let week_start = date
                    .checked_sub_days(Days::new(u64::from(date.weekday().num_days_from_monday())))
                    .unwrap_or(date);
                (0..7)
                    .filter_map(|day| week_start.checked_add_days(Days::new(day)))
                    .filter_map(|day| activity.as_ref()?.daily_tokens.get(&day))
                    .copied()
                    .sum()
            }
        };
        counts.push((date, value));
    }
    let maximum = counts.iter().map(|(_, count)| *count).max().unwrap_or(0);
    let colors = [
        cx.theme().colors().element_background,
        cx.theme().colors().element_hover,
        cx.theme().colors().text_accent.opacity(0.55),
        cx.theme().colors().text_accent,
    ];
    let columns = counts.chunks(7).enumerate().map(|(column, days)| {
        v_flex()
            .gap(px(4.))
            .children(days.iter().enumerate().map(|(row, (date, count))| {
                let level = if *count == 0 || !loaded {
                    0
                } else if *count <= maximum / 3 {
                    1
                } else if *count <= maximum.saturating_mul(2) / 3 {
                    2
                } else {
                    3
                };
                div()
                    .id(("activity-day", column * 7 + row))
                    .size(px(13.))
                    .rounded(px(3.))
                    .bg(*colors
                        .get(level)
                        .unwrap_or(&cx.theme().colors().element_background))
                    .when(*date > today, |this| this.opacity(0.3))
                    .tooltip(Tooltip::text(format!(
                        "{}: {} recorded tokens",
                        date.format("%b %-d, %Y"),
                        compact_number(*count)
                    )))
            }))
    });
    let source = activity
        .as_ref()
        .and_then(|activity| activity.first_recorded_day)
        .map(|date| {
            format!(
                "Tokens recorded by the built-in agent on this device since {}",
                date.format("%b %-d, %Y")
            )
        })
        .unwrap_or_else(|| {
            if loaded {
                "Tokens recorded by the built-in agent on this device".to_string()
            } else {
                "Loading activity recorded on this device…".to_string()
            }
        });
    v_flex()
        .id("profile-settings-page")
        .flex_1()
        .min_w_0()
        .size_full()
        .pt(px(36.))
        .pb(px(28.))
        .bg(cx.theme().colors().background)
        .track_scroll(&state.scroll)
        .overflow_y_scroll()
        .child(
            v_flex()
                .w_full()
                .max_w(px(900.))
                .mx_auto()
                .gap(px(32.))
                .child(text(
                    ui::localized("Profile", cx),
                    26.,
                    34.,
                    cx.theme().colors().text,
                ))
                .child(
                    v_flex()
                        .items_center()
                        .pt(px(12.))
                        .pb(px(16.))
                        .gap(px(14.))
                        .child(
                            div()
                                .size(px(88.))
                                .rounded_full()
                                .bg(cx.theme().colors().text_accent)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(text(initials, 30., 36., cx.theme().colors().text)),
                        )
                        .child(text(name, 28., 36., cx.theme().colors().text))
                        .child(
                            h_flex()
                                .gap(px(10.))
                                .child(text(username, 14., 18., cx.theme().colors().text_muted))
                                .when_some(plan, |this, plan| {
                                    this.child(text(
                                        ui::localized("·", cx),
                                        14.,
                                        18.,
                                        cx.theme().colors().text_muted,
                                    ))
                                    .child(
                                        div()
                                            .border_1()
                                            .border_color(cx.theme().colors().border_selected)
                                            .rounded(px(6.))
                                            .px(px(9.))
                                            .py(px(3.))
                                            .child(text(
                                                plan,
                                                12.,
                                                16.,
                                                cx.theme().colors().text_accent,
                                            )),
                                    )
                                }),
                        ),
                )
                .child(
                    h_flex()
                        .h(px(90.))
                        .w_full()
                        .bg(cx.theme().colors().surface_background)
                        .border_1()
                        .border_color(cx.theme().colors().border)
                        .rounded(px(11.))
                        .children(
                            [
                                "Lifetime tokens",
                                "Peak tokens",
                                "Longest chat",
                                "Current streak",
                                "Longest streak",
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(index, label)| {
                                v_flex()
                                    .flex_1()
                                    .h(px(56.))
                                    .items_center()
                                    .justify_center()
                                    .gap(px(8.))
                                    .when(index > 0, |this| {
                                        this.border_l_1().border_color(cx.theme().colors().border)
                                    })
                                    .child(text(
                                        if loaded {
                                            values.get(index).cloned().unwrap_or_default()
                                        } else {
                                            "—".to_string()
                                        },
                                        21.,
                                        26.,
                                        cx.theme().colors().text,
                                    ))
                                    .child(text(
                                        ui::localized(label, cx),
                                        12.,
                                        16.,
                                        cx.theme().colors().text_muted,
                                    ))
                            }),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .gap(px(16.))
                        .child(
                            text(
                                ui::localized("Token activity", cx),
                                16.,
                                20.,
                                cx.theme().colors().text,
                            )
                            .flex_1(),
                        )
                        .children(
                            [
                                (ActivityPeriod::Daily, "Daily"),
                                (ActivityPeriod::Weekly, "Weekly"),
                                (ActivityPeriod::Cumulative, "Cumulative"),
                            ]
                            .into_iter()
                            .map(|(period, label)| {
                                ButtonLike::new(label)
                                    .size(ButtonSize::None)
                                    .corner_radius(px(5.))
                                    .when(state.period == period, |this| {
                                        this.background(cx.theme().colors().element_hover.into())
                                            .custom_style(|this| this.px(px(10.)).py(px(5.)))
                                    })
                                    .child(text(
                                        label,
                                        13.,
                                        16.,
                                        if state.period == period {
                                            cx.theme().colors().text_accent
                                        } else {
                                            cx.theme().colors().text_muted
                                        },
                                    ))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(state) = this.profile_page_state.as_mut() {
                                            state.period = period;
                                        }
                                        cx.notify();
                                    }))
                            }),
                        ),
                )
                .child(
                    v_flex()
                        .gap(px(16.))
                        .child(h_flex().gap(px(4.)).children(columns))
                        .child(
                            h_flex()
                                .w_full()
                                .justify_between()
                                .children((0..12).map(|index| {
                                    let month_index = (start.month0() + index) % 12;
                                    let labels = [
                                        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug",
                                        "Sept", "Oct", "Nov", "Dec",
                                    ];
                                    text(
                                        labels.get(month_index as usize).copied().unwrap_or(""),
                                        11.,
                                        14.,
                                        cx.theme().colors().text_muted,
                                    )
                                })),
                        )
                        .child(
                            h_flex()
                                .justify_end()
                                .gap(px(6.))
                                .child(text(
                                    ui::localized("Less", cx),
                                    10.,
                                    12.,
                                    cx.theme().colors().text_muted,
                                ))
                                .children(
                                    colors
                                        .into_iter()
                                        .map(|color| div().size(px(10.)).rounded(px(2.)).bg(color)),
                                )
                                .child(text(
                                    ui::localized("More", cx),
                                    10.,
                                    12.,
                                    cx.theme().colors().text_muted,
                                )),
                        )
                        .child(text(source, 11., 16., cx.theme().colors().text_muted))
                        .when_some(
                            activity
                                .as_ref()
                                .and_then(|activity| activity.error.clone()),
                            |this, error| {
                                this.child(text(error, 12., 18., cx.theme().status().error))
                            },
                        ),
                ),
        )
        .into_any_element()
}

fn compact_number(value: u64) -> String {
    if value >= 1_000_000_000 {
        format!("{:.1}bn", value as f64 / 1_000_000_000.)
    } else if value >= 1_000_000 {
        format!("{:.1}m", value as f64 / 1_000_000.)
    } else if value >= 1_000 {
        format!("{:.1}k", value as f64 / 1_000.)
    } else {
        value.to_string()
    }
}

fn format_chat_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

fn format_days(count: u64) -> String {
    if count == 1 {
        "1 day".into()
    } else {
        format!("{} days", count)
    }
}
