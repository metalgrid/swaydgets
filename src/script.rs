use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Label, Orientation};
use gtk_layer_shell::{Edge, Layer, LayerShell};
use log::{debug, error, info};
use mlua::{Lua, Value};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// A structure to hold the GTK widget created by a Lua script
pub struct LuaWidget {
    window: Option<ApplicationWindow>,
    update_interval: u64,
}

/// ScriptManager owns the Lua state and manages script execution
pub struct ScriptManager {
    app: Application,
    lua: Rc<Lua>,
    widgets: Vec<Rc<RefCell<LuaWidget>>>,
}

impl ScriptManager {
    /// Create a new ScriptManager
    pub fn new(app: &Application) -> Self {
        ScriptManager {
            app: app.clone(),
            lua: Rc::new(Lua::new()),
            widgets: Vec::new(),
        }
    }

    /// Load and execute a Lua script from the given path
    pub fn load_script(&mut self, script_path: &Path) -> Result<(), mlua::Error> {
        info!("Loading script: {:?}", script_path);

        let lua = self.lua.clone();
        // let globals = lua.globals();

        // Create widget state
        let widget = Rc::new(RefCell::new(LuaWidget {
            window: None,
            update_interval: 60, // Default update interval in seconds
        }));
        self.widgets.push(widget.clone());

        // Register GTK API functions
        self.register_gtk_api(&lua, widget.clone())?;

        // Register HTTP functions
        self.register_http_api(&lua)?;

        // Register helper functions
        self.register_helper_functions(&lua)?;

        // Execute the script
        let script_content = std::fs::read_to_string(script_path)?;
        lua.load(&script_content).exec()?;

        Ok(())
    }

    /// Register GTK API functions with Lua
    fn register_gtk_api(
        &self,
        lua: &Lua,
        widget: Rc<RefCell<LuaWidget>>,
    ) -> Result<(), mlua::Error> {
        let globals = lua.globals();
        let app = self.app.clone();

        {
            let create_window =
                lua.create_function(move |lua, (title, width, height): (String, i32, i32)| {
                    let app_clone = app.clone();
                    let widget_clone = widget.clone();

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
                        let set_margin = lua.create_function(
                            move |_, (_this, edge, margin): (mlua::Table, String, i32)| {
                                debug!("Setting margin: {} {:#?}", edge, margin);
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
                            },
                        )?;
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
                        let set_update_interval =
                            lua.create_function(move |_, (_this, interval): (mlua::Table, u64)| {
                                widget_clone.borrow_mut().update_interval = interval;
                                Ok(())
                            })?;
                        window_table.set("set_update_interval", set_update_interval)?;
                    }

                    // add_box method
                    {
                        let window_clone = window.clone();
                        let add_box = lua.create_function(
                            move |lua, (_this, orientation, spacing): (mlua::Table, String, i32)| {
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
                                        move |lua,
                                              (_this, text, font_size): (
                                            mlua::Table,
                                            String,
                                            i32,
                                        )| {
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
                                                    lua.create_function(move |_, (_this, text): (mlua::Table, String)| {
                                                        label_clone.set_text(&text);
                                                        Ok(())
                                                    })?;
                                                label_table.set("set_text", set_text)?;
                                            }

                                            // set_css method for custom styling
                                            {
                                                let label_clone = label.clone();
                                                let set_css = lua.create_function(
                                                    move |_, (_this, css): (mlua::Table, String)| {
                                                        let css_provider = gtk::CssProvider::new();
                                                        css_provider
                                                            .load_from_data(css.as_bytes())
                                                            .unwrap();

                                                        let style_context =
                                                            label_clone.style_context();
                                                        style_context.add_provider(
                                                &css_provider,
                                                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                                            );
                                                        Ok(())
                                                    },
                                                )?;
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
                                    let set_css = lua.create_function(move |_, (_this, css): (mlua::Table, String)| {
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
                            },
                        )?;
                        window_table.set("add_box", add_box)?;
                    }

                    Ok(window_table)
                })?;
            globals.set("create_window", create_window)?;
        }

        Ok(())
    }

    /// Register HTTP API functions with Lua
    fn register_http_api(&self, lua: &Lua) -> Result<(), mlua::Error> {
        let globals = lua.globals();

        let fetch_json = lua.create_function(|lua_ctx, url: String| {
            info!("Fetching JSON from: {}", url);
            match reqwest::blocking::get(&url) {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<JsonValue>() {
                            Ok(json) => {
                                let lua_value =
                                    ScriptManager::serde_json_to_lua_value(lua_ctx, json);
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

        Ok(())
    }

    /// Register helper functions with Lua
    fn register_helper_functions(&self, lua: &Lua) -> Result<(), mlua::Error> {
        let globals = lua.globals();

        // Print function for debugging
        let print = lua.create_function(|_, message: String| {
            info!("[Lua] {}", message);
            Ok(())
        })?;
        globals.set("log", print)?;

        Ok(())
    }

    /// Convert a serde_json::Value to an mlua::Value
    fn serde_json_to_lua_value(lua: &Lua, json: JsonValue) -> Value {
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
            JsonValue::String(s) => Value::String(lua.create_string(&s).unwrap()),
            JsonValue::Array(arr) => {
                let lua_table = lua.create_table().unwrap();

                for (i, val) in arr.into_iter().enumerate() {
                    let lua_val = ScriptManager::serde_json_to_lua_value(lua, val);
                    lua_table.set(i + 1, lua_val).unwrap();
                }

                Value::Table(lua_table)
            }
            JsonValue::Object(obj) => {
                let lua_table = lua.create_table().unwrap();

                for (k, v) in obj {
                    let lua_val = ScriptManager::serde_json_to_lua_value(lua, v);
                    lua_table.set(k, lua_val).unwrap();
                }

                Value::Table(lua_table)
            }
        }
    }

    /// Load all scripts from the scripts directory
    pub fn load_scripts(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Get the XDG config directory for our app
        let scripts_dir = if let Some(config_dir) = dirs::config_dir() {
            let mut path = config_dir;
            path.push("swaydgets");
            path.push("scripts");
            path
        } else {
            PathBuf::from("./scripts")
        };

        // Create directory if it doesn't exist
        if !scripts_dir.exists() {
            std::fs::create_dir_all(&scripts_dir)?;

            // Create example weather widget script
            let weather_script_path = scripts_dir.join("hello.lua");
            std::fs::write(&weather_script_path, include_str!("../scripts/hello.lua"))?;
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
                if let Err(e) = self.load_script(&path) {
                    error!("Failed to load script {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }
}
