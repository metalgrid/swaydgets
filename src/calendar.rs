use chrono::{Datelike, Local};
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Button, Calendar, Orientation};
use gtk_layer_shell::{Edge, Layer, LayerShell};
use log::info;

pub fn create_calendar(app: &Application) {
    info!("Creating calendar widget");

    // Create window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Sway Calendar")
        .default_width(300)
        .default_height(250)
        .build();

    // Layer shell setup
    window.init_layer_shell();
    window.set_layer(Layer::Background);
    window.auto_exclusive_zone_enable();
    window.set_size_request(300, 250);
    window.set_layer_shell_margin(Edge::Top, 25);
    window.set_layer_shell_margin(Edge::Left, 25);

    // Set app paintable for transparent background
    window.set_app_paintable(true);
    window.connect_draw(|_, cr| {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.paint().unwrap();
        false.into()
    });

    // Main container
    let vbox = gtk::Box::new(Orientation::Vertical, 10);
    vbox.set_margin(12);

    // Get current date
    let today = Local::now().date_naive();
    let year = today.year() as i32;
    let month = today.month() as i32 - 1; // Calendar months are 0-indexed
    let day = today.day() as i32;

    // Create calendar widget
    let calendar = Calendar::new();
    calendar.set_display_options(
        gtk::CalendarDisplayOptions::SHOW_HEADING
            | gtk::CalendarDisplayOptions::SHOW_DAY_NAMES
            | gtk::CalendarDisplayOptions::SHOW_WEEK_NUMBERS,
    );

    // Set calendar to start week on Monday (1 = Monday, 0 = Sunday)
    calendar.set_property("show-details", &false);
    // calendar.set_property("start-week-day", &1i32);

    // Set current date
    calendar.select_month(month as u32, year as u32);
    calendar.select_day(day as u32);
    calendar.mark_day(day as u32);

    // Navigation buttons
    let hbox = gtk::Box::new(Orientation::Horizontal, 5);
    hbox.set_halign(gtk::Align::Center);

    let prev_button = Button::with_label("◀ Previous");
    let next_button = Button::with_label("Next ▶");
    let today_button = Button::with_label("Today");

    hbox.pack_start(&prev_button, true, true, 5);
    hbox.pack_start(&today_button, true, true, 5);
    hbox.pack_start(&next_button, true, true, 5);

    // Calendar navigation logic
    let calendar_clone = calendar.clone();
    prev_button.connect_clicked(move |_| {
        let (year, month, _) = calendar_clone.date();
        if month == 0 {
            calendar_clone.select_month(11, year - 1);
        } else {
            calendar_clone.select_month(month - 1, year);
        }
    });

    let calendar_clone = calendar.clone();
    next_button.connect_clicked(move |_| {
        let (year, month, _) = calendar_clone.date();
        if month == 11 {
            calendar_clone.select_month(0, year + 1);
        } else {
            calendar_clone.select_month(month + 1, year);
        }
    });

    let calendar_clone = calendar.clone();
    today_button.connect_clicked(move |_| {
        let today = Local::now().date_naive();
        let year = today.year() as u32;
        let month = today.month() as u32 - 1;
        let day = today.day() as u32;
        calendar_clone.select_month(month, year);
        calendar_clone.select_day(day);
    });

    // Apply CSS styling
    let provider = gtk::CssProvider::new();
    provider
        .load_from_data(
            b"
            box { 
                background-color: rgba(40, 40, 40, 0.9); 
                border-radius: 12px;
                padding: 10px;
            }
            
            calendar {
                color: white;
                background: rgba(60, 60, 60, 0.7);
                border-radius: 8px;
                padding: 5px;
            }
            
            calendar:selected {
                background-color: #3584e4;
                color: white;
                border-radius: 20px;
            }
            
            calendar.header {
                color: white;
                font-weight: bold;
            }
            
            button {
                background-color: rgba(70, 70, 70, 0.8);
                color: white;
                border-radius: 4px;
                border: none;
                padding: 5px;
            }
            
            button:hover {
                background-color: rgba(90, 90, 90, 0.8);
            }
        ",
        )
        .unwrap();

    gtk::StyleContext::add_provider_for_screen(
        &gtk::gdk::Screen::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Add widgets to layout
    vbox.pack_start(&calendar, true, true, 0);
    vbox.pack_end(&hbox, false, false, 5);

    window.add(&vbox);
    window.show_all();
}
