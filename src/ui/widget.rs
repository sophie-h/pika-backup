mod location_tag;
mod status_icon;
mod wrap_box;

pub use location_tag::LocationTag;
pub use status_icon::StatusIcon;
pub use wrap_box::WrapBox;

use crate::ui;
use glib::prelude::*;

pub fn init() {
    ui::page_schedule::frequency::FrequencyObject::static_type();
    ui::page_schedule::prune_preset::PrunePresetObject::static_type();
    ui::page_schedule::weekday::WeekdayObject::static_type();
    ui::dialog_setup::folder_button::FolderButton::static_type();
    ui::dialog_setup::add_task::AddConfigTask::static_type();
    StatusIcon::static_type();
    WrapBox::static_type();
}
