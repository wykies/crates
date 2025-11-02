use std::path::Path;

mod discord;
mod email;

pub fn send_notification(msg: &str, config_folder: &Path) -> anyhow::Result<()> {
    match discord::Discord::send(msg, config_folder) {
        Ok(()) => return Ok(()),
        Err(e) => eprintln!("{e:?}"),
    };

    email::Email::send(msg, config_folder)
}
