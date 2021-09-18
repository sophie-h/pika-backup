use crate::config;

use glib::prelude::*;
use glib::subclass::prelude::*;
use std::cell::RefCell;

pub fn list() -> Vec<config::Frequency> {
    vec![
        config::Frequency::Hourly,
        config::Frequency::Daily {
            preferred_time: chrono::NaiveTime::from_hms(0, 0, 0),
        },
        config::Frequency::Weekly {
            preferred_weekday: chrono::Weekday::Mon,
        },
        config::Frequency::Monthly { preferred_day: 1 },
    ]
}

pub fn name(obj: &glib::Object) -> String {
    if let Some(obj) = obj.downcast_ref::<FrequencyObject>() {
        obj.frequency().name()
    } else {
        String::new()
    }
}

glib::wrapper! {
    pub struct FrequencyObject(ObjectSubclass<imp::FrequencyObject>);
}

impl FrequencyObject {
    pub fn new(frequency: config::Frequency) -> Self {
        let new: Self = glib::Object::new(&[]).unwrap();
        let priv_ = imp::FrequencyObject::from_instance(&new);
        priv_.frequency.replace(frequency);
        new
    }

    pub fn frequency(&self) -> config::Frequency {
        let priv_ = imp::FrequencyObject::from_instance(self);
        (*priv_.frequency.borrow()).clone()
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct FrequencyObject {
        pub frequency: RefCell<config::Frequency>,
    }

    impl ObjectImpl for FrequencyObject {}

    #[glib::object_subclass]
    impl ObjectSubclass for FrequencyObject {
        const NAME: &'static str = "PikaBackupUiPageScheduleFrequency";
        type Type = super::FrequencyObject;
        type ParentType = glib::Object;
    }
}