use glib;
use gtk::gdk::NotifyType;
use gtk::pango;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Button, Image, Label, Orientation};
use gtk_layer_shell::{Edge, Layer, LayerShell};
use log::info;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use swayipc::{Connection, EventType, Node};

use crate::config::{DockConfig, EdgeConfig};

// Create a dock attached to the given application
pub fn create_dock(app: &Application, config: &DockConfig) {
    // Store window info in thread-safe container
    let windows = Arc::new(Mutex::new(Vec::new()));
    let windows_clone = Arc::clone(&windows);

    // Spawn thread to listen for Sway window events
    thread::spawn(move || {
        let mut connection = Connection::new().expect("Failed to connect to Sway");
        update_window_list(&mut connection, &windows_clone);

        // Subscribe to window events
        match connection.subscribe(&[EventType::Window]) {
            Ok(events) => {
                for event in events {
                    if event.is_ok() {
                        // Update window list when any window event occurs
                        let mut conn = Connection::new().expect("Failed to connect to Sway");
                        update_window_list(&mut conn, &windows_clone);
                    }
                }
            }
            Err(e) => eprintln!("Failed to subscribe to events: {}", e),
        }
    });

    // Determine orientation based on edge
    let orientation = match config.edge {
        EdgeConfig::Left | EdgeConfig::Right => Orientation::Vertical,
        EdgeConfig::Top | EdgeConfig::Bottom => Orientation::Horizontal,
    };

    // Determine dimensions based on orientation
    let (width, height) = match orientation {
        Orientation::Horizontal => (800, 60),
        Orientation::Vertical => (60, 800),
        _ => (800, 60),
    };

    // Create main dock window
    let dock_window = ApplicationWindow::builder()
        .application(app)
        .title("Sway Dock")
        .default_width(width)
        .default_height(height)
        .build();

    // Set up layer shell
    dock_window.init_layer_shell();
    dock_window.set_layer(Layer::Top);
    dock_window.set_anchor(config.edge.to_edge(), true);

    // Set anchors for top/bottom edges
    match config.edge {
        EdgeConfig::Top | EdgeConfig::Bottom => {
            dock_window.set_anchor(Edge::Left, true);
            dock_window.set_anchor(Edge::Right, true);
        }
        EdgeConfig::Left | EdgeConfig::Right => {
            dock_window.set_anchor(Edge::Top, true);
            dock_window.set_anchor(Edge::Bottom, true);
        }
    }

    // Make window transparent
    dock_window.set_app_paintable(true);
    dock_window.connect_draw(|_, cr| {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.paint().unwrap();
        false.into()
    });

    // Create dock container with configured orientation
    let dock_box = GtkBox::new(orientation, 5);
    dock_box.set_halign(gtk::Align::Center);
    dock_box.set_margin(5);

    // Create detection area (small strip at the configured edge)
    let detection_window = ApplicationWindow::builder()
        .application(app)
        .title("Dock Detector")
        .default_width(width)
        .default_height(1)
        .height_request(1)
        .build();

    // Set up layer shell for detection window
    detection_window.init_layer_shell();
    detection_window.set_layer(Layer::Overlay);
    detection_window.set_anchor(config.edge.to_edge(), true);

    match config.edge {
        EdgeConfig::Top | EdgeConfig::Bottom => {
            detection_window.set_anchor(Edge::Left, true);
            detection_window.set_anchor(Edge::Right, true);
        }
        EdgeConfig::Left | EdgeConfig::Right => {
            detection_window.set_anchor(Edge::Top, true);
            detection_window.set_anchor(Edge::Bottom, true);
        }
    }

    // Hide dock initially
    dock_window.hide();

    // Show dock when mouse enters detector
    let dock_window_clone = dock_window.clone();
    detection_window.connect_enter_notify_event(move |_, _| {
        info!("Mouse entered dock detector");
        dock_window_clone.show_all();
        false.into()
    });

    // Use configured hide_timeout
    let hide_delay = config.hide_timeout;

    // Hide dock when mouse completely leaves it (not when it moves between children)
    dock_window.connect_leave_notify_event(move |window, event| {
        // Get the crossing detail - this tells us where the pointer went
        let detail = event.detail();

        // Only hide if the pointer actually left the window hierarchy
        // (Not just moved from parent to child or between children)
        if detail == NotifyType::Nonlinear
            || detail == NotifyType::NonlinearVirtual
            || detail == NotifyType::Ancestor
        {
            info!(
                "Mouse truly left dock, scheduling close (detail: {:?})",
                detail
            );
            let window_clone = window.clone();
            glib::timeout_add_local(Duration::from_millis(hide_delay), move || {
                window_clone.hide();
                false.into()
            });
        }

        false.into()
    });

    // Update dock contents periodically
    let windows_ref = Arc::clone(&windows);
    let dock_box_ref = dock_box.clone();
    let orientation = orientation.clone();
    glib::timeout_add_local(Duration::from_millis(1000), move || {
        update_dock_ui(&dock_box_ref, &windows_ref, orientation);
        true.into()
    });

    // Apply CSS styling
    let provider = gtk::CssProvider::new();
    provider
        .load_from_data(
            b"
            button {
                background-color: rgba(40, 40, 40, 0.8);
                border-radius: 6px;
                padding: 3px;
            }
            button:hover {
                background-color: rgba(80, 80, 80, 0.9);
            }
            label {
                color: white;
                font-size: 9px;
            }
        ",
        )
        .unwrap();
    gtk::StyleContext::add_provider_for_screen(
        &gtk::gdk::Screen::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    dock_window.add(&dock_box);
    detection_window.show_all();
}

// Update our list of Sway windows
fn update_window_list(connection: &mut Connection, windows: &Arc<Mutex<Vec<WindowInfo>>>) {
    if let Ok(tree) = connection.get_tree() {
        let mut window_list = Vec::new();
        extract_windows(&tree, &mut window_list);

        let mut windows_guard = windows.lock().unwrap();
        *windows_guard = window_list;
    }
}

// Extract window information from Sway tree
fn extract_windows(node: &Node, windows: &mut Vec<WindowInfo>) {
    // Check if this node is an application window
    if node.node_type == swayipc::NodeType::Con
        && !node.name.is_none()
        && (node.app_id.is_some() || node.window_properties.is_some())
    {
        // This is an app window
        let title = node.name.clone().unwrap_or_default();
        let app_id = node.app_id.clone().unwrap_or_else(|| {
            node.window_properties
                .as_ref()
                .and_then(|props| props.class.clone())
                .unwrap_or_default()
        });

        windows.push(WindowInfo {
            id: node.id,
            title,
            app_id,
            focused: node.focused,
        });
    }

    // Recursively check children
    for child in &node.nodes {
        extract_windows(child, windows);
    }
    for child in &node.floating_nodes {
        extract_windows(child, windows);
    }
}

// Update the dock UI with current windows
fn update_dock_ui(
    dock_box: &GtkBox,
    windows: &Arc<Mutex<Vec<WindowInfo>>>,
    orientation: Orientation,
) {
    // Clear existing children
    for child in dock_box.children() {
        dock_box.remove(&child);
    }

    // Get window list
    let windows_guard = match windows.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    // Add a button for each window
    for window in windows_guard.iter() {
        if window.title.is_empty() {
            continue;
        }

        // Create button for each window
        let button = Button::new();
        let container_box = match orientation {
            Orientation::Horizontal => GtkBox::new(Orientation::Vertical, 2),
            Orientation::Vertical => GtkBox::new(Orientation::Horizontal, 2),
            _ => GtkBox::new(Orientation::Horizontal, 2),
        };

        // Add icon
        let icon_name = get_icon_for_app(&window.app_id);
        let icon = Image::from_icon_name(Some(&icon_name), gtk::IconSize::Dnd);
        container_box.pack_start(&icon, true, true, 0);

        // Add label with orientation-aware positioning
        let label = Label::new(Some(&window.title));
        label.set_max_width_chars(10);
        label.set_ellipsize(pango::EllipsizeMode::End);
        container_box.pack_start(&label, false, false, 0);

        button.add(&container_box);

        // Connect click to focus window
        let window_id = window.id;
        button.connect_clicked(move |_| {
            if let Ok(mut conn) = Connection::new() {
                let _ = conn.run_command(format!("[con_id={}] focus", window_id));
            }
        });

        dock_box.pack_start(&button, false, false, 5);
    }

    dock_box.show_all();
}

// Try to find an appropriate icon for the app
fn get_icon_for_app(app_id: &str) -> String {
    match app_id.to_lowercase() {
        id if id.contains("firefox") => "firefox",
        id if id.contains("chrome") => "google-chrome",
        id if id.contains("terminal") => "terminal",
        id if id.contains("code") => "visual-studio-code",
        _ => "application-x-executable",
    }
    .to_string()
}

#[derive(Clone, Debug)]
struct WindowInfo {
    id: i64,
    title: String,
    app_id: String,
    focused: bool,
}
