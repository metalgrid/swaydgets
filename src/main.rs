use gtk::Application;
use gtk::prelude::*;
use log::info;

mod calendar;
mod dock;

fn main() {
    env_logger::init();
    let app = Application::builder()
        .application_id("com.example.sway_widgets")
        .build();

    info!("Starting Sway widgets application");

    app.connect_activate(|app| {
        // Create the calendar widget
        calendar::create_calendar(app);

        // Create the dock
        dock::create_dock(app);
    });

    app.run();
}
