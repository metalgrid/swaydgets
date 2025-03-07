use gtk::Application;
use gtk::prelude::*;
use log::info;

mod calendar;
mod config;
mod dock;

fn main() {
    env_logger::init();
    
    // Load configuration
    let config = config::load_config();
    info!("Configuration loaded: {:?}", config);
    
    let app = Application::builder()
        .application_id("com.example.sway_widgets")
        .build();

    info!("Starting Sway widgets application");

    app.connect_activate(move |app| {
        // Create the calendar widget if enabled
        if config.calendar.enabled {
            calendar::create_calendar(app, &config.calendar);
        }

        // Create the dock if enabled
        if config.dock.enabled {
            dock::create_dock(app, &config.dock);
        }
    });

    app.run();
}
