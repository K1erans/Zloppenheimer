use gpui::{ReadGlobal, ScrollHandle};
use language_model::LanguageModelRegistry;
use project::agent_server_store::{AgentId, ExternalAgentSource};
use settings::Settings as _;
use ui::{ButtonLike, ContextMenu, ContextMenuEntry, PopoverMenu, prelude::*};

use super::ai_page::text;
use crate::SettingsWindow;

pub(crate) fn render_model_connections_page(
    settings_window: &SettingsWindow,
    scroll_handle: &ScrollHandle,
    window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let providers = LanguageModelRegistry::read_global(cx).providers();
    let project = settings_window.active_project(cx);
    let store = project
        .as_ref()
        .map(|project| project.read(cx).agent_server_store().clone());
    let mut agents = store
        .as_ref()
        .map(|store| {
            let store = store.read(cx);
            store
                .external_agents()
                .map(|id| {
                    (
                        id.clone(),
                        store.agent_display_name(id).unwrap_or_else(|| id.0.clone()),
                        store.agent_source(id).unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let disabled_servers = agent_settings::AgentSettings::get_global(cx)
        .disabled_servers
        .clone();
    if let Some(configured) = settings::SettingsStore::global(cx)
        .get_content_for_file(settings::SettingsFile::User)
        .and_then(|content| content.agent_servers.as_ref())
    {
        for (id, configuration) in configured.iter() {
            if agents
                .iter()
                .any(|(existing, _, _)| existing.0.as_ref() == id)
            {
                continue;
            }
            let source = match configuration {
                settings::CustomAgentServerSettings::Registry { .. } => {
                    ExternalAgentSource::Registry
                }
                _ => ExternalAgentSource::Custom,
            };
            agents.push((AgentId(id.clone().into()), id.clone().into(), source));
        }
    }
    let selected = settings_window
        .selected_connection
        .clone()
        .or_else(|| agents.first().map(|(id, _, _)| format!("agent:{}", id.0)))
        .or_else(|| {
            providers
                .iter()
                .find(|provider| provider.is_authenticated(cx))
                .map(|provider| format!("provider:{}", provider.id().0))
        })
        .or_else(|| {
            providers
                .first()
                .map(|provider| format!("provider:{}", provider.id().0))
        });
    let mut rows = Vec::new();
    for (id, name, _) in &agents {
        let key = format!("agent:{}", id.0);
        rows.push(connection_row(
            key.clone(),
            name.clone(),
            if disabled_servers.contains(&id.0.to_string()) {
                "Disabled"
            } else {
                "ACP agent installed"
            },
            selected.as_ref() == Some(&key),
            Some((id.clone(), !disabled_servers.contains(&id.0.to_string()))),
            cx,
        ));
    }
    for provider in &providers {
        if !provider.is_authenticated(cx)
            && selected.as_deref() != Some(&format!("provider:{}", provider.id().0))
        {
            continue;
        }
        let key = format!("provider:{}", provider.id().0);
        rows.push(connection_row(
            key.clone(),
            provider.name().0,
            if provider.is_authenticated(cx) {
                "Account connected"
            } else {
                "Not connected"
            },
            selected.as_ref() == Some(&key),
            None,
            cx,
        ));
    }
    let mut detail = v_flex()
        .flex_1()
        .min_w_0()
        .p(px(24.))
        .gap(px(20.))
        .bg(gpui::rgb(0x232530));
    if let Some(key) = selected.as_deref() {
        if let Some(id) = key.strip_prefix("provider:") {
            if let Some(provider) = providers
                .iter()
                .find(|provider| provider.id().0.as_ref() == id)
            {
                detail = detail.child(super::llm_providers_page::render_provider_section(
                    settings_window,
                    provider,
                    true,
                    window,
                    cx,
                ));
            }
        } else if let Some(id) = key.strip_prefix("agent:") {
            let id = AgentId(id.to_string().into());
            if let Some((_, name, source)) = agents.iter().find(|(agent_id, _, _)| *agent_id == id)
            {
                let existing = super::external_agents_page::custom_agent_settings(&id, cx);
                detail = detail.child(
                    h_flex()
                        .justify_between()
                        .child(text(name.clone(), 18., 22., 0xE0DCE9))
                        .child(text(ui::localized("Installed", cx), 11., 14., 0xA3A7B9)),
                );
                if let Some(settings::CustomAgentServerSettings::Custom {
                    path, args, env, ..
                }) = &existing
                {
                    detail = detail
                        .child(group(vec![property(
                            "Display name",
                            "Name shown in your connections.",
                            name.to_string(),
                        )]))
                        .child(text(ui::localized("Runtime", cx), 13., 16., 0xB5B1C4))
                        .child(group(vec![
                            property(
                                "Binary path",
                                "Executable used for this connection.",
                                path.to_string_lossy().into_owned(),
                            ),
                            property(
                                "Launch arguments",
                                "Additional arguments passed on startup.",
                                args.join(" "),
                            ),
                        ]))
                        .child(text(ui::localized("Environment", cx), 13., 16., 0xB5B1C4))
                        .child(group(vec![property(
                            "Variables",
                            "Keys, base URLs, and connection-specific settings.",
                            format!("{} configured", env.len()),
                        )]));
                }
                let source = *source;
                let edit_id = id.clone();
                detail = detail.child(
                    h_flex()
                        .justify_between()
                        .child(
                            ButtonLike::new("configure-connection")
                                .child(text(
                                    ui::localized("Configure connection", cx),
                                    12.,
                                    16.,
                                    0xBDB1CD,
                                ))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let existing =
                                        super::external_agents_page::custom_agent_settings(
                                            &edit_id, cx,
                                        )
                                        .map(|settings| (edit_id.clone(), settings));
                                    super::external_agents_page::open_custom_agent_form(
                                        this, existing, window, cx,
                                    );
                                })),
                        )
                        .child(
                            ButtonLike::new("remove-connection")
                                .child(text(
                                    ui::localized("Remove connection", cx),
                                    12.,
                                    16.,
                                    0xC79EA5,
                                ))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    super::external_agents_page::remove_agent(&id, source, cx);
                                    this.selected_connection = None;
                                    cx.notify();
                                })),
                        ),
                );
            }
        }
    } else {
        detail = detail.child(text(
            ui::localized("Add a connection to get started.", cx),
            13.,
            20.,
            0xA1A4B8,
        ));
    }
    v_flex()
        .id("model-connections-page")
        .size_full()
        .min_w_0()
        .bg(gpui::rgb(0x232530))
        .pt(px(36.))
        .pb(px(28.))
        .px(px(32.))
        .gap(px(20.))
        .track_scroll(scroll_handle)
        .overflow_y_scroll()
        .child(
            v_flex()
                .gap(px(8.))
                .child(
                    ButtonLike::new("back-to-ai")
                        .size(ButtonSize::None)
                        .child(text(ui::localized("← AI settings", cx), 12., 16., 0xACA2BC))
                        .on_click(cx.listener(|this, _, window, cx| this.pop_sub_page(window, cx))),
                )
                .child(text(
                    ui::localized("Model connections", cx),
                    26.,
                    34.,
                    0xECECF2,
                )),
        )
        .child(
            h_flex()
                .w_full()
                .min_h(px(694.))
                .items_stretch()
                .border_1()
                .border_color(gpui::rgb(0x414453))
                .rounded(px(10.))
                .overflow_hidden()
                .child(
                    v_flex()
                        .w(px(250.))
                        .flex_none()
                        .bg(gpui::rgb(0x262935))
                        .border_r_1()
                        .border_color(gpui::rgb(0x414453))
                        .child(
                            h_flex()
                                .h(px(48.))
                                .px(px(14.))
                                .gap(px(10.))
                                .border_b_1()
                                .border_color(gpui::rgb(0x3C3F4C))
                                .child(
                                    text(ui::localized("Connections", cx), 13., 16., 0xDCD8E7)
                                        .flex_1(),
                                )
                                .child(add_connection_menu(settings_window, cx)),
                        )
                        .children(rows)
                        .child(div().px(px(14.)).py(px(16.)).child(text(
                            ui::localized("Use + to add a provider or custom ACP agent.", cx),
                            11.,
                            17.,
                            0x9196AB,
                        ))),
                )
                .child(detail),
        )
        .into_any_element()
}

fn connection_row(
    key: String,
    name: SharedString,
    status: &'static str,
    selected: bool,
    enabled: Option<(AgentId, bool)>,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    ButtonLike::new(key.clone())
        .size(ButtonSize::None)
        .full_width()
        .corner_radius(px(0.))
        .when(selected, |this| this.background(gpui::rgb(0x373342).into()))
        .child(
            v_flex()
                .gap(px(6.))
                .child(text(name, 14., 18., 0xDFD9E9))
                .child(text(ui::localized(status, cx), 11., 14., 0xA3A7B9))
                .flex_1(),
        )
        .when_some(enabled, |this, (id, enabled)| {
            this.child(
                ui::Switch::new(format!("enable-agent-{}", id.0), enabled.into())
                    .settings_style()
                    .on_click(move |state, _, cx| {
                        cx.stop_propagation();
                        let enabled = *state == ToggleState::Selected;
                        let id = id.0.to_string();
                        settings::update_settings_file(
                            <dyn fs::Fs>::global(cx),
                            cx,
                            move |settings, _| {
                                let disabled =
                                    &mut settings.agent.get_or_insert_default().disabled_servers;
                                disabled.retain(|disabled| *disabled != id);
                                if !enabled {
                                    disabled.push(id);
                                }
                            },
                        );
                    }),
            )
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            this.selected_connection = Some(key.clone());
            cx.notify();
        }))
        .custom_style(|this| {
            this.px(px(14.))
                .py(px(16.))
                .border_b_1()
                .border_color(gpui::rgb(0x3C3F4C))
        })
        .into_any_element()
}

fn add_connection_menu(
    settings_window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    let original_window = settings_window.original_window;
    let settings_window = cx.entity().downgrade();
    PopoverMenu::new("add-connection-menu")
        .anchor(gpui::Anchor::TopLeft)
        .trigger(
            ButtonLike::new("add-connection")
                .size(ButtonSize::None)
                .width(px(26.))
                .height((px(26.)).into())
                .corner_radius(px(5.))
                .background((gpui::rgb(0x3A3346)).into())
                .child(text(ui::localized("+", cx), 19., 24., 0xDCD8E7))
                .custom_style(|this| this.border_1().border_color(gpui::rgb(0x5D506C))),
        )
        .menu(move |window, cx| {
            let for_api = settings_window.clone();
            let for_custom = settings_window.clone();
            Some(ContextMenu::build(window, cx, move |menu, _, _| {
                menu.dropdown_style(px(320.))
                    .header("ADD CONNECTION")
                    .item(
                        ContextMenuEntry::new("Provider account")
                            .description("Choose a provider and sign in.")
                            .handler(move |_, cx| {
                                if let Some(original_window) = original_window {
                                    use util::ResultExt;
                                    original_window
                                        .update(cx, |_, window, cx| {
                                            window.activate_window();
                                            window.dispatch_action(
                                                Box::new(zed_actions::AcpRegistry),
                                                cx,
                                            );
                                        })
                                        .log_err();
                                }
                            }),
                    )
                    .item(
                        ContextMenuEntry::new("API key")
                            .description("Connect a model provider with your own key.")
                            .handler(move |window, cx| {
                                use util::ResultExt;
                                for_api
                                    .update(cx, |this, cx| {
                                        this.push_dynamic_sub_page(
                                            "LLM Providers",
                                            "Model connections",
                                            Some("llm_providers"),
                                            false,
                                            super::render_llm_providers_page,
                                            window,
                                            cx,
                                        );
                                    })
                                    .log_err();
                            }),
                    )
                    .item(
                        ContextMenuEntry::new("Custom ACP agent")
                            .description("Run an agent using an ACP command.")
                            .handler(move |window, cx| {
                                use util::ResultExt;
                                for_custom
                                    .update(cx, |this, cx| {
                                        super::external_agents_page::open_custom_agent_form(
                                            this, None, window, cx,
                                        )
                                    })
                                    .log_err();
                            }),
                    )
            }))
        })
}

fn group(rows: Vec<AnyElement>) -> Div {
    v_flex()
        .bg(gpui::rgb(0x282B37))
        .border_1()
        .border_color(gpui::rgb(0x414453))
        .rounded(px(9.))
        .overflow_hidden()
        .children(rows)
}

fn property(title: &'static str, description: &'static str, value: String) -> AnyElement {
    h_flex()
        .p(px(15.))
        .gap(px(20.))
        .border_b_1()
        .border_color(gpui::rgb(0x393C49))
        .child(
            v_flex()
                .flex_1()
                .gap(px(5.))
                .child(text(title, 13., 16., 0xDFDBE9))
                .child(text(description, 11., 16., 0xA1A5B9)),
        )
        .child(
            div()
                .w(px(240.))
                .min_h(px(32.))
                .px(px(10.))
                .py(px(7.))
                .border_1()
                .border_color(gpui::rgb(0x414453))
                .rounded(px(6.))
                .bg(gpui::rgb(0x2B2E3A))
                .child(text(
                    if value.is_empty() {
                        "Use default".to_string()
                    } else {
                        value
                    },
                    12.,
                    16.,
                    0xCECADA,
                )),
        )
        .into_any_element()
}
