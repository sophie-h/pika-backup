use std::error::Error;
use std::io::prelude::*;

use gdk_pixbuf::prelude::*;
use gio::prelude::*;
use gtk::prelude::*;

use crate::borg;
use crate::shared;
use crate::ui;
use crate::ui::globals::*;
use crate::ui::prelude::*;

mod about;
mod archives;
#[allow(dead_code)]
mod builder;
mod detail;
mod encryption_password;
mod globals;
mod new_backup;
mod overview;
pub mod prelude;
mod storage;
mod utils;

pub fn main() {
    if std::env::args().find(|x| x == "--syslog").is_none() {
        pretty_env_logger::try_init_timed()
            .unwrap_or_else(|e| eprintln!("!!! Log initialization failed: {}", e));
    } else {
        syslog::init(syslog::Facility::LOG_USER, log::LevelFilter::Trace, None)
            .unwrap_or_else(|e| eprintln!("!!! Syslog initialization failed: {}", e));
    }
    debug!("Logging initialized");

    gtk::init().expect("Failed to gtk::init()");
    let none: Option<&gio::Cancellable> = None;
    gtk_app()
        .register(none)
        .expect("Failed to gtk::Application::register()");
    gtk_app().connect_activate(init);
    gtk_app().connect_shutdown(on_shutdown);

    crate::globals::init();

    // Ctrl-C handling
    let (send, recv) = std::sync::mpsc::channel();
    // Use channel to call GtkApplicaton from main thread
    gtk::timeout_add(100, move || {
        if recv.try_recv().is_ok() {
            on_ctrlc();
        }
        Continue(true)
    });
    ctrlc::set_handler(move || {
        send.send(())
            .expect("Could not send Ctrl-C to main thread.");
    })
    .expect("Error setting Ctrl-C handler");

    gtk_app().run(&[]);
}

fn on_ctrlc() {
    gtk_app().release();
}

fn on_shutdown(app: &gtk::Application) {
    app.mark_busy();
    IS_SHUTDOWN.swap(std::sync::Arc::new(true));
    while !ACTIVE_MOUNTS.load().is_empty() {
        for backup_id in ACTIVE_MOUNTS.load().iter() {
            let config = &SETTINGS.load().backups[backup_id];
            if borg::Borg::new(config.clone()).umount().is_ok() {
                ACTIVE_MOUNTS.update(|mounts| {
                    mounts.remove(backup_id);
                });
            }
        }
    }

    debug!("Good bye!");
}

fn init(_app: &gtk::Application) {
    load_config();

    if let Some(screen) = gdk::Screen::get_default() {
        let provider = gtk::CssProvider::new();
        ui::utils::dialog_catch_err(
            provider.load_from_data(include_bytes!("../data/style.css")),
            "Could not load style sheet.",
        );
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let loader = gdk_pixbuf::PixbufLoader::new();
    loader
        .write(include_bytes!(concat!(data_dir!(), "/app.svg")))
        .unwrap_or_else(|e| error!("loader.write() failed: {}", e));
    loader
        .close()
        .unwrap_or_else(|e| error!("loader.close() failed: {}", e));
    if let Some(icon) = loader.get_pixbuf() {
        gtk::Window::set_default_icon(&icon);
    }

    init_actions();
    init_timeouts();
    borg::init_device_listening();

    ui::archives::init();
    ui::detail::init();
    ui::overview::main();

    gtk_app().set_accels_for_action("app.quit", &["<Ctrl>Q"]);

    main_ui()
        .window()
        .connect_delete_event(|_, _| gtk::Inhibit(!is_quit_okay()));

    gtk_app().add_window(&main_ui().window());

    main_ui().window().show_all();
    main_ui().window().present();
}

fn init_timeouts() {
    gtk::timeout_add(1000, move || {
        let inhibit_cookie = INHIBIT_COOKIE.get();

        if is_backup_running() {
            if inhibit_cookie.is_none() {
                INHIBIT_COOKIE.update(|c| {
                    *c = Some(gtk_app().inhibit(
                        Some(&main_ui().window()),
                        gtk::ApplicationInhibitFlags::LOGOUT
                            | gtk::ApplicationInhibitFlags::SUSPEND,
                        Some("Backup in Progress"),
                    ))
                });
            }
        } else if let Some(cookie) = inhibit_cookie {
            gtk_app().uninhibit(cookie);
            INHIBIT_COOKIE.update(|c| *c = None);
        }

        Continue(true)
    });
}

/// checks if there is any running backup
fn is_backup_running() -> bool {
    !BACKUP_COMMUNICATION.load().is_empty()
}

/// Checks if it's okay to quit and ask the user if necessary
fn is_quit_okay() -> bool {
    if is_backup_running() {
        ui::utils::dialog_yes_no(
            "Backup is still running. Do you want to abort the running backup?",
        )
    } else {
        true
    }
}

fn init_actions() {
    let action = gio::SimpleAction::new("detail", glib::VariantTy::new("s").ok());
    action.connect_activate(|_, backup_id: _| {
        if let Some(backup_id) = backup_id.and_then(|v| v.get_str()) {
            ui::detail::view_backup_conf(&backup_id.to_string());
            main_ui().window().present();
        }
    });
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("about", None);
    action.connect_activate(|_, _| ui::about::show());
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("quit", None);
    action.connect_activate(|_, _| {
        if is_quit_okay() {
            gtk_app().quit()
        }
    });
    gtk_app().add_action(&action);

    let action = gio::SimpleAction::new("archives", None);
    action.connect_activate(|_, _| ui::archives::show());
    gtk_app().add_action(&action);
}

fn config_path() -> std::path::PathBuf {
    let mut path = crate::globals::HOME_DIR.clone();
    path.push(env!("CARGO_PKG_NAME"));
    std::fs::create_dir(&path).unwrap_or_default();
    path.push("config.json");

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        ui::utils::dialog_catch_err(file.write_all(b"{ }"), "Could not create empty config file");
    }

    path
}

fn load_config_e() -> Result<(), Box<dyn Error>> {
    let file = std::fs::File::open(config_path())?;
    let conf: shared::Settings = serde_json::de::from_reader(file)?;
    SETTINGS.update(|s| *s = conf.clone());
    Ok(())
}

fn load_config() {
    utils::dialog_catch_err(load_config_e(), "Could not load config.");
}

fn write_config_e() -> Result<(), Box<dyn Error>> {
    let settings: &shared::Settings = &SETTINGS.load();
    let file = std::fs::File::create(config_path())?;
    serde_json::ser::to_writer_pretty(file, settings)?;
    Ok(())
}

fn write_config() {
    utils::dialog_catch_err(write_config_e(), "Could not write config.");
}
