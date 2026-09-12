use crate::{
    NewFile, Open, OpenMode, PathList, RecentWorkspace, SerializedWorkspaceLocation,
    ToggleWorkspaceSidebar, Workspace, WorkspaceSettings,
    item::{Item, ItemEvent},
    persistence::WorkspaceDb,
};
use agent_settings::AgentSettings;
use git::Clone as GitClone;
use gpui::{
    Action, App, ClickEvent, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight,
    InteractiveElement, ParentElement, Render, Styled, Task, TaskExt, WeakEntity, Window, actions,
    div, px, rgb,
};
use menu::{SelectNext, SelectPrevious};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings::{DefaultOpenBehavior, Settings};
use ui::{IconButtonShape, Tab, TabBar, Tooltip, prelude::*};
use util::ResultExt;
use zed_actions::assistant::ToggleFocus;

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize, JsonSchema, Action)]
#[action(namespace = welcome)]
#[serde(transparent)]
pub struct OpenRecentProject {
    pub index: usize,
}

actions!(
    zed,
    [
        /// Show the Zed welcome screen
        ShowWelcome
    ]
);

/// Width of the prompt card and the recent project list beneath it, chosen to
/// match the reading measure of the headline above them.
const CONTENT_WIDTH: f32 = 820.;

// The logo is brand artwork, so its colors are fixed rather than themed.
const LOGO_BG: u32 = 0x0f1218;
const LOGO_EDGE: u32 = 0x242a36;
const LOGO_WHITE: u32 = 0xeef2f7;
const LOGO_PURPLE: u32 = 0xa181d1;
const LOGO_BLUE: u32 = 0x3d84ed;
const LOGO_GREEN: u32 = 0x7cae5f;

/// The Zloppenheimer logo, drawn as a vector so it stays crisp at any size:
/// four staggered rounded bars (off-white, purple, blue, green) on a rounded
/// tile. Proportions are expressed as fractions of `size` to match the brand
/// artwork.
fn logo_tile(size: f32) -> impl IntoElement {
    let bar = |width_fraction: f32, color: u32, align_end: bool| {
        div()
            .flex()
            .w_full()
            .h(px(size * 0.061))
            .when(align_end, |row| row.justify_end())
            .child(
                div()
                    .w(px(size * width_fraction))
                    .h_full()
                    .rounded_full()
                    .bg(rgb(color)),
            )
    };

    div()
        .flex_none()
        .w(px(size))
        .h(px(size))
        .rounded(px(size * 0.215))
        .bg(rgb(LOGO_BG))
        .border_1()
        .border_color(rgb(LOGO_EDGE))
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(size * 0.052))
        .pl(px(size * 0.205))
        .pr(px(size * 0.21))
        .child(bar(0.585, LOGO_WHITE, false))
        .child(bar(0.435, LOGO_PURPLE, true))
        .child(bar(0.283, LOGO_BLUE, false))
        .child(bar(0.585, LOGO_GREEN, false))
}

pub struct WelcomePage {
    workspace: WeakEntity<Workspace>,
    focus_handle: FocusHandle,
    fallback_to_recent_projects: bool,
    embed_tab_strip: bool,
    recent_workspaces: Option<Vec<RecentWorkspace>>,
}

impl WelcomePage {
    pub fn new(
        workspace: WeakEntity<Workspace>,
        fallback_to_recent_projects: bool,
        embed_tab_strip: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        cx.on_focus(&focus_handle, window, |_, _, cx| cx.notify())
            .detach();

        if fallback_to_recent_projects {
            let fs = workspace
                .upgrade()
                .map(|ws| ws.read(cx).app_state().fs.clone());
            let db = WorkspaceDb::global(cx);
            cx.spawn_in(window, async move |this: WeakEntity<Self>, cx| {
                let Some(fs) = fs else { return };
                let workspaces = db
                    .recent_project_workspaces(fs.as_ref())
                    .await
                    .log_err()
                    .unwrap_or_default();

                this.update(cx, |this, cx| {
                    this.recent_workspaces = Some(workspaces);
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }

        WelcomePage {
            workspace,
            focus_handle,
            fallback_to_recent_projects,
            embed_tab_strip,
            recent_workspaces: None,
        }
    }

    fn select_next(&mut self, _: &SelectNext, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_next(cx);
        cx.notify();
    }

    fn select_previous(&mut self, _: &SelectPrevious, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_prev(cx);
        cx.notify();
    }

    fn open_recent_project(
        &mut self,
        action: &OpenRecentProject,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(recent_workspaces) = &self.recent_workspaces {
            if let Some(workspace) = recent_workspaces.get(action.index) {
                let is_local = matches!(workspace.location, SerializedWorkspaceLocation::Local);

                if is_local {
                    let paths = workspace.paths.paths().to_vec();
                    let open_mode = match WorkspaceSettings::get_global(cx).default_open_behavior {
                        DefaultOpenBehavior::ExistingWindow => OpenMode::Activate,
                        DefaultOpenBehavior::NewWindow => OpenMode::NewWindow,
                    };
                    self.workspace
                        .update(cx, |workspace, cx| {
                            workspace
                                .open_workspace_for_paths(open_mode, paths, window, cx)
                                .detach_and_log_err(cx);
                        })
                        .log_err();
                } else {
                    use zed_actions::OpenRecent;
                    window.dispatch_action(OpenRecent::default().boxed_clone(), cx);
                }
            }
        }
    }

    /// The project name shown in the headline, taken from the first visible
    /// worktree. Empty windows have no worktrees, so fall back to the product
    /// name.
    fn project_display_name(&self, cx: &App) -> SharedString {
        self.workspace
            .upgrade()
            .and_then(|workspace| {
                workspace
                    .read(cx)
                    .project()
                    .read(cx)
                    .visible_worktrees(cx)
                    .next()
                    .map(|worktree| {
                        SharedString::from(worktree.read(cx).root_name_str().to_string())
                    })
            })
            .unwrap_or_else(|| "Zloppenheimer".into())
    }

    /// Hands focus to the agent panel, where the real thread controls live.
    fn focus_agent_panel(&self) -> impl Fn(&ClickEvent, &mut Window, &mut App) + use<> {
        let focus_handle = self.focus_handle.clone();
        move |_, window, cx| {
            focus_handle.dispatch_action(&ToggleFocus, window, cx);
        }
    }

    /// The prompt card is only an entry point into the agent panel: the model,
    /// thinking-budget and permission controls it mirrors belong to the panel
    /// itself, so the card deliberately shows no stand-ins for them and instead
    /// hands focus over when any part of it is clicked.
    fn render_home_tab_strip(&self, _cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let focus_handle = self.focus_handle.clone();
        TabBar::new("welcome-tab-bar")
            .document_style(true)
            .child(
                Tab::new("welcome-new-thread")
                    .document_style(true)
                    .toggle_state(true)
                    .child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .gap(px(10.))
                            .child(
                                Icon::new(IconName::Thread)
                                    .size(IconSize::Small)
                                    .color(Color::Muted),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(13.))
                                    .line_height(px(16.))
                                    .text_ellipsis()
                                    .child("New thread"),
                            )
                            .child(
                                Icon::new(IconName::Close)
                                    .size(IconSize::Indicator)
                                    .color(Color::Muted),
                            ),
                    ),
            )
            .child(
                IconButton::new("welcome-tab-new", IconName::Plus)
                    .width(px(36.))
                    .height(px(36.).into())
                    .corner_radius(px(8.))
                    .icon_size(IconSize::Custom(rems_from_px(18_f32)))
                    .tooltip(Tooltip::text("New…"))
                    .on_click({
                        let focus_handle = focus_handle.clone();
                        move |_, window, cx| {
                            focus_handle.dispatch_action(&NewFile, window, cx);
                        }
                    }),
            )
    }

    fn render_prompt_panel(
        &self,
        tab_index: isize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let background = cx.theme().colors().elevated_surface_background;
        let surface = cx.theme().colors().surface_background;
        let border = cx.theme().colors().border_selected;
        let border_hovered = cx.theme().colors().border_focused;
        let send_background = cx.theme().colors().border_focused;
        let send_icon = cx.theme().colors().editor_background;

        v_flex()
            .id("welcome-prompt-panel")
            .tab_index(tab_index)
            .w(px(CONTENT_WIDTH))
            .max_w_full()
            .bg(background)
            .border_1()
            .border_color(border)
            .rounded(px(16.))
            .overflow_hidden()
            .cursor_pointer()
            .hover(move |style| style.border_color(border_hovered))
            .on_click(self.focus_agent_panel())
            .child(
                h_flex()
                    .w_full()
                    .min_h(px(94.))
                    .items_start()
                    .p(px(24.))
                    .gap(px(18.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(18.))
                            .line_height(px(28.))
                            .text_color(cx.theme().colors().text_placeholder)
                            .child("Ask for changes, or describe an idea…"),
                    )
                    .child(
                        IconButton::new("welcome-prompt-attach", IconName::Paperclip)
                            .size(ButtonSize::None)
                            .width(px(32.))
                            .height(px(36.).into())
                            .icon_size(IconSize::Custom(rems_from_px(21_f32)))
                            .icon_color(Color::Muted)
                            .tooltip(Tooltip::text("Attach a File in the Agent Panel"))
                            .on_click(self.focus_agent_panel()),
                    )
                    .child(
                        IconButton::new("welcome-prompt-send", IconName::ArrowUp)
                            .shape(IconButtonShape::Circle)
                            .width(px(36.))
                            .size(ButtonSize::None)
                            .icon_size(IconSize::Custom(rems_from_px(21_f32)))
                            .background(send_background)
                            .icon_color(Color::Custom(send_icon))
                            .tooltip(Tooltip::text("Open the Agent Panel"))
                            .on_click(self.focus_agent_panel()),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .h(px(52.))
                    .px(px(16.))
                    .gap(px(12.))
                    .border_t_1()
                    .border_color(cx.theme().colors().border)
                    .bg(surface)
                    .child(div().flex_1()),
            )
    }

    fn render_action_button(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        action: Box<dyn Action>,
        tab_index: isize,
    ) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();

        Button::new(id, label)
            .tab_index(tab_index)
            .style(ButtonStyle::Outlined)
            .start_icon(Icon::new(icon).color(Color::Muted).size(IconSize::Small))
            .on_click(move |_, window, cx| focus_handle.dispatch_action(&*action, window, cx))
    }

    fn render_recent_project(
        &self,
        project_index: usize,
        tab_index: isize,
        location: &SerializedWorkspaceLocation,
        paths: &PathList,
    ) -> impl IntoElement {
        let name = project_name(paths);
        let icon = match location {
            SerializedWorkspaceLocation::Local => IconName::Folder,
            SerializedWorkspaceLocation::Remote(_) => IconName::Server,
        };
        let focus_handle = self.focus_handle.clone();
        let action = OpenRecentProject {
            index: project_index,
        };

        Button::new(("recent-project", project_index), name)
            .tab_index(tab_index)
            .full_width()
            .start_icon(Icon::new(icon).color(Color::Muted).size(IconSize::Small))
            .on_click(move |_, window, cx| focus_handle.dispatch_action(&action, window, cx))
    }
}

impl Render for WelcomePage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let background = cx.theme().colors().editor_background;
        let ai_enabled = AgentSettings::get_global(cx).enabled(cx);
        let project_name = self.project_display_name(cx);

        let mut next_tab_index: isize = 0;
        let mut tab_index = || {
            let index = next_tab_index;
            next_tab_index += 1;
            index
        };

        let prompt_panel = ai_enabled.then(|| self.render_prompt_panel(tab_index(), cx));

        let action_buttons = h_flex()
            .debug_selector(|| "welcome-actions".into())
            .max_w_full()
            .flex_wrap()
            .justify_center()
            .gap_2()
            .child(self.render_action_button(
                "welcome-new-file",
                IconName::Plus,
                "New File",
                NewFile.boxed_clone(),
                tab_index(),
            ))
            .child(self.render_action_button(
                "welcome-open-project",
                IconName::FolderOpen,
                "Open Project",
                Open::DEFAULT.boxed_clone(),
                tab_index(),
            ))
            .child(self.render_action_button(
                "welcome-clone-repo",
                IconName::CloudDownload,
                "Clone Repository",
                GitClone.boxed_clone(),
                tab_index(),
            ))
            .when(ai_enabled, |this| {
                this.child(self.render_action_button(
                    "welcome-open-sidebar",
                    IconName::ThreadsSidebarLeftClosed,
                    "Threads",
                    ToggleWorkspaceSidebar.boxed_clone(),
                    tab_index(),
                ))
            });

        let recent_projects = self
            .recent_workspaces
            .as_ref()
            .into_iter()
            .flatten()
            .take(5)
            .enumerate()
            .map(|(index, workspace)| {
                self.render_recent_project(
                    index,
                    next_tab_index + index as isize,
                    &workspace.location,
                    &workspace.identity_paths,
                )
            })
            .collect::<Vec<_>>();

        let showing_recent_projects =
            self.fallback_to_recent_projects && !recent_projects.is_empty();

        h_flex()
            .key_context("Welcome")
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::open_recent_project))
            .size_full()
            .bg(background)
            .when(self.embed_tab_strip && ai_enabled, |this| {
                this.flex().flex_col().child(self.render_home_tab_strip(cx))
            })
            .child(
                v_flex()
                    .id("welcome-content")
                    .flex_1()
                    .size_full()
                    .when(ai_enabled, |this| this.pb(px(72.)))
                    .when(!ai_enabled, |this| this.p_8().overflow_y_scroll())
                    .justify_center()
                    .child(
                        v_flex()
                            .w_full()
                            .flex_none()
                            // Auto margins center spare space without moving
                            // overflowing content above the scroll origin.
                            .when(!ai_enabled, |this| this.my_auto())
                            .items_center()
                            .gap(px(26.))
                            .when(!ai_enabled, |this| {
                                this.child(
                                    h_flex()
                                        .debug_selector(|| "welcome-brand".into())
                                        .gap_3()
                                        .child(logo_tile(40.))
                                        .child(
                                            div()
                                                .text_xl()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child("Zloppenheimer"),
                                        ),
                                )
                            })
                            .child(
                                v_flex()
                                    .debug_selector(|| "welcome-headline".into())
                                    .max_w_full()
                                    .items_center()
                                    .gap(px(10.))
                                    .pb(px(6.))
                                    .child(
                                        div()
                                            .max_w_full()
                                            .text_center()
                                            .text_size(px(28.))
                                            .line_height(px(36.))
                                            .tracking(rems(-0.035))
                                            .text_color(cx.theme().colors().text)
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("What are we building?"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(14.))
                                            .line_height(px(22.))
                                            .text_color(cx.theme().colors().text_placeholder)
                                            .child(format!("Start a thread in {project_name}.")),
                                    ),
                            )
                            .children(prompt_panel)
                            .when(ai_enabled, |this| {
                                this.child(
                                    h_flex()
                                        .gap(px(8.))
                                        .justify_center()
                                        .text_size(px(12.))
                                        .line_height(px(18.))
                                        .text_color(cx.theme().colors().text_muted)
                                        .child("Enter to send")
                                        .child("·")
                                        .child("Shift + Enter for a new line"),
                                )
                            })
                            .when(!ai_enabled, |this| this.child(action_buttons))
                            .when(!ai_enabled && showing_recent_projects, |this| {
                                this.child(
                                    v_flex()
                                        .w(px(CONTENT_WIDTH))
                                        .max_w_full()
                                        .gap_0p5()
                                        .child(
                                            div().px_2().pb_1().child(
                                                Label::new("Recent Projects")
                                                    .size(LabelSize::XSmall)
                                                    .color(Color::Muted),
                                            ),
                                        )
                                        .children(recent_projects),
                                )
                            }),
                    ),
            )
    }
}

impl EventEmitter<ItemEvent> for WelcomePage {}

impl Focusable for WelcomePage {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Item for WelcomePage {
    type Event = ItemEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        "New thread".into()
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::Thread).size(IconSize::Small))
    }

    fn telemetry_event_text(&self) -> Option<&'static str> {
        Some("New Welcome Page Opened")
    }

    fn show_toolbar(&self) -> bool {
        false
    }

    fn to_item_events(event: &Self::Event, f: &mut dyn FnMut(crate::item::ItemEvent)) {
        f(*event)
    }
}

impl crate::SerializableItem for WelcomePage {
    fn serialized_item_kind() -> &'static str {
        "WelcomePage"
    }

    fn cleanup(
        workspace_id: crate::WorkspaceId,
        alive_items: Vec<crate::ItemId>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui::Result<()>> {
        crate::delete_unloaded_items(
            alive_items,
            workspace_id,
            "welcome_pages",
            &persistence::WelcomePagesDb::global(cx),
            cx,
        )
    }

    fn deserialize(
        _project: Entity<project::Project>,
        workspace: gpui::WeakEntity<Workspace>,
        workspace_id: crate::WorkspaceId,
        item_id: crate::ItemId,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui::Result<Entity<Self>>> {
        if persistence::WelcomePagesDb::global(cx)
            .get_welcome_page(item_id, workspace_id)
            .ok()
            .is_some_and(|is_open| is_open)
        {
            Task::ready(Ok(
                cx.new(|cx| WelcomePage::new(workspace, true, false, window, cx))
            ))
        } else {
            Task::ready(Err(anyhow::anyhow!("No welcome page to deserialize")))
        }
    }

    fn serialize(
        &mut self,
        workspace: &mut Workspace,
        item_id: crate::ItemId,
        _closing: bool,
        cx: &mut Context<Self>,
    ) -> Option<Task<gpui::Result<()>>> {
        let workspace_id = workspace.database_id()?;
        let db = persistence::WelcomePagesDb::global(cx);
        Some(cx.background_spawn(
            async move { db.save_welcome_page(item_id, workspace_id, true).await },
        ))
    }

    fn should_serialize(&self, event: &Self::Event) -> bool {
        event == &ItemEvent::UpdateTab
    }
}

mod persistence {
    use crate::WorkspaceDb;
    use db::{
        query,
        sqlez::{domain::Domain, thread_safe_connection::ThreadSafeConnection},
        sqlez_macros::sql,
    };

    pub struct WelcomePagesDb(ThreadSafeConnection);

    impl Domain for WelcomePagesDb {
        const NAME: &str = stringify!(WelcomePagesDb);

        const MIGRATIONS: &[&str] = (&[sql!(
                    CREATE TABLE welcome_pages (
                        workspace_id INTEGER,
                        item_id INTEGER UNIQUE,
                        is_open INTEGER DEFAULT FALSE,

                        PRIMARY KEY(workspace_id, item_id),
                        FOREIGN KEY(workspace_id) REFERENCES workspaces(workspace_id)
                        ON DELETE CASCADE
                    ) STRICT;
        )]);
    }

    db::static_connection!(WelcomePagesDb, [WorkspaceDb]);

    impl WelcomePagesDb {
        query! {
            pub async fn save_welcome_page(
                item_id: crate::ItemId,
                workspace_id: crate::WorkspaceId,
                is_open: bool
            ) -> Result<()> {
                INSERT OR REPLACE INTO welcome_pages(item_id, workspace_id, is_open)
                VALUES (?, ?, ?)
            }
        }

        query! {
            pub fn get_welcome_page(
                item_id: crate::ItemId,
                workspace_id: crate::WorkspaceId
            ) -> Result<bool> {
                SELECT is_open
                FROM welcome_pages
                WHERE item_id = ? AND workspace_id = ?
            }
        }
    }
}

/// The display name for a recent project: its root directory names joined
/// together, so a multi-root project is labelled the same wherever it is listed.
pub fn project_name(paths: &PathList) -> String {
    let joined = paths
        .paths()
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    if joined.is_empty() {
        "Untitled".to_string()
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_name_empty() {
        let paths = PathList::new::<&str>(&[]);
        assert_eq!(project_name(&paths), "Untitled");
    }

    #[test]
    fn test_project_name_single() {
        let paths = PathList::new(&["/home/user/my-project"]);
        assert_eq!(project_name(&paths), "my-project");
    }

    #[test]
    fn test_project_name_multiple() {
        // PathList sorts lexicographically, so filenames appear in alpha order
        let paths = PathList::new(&["/home/user/zed", "/home/user/api"]);
        assert_eq!(project_name(&paths), "api, zed");
    }

    #[test]
    fn test_project_name_root_path_filtered() {
        // A bare root "/" has no file_name(), falls back to "Untitled"
        let paths = PathList::new(&["/"]);
        assert_eq!(project_name(&paths), "Untitled");
    }
}
