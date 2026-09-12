use std::sync::Arc;

use agent_settings::AgentSettings;
use editor::{Editor, EditorEvent};
use gpui::{Entity, ScrollHandle};
use language_model::{LanguageModel, LanguageModelRegistry};
use settings::{LanguageModelSelection, Settings, update_settings_file};
use ui::{ButtonLike, ContextMenu, PopoverMenu, Switch, prelude::*};

use crate::SettingsWindow;

pub(crate) struct AiPageState {
    search: Entity<Editor>,
    scroll: ScrollHandle,
    pub advanced: bool,
}

pub(crate) fn render_ai_page(
    settings_window: &mut SettingsWindow,
    window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    if settings_window.ai_page_state.is_none() {
        let search = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_text_style_refinement(gpui::TextStyleRefinement {
                font_size: Some(rems_from_px(12_f32).into()),
                line_height: Some(gpui::relative(16. / 12.)),
                ..Default::default()
            });
            editor.set_placeholder_text("Search available models…", window, cx);
            editor
        });
        cx.subscribe(&search, |_, _, event, cx| {
            if matches!(event, EditorEvent::Edited { .. }) {
                cx.notify();
            }
        })
        .detach();
        settings_window.ai_page_state = Some(AiPageState {
            search,
            scroll: ScrollHandle::new(),
            advanced: false,
        });
    }
    let Some(state) = settings_window.ai_page_state.as_ref() else {
        return gpui::Empty.into_any_element();
    };
    let query = state.search.read(cx).text(cx).to_lowercase();
    let registry = LanguageModelRegistry::global(cx);
    let mut models: Vec<_> = registry.read(cx).available_models(cx).collect();
    let available_count = models.len();
    models.sort_by_key(|model| {
        (
            model.provider_name().0.to_string(),
            model.name().0.to_string(),
        )
    });
    let settings = AgentSettings::get_global(cx);
    let selection = settings.default_model.clone();
    let selected_model = registry
        .read(cx)
        .default_model()
        .map(|configured| configured.model);
    let enabled = models
        .iter()
        .filter(|model| !is_hidden(model, settings))
        .count();
    let model_label = selected_model
        .as_ref()
        .map(|model| model.name().0.clone())
        .unwrap_or_else(|| "Choose a model".into());
    let model_menu = PopoverMenu::new("default-model-menu")
        .trigger(dropdown("default-model", model_label, cx))
        .anchor(gpui::Anchor::TopRight)
        .menu({
            let models = models.clone();
            move |window, cx| {
                let models = models.clone();
                Some(ContextMenu::build(window, cx, move |menu, _, _| {
                    models
                        .into_iter()
                        .fold(menu.dropdown_style(px(340.)), |menu, model| {
                            menu.entry(model.name().0.clone(), None, move |_, cx| {
                                let selection = model_selection(model.as_ref());
                                update_settings_file(
                                    <dyn fs::Fs>::global(cx),
                                    cx,
                                    move |settings, _| {
                                        let agent = settings.agent.get_or_insert_default();
                                        agent.hidden_models.retain(|hidden| {
                                            hidden.provider != selection.provider
                                                || hidden.model != selection.model
                                        });
                                        agent.set_model(selection.clone());
                                    },
                                );
                            })
                        })
                }))
            }
        });
    let effort_levels = selected_model
        .as_ref()
        .map(|model| model.supported_effort_levels())
        .unwrap_or_default();
    let effort_label = selection
        .as_ref()
        .and_then(|selection| selection.effort.as_ref())
        .and_then(|effort| {
            effort_levels
                .iter()
                .find(|level| level.value.as_ref() == effort)
        })
        .or_else(|| effort_levels.iter().find(|level| level.is_default))
        .map(|level| level.name.clone())
        .unwrap_or_else(|| "Model default".into());
    let effort_control = PopoverMenu::new("default-effort-menu")
        .trigger(dropdown("default-effort", effort_label, cx).disabled(effort_levels.is_empty()))
        .anchor(gpui::Anchor::TopRight)
        .menu(move |window, cx| {
            let effort_levels = effort_levels.clone();
            let selection = selection.clone();
            Some(ContextMenu::build(window, cx, move |menu, _, _| {
                effort_levels
                    .into_iter()
                    .fold(menu.dropdown_style(px(220.)), |menu, effort| {
                        let selection = selection.clone();
                        menu.entry(effort.name, None, move |_, cx| {
                            if let Some(mut selection) = selection.clone() {
                                selection.effort = Some(effort.value.to_string());
                                selection.enable_thinking = true;
                                update_settings_file(
                                    <dyn fs::Fs>::global(cx),
                                    cx,
                                    move |settings, _| {
                                        settings
                                            .agent
                                            .get_or_insert_default()
                                            .set_model(selection.clone());
                                    },
                                );
                            }
                        })
                    })
            }))
        });
    let context_label: SharedString = selected_model
        .as_ref()
        .map(|model| {
            let tokens = model.max_token_count();
            if tokens >= 1_000_000 && tokens % 1_000_000 == 0 {
                format!("{}M", tokens / 1_000_000)
            } else {
                format!("{}K", tokens / 1_000)
            }
            .into()
        })
        .unwrap_or_else(|| "Model default".into());
    let rows = models
        .into_iter()
        .filter(|model| {
            model.name().0.to_lowercase().contains(&query)
                || model.provider_name().0.to_lowercase().contains(&query)
        })
        .map(|model| {
            let hidden = is_hidden(&model, settings);
            let is_default = selected_model.as_ref().is_some_and(|selected| {
                selected.id() == model.id() && selected.provider_id() == model.provider_id()
            });
            let selection = model_selection(model.as_ref());
            let description = if is_default {
                "Default model"
            } else if hidden {
                "Hidden from model picker"
            } else {
                "Show in model picker"
            };
            row(
                model.name().0.clone(),
                ui::localized(description, cx),
                Switch::new(
                    format!("visible-{}-{}", model.provider_id().0, model.id().0),
                    (!hidden).into(),
                )
                .settings_style()
                .disabled(is_default)
                .on_click(move |state, _, cx| {
                    let hidden = *state != ToggleState::Selected;
                    let selection = selection.clone();
                    update_settings_file(<dyn fs::Fs>::global(cx), cx, move |settings, _| {
                        let models = &mut settings.agent.get_or_insert_default().hidden_models;
                        models.retain(|model| {
                            model.provider != selection.provider || model.model != selection.model
                        });
                        if hidden {
                            models.push(selection.clone());
                        }
                    });
                })
                .into_any_element(),
                57.,
                cx,
            )
            .into_any_element()
        })
        .collect::<Vec<_>>();
    let title = v_flex()
        .gap(px(9.))
        .child(text(
            ui::localized("AI", cx),
            26.,
            34.,
            cx.theme().colors().text,
        ))
        .child(text(
            ui::localized(
                "Choose the models in your chat picker and set your defaults.",
                cx,
            ),
            13.,
            20.,
            cx.theme().colors().text_muted,
        ));
    v_flex()
        .id("ai-settings-page")
        .flex_1()
        .min_w_0()
        .size_full()
        .bg(cx.theme().colors().background)
        .pt(px(36.))
        .pb(px(28.))
        .track_scroll(&state.scroll)
        .overflow_y_scroll()
        .child(
            v_flex()
                .w_full()
                .max_w(px(900.))
                .mx_auto()
                .gap(px(16.))
                .child(title)
                .child(text(
                    ui::localized("Defaults", cx),
                    14.,
                    20.,
                    cx.theme().colors().text,
                ))
                .child(
                    card(cx)
                        .child(row(
                            ui::localized("Default model", cx),
                            ui::localized("Used when you choose Default in the chat composer.", cx),
                            model_menu.into_any_element(),
                            65.,
                            cx,
                        ))
                        .child(row(
                            ui::localized("Reasoning", cx),
                            ui::localized("Starting effort for new conversations.", cx),
                            effort_control.into_any_element(),
                            65.,
                            cx,
                        ))
                        .child(row(
                            ui::localized("Context window", cx),
                            ui::localized("Starting context size when supported by the model.", cx),
                            dropdown("default-context", context_label, cx)
                                .disabled(true)
                                .into_any_element(),
                            65.,
                            cx,
                        )),
                )
                .child(
                    h_flex()
                        .justify_between()
                        .child(text(
                            ui::localized("Models in your picker", cx),
                            14.,
                            20.,
                            cx.theme().colors().text,
                        ))
                        .child(text(
                            format!("{enabled} enabled"),
                            11.,
                            14.,
                            cx.theme().colors().text_muted,
                        )),
                )
                .child(
                    card(cx)
                        .child(
                            h_flex()
                                .px(px(16.))
                                .py(px(12.))
                                .gap(px(10.))
                                .border_b_1()
                                .border_color(cx.theme().colors().border)
                                .child(
                                    Icon::new(IconName::MagnifyingGlass)
                                        .size(IconSize::Custom(rems_from_px(16_f32)))
                                        .color(Color::Muted),
                                )
                                .child(state.search.clone()),
                        )
                        .child(
                            v_flex()
                                .id("settings-model-list")
                                .max_h(px(285.))
                                .overflow_y_scroll()
                                .children(rows),
                        )
                        .when(available_count == 0, |this| {
                            this.child(div().p(px(16.)).child(text(
                                ui::localized("Connect a provider to see available models.", cx),
                                12.,
                                18.,
                                cx.theme().colors().text_muted,
                            )))
                        }),
                )
                .child(
                    h_flex()
                        .h(px(56.))
                        .px(px(16.))
                        .border_1()
                        .border_color(cx.theme().colors().border)
                        .rounded(px(9.))
                        .bg(cx.theme().colors().surface_background)
                        .child(
                            v_flex()
                                .flex_1()
                                .gap(px(4.))
                                .child(text(
                                    ui::localized("Model connections", cx),
                                    13.,
                                    16.,
                                    cx.theme().colors().text,
                                ))
                                .child(text(
                                    ui::localized(
                                        "Connect a provider to make its models available.",
                                        cx,
                                    ),
                                    11.,
                                    14.,
                                    cx.theme().colors().text_muted,
                                )),
                        )
                        .child(
                            ButtonLike::new("connect-model")
                                .height((px(32.)).into())
                                .corner_radius(px(6.))
                                .background((cx.theme().colors().element_hover).into())
                                .child(text(
                                    ui::localized("＋ Connect model", cx),
                                    12.,
                                    16.,
                                    cx.theme().colors().text_accent,
                                ))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.push_dynamic_sub_page(
                                        "Model connections",
                                        "AI",
                                        Some("model_connections"),
                                        false,
                                        super::render_model_connections_page,
                                        window,
                                        cx,
                                    );
                                }))
                                .custom_style({
                                    let border_selected = cx.theme().colors().border_selected;
                                    move |this| {
                                        this.px(px(11.)).border_1().border_color(border_selected)
                                    }
                                }),
                        ),
                ),
        )
        .into_any_element()
}

fn is_hidden(model: &Arc<dyn LanguageModel>, settings: &AgentSettings) -> bool {
    settings.hidden_models.iter().any(|hidden| {
        hidden.provider.0 == model.provider_id().0.as_ref() && hidden.model == model.id().0.as_ref()
    })
}

fn model_selection(model: &dyn LanguageModel) -> LanguageModelSelection {
    LanguageModelSelection {
        provider: model.provider_id().0.to_string().into(),
        model: model.id().0.to_string(),
        enable_thinking: model.supports_thinking(),
        effort: model
            .default_effort_level()
            .map(|effort| effort.value.to_string()),
        speed: None,
    }
}

pub(super) fn text(
    value: impl Into<SharedString>,
    size: f32,
    line_height: f32,
    color: impl Into<gpui::Hsla>,
) -> Div {
    div()
        .text_size(px(size))
        .line_height(px(line_height))
        .text_color(color.into())
        .child(value.into())
}

pub(super) fn card(cx: &App) -> Div {
    v_flex()
        .w_full()
        .bg(cx.theme().colors().surface_background)
        .border_1()
        .border_color(cx.theme().colors().border)
        .rounded(px(10.))
        .overflow_hidden()
}

fn row(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: AnyElement,
    height: f32,
    cx: &App,
) -> Div {
    h_flex()
        .w_full()
        .min_h(px(height))
        .px(px(16.))
        .py(px(12.))
        .gap(px(22.))
        .border_b_1()
        .border_color(cx.theme().colors().border)
        .child(
            v_flex()
                .flex_1()
                .gap(px(5.))
                .child(text(title, 13., 16., cx.theme().colors().text))
                .child(text(description, 11., 17., cx.theme().colors().text_muted)),
        )
        .child(control)
}

fn dropdown(id: &'static str, label: SharedString, cx: &App) -> ButtonLike {
    let border = cx.theme().colors().border;
    ButtonLike::new(id)
        .height((px(32.)).into())
        .corner_radius(px(6.))
        .child(
            h_flex()
                .gap(px(12.))
                .child(text(label, 12., 16., cx.theme().colors().text))
                .child(
                    Icon::new(IconName::ChevronDown)
                        .size(IconSize::Custom(rems_from_px(13_f32)))
                        .color(Color::Muted),
                ),
        )
        .custom_style(move |this| this.px(px(10.)).border_1().border_color(border))
}
