use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Label, Orientation};
use gtk_layer_shell::{Edge, Layer, LayerShell};
use log::{error, info};
use mlua::{Function, Lua, Table, Value};
use once_cell::sync::OnceCell;
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// A structure to hold the GTK widget created by a Lua script
pub struct LuaWidget {
    window: Option<ApplicationWindow>,
    update_interval: u64,
}

/// The GTK API exposed to Lua
struct GtkApi {
    app: Application,
    widget: Rc<RefCell<LuaWidget>>,
}

/// Load and execute a Lua script from the given path
pub fn load_script(app: &Application, script_path: &Path) -> Result<(), mlua::Error> {
    info!("Loading script: {:?}", script_path);

    let lua = Lua::new();
    let globals = lua.globals();

    // Create widget state
    let widget = Rc::new(RefCell::new(LuaWidget {
        window: None,
        update_interval: 60, // Default update interval in seconds
    }));

    // Create the GTK API for Lua
    let gtk_api = GtkApi {
        app: app.clone(),
        widget: widget.clone(),
    };

    // Register GTK API functions
    {
        let create_window =
            lua.create_function(move |lua, (title, width, height): (String, i32, i32)| {
                let app_clone = gtk_api.app.clone();
                let widget_clone = gtk_api.widget.clone();

                info!("Creating window: {}", title);
                let window = ApplicationWindow::builder()
                    .application(&app_clone)
                    .title(&title)
                    .default_width(width)
                    .default_height(height)
                    .build();

                window.init_layer_shell();
                window.set_layer(Layer::Background);
                window.set_anchor(Edge::Top, true);
                window.set_anchor(Edge::Left, true);

                // Make window transparent
                window.set_app_paintable(true);
                window.connect_draw(|_, cr| {
                    cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
                    cr.paint().unwrap();
                    false.into()
                });

                widget_clone.borrow_mut().window = Some(window.clone());

                // Create a table to hold window methods
                let window_table = lua.create_table()?;

                // set_margin method
                {
                    let window_clone = window.clone();
                    let set_margin =
                        lua.create_function(move |_, (edge, margin): (String, i32)| {
                            let edge = match edge.as_str() {
                                "top" => Edge::Top,
                                "bottom" => Edge::Bottom,
                                "left" => Edge::Left,
                                "right" => Edge::Right,
                                _ => {
                                    return Err(mlua::Error::RuntimeError(
                                        "Invalid edge".to_string(),
                                    ));
                                }
                            };
                            window_clone.set_layer_shell_margin(edge, margin);
                            Ok(())
                        })?;
                    window_table.set("set_margin", set_margin)?;
                }

                // show method
                {
                    let window_clone = window.clone();
                    let show = lua.create_function(move |_, ()| {
                        window_clone.show_all();
                        Ok(())
                    })?;
                    window_table.set("show", show)?;
                }

                // set_update_interval method
                {
                    let widget_clone = widget_clone.clone();
                    let set_update_interval = lua.create_function(move |_, interval: u64| {
                        widget_clone.borrow_mut().update_interval = interval;
                        Ok(())
                    })?;
                    window_table.set("set_update_interval", set_update_interval)?;
                }

                // add_box method
                {
                    let window_clone = window.clone();
                    let add_box =
                        lua.create_function(move |lua, (orientation, spacing): (String, i32)| {
                            let orientation = match orientation.as_str() {
                                "vertical" => Orientation::Vertical,
                                "horizontal" => Orientation::Horizontal,
                                _ => {
                                    return Err(mlua::Error::RuntimeError(
                                        "Invalid orientation".to_string(),
                                    ));
                                }
                            };

                            let container = GtkBox::new(orientation, spacing);
                            window_clone.add(&container);

                            // Create a table to hold box methods
                            let box_table = lua.create_table()?;

                            // add_label method
                            {
                                let container_clone = container.clone();
                                let add_label = lua.create_function(
                                    move |_, (text, font_size): (String, i32)| {
                                        let label = Label::new(Some(&text));

                                        // Apply CSS for font size
                                        let css_provider = gtk::CssProvider::new();
                                        let css = format!(
                                            "label {{ font-size: {}px; color: white; }}",
                                            font_size
                                        );
                                        css_provider.load_from_data(css.as_bytes()).unwrap();

                                        let style_context = label.style_context();
                                        style_context.add_provider(
                                            &css_provider,
                                            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                                        );

                                        container_clone.pack_start(&label, true, true, 0);

                                        // Create a table to hold label methods
                                        let label_table = lua.create_table()?;

                                        // set_text method
                                        {
                                            let label_clone = label.clone();
                                            let set_text =
                                                lua.create_function(move |_, text: String| {
                                                    label_clone.set_text(&text);
                                                    Ok(())
                                                })?;
                                            label_table.set("set_text", set_text)?;
                                        }

                                        // set_css method for custom styling
                                        {
                                            let label_clone = label.clone();
                                            let set_css =
                                                lua.create_function(move |_, css: String| {
                                                    let css_provider = gtk::CssProvider::new();
                                                    css_provider
                                                        .load_from_data(css.as_bytes())
                                                        .unwrap();

                                                    let style_context = label_clone.style_context();
                                                    style_context.add_provider(
                                                        &css_provider,
                                                        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                                                    );
                                                    Ok(())
                                                })?;
                                            label_table.set("set_css", set_css)?;
                                        }

                                        Ok(label_table)
                                    },
                                )?;
                                box_table.set("add_label", add_label)?;
                            }

                            // set_css method for the box
                            {
                                let container_clone = container.clone();
                                let set_css = lua.create_function(move |_, css: String| {
                                    let css_provider = gtk::CssProvider::new();
                                    css_provider.load_from_data(css.as_bytes()).unwrap();

                                    let style_context = container_clone.style_context();
                                    style_context.add_provider(
                                        &css_provider,
                                        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                                    );
                                    Ok(())
                                })?;
                                box_table.set("set_css", set_css)?;
                            }

                            Ok(box_table)
                        })?;
                    window_table.set("add_box", add_box)?;
                }

                Ok(window_table)
            })?;
        globals.set("create_window", create_window)?;
    }

    // HTTP functions for weather API
    {
        let fetch_json = lua.create_function(|_, url: String| {
            info!("Fetching JSON from: {}", url);
            match reqwest::blocking::get(&url) {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<JsonValue>() {
                            Ok(json) => {
                                let lua_value = serde_json_to_lua_value(json);
                                Ok(lua_value)
                            }
                            Err(err) => Err(mlua::Error::RuntimeError(format!(
                                "Failed to parse JSON: {}",
                                err
                            ))),
                        }
                    } else {
                        Err(mlua::Error::RuntimeError(format!(
                            "HTTP error: {}",
                            response.status()
                        )))
                    }
                }
                Err(err) => Err(mlua::Error::RuntimeError(format!(
                    "Failed to fetch URL: {}",
                    err
                ))),
            }
        })?;
        globals.set("fetch_json", fetch_json)?;
    }

    // Schedule function for periodic updates
    // {
    //     let schedule_update = lua.create_function(|lua, func: Function| {
    //         // Store the function in the Lua registry to keep it alive
    //         let func_ref = lua.create_registry_value(func)?;

    //         // Clone necessary items for the thread
    //         let lua_clone = lua.clone();
    //         let func_ref_clone = func_ref.clone();

    //         // Spawn a thread that will periodically call the update function
    //         thread::spawn(move || {
    //             loop {
    //                 // Wait before each update
    //                 thread::sleep(Duration::from_secs(5)); // Update every 5 seconds initially

    //                 // Get the function from the registry and call it
    //                 let func: Function = match lua_clone.registry_value(&func_ref_clone) {
    //                     Ok(f) => f,
    //                     Err(e) => {
    //                         error!("Failed to get update function: {}", e);
    //                         break;
    //                     }
    //                 };

    //                 if let Err(e) = func.call::<_, ()>(()) {
    //                     error!("Error in update function: {}", e);
    //                 }
    //             }
    //         });

    //         Ok(())
    //     })?;
    //     globals.set("schedule_update", schedule_update)?;
    // }

    // Helper functions
    {
        // Print function for debugging
        let print = lua.create_function(|_, message: String| {
            info!("[Lua] {}", message);
            Ok(())
        })?;
        globals.set("log", print)?;
    }

    // Execute the script
    let script_content = std::fs::read_to_string(script_path)?;
    lua.load(&script_content).exec()?;

    Ok(())
}

/// Convert a serde_json::Value to an mlua::Value
fn serde_json_to_lua_value(json: JsonValue) -> Value {
    match json {
        JsonValue::Null => Value::Nil,
        JsonValue::Bool(b) => Value::Boolean(b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                Value::Number(f)
            } else {
                Value::Nil
            }
        }
        JsonValue::String(s) => Value::String(Lua::new().create_string(&s).unwrap()),
        JsonValue::Array(arr) => {
            let lua = Lua::new();
            let lua_table = lua.create_table().unwrap();

            for (i, val) in arr.into_iter().enumerate() {
                let lua_val = serde_json_to_lua_value(val);
                lua_table.set(i + 1, lua_val).unwrap();
            }

            Value::Table(lua_table)
        }
        JsonValue::Object(obj) => {
            let lua = Lua::new();
            let lua_table = lua.create_table().unwrap();

            for (k, v) in obj {
                let lua_val = serde_json_to_lua_value(v);
                lua_table.set(k, lua_val).unwrap();
            }

            Value::Table(lua_table)
        }
    }
}

/// Load all scripts from the scripts directory
pub fn load_scripts(app: &Application) -> Result<(), Box<dyn std::error::Error>> {
    // Get the XDG config directory for our app
    let mut scripts_dir = if let Some(config_dir) = dirs::config_dir() {
        let mut path = config_dir;
        path.push("swi");
        path.push("scripts");
        path
    } else {
        PathBuf::from("./scripts")
    };

    // Create directory if it doesn't exist
    if !scripts_dir.exists() {
        std::fs::create_dir_all(&scripts_dir)?;

        // Create example weather widget script
        let weather_script_path = scripts_dir.join("weather.lua");
        std::fs::write(&weather_script_path, include_str!("../scripts/weather.lua"))?;
        info!(
            "Created example weather script at {:?}",
            weather_script_path
        );
    }

    // Load all .lua scripts
    for entry in std::fs::read_dir(scripts_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "lua") {
            if let Err(e) = load_script(app, &path) {
                error!("Failed to load script {:?}: {}", path, e);
            }
        }
    }

    Ok(())
}
