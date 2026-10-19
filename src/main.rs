//! A GPUI component gallery that runs on Android, desktop, and the web.
//!
//! Native: `gpuiforge run android` or `gpuiforge run desktop`.
//! Web: `trunk serve` (see README).

use gpui::{prelude::*, *};
use gpui_effects::{FrostedGlass, FrostedGlassAppearance};
use gpui_platform::application;
#[cfg(target_family = "wasm")]
use web_time::Instant;
#[cfg(not(target_family = "wasm"))]
use std::time::Instant;
use uic::components::{
    badge::Badge,
    bottom_sheet::BottomSheet,
    glass::{GlassSegmentedAppearance, GlassSegmentedControl},
    input::InputEvent,
    modal::{self, Modal},
    notification::{self, Notification, NotificationPlacement},
    number_input::{NumberInput, NumberInputChanged, NumberInputOptions, NumberInputState},
    progress::Progress,
    selection::{Checkbox, RadioGroup, Switch},
    slider::{Slider, SliderEvent, SliderState},
    tabs::{TabVariant, Tabs},
    toast,
};

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

const BG: u32 = 0x11111b;
const SURFACE: u32 = 0x1e1e2e;
const SURFACE_ALT: u32 = 0x181825;
const BORDER: u32 = 0x313244;
const TEXT: u32 = 0xcdd6f4;
const MUTED: u32 = 0x9399b2;
const ACCENT: u32 = 0x89b4fa;
const GREEN: u32 = 0xa6e3a1;
const RED: u32 = 0xf38ba8;
const YELLOW: u32 = 0xf9e2af;
const ON_ACCENT: u32 = 0x11111b;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[gpui_platform::main]
fn main() {
    // Wasm needs its panic hook and logging installed before the app starts.
    #[cfg(target_family = "wasm")]
    gpui_platform::web_init();
    application().run(|cx: &mut App| {
        uic::init(cx);
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| Gallery::new(window, cx))
        })
        .unwrap();
    });
}

// ---------------------------------------------------------------------------
// Section model
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Page {
    Basics,
    Inputs,
    Controls,
    Glass,
    Overlays,
    Feedback,
    Data,
}

impl Page {
    const ALL: [Page; 7] = [
        Page::Basics,
        Page::Inputs,
        Page::Controls,
        Page::Glass,
        Page::Overlays,
        Page::Feedback,
        Page::Data,
    ];

    fn label(self) -> &'static str {
        match self {
            Page::Basics => "Basics",
            Page::Inputs => "Inputs",
            Page::Controls => "Controls",
            Page::Glass => "Glass",
            Page::Overlays => "Overlays",
            Page::Feedback => "Feedback",
            Page::Data => "Data",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Page::Basics => "◆",
            Page::Inputs => "✎",
            Page::Controls => "◉",
            Page::Glass => "◇",
            Page::Overlays => "▤",
            Page::Feedback => "!",
            Page::Data => "☰",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
enum Flavor {
    #[default]
    Balanced,
    Compact,
    Spacious,
}

impl Flavor {
    fn label(self) -> &'static str {
        match self {
            Flavor::Balanced => "Balanced",
            Flavor::Compact => "Compact",
            Flavor::Spacious => "Spacious",
        }
    }
}

// ---------------------------------------------------------------------------
// Gallery state
// ---------------------------------------------------------------------------

struct Gallery {
    page: Page,
    scroll: ScrollHandle,

    // Controls
    check_a: bool,
    check_b: bool,
    switch_a: bool,
    switch_b: bool,
    radio: u8,
    flavor: Flavor,
    volume: Entity<SliderState>,
    number: Entity<NumberInputState>,
    _subscriptions: Vec<Subscription>,

    // Inputs
    name: Entity<uic::components::input::TextInput>,
    password: Entity<uic::components::input::TextInput>,
    notes: Entity<uic::components::input::TextInput>,
    name_value: String,
    notes_len: usize,

    // Feedback / misc
    progress: f64,
    counter: u32,
    log: Vec<String>,

    // Glass page
    glass_tab: &'static str,
    glass_vertical: u8,
    glass_animated: bool,
    glass_opaque: bool,
    glass_shape: bool,

    // Render-counter FPS. `render` is where a frame is built, so counting calls over a
    // one-second window reports how fast the app is actually producing frames.
    frame_stamps: Vec<Instant>,
    fps: f32,
}

impl Gallery {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let volume = cx.new(|cx| SliderState::new(0.6, 0.0..=1.0, cx).step(0.05));

        let volume_subscription = cx.subscribe(&volume, |this, _, event: &SliderEvent, cx| {
            if let SliderEvent::Changed(value) = event {
                this.note(format!("volume committed → {value:.2}"), cx);
            }
        });

        let number = cx.new(|cx| {
            NumberInputState::new(
                1.0,
                NumberInputOptions {
                    min: Some(0.0),
                    max: Some(10.0),
                    step: 0.5,
                    precision: 1,
                },
                window,
                cx,
            )
        });
        let number_subscription = cx.subscribe(&number, |this, _, event: &NumberInputChanged, cx| {
            this.note(format!("quantity → {}", event.value), cx);
        });

        let name = cx.new(|cx| {
            uic::components::input::TextInput::new(cx)
                .aria_label("Display name")
                .placeholder("Type your name")
        });
        let name_subscription = cx.subscribe(&name, |this, _, event: &InputEvent, cx| {
            this.name_value = event.text().to_string();
            cx.notify();
        });

        let password = cx.new(|cx| {
            uic::components::input::TextInput::new(cx)
                .password()
                .aria_label("Password")
                .placeholder("Password")
        });

        let notes = cx.new(|cx| {
            uic::components::input::TextInput::new(cx)
                .multiline()
                .aria_label("Notes")
                .placeholder("Multi-line notes…")
        });
        let notes_subscription = cx.subscribe(&notes, |this, _, event: &InputEvent, cx| {
            this.notes_len = event.text().chars().count();
            cx.notify();
        });

        Self {
            page: Page::Basics,
            scroll: ScrollHandle::new(),
            check_a: true,
            check_b: false,
            switch_a: false,
            switch_b: true,
            radio: 1,
            flavor: Flavor::Balanced,
            volume,
            number,
            name,
            password,
            notes,
            name_value: String::new(),
            notes_len: 0,
            _subscriptions: vec![
                volume_subscription,
                number_subscription,
                name_subscription,
                notes_subscription,
            ],
            progress: 0.35,
            counter: 0,
            log: vec!["Gallery ready.".into()],
            glass_tab: "overview",
            glass_vertical: 0,
            glass_animated: true,
            glass_opaque: false,
            glass_shape: true,
            frame_stamps: Vec::with_capacity(240),
            fps: 0.0,
        }
    }

    fn note(&mut self, message: impl Into<String>, cx: &mut Context<Self>) {
        self.log.insert(0, message.into());
        self.log.truncate(40);
        cx.notify();
    }
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        self.frame_stamps.push(now);
        self.frame_stamps
            .retain(|t| now.duration_since(*t).as_secs_f32() < 1.0);
        self.fps = self.frame_stamps.len() as f32;
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .child(self.header(cx))
            .child(
                div()
                    .id("gallery-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .flex()
                    .flex_col()
                    .gap_4()
                    .px_4()
                    .py_4()
                    .pb(px(96.))
                    .child(match self.page {
                        Page::Basics => self.page_basics(cx).into_any_element(),
                        Page::Inputs => self.page_inputs(cx).into_any_element(),
                        Page::Controls => self.page_controls(cx).into_any_element(),
                        Page::Glass => self.page_glass(cx).into_any_element(),
                        Page::Overlays => self.page_overlays(cx).into_any_element(),
                        Page::Feedback => self.page_feedback(cx).into_any_element(),
                        Page::Data => self.page_data(cx).into_any_element(),
                    }),
            )
            .child(self.bottom_bar(cx))
            .child(toast::layer(cx))
            .child(notification::layer(cx))
            .child(modal::layer(cx))
    }
}

impl Gallery {
    fn header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .px_4()
            .pt_4()
            .pb_3()
            .bg(rgb(SURFACE_ALT))
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_2xl().child("GPUI Gallery"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded_full()
                                    .bg(rgb(SURFACE))
                                    .text_xs()
                                    .text_color(rgb(if self.fps >= 55.0 { GREEN } else if self.fps >= 30.0 { YELLOW } else { RED }))
                                    .child(format!("{:.0} fps", self.fps)),
                            )
                            .child(
                                div()
                                    .id("header-page")
                                    .px_3()
                                    .py_1()
                                    .rounded_full()
                                    .bg(rgb(ACCENT))
                                    .text_color(rgb(ON_ACCENT))
                                    .text_xs()
                                    .child(self.page.label()),
                            ),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED))
                    .child("Android · GPUI + uic components"),
            )
            .child(
                Tabs::new("top-tabs", self.page)
                    .variant(TabVariant::Underline)
                    .tab(Page::Basics, div().child("Basics"))
                    .tab(Page::Inputs, div().child("Inputs"))
                    .tab(Page::Controls, div().child("Controls"))
                    .tab(Page::Glass, div().child("Glass"))
                    .tab(Page::Overlays, div().child("Overlays"))
                    .tab(Page::Feedback, div().child("Feedback"))
                    .tab(Page::Data, div().child("Data"))
                    .on_change({
                        let entity = entity.clone();
                        move |page, _, cx| {
                            entity.update(cx, |this, cx| {
                                this.page = page;
                                this.scroll.set_offset(point(px(0.), px(0.)));
                                cx.notify();
                            });
                        }
                    }),
            )
    }

    // -- Basics ------------------------------------------------------------

    fn page_basics(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.section(
                "Typography",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_3xl().child("text_3xl"))
                    .child(div().text_2xl().child("text_2xl"))
                    .child(div().text_xl().child("text_xl"))
                    .child(div().text_lg().child("text_lg"))
                    .child(div().text_base().child("text_base — the default body size"))
                    .child(div().text_sm().text_color(rgb(MUTED)).child("text_sm, muted"))
                    .child(div().text_xs().text_color(rgb(MUTED)).child("text_xs, muted"))
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .child("font_weight(BOLD)"),
                    )
                    .child(
                        div()
                            .italic()
                            .child("italic() for emphasis and zh 中文混排测试"),
                    ),
            ))
            .child(self.section(
                "Colors",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children(
                        [
                            ("accent", ACCENT),
                            ("green", GREEN),
                            ("red", RED),
                            ("yellow", YELLOW),
                            ("muted", MUTED),
                            ("surface", SURFACE),
                        ]
                        .into_iter()
                        .map(|(name, color)| {
                            div()
                                .w(px(96.))
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().h(px(40.)).rounded_lg().bg(rgb(color)))
                                .child(div().text_xs().text_color(rgb(MUTED)).child(name))
                        }),
                    ),
            ))
            .child(self.section(
                "Shape & elevation",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .w(px(80.))
                            .h(px(56.))
                            .rounded_lg()
                            .bg(rgb(SURFACE))
                            .border_1()
                            .border_color(rgb(BORDER)),
                    )
                    .child(div().w(px(80.)).h(px(56.)).rounded_full().bg(rgb(SURFACE)))
                    .child(
                        div()
                            .w(px(80.))
                            .h(px(56.))
                            .rounded_2xl()
                            .bg(rgb(SURFACE))
                            .shadow_lg(),
                    )
                    .child(
                        div()
                            .w(px(80.))
                            .h(px(56.))
                            .rounded_lg()
                            .border_2()
                            .border_dashed()
                            .border_color(rgb(ACCENT)),
                    ),
            ))
            .child(self.section(
                "Buttons",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(pill("btn-primary", "Primary", ACCENT, ON_ACCENT))
                    .child(pill("btn-ghost", "Ghost", SURFACE, TEXT))
                    .child(pill("btn-danger", "Danger", RED, ON_ACCENT))
                    .child(
                        div()
                            .id("btn-counter")
                            .role(Role::Button)
                            .aria_label("Counter")
                            .px_4()
                            .py_2()
                            .rounded_full()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .text_color(rgb(TEXT))
                            .child(format!("Tapped {}×", self.counter))
                            .on_click({
                                let entity = entity.clone();
                                move |_, _, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.counter += 1;
                                        cx.notify();
                                    });
                                }
                            }),
                    ),
            ))
            .child(self.section(
                "Badges",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_3()
                    .items_center()
                    .child(Badge::new("badge-count", div().size(px(36.)).rounded_lg().bg(rgb(SURFACE))).count(3))
                    .child(
                        Badge::new("badge-max", div().size(px(36.)).rounded_lg().bg(rgb(SURFACE)))
                            .count(120)
                            .max(99),
                    )
                    .child(Badge::new("badge-dot", div().size(px(36.)).rounded_lg().bg(rgb(SURFACE))).dot())
                    .child(
                        Badge::new("badge-text", div().size(px(36.)).rounded_lg().bg(rgb(SURFACE)))
                            .text("NEW"),
                    ),
            ))
    }

    // -- Inputs ------------------------------------------------------------

    fn page_inputs(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        let name_value = self.name_value.clone();
        let notes_len = self.notes_len;
        let number_value = self.number.read(_cx).value();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.section(
                "Text field",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        uic::components::input::Input::new(&self.name)
                            .w_full()
                            .px_3()
                            .py_2()
                            .rounded_lg()
                            .bg(rgb(SURFACE))
                            .border_1()
                            .border_color(rgb(BORDER)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("live value: {name_value:?}")),
                    ),
            ))
            .child(self.section(
                "Password field",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        uic::components::input::Input::new(&self.password)
                            .w_full()
                            .px_3()
                            .py_2()
                            .rounded_lg()
                            .bg(rgb(SURFACE))
                            .border_1()
                            .border_color(rgb(BORDER)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child("Masked on screen and in accessibility output."),
                    ),
            ))
            .child(self.section(
                "Multiline field",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        uic::components::input::Input::new(&self.notes)
                            .w_full()
                            .h(px(96.))
                            .px_3()
                            .py_2()
                            .rounded_lg()
                            .bg(rgb(SURFACE))
                            .border_1()
                            .border_color(rgb(BORDER)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("{notes_len} characters")),
                    ),
            ))
            .child(self.section(
                "Number input",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        NumberInput::new(&self.number)
                            .label("Quantity")
                            .w_full(),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("value: {number_value} (0–10, step 0.5)")),
                    ),
            ))
    }

    // -- Controls ----------------------------------------------------------

    fn page_controls(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let volume = self.volume.read(cx).value();
        let entity = cx.entity();
        let check_entity = entity.clone();
        let check2_entity = entity.clone();
        let switch_entity = entity.clone();
        let switch2_entity = entity.clone();
        let radio_entity = entity.clone();
        let flavor_entity = entity.clone();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.section(
                "Checkbox",
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        Checkbox::new("check-a", self.check_a)
                            .label_text("Enable feature A")
                            .on_change(move |checked, _, cx| {
                                check_entity.update(cx, |this, cx| {
                                    this.check_a = checked;
                                    cx.notify();
                                });
                            }),
                    )
                    .child(
                        Checkbox::new("check-b", self.check_b)
                            .label_text("Enable feature B")
                            .on_change(move |checked, _, cx| {
                                check2_entity.update(cx, |this, cx| {
                                    this.check_b = checked;
                                    cx.notify();
                                });
                            }),
                    ),
            ))
            .child(self.section(
                "Switch",
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child("Wi-Fi")
                            .child(
                                Switch::new("switch-a", self.switch_a)
                                    .label("Wi-Fi")
                                    .on_change(move |checked, _, cx| {
                                        switch_entity.update(cx, |this, cx| {
                                            this.switch_a = checked;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child("Bluetooth")
                            .child(
                                Switch::new("switch-b", self.switch_b)
                                    .label("Bluetooth")
                                    .on_change(move |checked, _, cx| {
                                        switch2_entity.update(cx, |this, cx| {
                                            this.switch_b = checked;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    ),
            ))
            .child(self.section(
                "Radio group",
                RadioGroup::new("radio-flavor", self.radio)
                    .option(1_u8, "Small")
                    .option(2_u8, "Medium")
                    .option(3_u8, "Large")
                    .on_change(move |value, _, cx| {
                        radio_entity.update(cx, |this, cx| {
                            this.radio = value;
                            cx.notify();
                        });
                    }),
            ))
            .child(self.section(
                "Segmented",
                div()
                    .flex()
                    .gap_2()
                    .children(Flavor::ALL.into_iter().map(|flavor| {
                        let active = self.flavor == flavor;
                        let entity = flavor_entity.clone();
                        div()
                            .id(gpui::ElementId::Name(
                                format!("flavor-{flavor:?}").into(),
                            ))
                            .role(Role::Button)
                            .aria_label(flavor.label())
                            .flex_1()
                            .py_2()
                            .rounded_lg()
                            .text_center()
                            .text_sm()
                            .bg(if active { rgb(ACCENT) } else { rgb(SURFACE) })
                            .text_color(if active { rgb(ON_ACCENT) } else { rgb(TEXT) })
                            .cursor_pointer()
                            .child(flavor.label())
                            .on_click(move |_, _, cx| {
                                entity.update(cx, |this, cx| {
                                    this.flavor = flavor;
                                    cx.notify();
                                });
                            })
                    })),
            ))
            .child(self.section(
                "Slider",
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        Slider::new(&self.volume)
                            .label("Volume")
                            .w_full(),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("value: {volume:.2}")),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(small_button("vol-down", "−0.1", {
                                let volume = self.volume.clone();
                                move |_, cx| {
                                    let current = volume.read(cx).value();
                                    volume.update(cx, |state, cx| {
                                        state.set_value((current - 0.1).max(0.0), cx);
                                    });
                                }
                            }))
                            .child(small_button("vol-up", "+0.1", {
                                let volume = self.volume.clone();
                                move |_, cx| {
                                    let current = volume.read(cx).value();
                                    volume.update(cx, |state, cx| {
                                        state.set_value((current + 0.1).min(1.0), cx);
                                    });
                                }
                            })),
                    ),
            ))
    }

    // -- Glass -------------------------------------------------------------

    fn page_glass(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();

        // Refraction targets: saturated shapes so the lens has something to bend.
        let blobs = if self.glass_shape {
            div()
                .absolute()
                .left(px(0.))
                .top(px(0.))
                .right(px(0.))
                .bottom(px(0.))
                .child(
                    div()
                        .absolute()
                        .left(px(12.))
                        .top(px(24.))
                        .size(px(120.))
                        .rounded_full()
                        .bg(rgb(0xf38ba8)),
                )
                .child(
                    div()
                        .absolute()
                        .right(px(20.))
                        .top(px(8.))
                        .size(px(96.))
                        .rounded_full()
                        .bg(rgb(0x89b4fa)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(120.))
                        .bottom(px(10.))
                        .size(px(110.))
                        .rounded_full()
                        .bg(rgb(0xa6e3a1)),
                )
                .child(
                    div()
                        .absolute()
                        .right(px(70.))
                        .bottom(px(36.))
                        .size(px(72.))
                        .rounded_full()
                        .bg(rgb(0xf9e2af)),
                )
                .into_any_element()
        } else {
            div().into_any_element()
        };

        let segmented_change = entity.clone();
        let vertical_change = entity.clone();
        let animated_entity = entity.clone();
        let opaque_entity = entity.clone();
        let shape_entity = entity.clone();

        let tab = self.glass_tab;
        let vertical = self.glass_vertical;

        div()
            .flex()
            .flex_col()
            .gap_4()
            // ---- Liquid glass segmented control -----------------------------
            .child(self.section(
                "Liquid glass · segmented control",
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(
                                "Drag the lens sideways, or just tap a segment. Press-and-hold scales the selected lens to 135%.",
                            ),
                    )
                    // The lens refracts whatever is painted behind it, so it needs a
                    // colourful, high-contrast backdrop to be visible.
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(188.))
                            .rounded_2xl()
                            .overflow_hidden()
                            .bg(rgb(0x101018))
                            .child(blobs)
                            .child(
                                div()
                                    .absolute()
                                    .left(px(0.))
                                    .right(px(0.))
                                    .top(px(0.))
                                    .bottom(px(0.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        GlassSegmentedControl::new("glass-view", tab)
                                            .label("Workspace view")
                                            .option("overview", div().px_3().py_1().child("Overview"))
                                            .option("activity", div().px_3().py_1().child("Activity"))
                                            .disabled_option("archive", div().px_3().py_1().child("Archive"))
                                            .appearance(GlassSegmentedAppearance::light())
                                            .text_color(rgb(0xffffff))
                                            .text_size(px(15.))
                                            .rounded(px(22.))
                                            .animated(self.glass_animated)
                                            .reduced_transparency(self.glass_opaque)
                                            .selection_press_scale(1.35)
                                            .surface_press_scale(1.05)
                                            .on_change(move |value, _, cx| {
                                                segmented_change.update(cx, |this, cx| {
                                                    this.glass_tab = value;
                                                    this.note(format!("glass tab → {value}"), cx);
                                                });
                                            }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("selected: {tab}")),
                    ),
            ))
            // ---- Liquid glass · vertical + tappable options ----------------
            .child(self.section(
                "Liquid glass · vertical",
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(240.))
                            .rounded_2xl()
                            .overflow_hidden()
                            .bg(rgb(0x0d1117))
                            .child(
                                div()
                                    .absolute()
                                    .left(px(24.))
                                    .top(px(18.))
                                    .size(px(150.))
                                    .rounded_full()
                                    .bg(rgb(0x89b4fa)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .right(px(16.))
                                    .bottom(px(16.))
                                    .size(px(170.))
                                    .rounded_full()
                                    .bg(rgb(0xcba6f7)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px(0.))
                                    .right(px(0.))
                                    .top(px(0.))
                                    .bottom(px(0.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        GlassSegmentedControl::new("glass-size", vertical)
                                            .option(0_u8, div().px_4().py_1().child("Small"))
                                            .option(1_u8, div().px_4().py_1().child("Medium"))
                                            .option(2_u8, div().px_4().py_1().child("Large"))
                                            .flex_col()
                                            .items_stretch()
                                            .appearance(GlassSegmentedAppearance::dark())
                                            .text_color(rgb(0xffffff))
                                            .rounded(px(22.))
                                            .animated(self.glass_animated)
                                            .reduced_transparency(self.glass_opaque)
                                            .on_change(move |value, _, cx| {
                                                vertical_change.update(cx, |this, cx| {
                                                    this.glass_vertical = value;
                                                    this.note(format!("glass size → {value}"), cx);
                                                });
                                            }),
                                    ),
                            ),
                    ),
            ))
            // ---- Frosted glass panels --------------------------------------
            .child(self.section(
                "Frosted glass · backdrop blur",
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(
                                "FrostedGlass blurs whatever is already painted behind it; its own children stay sharp.",
                            ),
                    )
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(px(220.))
                            .rounded_2xl()
                            .overflow_hidden()
                            .bg(rgb(0x1b1030))
                            // Colourful background so the blur is observable.
                            .child(
                                div()
                                    .absolute()
                                    .left(px(-20.))
                                    .top(px(-10.))
                                    .size(px(160.))
                                    .rounded_full()
                                    .bg(rgb(0xf38ba8)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px(90.))
                                    .top(px(70.))
                                    .size(px(180.))
                                    .rounded_full()
                                    .bg(rgb(0x89b4fa)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .right(px(-30.))
                                    .bottom(px(-20.))
                                    .size(px(200.))
                                    .rounded_full()
                                    .bg(rgb(0xa6e3a1)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .right(px(90.))
                                    .top(px(-10.))
                                    .size(px(120.))
                                    .rounded_full()
                                    .bg(rgb(0xf9e2af)),
                            )
                            // The frosted pane sits on top of those shapes.
                            .child(
                                div()
                                    .absolute()
                                    .left(px(0.))
                                    .right(px(0.))
                                    .top(px(0.))
                                    .bottom(px(0.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        FrostedGlass::with_appearance(
                                            FrostedGlassAppearance::dark()
                                                .blur_radius(px(14.))
                                                .sheen(0.05),
                                        )
                                        .w(px(280.))
                                        .p_4()
                                        .rounded(px(24.))
                                        .text_color(rgb(0xffffff))
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(div().text_lg().child("Frosted pane"))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xcdd6f4))
                                                .child("Text stays crisp while the colour behind it blurs."),
                                        ),
                                    ),
                            ),
                    ),
            ))
            // ---- Toggles ---------------------------------------------------
            .child(self.section(
                "Glass options",
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child("Animated lens")
                            .child(
                                Switch::new("glass-animated", self.glass_animated)
                                    .label("Animated lens")
                                    .on_change(move |checked, _, cx| {
                                        animated_entity.update(cx, |this, cx| {
                                            this.glass_animated = checked;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child("Reduced transparency")
                            .child(
                                Switch::new("glass-opaque", self.glass_opaque)
                                    .label("Reduced transparency")
                                    .on_change(move |checked, _, cx| {
                                        opaque_entity.update(cx, |this, cx| {
                                            this.glass_opaque = checked;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child("Show colour shapes")
                            .child(
                                Switch::new("glass-shape", self.glass_shape)
                                    .label("Show colour shapes")
                                    .on_change(move |checked, _, cx| {
                                        shape_entity.update(cx, |this, cx| {
                                            this.glass_shape = checked;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    ),
            ))
    }

    // -- Overlays ----------------------------------------------------------

    fn page_overlays(&mut self, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.section(
                "Modal",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child("A modal traps focus and dims the page behind it."),
                    )
                    .child(pill_action("open-modal", "Open modal", ACCENT, ON_ACCENT, |_, window, cx| {
                        modal::show(
                            Modal::new(|_, _| {
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(div().text_lg().child("Hello from a modal"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(rgb(MUTED))
                                            .child(
                                                "Press OK or Cancel. The back button also closes it.",
                                            ),
                                    )
                            })
                            .title_text("Modal demo")
                            .ok_text(|_, _| div().child("Got it"))
                            .cancel_label("Dismiss")
                            .on_ok(|_, _| true),
                            window,
                            cx,
                        );
                    })),
            ))
            .child(self.section(
                "Bottom sheet",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child("Slides up from the bottom edge; drag down to dismiss."),
                    )
                    .child(pill_action("open-sheet", "Open sheet", SURFACE, TEXT, |_, window, cx| {
                        BottomSheet::new(|_, _| {
                            div()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .p_4()
                                .child(div().text_lg().child("Bottom sheet"))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(MUTED))
                                        .child("Native-style sheet with drag-to-dismiss."),
                                )
                                .child(div().h(px(120.)).w_full().rounded_lg().bg(rgb(SURFACE)))
                        })
                        .title("Sheet")
                        .drag_to_dismiss(true)
                        .close_on_backdrop(true)
                        .show(window, cx);
                    })),
            ))
            .child(self.section(
                "Tooltip-free hover card",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_3()
                            .rounded_lg()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(SURFACE))
                            .child(div().text_sm().child("This card is a plain element tree."))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child(
                                        "Use `.shadow_lg()` plus a border for an elevated surface.",
                                    ),
                            ),
                    ),
            ))
    }

    // -- Feedback ----------------------------------------------------------

    fn page_feedback(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let progress = self.progress;
        let log = self.log.clone();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.section(
                "Toast",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(pill_action("toast-info", "Info", SURFACE, TEXT, |_, _, cx| {
                        toast::info("Something informational", cx);
                    }))
                    .child(pill_action("toast-ok", "Success", GREEN, ON_ACCENT, |_, _, cx| {
                        toast::success("Saved successfully", cx);
                    }))
                    .child(pill_action("toast-warn", "Warning", YELLOW, ON_ACCENT, |_, _, cx| {
                        toast::warn("Low disk space", cx);
                    }))
                    .child(pill_action("toast-err", "Error", RED, ON_ACCENT, |_, _, cx| {
                        toast::error("Upload failed", cx);
                    })),
            ))
            .child(self.section(
                "Notification",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child("Notifications carry a title, body, and status icon."),
                    )
                    .child(pill_action("notify", "Show notification", ACCENT, ON_ACCENT, |_, _, cx| {
                        notification::show(
                            Notification::new(
                                "Export complete",
                                "The archive was written to the selected folder.",
                            )
                            .success()
                            .placement(NotificationPlacement::BottomRight)
                            .persistent(),
                            cx,
                        );
                    })),
            ))
            .child(self.section(
                "Progress",
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        Progress::new("progress-bar", progress)
                            .label("Determinate")
                            .w_full(),
                    )
                    .child(Progress::indeterminate("progress-spin").w_full())
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(small_button("prog-down", "−10%", {
                                let entity = entity.clone();
                                move |_, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.progress = (this.progress - 0.1).max(0.0);
                                        cx.notify();
                                    });
                                }
                            }))
                            .child(small_button("prog-up", "+10%", {
                                let entity = entity.clone();
                                move |_, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.progress = (this.progress + 0.1).min(1.0);
                                        cx.notify();
                                    });
                                }
                            })),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("{:.0}%", progress * 100.0)),
                    ),
            ))
            .child(self.section(
                "Event log",
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(log.into_iter().take(10).map(|line| {
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(format!("· {line}"))
                    })),
            ))
    }

    // -- Data --------------------------------------------------------------

    fn page_data(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.section(
                "List rows",
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(["Rendering", "Layout", "Text", "Input", "Accessibility"].into_iter().enumerate().map(|(i, name)| {
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_3()
                            .rounded_lg()
                            .bg(if i % 2 == 0 { rgb(SURFACE) } else { rgb(SURFACE_ALT) })
                            .child(div().text_sm().child(name))
                            .child(div().text_xs().text_color(rgb(MUTED)).child(format!("row {}", i + 1)))
                    })),
            ))
            .child(self.section(
                "Card grid",
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children((1..=6).map(|n| {
                        div()
                            .w(px(100.))
                            .h(px(72.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_lg()
                            .bg(rgb(SURFACE))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .child(div().text_sm().text_color(rgb(MUTED)).child(format!("cell {n}")))
                    })),
            ))
            .child(self.section(
                "Live counter with log",
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(small_button("data-inc", "+1", {
                                let entity = entity.clone();
                                move |_, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.counter += 1;
                                        this.note(format!("counter → {}", this.counter), cx);
                                    });
                                }
                            }))
                            .child(small_button("data-reset", "Reset", {
                                let entity = entity.clone();
                                move |_, cx| {
                                    entity.update(cx, |this, cx| {
                                        this.counter = 0;
                                        this.note("counter reset", cx);
                                    });
                                }
                            })),
                    )
                    .child(div().text_3xl().child(self.counter.to_string())),
            ))
    }

    // -- Chrome ------------------------------------------------------------

    fn section(&self, title: &'static str, body: impl IntoElement) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded_2xl()
            .bg(rgb(SURFACE_ALT))
            .border_1()
            .border_color(rgb(BORDER))
            .child(div().text_xs().text_color(rgb(MUTED)).child(title.to_uppercase()))
            .child(body)
    }

    fn bottom_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        div()
            .absolute()
            .left(px(0.))
            .right(px(0.))
            .bottom(px(0.))
            .h(px(76.))
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .bg(rgb(SURFACE_ALT))
            .border_t_1()
            .border_color(rgb(BORDER))
            .children(Page::ALL.into_iter().map(|page| {
                let active = self.page == page;
                let entity = entity.clone();
                div()
                    .id(gpui::ElementId::Name(format!("tab-{page:?}").into()))
                    .role(Role::Button)
                    .aria_label(page.label())
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .py_2()
                    .cursor_pointer()
                    .child(
                        div()
                            .text_lg()
                            .text_color(if active { rgb(ACCENT) } else { rgb(MUTED) })
                            .child(page.glyph()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if active { rgb(ACCENT) } else { rgb(MUTED) })
                            .child(page.label()),
                    )
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |this, cx| {
                            this.page = page;
                            this.scroll.set_offset(point(px(0.), px(0.)));
                            cx.notify();
                        });
                    })
            }))
    }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn pill(id: &'static str, label: &'static str, bg: u32, fg: u32) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .px_4()
        .py_2()
        .rounded_full()
        .bg(rgb(bg))
        .text_color(rgb(fg))
        .text_sm()
        .cursor_pointer()
        .child(label)
}

fn pill_action(
    id: &'static str,
    label: &'static str,
    bg: u32,
    fg: u32,
    action: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    pill(id, label, bg, fg).on_click(action)
}

fn small_button(
    id: &'static str,
    label: &'static str,
    action: impl Fn(&mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .px_3()
        .py_1()
        .rounded_lg()
        .bg(rgb(SURFACE))
        .border_1()
        .border_color(rgb(BORDER))
        .text_xs()
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .child(label)
        .on_click(move |_, window, cx| action(window, cx))
}

impl Flavor {
    const ALL: [Flavor; 3] = [Flavor::Balanced, Flavor::Compact, Flavor::Spacious];
}
