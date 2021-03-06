use gtk::prelude::*;

use crate::ui;

use crate::config;
use crate::ui::prelude::*;

pub struct Ask {
    repo: config::Repository,
    purpose: String,
}

impl Ask {
    pub const fn new(repo: config::Repository, purpose: String) -> Self {
        Self { repo, purpose }
    }

    pub async fn run(&self) -> Option<config::Password> {
        let ui = ui::builder::DialogEncryptionPassword::new();

        ui.dialog().set_transient_for(Some(&main_ui().window()));

        ui.description().set_text(&gettextf(
            "The operation “{}” requires the encryption password of the repository on “{}”.",
            &[&self.purpose, &self.repo.location()],
        ));

        ui.password().grab_focus();

        ui.dialog().present();

        let response = ui.dialog().run_future().await;
        let password = config::Password::new(ui.password().text().to_string());
        ui.dialog().close();
        ui.dialog().hide();

        if gtk::ResponseType::Apply == response {
            Some(password)
        } else {
            None
        }
    }
}
