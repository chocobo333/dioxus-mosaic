use crate::drag_drop::{calculate_drop_zone, get_drop_zone_style, DragState, DropZone};
use crate::layout::MosaicLayout;
use crate::types::TileId;
use dioxus::prelude::*;

/// Wrapper for a tile with controls (split, close, drag-drop)
///
/// This component provides the UI controls for managing a tile,
/// including splitting it horizontally/vertically, closing it, and drag-and-drop reordering.
#[component]
pub fn TilePane(
    tile_id: TileId,
    title_component: Element,
    locked: bool,
    on_split_horizontal: EventHandler<()>,
    on_split_vertical: EventHandler<()>,
    on_close: EventHandler<()>,
    children: Element,
) -> Element {
    // Get drag state and layout from context
    let mut drag_state = use_context::<Signal<DragState>>();
    let mut layout = use_context::<Signal<MosaicLayout>>();

    // Cross-platform tile rect for drop zone calculation: (left, top, width, height)
    let mut tile_rect = use_signal(|| None::<(f64, f64, f64, f64)>);

    // Track current drop zone when this tile is hovered during drag
    let mut current_drop_zone = use_signal(|| None::<DropZone>);

    // Check if this tile is currently being dragged
    let is_being_dragged = drag_state.read().dragging_tile_id.as_ref() == Some(&tile_id);

    // Check if drag is active and this tile is being hovered
    let is_drag_active = drag_state.read().is_dragging();
    // Pre-calculate opacity for dragged tile
    let tile_opacity = if is_being_dragged { "0.4" } else { "1.0" };

    // Pre-calculate cursor style
    let header_cursor = if is_drag_active { "grabbing" } else { "grab" };

    let tile_id_ondrop = tile_id.clone();
    let tile_id_ondragover = tile_id.clone();

    rsx! {
        div {
            class: "mosaic-tile-pane",
            onmounted: move |evt| {
                spawn(async move {
                    if let Ok(rect) = evt.data().get_client_rect().await {
                        tile_rect.set(Some((
                            rect.min_x(),
                            rect.min_y(),
                            rect.width(),
                            rect.height(),
                        )));
                    }
                });
            },
            // Handle drag over for drop zone detection
            ondragover: move |evt| {
                evt.prevent_default(); // Required to allow drop
                evt.data_transfer().set_drop_effect("move");

                // Don't allow dropping on itself
                let current_dragging = drag_state.read().dragging_tile_id.clone();
                if current_dragging.as_ref() == Some(&tile_id_ondragover) {
                    return;
                }

                if let Some((left, top, width, height)) = tile_rect() {
                    let mouse_x = evt.client_coordinates().x as f64;
                    let mouse_y = evt.client_coordinates().y as f64;

                    if let Some(zone) = calculate_drop_zone(
                        mouse_x,
                        mouse_y,
                        left,
                        top,
                        width,
                        height,
                    ) {
                        current_drop_zone.set(Some(zone));
                        drag_state.write().update_hover(tile_id_ondragover.clone(), zone);
                    } else {
                        current_drop_zone.set(None);
                        drag_state.write().clear_hover();
                    }
                }
            },
            ondragleave: move |_evt| {
                current_drop_zone.set(None);
            },
            ondrop: move |evt| {
                evt.prevent_default();

                // Get the dragged tile ID and drop zone
                let dragged_tile = match drag_state.read().dragging_tile_id.clone() {
                    Some(tid) => tid,
                    None => return,
                };

                let zone = match current_drop_zone() {
                    Some(z) => z,
                    None => return,
                };

                // Don't drop on itself
                if dragged_tile == tile_id_ondrop {
                    return;
                }

                // Perform the layout mutation
                let success = layout.write().insert_tile_with_split(&dragged_tile, &tile_id_ondrop, zone);

                if success {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::log_1(
                        &format!(
                            "Successfully dropped {} on {} at {:?}",
                            dragged_tile, tile_id_ondrop, zone
                        )
                        .into(),
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    eprintln!(
                        "Successfully dropped {} on {} at {:?}",
                        dragged_tile, tile_id_ondrop, zone
                    );
                } else {
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::log_1(
                        &format!("Failed to drop {} on {} at {:?}", dragged_tile, tile_id_ondrop, zone)
                            .into(),
                    );
                    #[cfg(not(target_arch = "wasm32"))]
                    eprintln!(
                        "Failed to drop {} on {} at {:?}",
                        dragged_tile, tile_id_ondrop, zone
                    );
                }

                // Clear drag state
                drag_state.write().end_drag();
                current_drop_zone.set(None);
            },
            style: "
                background-color: #1a1d24;
                border: 1px solid #2a2f3a;
                border-radius: 8px;
                overflow: hidden;
                display: flex;
                flex-direction: column;
                height: 100%;
                position: relative;
                opacity: {tile_opacity};
                transition: opacity 0.2s ease;
            ",

            // Tile header with controls (draggable)
            div {
                class: "mosaic-tile-header",
                draggable: true,
                ondragstart: move |evt| {
                    let _ = evt.data_transfer().set_data("text/plain", &tile_id);
                    evt.data_transfer().set_effect_allowed("move");

                    let mouse_x = evt.client_coordinates().x as f64;
                    let mouse_y = evt.client_coordinates().y as f64;
                    drag_state.write().start_drag(tile_id.clone(), mouse_x, mouse_y);
                },
                ondragend: move |_evt| {
                    drag_state.write().end_drag();
                    current_drop_zone.set(None);
                },
                ondrag: move |evt| {
                    let mouse_x = evt.client_coordinates().x as f64;
                    let mouse_y = evt.client_coordinates().y as f64;

                    // Only update if position actually changed (drag events can fire at 0,0)
                    if mouse_x != 0.0 || mouse_y != 0.0 {
                        drag_state.write().update_position(mouse_x, mouse_y);
                    }
                },
                style: "
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    padding: 0.5rem 0.75rem;
                    border-bottom: 1px solid #2a2f3a;
                    background-color: #14161c;
                    flex-shrink: 0;
                    cursor: {header_cursor};
                    user-select: none;
                ",

                // Title
                div {
                    style: "
                        font-size: 0.875rem;
                        font-weight: 600;
                        color: #ffffff;
                        margin: 0;
                        flex: 1;
                        pointer-events: none;
                    ",
                    {title_component}
                }

                // Controls
                div {
                    class: "mosaic-tile-controls",
                    style: "display: flex; gap: 0.25rem; align-items: center;",

                    // Split horizontal button - TEMPORARILY DISABLED
                    // button {
                    //     onclick: move |_| on_split_horizontal.call(()),
                    //     title: "Split horizontally",
                    //     style: "
                    //         background: none;
                    //         border: 1px solid #3a4050;
                    //         color: #888;
                    //         cursor: pointer;
                    //         font-size: 0.75rem;
                    //         padding: 0.25rem 0.5rem;
                    //         border-radius: 3px;
                    //         transition: all 0.2s ease;
                    //     ",
                    //     "⬌"
                    // }

                    // Split vertical button - TEMPORARILY DISABLED
                    // button {
                    //     onclick: move |_| on_split_vertical.call(()),
                    //     title: "Split vertically",
                    //     style: "
                    //         background: none;
                    //         border: 1px solid #3a4050;
                    //         color: #888;
                    //         cursor: pointer;
                    //         font-size: 0.75rem;
                    //         padding: 0.25rem 0.5rem;
                    //         border-radius: 3px;
                    //         transition: all 0.2s ease;
                    //     ",
                    //     "⬍"
                    // }

                    // Close button (only if not locked)
                    if !locked {
                        button {
                            onclick: move |_| on_close.call(()),
                            title: "Close",
                            style: "
                                background: none;
                                border: 1px solid #3a4050;
                                color: #d66;
                                cursor: pointer;
                                font-size: 0.75rem;
                                padding: 0.25rem 0.5rem;
                                border-radius: 3px;
                                transition: all 0.2s ease;
                            ",
                            "✕"
                        }
                    }
                }
            }

            // Tile content
            div {
                class: "mosaic-tile-content",
                style: "
                    flex: 1;
                    overflow: auto;
                    min-height: 0;
                ",
                {children}
            }

            // Drop zone overlays (only show when drag is active and hovering over this tile)
            if is_drag_active && !is_being_dragged {
                // Top drop zone
                div {
                    class: "drop-zone drop-zone-top",
                    style: get_drop_zone_style(
                        DropZone::Top,
                        current_drop_zone() == Some(DropZone::Top)
                    ),
                }

                // Bottom drop zone
                div {
                    class: "drop-zone drop-zone-bottom",
                    style: get_drop_zone_style(
                        DropZone::Bottom,
                        current_drop_zone() == Some(DropZone::Bottom)
                    ),
                }

                // Left drop zone
                div {
                    class: "drop-zone drop-zone-left",
                    style: get_drop_zone_style(
                        DropZone::Left,
                        current_drop_zone() == Some(DropZone::Left)
                    ),
                }

                // Right drop zone
                div {
                    class: "drop-zone drop-zone-right",
                    style: get_drop_zone_style(
                        DropZone::Right,
                        current_drop_zone() == Some(DropZone::Right)
                    ),
                }
            }
        }
    }
}
