use std::rc::Rc;

use acp_thread::{AgentModelId, AgentModelInfo, AgentModelSelector};
use agent_settings::AgentSettings;
use gpui::{Entity, FocusHandle};
use picker::popover_menu::PickerPopoverMenu;
use settings::Settings as _;
use ui::{ButtonLike, PopoverMenuHandle, Tooltip, prelude::*};

use crate::ui::ModelSelectorTooltip;
use crate::{ModelSelector, model_selector::acp_model_selector};

pub struct ModelSelectorPopover {
    selector: Entity<ModelSelector>,
    menu_handle: PopoverMenuHandle<ModelSelector>,
}

impl ModelSelectorPopover {
    pub(crate) fn new(
        selector: Rc<dyn AgentModelSelector>,
        menu_handle: PopoverMenuHandle<ModelSelector>,
        focus_handle: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            selector: cx
                .new(move |cx| acp_model_selector(selector, focus_handle.clone(), window, cx)),
            menu_handle,
        }
    }

    pub fn toggle(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu_handle.toggle(window, cx);
    }

    pub fn active_model<'a>(&self, cx: &'a App) -> Option<&'a AgentModelInfo> {
        self.selector.read(cx).delegate.active_model()
    }

    pub fn cycle_favorite_models(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.selector.update(cx, |selector, cx| {
            selector.delegate.cycle_favorite_models(window, cx);
        });
    }
}

impl Render for ModelSelectorPopover {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selector = self.selector.read(cx);
        let model = selector.delegate.active_model();
        let model_name = model
            .as_ref()
            .map(|model| model.name.clone())
            .unwrap_or_else(|| SharedString::from("Select a Model"));
        let is_settings_default = match (
            model.as_ref(),
            AgentSettings::try_get(cx).and_then(|settings| settings.default_model.as_ref()),
        ) {
            (Some(model), Some(selection)) => {
                model.id
                    == AgentModelId::new(format!("{}/{}", selection.provider.0, selection.model))
            }
            _ => false,
        };
        let deployed = self.menu_handle.is_deployed();
        let label = if deployed && is_settings_default {
            SharedString::from("Default")
        } else {
            model_name
        };

        let icon = if deployed {
            IconName::ChevronUp
        } else {
            IconName::ChevronDown
        };

        let show_cycle_row = selector.delegate.favorites_count() > 1;

        let tooltip = Tooltip::element({
            move |_, _cx| {
                ModelSelectorTooltip::new()
                    .show_cycle_row(show_cycle_row)
                    .into_any_element()
            }
        });

        PickerPopoverMenu::new(
            self.selector.clone(),
            ButtonLike::new("active-model")
                .size(ButtonSize::None)
                .when(deployed, |this| {
                    this.background(cx.theme().colors().element_hover)
                        .corner_radius(px(4.))
                })
                .child(
                    h_flex()
                        .gap(px(7.))
                        .child(
                            Label::new(label)
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        )
                        .child(
                            Icon::new(icon)
                                .size(IconSize::XSmall)
                                .color(Color::Muted),
                        ),
                ),
            tooltip,
            gpui::Anchor::BottomLeft,
            cx,
        )
        .with_handle(self.menu_handle.clone())
        .render(window, cx)
    }
}
