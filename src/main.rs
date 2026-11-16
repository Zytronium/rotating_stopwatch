
use freya::animation::{use_animation, OnFinish, AnimNum, OnCreation, Function};
use freya::prelude::*;
use freya::borderless::BorderlessPlugin;
use freya::winit::dpi::{LogicalSize, PhysicalPosition};

// Unique gesture identifier for title bar dragging.
#[derive(Clone, Copy, PartialEq)]
struct TitlebarDrag;

const WINDOW_WIDTH: f64 = 807.0;
const WINDOW_HEIGHT: f64 = 625.0;

const FRAME_WIDTH: f64 = 680.0;
const FRAME_HEIGHT: f64 = 472.0;

const TITLEBAR_HEIGHT: f64 = 30.0;
const BUTTON_WIDTH: f64 = 42.0;
const BUTTONS_WIDTH: f64 = BUTTON_WIDTH * 3.0;
const ROTATION_SPEED: f32 = 5.0;
const ROTATION_DURATION: u64 = (365.0 / ROTATION_SPEED as f64 * 500.0) as u64;
const WINDOW_PADDING: f64 = 16.0;

fn main() {
    launch(
        LaunchConfig::new()
            .with_plugin(
                BorderlessPlugin::new()
                    .with_corner_radius(12.)
            )
            .with_window(
                WindowConfig::new(app)
                    .with_size(WINDOW_WIDTH, WINDOW_HEIGHT)
                    .with_decorations(false)
                    .with_transparency(true)
                    .with_background(Color::TRANSPARENT)
            )
    );
}

fn app() -> impl IntoElement {
    let window = Platform::get().current_window();

    // Fixed screen-space center of the window.
    // This only changes when the user finishes dragging.
    let center_anchor = use_state(|| None::<(f64, f64)>);

    // Prevent animation-driven positioning while the OS handles dragging.
    let is_dragging = use_state(|| false);

    let rotation_animation = use_animation(|ctx| {
        ctx.on_creation(OnCreation::Run);
        ctx.on_finish(OnFinish::restart());

        AnimNum::new(0.0, 365.0)
            .time(ROTATION_DURATION)
            .function(Function::Linear)
    });

    let rotation = rotation_animation.get().value();
    let angle = (rotation as f64).to_radians();

    let sin = angle.sin().abs();
    let cos = angle.cos().abs();

    let window_width =
        FRAME_WIDTH * cos + FRAME_HEIGHT * sin + WINDOW_PADDING * 2.0;

    let window_height =
        FRAME_HEIGHT * cos + FRAME_WIDTH * sin + WINDOW_PADDING * 2.0;

    // Keep the window centered around the stored anchor.
    // Never derive the anchor from animation-driven window movement.
    {
        let mut center_anchor = center_anchor.clone();
        let is_dragging = is_dragging.clone();

        Platform::get().with_window(
            Platform::window_id(),
            move |native_window| {
                if native_window.is_maximized()
                    || *is_dragging.read()
                {
                    return;
                }

                let Ok(current_position) = native_window.outer_position() else {
                    return;
                };

                let current_outer_size = native_window.outer_size();
                let current_inner_size = native_window.inner_size();
                let scale_factor = native_window.scale_factor();

                // Initialize the anchor from the actual window geometry.
                // Do not assume that (0, 0) is a valid window center.
                let existing_anchor = *center_anchor.read();

                let (center_x, center_y) = match existing_anchor {
                    Some(center) => center,

                    None => {
                        let center = (
                            current_position.x as f64
                                + current_outer_size.width as f64 / 2.0,
                            current_position.y as f64
                                + current_outer_size.height as f64 / 2.0,
                        );

                        center_anchor.set(Some(center));

                        return;
                    }
                };

                let target_width =
                    (window_width * scale_factor).round() as u32;

                let target_height =
                    (window_height * scale_factor).round() as u32;

                let horizontal_frame =
                    current_outer_size.width.saturating_sub(current_inner_size.width);

                let vertical_frame =
                    current_outer_size.height.saturating_sub(current_inner_size.height);

                let target_outer_width = target_width + horizontal_frame;
                let target_outer_height = target_height + vertical_frame;

                let target_x =
                    (center_x - target_outer_width as f64 / 2.0).round() as i32;

                let target_y =
                    (center_y - target_outer_height as f64 / 2.0).round() as i32;

                if current_inner_size.width != target_width
                    || current_inner_size.height != target_height
                {
                    let _ = native_window.request_inner_size(
                        LogicalSize::new(window_width, window_height)
                    );
                }

                if current_position.x != target_x
                    || current_position.y != target_y
                {
                    native_window.set_outer_position(
                        PhysicalPosition::new(target_x, target_y)
                    );
                }
            }
        );
    }

    let titlebar_color = window
        .accent_color
        .read()
        .0
        .map(|color| {
            let [r, g, b, _] = color.to_u8_array();
            Color::from_rgb(r, g, b)
        })
        .unwrap_or(Color::from_rgb(24, 24, 24));

    let minimize_hovered = use_state(|| false);
    let maximize_hovered = use_state(|| false);
    let close_hovered = use_state(|| false);

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .center()

        // Detect presses on the actual rotated title bar.
        .on_global_pointer_down({
            let mut is_dragging = is_dragging.clone();

            move |event: Event<PointerEventData>| {
                if event.button() != Some(MouseButton::Left) {
                    return;
                }

                let (px, py) = event.global_location().to_tuple();

                let cx = window_width / 2.0;
                let cy = window_height / 2.0;

                let dx = px - cx;
                let dy = py - cy;

                let angle = (rotation as f64).to_radians();

                let local_x =
                    dx * angle.cos() + dy * angle.sin()
                        + FRAME_WIDTH / 2.0;

                let local_y =
                    -dx * angle.sin() + dy * angle.cos()
                        + FRAME_HEIGHT / 2.0;

                let draggable_width = FRAME_WIDTH - BUTTONS_WIDTH;

                let inside_titlebar =
                    local_x >= 0.0
                        && local_x <= FRAME_WIDTH
                        && local_y >= 0.0
                        && local_y <= TITLEBAR_HEIGHT;

                if !inside_titlebar {
                    return;
                }

                // Handle window buttons using rotated coordinates.
                if local_x >= draggable_width {
                    let platform = Platform::get();
                    let window_id = Platform::window_id();

                    if local_x >= FRAME_WIDTH - BUTTON_WIDTH {
                        // Close
                        platform.close_window(window_id);
                    } else if local_x >= FRAME_WIDTH - BUTTON_WIDTH * 2.0 {
                        // Maximize / restore
                        platform.with_window(window_id, |window| {
                            window.set_maximized(!window.is_maximized());
                        });
                    } else {
                        // Minimize
                        platform.with_window(window_id, |window| {
                            window.set_minimized(true);
                        });
                    }

                    return;
                }

                // Preserve double-click maximize behavior.
                if EventsCombos::<TitlebarDrag>::pressed(
                    event.global_location()
                ).is_double() {
                    Platform::get().with_window(
                        Platform::window_id(),
                        |window| {
                            window.set_maximized(!window.is_maximized());
                        }
                    );

                    return;
                }

                // Suspend animation-driven window positioning.
                // The operating system now owns the window movement.
                is_dragging.set(true);
            }
        })

        // Start native window dragging after pointer movement.
        .on_global_pointer_move({
            let mut minimize = minimize_hovered.clone();
            let mut maximize = maximize_hovered.clone();
            let mut close = close_hovered.clone();

            move |event: Event<PointerEventData>| {
                let (px, py) = event.global_location().to_tuple();

                let cx = window_width / 2.0;
                let cy = window_height / 2.0;

                let dx = px - cx;
                let dy = py - cy;

                let angle = (rotation as f64).to_radians();

                // Convert screen coordinates into rotated frame coordinates.
                let local_x =
                    dx * angle.cos() + dy * angle.sin()
                        + FRAME_WIDTH / 2.0;

                let local_y =
                    -dx * angle.sin() + dy * angle.cos()
                        + FRAME_HEIGHT / 2.0;

                let draggable_width = FRAME_WIDTH - BUTTONS_WIDTH;

                let inside_titlebar =
                    local_x >= 0.0
                        && local_x <= FRAME_WIDTH
                        && local_y >= 0.0
                        && local_y <= TITLEBAR_HEIGHT;

                // Determine which button is actually under the cursor.
                *minimize.write() =
                    inside_titlebar
                        && local_x >= draggable_width
                        && local_x < draggable_width + BUTTON_WIDTH;

                *maximize.write() =
                    inside_titlebar
                        && local_x >= draggable_width + BUTTON_WIDTH
                        && local_x < draggable_width + BUTTON_WIDTH * 2.0;

                *close.write() =
                    inside_titlebar
                        && local_x >= draggable_width + BUTTON_WIDTH * 2.0
                        && local_x <= FRAME_WIDTH;

                // Preserve native window dragging.
                if EventsCombos::<TitlebarDrag>::moved(
                    event.global_location()
                ) {
                    Platform::get().with_window(
                        Platform::window_id(),
                        |window| {
                            let _ = window.drag_window();
                        }
                    );
                }
            }
        })

        // Release the drag gesture.
        .on_global_pointer_up({
            let mut is_dragging = is_dragging.clone();
            let mut center_anchor = center_anchor.clone();

            move |_: Event<PointerEventData>| {
                EventsCombos::<TitlebarDrag>::released();

                Platform::get().with_window(
                    Platform::window_id(),
                    move |native_window| {
                        if let Ok(position) = native_window.outer_position() {
                            let size = native_window.outer_size();

                            center_anchor.set(Some((
                                position.x as f64 + size.width as f64 / 2.0,
                                position.y as f64 + size.height as f64 / 2.0,
                            )));
                        }

                        is_dragging.set(false);
                    }
                );
            }
        })

        .child(
            // Rotated application frame
            rect()
                .width(Size::px(FRAME_WIDTH as f32))
                .height(Size::px(FRAME_HEIGHT as f32))
                .vertical()
                .rotate(rotation)
                .corner_radius(10.)

                // Main content, overlapping title bar by 1px.
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px((FRAME_HEIGHT - TITLEBAR_HEIGHT) as f32))
                        .position(
                            Position::new_absolute()
                                .left(0.)
                                .top(TITLEBAR_HEIGHT as f32 - 1.)
                        )
                        .corner_radius(CornerRadius::new(0., 0., 10., 10.))
                        .center()
                        .background((32, 32, 32))
                        .color(Color::WHITE)
                        .child(label().text("Hello World!"))
                )

                // Custom title bar
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(TITLEBAR_HEIGHT as f32))
                        .horizontal()
                        .content(Content::flex())
                        .corner_radius(CornerRadius::new(10., 10., 0., 0.))
                        .background(titlebar_color)
                        .cross_align(Alignment::Center)

                        // Title
                        .child(
                            rect()
                                .width(Size::flex(1.))
                                .height(Size::fill())
                                .padding(6.)
                                .child(
                                    label()
                                        .text("Slightly Rotated Stopwatch")
                                        .color(Color::WHITE)
                                        .font_size(12)
                                )
                        )

                        // Window controls
                        .child(
                            rect()
                                .width(Size::px(BUTTONS_WIDTH as f32))
                                .height(Size::fill())
                                .horizontal()

                                // Minimize
                                .child(
                                    rect()
                                        .width(Size::px(BUTTON_WIDTH as f32))
                                        .height(Size::fill())
                                        .background(if *minimize_hovered.read() {
                                            Color::from_argb(35, 255, 255, 255)
                                        } else {
                                            titlebar_color
                                        })
                                        .center()
                                        .child(label().text("−").color(Color::WHITE))
                                )

                                // Maximize / restore
                                .child(
                                    rect()
                                        .width(Size::px(BUTTON_WIDTH as f32))
                                        .height(Size::fill())
                                        .background(if *maximize_hovered.read() {
                                            Color::from_argb(35, 255, 255, 255)
                                        } else {
                                            titlebar_color
                                        })
                                        .center()
                                        .child(label().text("□").color(Color::WHITE))
                                )

                                // Close
                                .child(
                                    rect()
                                        .width(Size::px(BUTTON_WIDTH as f32))
                                        .height(Size::fill())
                                        .background(if *close_hovered.read() {
                                            Color::from_argb(255, 190, 45, 45)
                                        } else {
                                            titlebar_color
                                        })
                                        .corner_radius(CornerRadius::new(0., 10., 0., 0.))
                                        .center()
                                        .child(label().text("×").color(Color::WHITE))
                                )
                        )
                )
        )
}
