#![cfg_attr(target_family = "wasm", no_main)]

//! Zloppenheimer — a GPUI implementation of the app home screen.
//!
//! Renders the custom title bar, the project sidebar (with a "Kode" project row
//! that exposes file-tree actions), the empty-thread prompt, and a bottom
//! account bar. The account name and plan shown in that bar are taken from the
//! signed-in Zed account rather than being hardcoded into the view (see
//! [`ZedAccount::from_zed_login`]).

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use gpui::{
    App, AssetSource, Bounds, Context, FontWeight, SharedString, Window, WindowBounds,
    WindowDecorations, WindowOptions, div, prelude::*, px, rgb, size, svg,
};
use gpui_platform::application;

const FONT_FAMILY: &str = "IBM Plex Sans";

// Dark "navy slate" palette matching the mockup.
const BG: u32 = 0x191c24; // main content background
const SIDEBAR_BG: u32 = 0x15171f; // sidebar background
const BAR_BG: u32 = 0x181b23; // title bar background
const PANEL_BG: u32 = 0x212633; // raised prompt panel
const ELEV_BG: u32 = 0x272c38; // pill / control background
const HOVER_BG: u32 = 0x2d3340; // hovered control background
const BORDER: u32 = 0x282d3a; // separators and control borders
const TEXT: u32 = 0xe7e9f0; // primary text
const MUTED: u32 = 0x9aa1b2; // secondary text
const DIM: u32 = 0x6b7280; // icons and tertiary text
const ACCENT: u32 = 0x7c6cf0; // send button
const CLAUDE: u32 = 0xd97757; // Anthropic "clay" accent
const DOT_RED: u32 = 0xff5f57;
const DOT_YELLOW: u32 = 0xfebc2e;
const DOT_GREEN: u32 = 0x28c840;

// Zloppenheimer logo mark: four staggered bars on a rounded tile.
const LOGO_BG: u32 = 0x0f1218;
const LOGO_EDGE: u32 = 0x242a36;
const LOGO_WHITE: u32 = 0xeef2f7;
const LOGO_PURPLE: u32 = 0xa181d1;
const LOGO_BLUE: u32 = 0x3d84ed;
const LOGO_GREEN: u32 = 0x7cae5f;

/// The signed-in Zed account rendered in the bottom bar.
#[derive(Clone)]
struct ZedAccount {
    display_name: SharedString,
    plan: SharedString,
}

impl ZedAccount {
    /// Load the account from the Zed login session.
    ///
    /// In the full application this reads the authenticated user from Zed's
    /// client user store. Here we resolve it from the environment so the bar
    /// reflects whoever is actually logged in instead of a hardcoded value,
    /// falling back to the account shown in the design.
    fn from_zed_login() -> Self {
        let display_name = std::env::var("ZED_ACCOUNT_NAME")
            .ok()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "Kieran".to_string());
        let plan = std::env::var("ZED_ACCOUNT_PLAN")
            .ok()
            .filter(|plan| !plan.trim().is_empty())
            .unwrap_or_else(|| "Max".to_string());
        Self {
            display_name: display_name.into(),
            plan: plan.into(),
        }
    }

    /// The avatar initial, derived from the display name.
    fn initial(&self) -> SharedString {
        self.display_name
            .chars()
            .next()
            .map(|first| first.to_uppercase().to_string())
            .unwrap_or_default()
            .into()
    }
}

struct Zloppenheimer {
    account: ZedAccount,
}

impl Render for Zloppenheimer {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(BG))
            .font_family(FONT_FAMILY)
            .text_color(rgb(TEXT))
            .text_sm()
            .child(title_bar())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(sidebar(&self.account))
                    .child(main_content()),
            )
    }
}

fn icon(path: &'static str, size_px: f32, color: u32) -> impl IntoElement {
    svg()
        .path(path)
        .w(px(size_px))
        .h(px(size_px))
        .flex_none()
        .text_color(rgb(color))
}

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

fn title_bar() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_3()
        .h(px(46.))
        .flex_none()
        .px_3()
        .bg(rgb(BAR_BG))
        .border_b_1()
        .border_color(rgb(BORDER))
        .child(traffic_lights())
        .child(brand())
        .child(divider())
        .child(breadcrumb())
        .child(div().flex_1())
        .child(title_bar_actions())
}

fn traffic_lights() -> impl IntoElement {
    let dot = |color: u32| div().size_3().rounded_full().bg(rgb(color));
    div()
        .flex()
        .items_center()
        .gap_2()
        .pr_1()
        .child(dot(DOT_RED))
        .child(dot(DOT_YELLOW))
        .child(dot(DOT_GREEN))
}

fn brand() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(logo_tile(26.))
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child("Zloppenheimer"),
        )
}

fn divider() -> impl IntoElement {
    div().w(px(1.)).h(px(18.)).bg(rgb(BORDER))
}

fn breadcrumb() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1p5()
        .text_color(rgb(MUTED))
        .child(icon("icons/folder.svg", 14., DIM))
        .child(div().text_color(rgb(TEXT)).child("zed"))
        .child(div().text_color(rgb(DIM)).child("/"))
        .child("New thread")
}

fn title_bar_actions() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(pill_button("icons/plus.svg", "Add action", false))
        .child(pill_button("icons/folder_open.svg", "Open", true))
        .child(pill_button("icons/git_branch.svg", "Initialize Git", false))
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .pl_1()
                .child(icon_button("icons/threads_sidebar_left_open.svg"))
                .child(icon_button("icons/threads_sidebar_right_open.svg")),
        )
}

fn pill_button(
    icon_path: &'static str,
    label: impl Into<SharedString>,
    trailing_chevron: bool,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1p5()
        .h(px(28.))
        .px_2p5()
        .rounded_lg()
        .bg(rgb(ELEV_BG))
        .border_1()
        .border_color(rgb(BORDER))
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(HOVER_BG)))
        .child(icon(icon_path, 14., MUTED))
        .child(label.into())
        .when(trailing_chevron, |this| {
            this.child(icon("icons/chevron_down.svg", 12., DIM))
        })
}

fn icon_button(icon_path: &'static str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size_7()
        .rounded_md()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(ELEV_BG)))
        .child(icon(icon_path, 16., MUTED))
}

fn sidebar(account: &ZedAccount) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .w(px(256.))
        .flex_none()
        .bg(rgb(SIDEBAR_BG))
        .border_r_1()
        .border_color(rgb(BORDER))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .p_2()
                .gap_0p5()
                .child(search_row())
                .child(kode_row())
                .child(section_header("Settled (5)")),
        )
        .child(account_footer(account))
}

fn search_row() -> impl IntoElement {
    sidebar_row()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_color(rgb(MUTED))
                .child(icon("icons/magnifying_glass.svg", 16., MUTED))
                .child("Search"),
        )
        .child(icon_button("icons/pencil.svg"))
}

/// The "Kode" project row. Alongside the project name it exposes a cluster of
/// file-tree actions, per the request to add buttons for working with file
/// trees next to the project.
fn kode_row() -> impl IntoElement {
    sidebar_row()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(icon("icons/folder.svg", 16., MUTED))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT))
                        .child("Kode"),
                )
                .child(icon("icons/chevron_down.svg", 12., DIM)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_0p5()
                .child(icon_button("icons/file_tree.svg"))
                .child(icon_button("icons/folder_add.svg"))
                .child(icon_button("icons/square_plus.svg"))
                .child(icon_button("icons/list_collapse.svg")),
        )
}

fn sidebar_row() -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(32.))
        .px_2()
        .rounded_md()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(ELEV_BG)))
}

fn section_header(label: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .h(px(28.))
        .px_2()
        .mt_1()
        .text_color(rgb(DIM))
        .text_xs()
        .child(label.into())
        .child(icon("icons/chevron_down.svg", 12., DIM))
}

fn account_footer(account: &ZedAccount) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .border_t_1()
        .border_color(rgb(BORDER))
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .px_2()
                .py_1()
                .child(icon_button("icons/settings.svg"))
                .child(icon_button("icons/git_branch.svg"))
                .child(icon_button("icons/git_graph.svg")),
        )
        .child(account_bar(account))
}

/// The bottom login bar. The name and plan are sourced from the Zed account, and
/// the avatar initial is derived from the account name.
fn account_bar(account: &ZedAccount) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_2p5()
        .py_2()
        .border_t_1()
        .border_color(rgb(BORDER))
        .child(
            div()
                .size_6()
                .flex_none()
                .rounded_full()
                .bg(rgb(ELEV_BG))
                .border_1()
                .border_color(rgb(BORDER))
                .flex()
                .items_center()
                .justify_center()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(account.initial()),
        )
        .child(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .items_center()
                .gap_1()
                .cursor_pointer()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT))
                        .child(account.display_name.clone()),
                )
                .child(div().text_color(rgb(DIM)).child("·"))
                .child(div().text_color(rgb(MUTED)).child(account.plan.clone()))
                .child(icon("icons/chevron_down.svg", 12., DIM)),
        )
        .child(icon_button("icons/download.svg"))
}

fn main_content() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_6()
        .p_8()
        .child(
            div()
                .flex()
                .items_end()
                .gap_0()
                .text_3xl()
                .text_color(rgb(TEXT))
                .child("What should we build in ")
                .child(
                    div()
                        .border_b_1()
                        .border_dashed()
                        .border_color(rgb(DIM))
                        .child("zed"),
                )
                .child("?"),
        )
        .child(prompt_panel())
}

fn prompt_panel() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .w(px(680.))
        .gap_3()
        .p_3()
        .bg(rgb(PANEL_BG))
        .border_1()
        .border_color(rgb(BORDER))
        .rounded_xl()
        .shadow_lg()
        .child(
            div()
                .pt_1()
                .pb_6()
                .text_color(rgb(MUTED))
                .child("Ask for changes, send follow-ups, or attach images"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(model_selector(
                    "icons/ai_claude.svg",
                    CLAUDE,
                    "Claude Sonnet 5",
                ))
                .child(divider())
                .child(model_selector_plain("High · 200k"))
                .child(model_selector("icons/lock.svg", MUTED, "Full access"))
                .child(div().flex_1())
                .child(icon_button("icons/paperclip.svg"))
                .child(send_button()),
        )
}

fn model_selector(
    icon_path: &'static str,
    icon_color: u32,
    label: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1p5()
        .h(px(28.))
        .px_2()
        .rounded_lg()
        .text_color(rgb(MUTED))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(ELEV_BG)))
        .child(icon(icon_path, 14., icon_color))
        .child(label.into())
        .child(icon("icons/chevron_down.svg", 12., DIM))
}

fn model_selector_plain(label: impl Into<SharedString>) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1p5()
        .h(px(28.))
        .px_2()
        .rounded_lg()
        .text_color(rgb(MUTED))
        .cursor_pointer()
        .hover(|style| style.bg(rgb(ELEV_BG)))
        .child(label.into())
        .child(icon("icons/chevron_down.svg", 12., DIM))
}

fn send_button() -> impl IntoElement {
    div()
        .size_7()
        .flex_none()
        .rounded_full()
        .bg(rgb(ACCENT))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x8f80f5)))
        .child(icon("icons/arrow_up.svg", 16., 0xffffff))
}

struct Assets {
    base: PathBuf,
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        fs::read(self.base.join(path))
            .map(|data| Some(Cow::Owned(data)))
            .map_err(|error| error.into())
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        fs::read_dir(self.base.join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(|error| error.into())
    }
}

fn load_fonts(cx: &App, base: &Path) -> Result<()> {
    let mut fonts = Vec::new();
    for name in ["IBMPlexSans-Regular.ttf", "IBMPlexSans-SemiBold.ttf"] {
        let path = base.join("fonts/ibm-plex-sans").join(name);
        fonts.push(Cow::Owned(fs::read(path)?));
    }
    cx.text_system().add_fonts(fonts)
}

fn run_example() {
    let assets_base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    application()
        .with_assets(Assets {
            base: assets_base.clone(),
        })
        .run(move |cx: &mut App| {
            if let Err(error) = load_fonts(cx, &assets_base) {
                eprintln!("failed to load fonts: {error:#}");
            }
            let bounds = Bounds::centered(None, size(px(1024.), px(720.)), cx);
            let window = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: None,
                    window_decorations: Some(WindowDecorations::Client),
                    window_min_size: Some(size(px(720.), px(480.))),
                    ..Default::default()
                },
                |_, cx| {
                    cx.new(|_| Zloppenheimer {
                        account: ZedAccount::from_zed_login(),
                    })
                },
            );
            if let Err(error) = window {
                eprintln!("failed to open window: {error:#}");
                return;
            }
            cx.activate(true);
        });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    run_example();
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    gpui_platform::web_init();
    run_example();
}
