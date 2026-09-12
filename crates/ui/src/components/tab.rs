use std::cmp::Ordering;

use gpui::{AnyElement, IntoElement, Stateful};
use smallvec::SmallVec;

use crate::prelude::*;

const START_TAB_SLOT_SIZE: Pixels = px(12.);
const END_TAB_SLOT_SIZE: Pixels = px(14.);

/// The position of a [`Tab`] within a list of tabs.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TabPosition {
    /// The tab is first in the list.
    First,

    /// The tab is in the middle of the list (i.e., it is not the first or last tab).
    ///
    /// The [`Ordering`] is where this tab is positioned with respect to the selected tab.
    Middle(Ordering),

    /// The tab is last in the list.
    Last,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TabCloseSide {
    Start,
    End,
}

#[derive(IntoElement, RegisterComponent)]
pub struct Tab {
    div: Stateful<Div>,
    selected: bool,
    position: TabPosition,
    close_side: TabCloseSide,
    document_style: bool,
    terminal_style: bool,
    start_slot: Option<AnyElement>,
    end_slot: Option<AnyElement>,
    children: SmallVec<[AnyElement; 2]>,
}

impl Tab {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            div: div()
                .id(id.clone())
                .debug_selector(|| format!("TAB-{}", id)),
            selected: false,
            position: TabPosition::First,
            close_side: TabCloseSide::End,
            document_style: false,
            terminal_style: false,
            start_slot: None,
            end_slot: None,
            children: SmallVec::new(),
        }
    }

    pub fn position(mut self, position: TabPosition) -> Self {
        self.position = position;
        self
    }

    pub fn terminal_style(mut self, enabled: bool) -> Self {
        self.terminal_style = enabled;
        self
    }

    pub fn document_style(mut self, enabled: bool) -> Self {
        self.document_style = enabled;
        self
    }

    pub fn close_side(mut self, close_side: TabCloseSide) -> Self {
        self.close_side = close_side;
        self
    }

    pub fn start_slot<E: IntoElement>(mut self, element: impl Into<Option<E>>) -> Self {
        self.start_slot = element.into().map(IntoElement::into_any_element);
        self
    }

    pub fn end_slot<E: IntoElement>(mut self, element: impl Into<Option<E>>) -> Self {
        self.end_slot = element.into().map(IntoElement::into_any_element);
        self
    }

    pub fn content_height(cx: &App) -> Pixels {
        DynamicSpacing::Base32.px(cx) - px(1.)
    }

    pub fn container_height(cx: &App) -> Pixels {
        DynamicSpacing::Base32.px(cx)
    }
}

impl InteractiveElement for Tab {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.div.interactivity()
    }
}

impl StatefulInteractiveElement for Tab {}

impl Toggleable for Tab {
    fn toggle_state(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl ParentElement for Tab {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Tab {
    #[allow(refining_impl_trait)]
    fn render(self, _: &mut Window, cx: &mut App) -> Stateful<Div> {
        let (text_color, tab_bg, _tab_hover_bg, _tab_active_bg) = match self.selected {
            false => (
                cx.theme().colors().text_muted,
                cx.theme().colors().tab_inactive_background,
                cx.theme().colors().ghost_element_hover,
                cx.theme().colors().ghost_element_active,
            ),
            true => (
                cx.theme().colors().text,
                cx.theme().colors().tab_active_background,
                cx.theme().colors().element_hover,
                cx.theme().colors().element_active,
            ),
        };

        let (start_slot, end_slot) = {
            let start_slot = (!self.document_style || self.start_slot.is_some()).then(|| {
                h_flex()
                    .size(START_TAB_SLOT_SIZE)
                    .justify_center()
                    .children(self.start_slot)
            });
            let end_slot = (!self.document_style || self.end_slot.is_some()).then(|| {
                h_flex()
                    .size(END_TAB_SLOT_SIZE)
                    .when(self.document_style, |this| this.size(px(10.)))
                    .justify_center()
                    .children(self.end_slot)
            });
            match self.close_side {
                TabCloseSide::End => (start_slot, end_slot),
                TabCloseSide::Start => (end_slot, start_slot),
            }
        };

        self.div
            .h(Tab::container_height(cx))
            .bg(tab_bg)
            .border_color(cx.theme().colors().border)
            .map(|this| {
                if self.terminal_style {
                    this.h(px(28.))
                        .rounded(px(5.))
                        .border_0()
                        .bg(if self.selected {
                            cx.theme().colors().tab_active_background
                        } else {
                            gpui::transparent_black()
                        })
                } else if self.document_style {
                    this.h(px(48.))
                        .border_b_2()
                        .when(self.selected, |this| {
                            this.w(px(174.))
                                .min_w(px(174.))
                                .max_w(px(174.))
                                .bg(cx.theme().colors().tab_active_background)
                                .border_color(cx.theme().colors().border_focused)
                        })
                        .when(!self.selected, |this| {
                            this.border_color(gpui::transparent_black())
                        })
                } else {
                    match self.position {
                        TabPosition::First => {
                            if self.selected {
                                this.pl_px().border_r_1().pb_px()
                            } else {
                                this.pl_px().pr_px().border_b_1()
                            }
                        }
                        TabPosition::Last => {
                            if self.selected {
                                this.border_l_1().border_r_1().pb_px()
                            } else {
                                this.pl_px().border_b_1().border_r_1()
                            }
                        }
                        TabPosition::Middle(Ordering::Equal) => {
                            this.border_l_1().border_r_1().pb_px()
                        }
                        TabPosition::Middle(Ordering::Less) => {
                            this.border_l_1().pr_px().border_b_1()
                        }
                        TabPosition::Middle(Ordering::Greater) => {
                            this.border_r_1().pl_px().border_b_1()
                        }
                    }
                }
            })
            .cursor_pointer()
            .child(
                h_flex()
                    .group("")
                    .relative()
                    .h(Tab::content_height(cx))
                    .px(DynamicSpacing::Base04.px(cx))
                    .gap(DynamicSpacing::Base04.rems(cx))
                    .when(self.document_style, |this| {
                        this.h(px(46.)).px(px(18.)).gap(px(10.)).min_w_0().w_full()
                    })
                    .when(self.terminal_style, |this| {
                        this.h(px(28.)).px(px(10.)).gap(px(8.))
                    })
                    .text_color(text_color)
                    .children(start_slot)
                    .map(|this| {
                        if self.document_style {
                            this.child(h_flex().flex_1().min_w_0().children(self.children))
                        } else {
                            this.children(self.children)
                        }
                    })
                    .children(end_slot),
            )
    }
}

impl Component for Tab {
    fn scope() -> ComponentScope {
        ComponentScope::Navigation
    }

    fn description() -> &'static str {
        "A tab component that can be used in a tabbed interface, \
        supporting different positions and states."
    }

    fn preview(_window: &mut Window, _cx: &mut App) -> AnyElement {
        v_flex()
            .gap_6()
            .children(vec![example_group_with_title(
                "Variations",
                vec![
                    single_example(
                        "Default",
                        Tab::new("default").child("Default Tab").into_any_element(),
                    ),
                    single_example(
                        "Selected",
                        Tab::new("selected")
                            .toggle_state(true)
                            .child("Selected Tab")
                            .into_any_element(),
                    ),
                    single_example(
                        "First",
                        Tab::new("first")
                            .position(TabPosition::First)
                            .child("First Tab")
                            .into_any_element(),
                    ),
                    single_example(
                        "Middle",
                        Tab::new("middle")
                            .position(TabPosition::Middle(Ordering::Equal))
                            .child("Middle Tab")
                            .into_any_element(),
                    ),
                    single_example(
                        "Last",
                        Tab::new("last")
                            .position(TabPosition::Last)
                            .child("Last Tab")
                            .into_any_element(),
                    ),
                ],
            )])
            .into_any_element()
    }
}
