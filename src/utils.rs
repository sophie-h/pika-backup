use crate::config;
use crate::prelude::*;

use futures::StreamExt;
use gio::prelude::*;
use std::convert::TryInto;

pub fn prepare_config_file<V: serde::Serialize>(
    filename: &str,
    default_value: V,
) -> std::io::Result<std::path::PathBuf> {
    let mut path = glib::user_config_dir();
    path.push(env!("CARGO_PKG_NAME"));
    std::fs::create_dir_all(&path)?;
    path.push(filename);

    if let Ok(file) = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        serde_json::ser::to_writer_pretty(file, &default_value)?;
    }

    Ok(path)
}

pub trait LookupConfigId {
    type Item;

    fn get_result_mut(
        &mut self,
        key: &ConfigId,
    ) -> Result<&mut Self::Item, config::error::BackupNotFound>;
    fn get_result(&self, key: &ConfigId) -> Result<&Self::Item, config::error::BackupNotFound>;
}

pub fn mount_uuid(mount: &gio::Mount) -> Option<String> {
    let volume = mount.volume();

    volume
        .as_ref()
        .and_then(gio::Volume::uuid)
        .or_else(|| volume.as_ref().and_then(|v| v.identifier("uuid")))
        .as_ref()
        .map(std::string::ToString::to_string)
}

pub async fn listen_remote_app_running(app_id: &str, update: fn(bool)) -> Result<(), zbus::Error> {
    let conn = zbus::Connection::session().await?;
    let proxy = zbus::fdo::DBusProxy::new(&conn).await?;

    let has_owner = proxy.name_has_owner(app_id.try_into().unwrap()).await?;
    update(has_owner);

    let mut stream = proxy.receive_name_owner_changed().await?;

    while let Some(signal) = stream.next().await {
        let args = signal.args()?;
        if args.name == app_id {
            debug!(
                "Remote app '{}' owner status changed: {:?}",
                args.name, args.new_owner
            );
            update(args.new_owner.is_some());
        }
    }

    Ok(())
}
