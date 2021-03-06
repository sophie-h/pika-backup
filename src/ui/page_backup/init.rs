use adw::prelude::*;

use crate::ui;

use crate::ui::prelude::*;

use super::events;

pub fn init() {
    main_ui()
        .backup_run()
        .connect_clicked(|_| Handler::run(events::on_backup_run()));

    main_ui()
        .detail_status_row()
        .add_prefix(&main_ui().status_graphic());

    // Backup details
    main_ui()
        .detail_status_row()
        .connect_activated(|_| ui::dialog_info::show());

    main_ui()
        .detail_repo_row()
        .add_prefix(&main_ui().detail_repo_icon());

    main_ui()
        .detail_repo_row()
        .connect_activated(|_| Handler::run(ui::dialog_storage::show()));

    main_ui()
        .leaflet()
        .connect_visible_child_notify(|_| events::on_stack_changed());

    main_ui()
        .detail_stack()
        .connect_visible_child_notify(|_| events::on_stack_changed());

    main_ui()
        .include_home()
        .connect_active_notify(|_| Handler::run(events::on_include_home_changed()));

    main_ui()
        .add_include()
        .connect_clicked(|_| Handler::run(events::add_include()));
    main_ui()
        .add_exclude()
        .connect_clicked(|_| Handler::run(events::add_exclude()));

    main_ui()
        .stop_backup_create()
        .connect_clicked(|_| Handler::run(events::on_stop_backup_create()));

    main_ui().status_spinner().connect_map(|s| s.start());
    main_ui().status_spinner().connect_unmap(|s| s.stop());
}
