use gpui::{Action, ClickEvent, FocusHandle, prelude::*};
use language_model::DisabledReason;
use ui::{Chip, ElevationIndex, KeyBinding, ListItem, ListItemSpacing, Tooltip, prelude::*};
use zed_actions::agent::ToggleModelSelector;

use crate::CycleFavoriteModels;

enum ModelIcon {
    Name(IconName),
    Path(SharedString),
}

#[derive(IntoElement)]
pub struct ModelSelectorHeader {
    title: SharedString,
    has_border: bool,
}

impl ModelSelectorHeader {
    pub fn new(title: impl Into<SharedString>, has_border: bool) -> Self {
        Self {
            title: title.into(),
            has_border,
        }
    }
}

impl RenderOnce for ModelSelectorHeader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .px_2()
            .pb_1()
            .when(self.has_border, |this| {
                this.mt_1()
                    .pt_2()
                    .border_t_1()
                    .border_color(cx.theme().colors().border_variant)
            })
            .child(
                Label::new(self.title)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
    }
}

#[derive(IntoElement)]
pub struct ModelSelectorListItem {
    index: usize,
    title: SharedString,
    subtitle: Option<SharedString>,
    icon: Option<ModelIcon>,
    is_selected: bool,
    is_focused: bool,
    is_latest: bool,
    is_favorite: bool,
    disabled: Option<DisabledReason>,
    on_toggle_favorite: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    cost_info: Option<SharedString>,
}

impl ModelSelectorListItem {
    pub fn new(index: usize, title: impl Into<SharedString>) -> Self {
        Self {
            index,
            title: title.into(),
            subtitle: None,
            icon: None,
            is_selected: false,
            is_focused: false,
            is_latest: false,
            is_favorite: false,
            disabled: None,
            on_toggle_favorite: None,
            cost_info: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(ModelIcon::Name(icon));
        self
    }

    pub fn icon_path(mut self, path: SharedString) -> Self {
        self.icon = Some(ModelIcon::Path(path));
        self
    }

    pub fn is_selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
        self
    }

    pub fn disabled(mut self, disabled: Option<DisabledReason>) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn is_focused(mut self, is_focused: bool) -> Self {
        self.is_focused = is_focused;
        self
    }

    pub fn is_latest(mut self, is_latest: bool) -> Self {
        self.is_latest = is_latest;
        self
    }

    pub fn is_favorite(mut self, is_favorite: bool) -> Self {
        self.is_favorite = is_favorite;
        self
    }

    pub fn on_toggle_favorite(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_favorite = Some(Box::new(handler));
        self
    }

    pub fn cost_info(mut self, cost_info: Option<SharedString>) -> Self {
        self.cost_info = cost_info;
        self
    }
}

impl RenderOnce for ModelSelectorListItem {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_disabled = self.disabled.is_some();

        let model_icon_color = if self.is_selected {
            Color::Accent
        } else if is_disabled {
            Color::Disabled
        } else {
            Color::Muted
        };

        let is_favorite = self.is_favorite;

        ListItem::new(self.index)
            .inset(false)
            .when(self.subtitle.is_none(), |this| this.height(px(36.)))
            .horizontal_padding(px(12.))
            .corner_radius(px(6.))
            .selected_background(cx.theme().colors().element_hover)
            .spacing(ListItemSpacing::Sparse)
            .toggle_state(self.is_focused || self.is_selected)
            .when_some(self.disabled, |this, disabled_reason| {
                this.disabled(true)
                    .tooltip(Tooltip::text(disabled_reason.0))
            })
            .child(
                h_flex()
                    .w_full()
                    .gap(px(10.))
                    .when_some(self.icon, |this, icon| {
                        this.child(
                            match icon {
                                ModelIcon::Name(icon_name) => Icon::new(icon_name),
                                ModelIcon::Path(icon_path) => Icon::from_external_svg(icon_path),
                            }
                            .color(model_icon_color)
                            .size(IconSize::Small),
                        )
                    })
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(4.))
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .line_height(px(18.))
                                    .text_color(if is_disabled {
                                        Color::Disabled.color(cx)
                                    } else {
                                        cx.theme().colors().text
                                    })
                                    .truncate()
                                    .child(self.title),
                            )
                            .when_some(self.subtitle, |this, subtitle| {
                                this.child(
                                    div()
                                        .text_size(px(11.))
                                        .line_height(px(16.))
                                        .text_color(cx.theme().colors().text_muted)
                                        .child(subtitle),
                                )
                            }),
                    )
                    .when(self.is_latest, |parent| parent.child(Chip::new("Latest")))
                    .when_some(self.cost_info, |this, cost_info| {
                        let tooltip_text = if cost_info.ends_with('×') {
                            format!("Cost Multiplier: {}", cost_info)
                        } else if cost_info.contains('$') {
                            format!("Cost per Million Tokens: {}", cost_info)
                        } else {
                            format!("Cost: {}", cost_info)
                        };

                        this.child(Chip::new(cost_info).tooltip(Tooltip::text(tooltip_text)))
                    }),
            )
            .end_slot(
                h_flex()
                    .gap_1p5()
                    .when(self.is_selected, |this| {
                        this.child(
                            Icon::new(IconName::Check)
                                .size(IconSize::Custom(rems_from_px(16_f32)))
                                .color(Color::Accent),
                        )
                    })
                    .when(is_disabled, |this| {
                        this.child(Icon::new(IconName::Info).color(Color::Muted))
                    }),
            )
            .when(!is_disabled, |this| {
                this.end_slot_on_hover(div().pr_1p5().when_some(self.on_toggle_favorite, {
                    |this, handle_click| {
                        let (icon, color, tooltip) = if is_favorite {
                            (IconName::StarFilled, Color::Accent, "Unfavorite Model")
                        } else {
                            (IconName::Star, Color::Default, "Favorite Model")
                        };
                        this.child(
                            IconButton::new(("toggle-favorite", self.index), icon)
                                .layer(ElevationIndex::ElevatedSurface)
                                .icon_color(color)
                                .icon_size(IconSize::Small)
                                .tooltip(Tooltip::text(tooltip))
                                .on_click(move |event, window, cx| {
                                    (handle_click)(event, window, cx)
                                }),
                        )
                    }
                }))
            })
    }
}

#[derive(IntoElement)]
pub struct ModelSelectorFooter {
    action: Box<dyn Action>,
    focus_handle: FocusHandle,
}

impl ModelSelectorFooter {
    pub fn new(action: Box<dyn Action>, focus_handle: FocusHandle) -> Self {
        Self {
            action,
            focus_handle,
        }
    }
}

impl RenderOnce for ModelSelectorFooter {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let action = self.action;
        let focus_handle = self.focus_handle;

        v_flex()
            .w_full()
            .border_t_1()
            .border_color(cx.theme().colors().border)
            .child(
                v_flex()
                    .id("manage-models")
                    .w_full()
                    .px(px(12.))
                    .py(px(8.))
                    .gap(px(4.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|this| this.bg(cx.theme().colors().element_hover))
                    .tooltip({
                        let action = action.boxed_clone();
                        move |_, cx| {
                            Tooltip::for_action_in(
                                "Manage models",
                                action.as_ref(),
                                &focus_handle,
                                cx,
                            )
                        }
                    })
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .child("Manage models…"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .text_color(cx.theme().colors().text_muted)
                            .child("Choose visible models and your default"),
                    )
                    .on_click(move |_, window, cx| {
                        window.dispatch_action(action.boxed_clone(), cx);
                    }),
            )
    }
}

#[derive(IntoElement)]
pub struct ModelSelectorTooltip {
    show_cycle_row: bool,
}

impl ModelSelectorTooltip {
    pub fn new() -> Self {
        Self {
            show_cycle_row: true,
        }
    }

    pub fn show_cycle_row(mut self, show: bool) -> Self {
        self.show_cycle_row = show;
        self
    }
}

impl RenderOnce for ModelSelectorTooltip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                h_flex()
                    .gap_2()
                    .justify_between()
                    .child(Label::new("Change Model"))
                    .child(KeyBinding::for_action(&ToggleModelSelector, cx)),
            )
            .when(self.show_cycle_row, |this| {
                this.child(
                    h_flex()
                        .pt_1()
                        .gap_2()
                        .border_t_1()
                        .border_color(cx.theme().colors().border_variant)
                        .justify_between()
                        .child(Label::new("Cycle Favorite Models"))
                        .child(KeyBinding::for_action(&CycleFavoriteModels, cx)),
                )
            })
    }
}
