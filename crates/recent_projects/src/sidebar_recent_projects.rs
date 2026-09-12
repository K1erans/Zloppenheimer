use std::sync::Arc;

use fuzzy_nucleo::{StringMatch, StringMatchCandidate, match_strings};
use gpui::{
    Action, AnyElement, App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable,
    Subscription, Task, TaskExt, WeakEntity, Window,
};
use picker::{Picker, PickerDelegate};
use remote::RemoteConnectionOptions;
use settings::Settings;
use ui::{ButtonLike, ListItem, ListItemSpacing, Tooltip, prelude::*};
use util::{ResultExt, paths::PathExt};
use workspace::{
    MultiWorkspace, OpenMode, OpenOptions, ProjectGroupKey, RecentWorkspace,
    SerializedWorkspaceLocation, Workspace, WorkspaceDb, notifications::DetachAndPromptErr,
};

use crate::{icon_for_remote_connection, open_remote_project};

pub struct SidebarRecentProjects {
    pub picker: Entity<Picker<SidebarRecentProjectsDelegate>>,
    _subscription: Subscription,
}

impl SidebarRecentProjects {
    pub fn popover(
        workspace: WeakEntity<Workspace>,
        window_project_groups: Vec<ProjectGroupKey>,
        _focus_handle: FocusHandle,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let fs = workspace
            .upgrade()
            .map(|ws| ws.read(cx).app_state().fs.clone());

        cx.new(|cx| {
            let delegate = SidebarRecentProjectsDelegate {
                workspace,
                window_project_groups,
                workspaces: Vec::new(),
                filtered_workspaces: Vec::new(),
                selected_index: 0,
                focus_handle: cx.focus_handle(),
            };

            let picker: Entity<Picker<SidebarRecentProjectsDelegate>> = cx.new(|cx| {
                Picker::list(delegate, window, cx)
                    .list_measure_all()
                    .show_scrollbar(true)
                    .initial_width(rems(350. / 16.))
                    .popover()
            });

            let picker_focus_handle = picker.focus_handle(cx);
            picker.update(cx, |picker, _| {
                picker.delegate.focus_handle = picker_focus_handle;
            });

            let _subscription =
                cx.subscribe(&picker, |_this: &mut Self, _, _, cx| cx.emit(DismissEvent));

            let db = WorkspaceDb::global(cx);
            cx.spawn_in(window, async move |this, cx| {
                let Some(fs) = fs else { return };
                let workspaces = db
                    .recent_project_workspaces(fs.as_ref())
                    .await
                    .log_err()
                    .unwrap_or_default();
                this.update_in(cx, move |this, window, cx| {
                    this.picker.update(cx, move |picker, cx| {
                        picker.delegate.set_workspaces(workspaces);
                        picker.update_matches(picker.query(cx), window, cx)
                    })
                })
                .ok();
            })
            .detach();

            picker.focus_handle(cx).focus(window, cx);

            Self {
                picker,
                _subscription,
            }
        })
    }
}

impl EventEmitter<DismissEvent> for SidebarRecentProjects {}

impl Focusable for SidebarRecentProjects {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.picker.focus_handle(cx)
    }
}

impl Render for SidebarRecentProjects {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("SidebarRecentProjects")
            .w(px(350.))
            .child(self.picker.clone())
    }
}

pub struct SidebarRecentProjectsDelegate {
    workspace: WeakEntity<Workspace>,
    window_project_groups: Vec<ProjectGroupKey>,
    workspaces: Vec<RecentWorkspace>,
    filtered_workspaces: Vec<StringMatch>,
    selected_index: usize,
    focus_handle: FocusHandle,
}

impl SidebarRecentProjectsDelegate {
    pub fn set_workspaces(&mut self, workspaces: Vec<RecentWorkspace>) {
        self.workspaces = workspaces;
    }
}

impl EventEmitter<DismissEvent> for SidebarRecentProjectsDelegate {}

impl PickerDelegate for SidebarRecentProjectsDelegate {
    type ListItem = AnyElement;

    fn name() -> &'static str {
        "sidebar recent projects"
    }

    fn placeholder_text(&self, _window: &mut Window, cx: &mut App) -> Arc<str> {
        ui::localized("Search projects…", cx).to_string().into()
    }

    fn dropdown_style(&self) -> bool {
        true
    }

    fn project_switcher_style(&self) -> bool {
        true
    }

    fn render_editor(
        &self,
        editor: &Arc<dyn ui_input::ErasedEditor>,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<Div> {
        Some(
            h_flex()
                .h(px(48.))
                .px(px(14.))
                .gap(px(10.))
                .flex_none()
                .border_b_1()
                .border_color(cx.theme().colors().border)
                .child(
                    Icon::new(IconName::MagnifyingGlass)
                        .size(IconSize::Custom(rems(17. / 16.)))
                        .color(Color::Muted),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(14.))
                        .line_height(px(18.))
                        .text_color(cx.theme().colors().text_muted)
                        .child(editor.render(window, cx)),
                )
                .child(
                    div()
                        .border_1()
                        .border_color(cx.theme().colors().border)
                        .rounded(px(4.))
                        .px(px(4.))
                        .py(px(2.))
                        .text_size(px(10.))
                        .line_height(px(12.))
                        .text_color(cx.theme().colors().text_muted)
                        .child("Esc"),
                ),
        )
    }

    fn match_count(&self) -> usize {
        self.filtered_workspaces.len()
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn set_selected_index(
        &mut self,
        ix: usize,
        _window: &mut Window,
        _cx: &mut Context<Picker<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn update_matches(
        &mut self,
        query: String,
        _: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Task<()> {
        let query = query.trim_start();
        let case = fuzzy_nucleo::Case::smart_if_uppercase_in(query);
        let is_empty_query = query.is_empty();

        let current_workspace_id = self
            .workspace
            .upgrade()
            .and_then(|ws| ws.read(cx).database_id());

        let current_project_group = self
            .workspace
            .upgrade()
            .map(|workspace| workspace.read(cx).project_group_key(cx));

        let candidates: Vec<_> = self
            .workspaces
            .iter()
            .enumerate()
            .map(|(id, workspace)| {
                let combined_string = workspace
                    .identity_paths
                    .ordered_paths()
                    .map(|path| path.compact().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .concat();
                StringMatchCandidate::new(id, &combined_string)
            })
            .collect();

        if is_empty_query {
            self.filtered_workspaces = candidates
                .into_iter()
                .map(|candidate| StringMatch {
                    candidate_id: candidate.id,
                    score: 0.0,
                    positions: Vec::new(),
                    string: candidate.string,
                })
                .collect();
        } else {
            self.filtered_workspaces = match_strings(
                &candidates,
                query,
                case,
                fuzzy_nucleo::LengthPenalty::On,
                100,
            );
        }

        self.filtered_workspaces.sort_by_key(|hit| {
            self.workspaces
                .get(hit.candidate_id)
                .map(|workspace| {
                    if Some(workspace.workspace_id) == current_workspace_id
                        || current_project_group
                            .as_ref()
                            .is_some_and(|key| key.matches(&workspace.project_group_key()))
                    {
                        0
                    } else if self
                        .window_project_groups
                        .iter()
                        .any(|key| key.matches(&workspace.project_group_key()))
                    {
                        1
                    } else {
                        2
                    }
                })
                .unwrap_or(2)
        });
        self.selected_index = 0;
        Task::ready(())
    }

    fn confirm(&mut self, _secondary: bool, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        let Some(hit) = self.filtered_workspaces.get(self.selected_index) else {
            return;
        };
        let Some(recent_workspace) = self.workspaces.get(hit.candidate_id) else {
            return;
        };

        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        if workspace
            .read(cx)
            .project_group_key(cx)
            .matches(&recent_workspace.project_group_key())
        {
            cx.emit(DismissEvent);
            return;
        }

        match &recent_workspace.location {
            SerializedWorkspaceLocation::Local => {
                if let Some(handle) = window.window_handle().downcast::<MultiWorkspace>() {
                    let paths = recent_workspace.paths.paths().to_vec();
                    cx.defer(move |cx| {
                        if let Some(task) = handle
                            .update(cx, |multi_workspace, window, cx| {
                                multi_workspace.open_project(paths, OpenMode::Activate, window, cx)
                            })
                            .log_err()
                        {
                            task.detach_and_log_err(cx);
                        }
                    });
                }
            }
            SerializedWorkspaceLocation::Remote(connection) => {
                let mut connection = connection.clone();
                workspace.update(cx, |workspace, cx| {
                    let app_state = workspace.app_state().clone();
                    let replace_window = window.window_handle().downcast::<MultiWorkspace>();
                    let open_options = OpenOptions {
                        requesting_window: replace_window,
                        ..Default::default()
                    };
                    if let RemoteConnectionOptions::Ssh(connection) = &mut connection {
                        crate::RemoteSettings::get_global(cx)
                            .fill_connection_options_from_settings(connection);
                    };
                    let paths = recent_workspace.paths.paths().to_vec();
                    cx.spawn_in(window, async move |_, cx| {
                        open_remote_project(connection.clone(), paths, app_state, open_options, cx)
                            .await
                    })
                    .detach_and_prompt_err(
                        "Failed to open project",
                        window,
                        cx,
                        |_, _, _| None,
                    );
                });
            }
        }
        cx.emit(DismissEvent);
    }

    fn dismissed(&mut self, _window: &mut Window, _cx: &mut Context<Picker<Self>>) {}

    fn no_matches_text(&self, _window: &mut Window, _cx: &mut App) -> Option<SharedString> {
        let text = if self.workspaces.is_empty() {
            "Recently opened projects will show up here"
        } else {
            "No matches"
        };
        Some(text.into())
    }

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        _window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let hit = self.filtered_workspaces.get(ix)?;
        let workspace = self.workspaces.get(hit.candidate_id)?;

        let ordered_paths: Vec<_> = workspace
            .identity_paths
            .ordered_paths()
            .map(|p| p.compact().to_string_lossy().to_string())
            .collect();

        let tooltip_path: SharedString = match &workspace.location {
            SerializedWorkspaceLocation::Remote(options) => {
                let host = options.display_name();
                if ordered_paths.len() == 1 {
                    format!("{} ({})", ordered_paths[0], host).into()
                } else {
                    format!("{}\n({})", ordered_paths.join("\n"), host).into()
                }
            }
            _ => ordered_paths.join("\n").into(),
        };

        let in_this_window = self
            .window_project_groups
            .iter()
            .any(|key| key.matches(&workspace.project_group_key()));
        let current_workspace = self
            .workspace
            .upgrade()
            .and_then(|workspace| workspace.read(cx).database_id());
        let is_current = Some(workspace.workspace_id) == current_workspace
            || self.workspace.upgrade().is_some_and(|current| {
                current
                    .read(cx)
                    .project_group_key(cx)
                    .matches(&workspace.project_group_key())
            });
        let in_this_window = in_this_window || is_current;
        let previous_in_window = ix
            .checked_sub(1)
            .and_then(|previous| self.filtered_workspaces.get(previous))
            .and_then(|hit| self.workspaces.get(hit.candidate_id))
            .is_some_and(|workspace| {
                Some(workspace.workspace_id) == current_workspace
                    || self
                        .window_project_groups
                        .iter()
                        .any(|key| key.matches(&workspace.project_group_key()))
            });
        let heading = if ix == 0 || previous_in_window != in_this_window {
            Some(if in_this_window {
                "THIS WINDOW"
            } else {
                "RECENT PROJECTS"
            })
        } else {
            None
        };
        let name = workspace::welcome::project_name(&workspace.identity_paths);

        let icon = icon_for_remote_connection(match &workspace.location {
            SerializedWorkspaceLocation::Local => None,
            SerializedWorkspaceLocation::Remote(options) => Some(options),
        });

        Some(
            v_flex()
                .w_full()
                .when_some(heading, |this, heading| {
                    this.child(
                        div()
                            .pt(px(if ix == 0 { 14. } else { 18. }))
                            .pb(px(8.))
                            .px(px(14.))
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .text_color(cx.theme().colors().text_muted)
                            .child(ui::localized(heading, cx)),
                    )
                })
                .child(
                    div().px(px(if in_this_window { 8. } else { 0. })).child(
                        ListItem::new(ix)
                            .height(px(if in_this_window { 38. } else { 36. }))
                            .horizontal_padding(px(if in_this_window { 10. } else { 18. }))
                            .corner_radius(px(6.))
                            .toggle_state(selected)
                            .selected_background(cx.theme().colors().element_active)
                            .spacing(ListItemSpacing::Dense)
                            .child(
                                h_flex()
                                    .w_full()
                                    .min_w_0()
                                    .gap(px(10.))
                                    .child(
                                        Icon::new(icon)
                                            .size(IconSize::Custom(rems(17. / 16.)))
                                            .color(if selected {
                                                Color::Accent
                                            } else {
                                                Color::Muted
                                            }),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .text_size(px(if in_this_window { 14. } else { 13. }))
                                            .line_height(px(if in_this_window { 18. } else { 16. }))
                                            .text_color(cx.theme().colors().text)
                                            .text_ellipsis()
                                            .child(name),
                                    )
                                    .when(is_current, |this| {
                                        this.child(
                                            div()
                                                .text_size(px(14.))
                                                .line_height(px(18.))
                                                .text_color(cx.theme().colors().text_accent)
                                                .child("✓"),
                                        )
                                    }),
                            )
                            .tooltip(move |_, cx| {
                                Tooltip::with_meta(
                                    "Open Project in This Window",
                                    None,
                                    tooltip_path.clone(),
                                    cx,
                                )
                            }),
                    ),
                )
                .into_any_element(),
        )
    }

    fn render_footer(&self, _: &mut Window, cx: &mut Context<Picker<Self>>) -> Option<AnyElement> {
        let actions = [
            (
                "open_local_folder",
                "Local folder",
                "Open a folder or existing Git checkout",
                IconName::FolderAdd,
                "workspace::Open",
            ),
            (
                "clone_git_url",
                "Git URL",
                "Clone a repository from a remote URL",
                IconName::Link,
                "git::Clone",
            ),
            (
                "clone_github",
                "GitHub repository",
                "Find and clone a GitHub repository",
                IconName::Github,
                "git::CloneGitHub",
            ),
        ];
        Some(
            v_flex()
                .pb(px(8.))
                .border_t_1()
                .border_color(cx.theme().colors().border)
                .child(
                    div()
                        .pt(px(14.))
                        .pb(px(8.))
                        .px(px(14.))
                        .text_size(px(11.))
                        .line_height(px(16.))
                        .text_color(cx.theme().colors().text_muted)
                        .child(ui::localized("ADD PROJECT", cx)),
                )
                .children(actions.map(|(id, label, description, icon, action_name)| {
                    ButtonLike::new(id)
                        .full_width()
                        .height(px(60.).into())
                        .size(ButtonSize::None)
                        .style(ButtonStyle::Transparent)
                        .child(
                            h_flex()
                                .w_full()
                                .px(px(16.))
                                .gap(px(12.))
                                .child(h_flex().w(px(22.)).flex_none().justify_center().child(
                                    Icon::new(icon).size(IconSize::Medium).color(Color::Accent),
                                ))
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap(px(4.))
                                        .text_left()
                                        .child(
                                            div()
                                                .text_size(px(14.))
                                                .line_height(px(20.))
                                                .text_color(cx.theme().colors().text)
                                                .child(ui::localized(label, cx)),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(11.))
                                                .line_height(px(17.))
                                                .text_color(cx.theme().colors().text_muted)
                                                .child(ui::localized(description, cx)),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(15.))
                                        .line_height(px(18.))
                                        .text_color(cx.theme().colors().text_muted)
                                        .child("›"),
                                ),
                        )
                        .on_click(cx.listener(move |_, _, window, cx| {
                            let action = if action_name == "workspace::Open" {
                                Some(
                                    workspace::Open {
                                        create_new_window: Some(false),
                                    }
                                    .boxed_clone(),
                                )
                            } else {
                                cx.build_action(action_name, None).log_err()
                            };
                            if let Some(action) = action {
                                window.dispatch_action(action, cx);
                                cx.emit(DismissEvent);
                            }
                        }))
                }))
                .into_any_element(),
        )
    }
}
