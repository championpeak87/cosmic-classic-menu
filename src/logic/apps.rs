use crate::fl;
use std::{fmt::Display, string::String, sync::Arc};

use cosmic::desktop::DesktopEntryData;
use freedesktop_desktop_entry::DesktopEntry;
use serde::{Deserialize, Serialize};

pub fn load_apps() -> Vec<Arc<DesktopEntryData>> {
    let mut locale = current_locale::current_locale().ok();
    if let Some(_locale) = locale {
        // TODO: Temporary fix for the locale issue
        locale = Some(_locale.split_at(2).0.to_string());
    }
    let mut all_entries: Vec<Arc<DesktopEntryData>> =
        cosmic::desktop::load_applications(locale.as_slice(), false, None)
            .into_iter()
            .map(Arc::new)
            .collect();
    all_entries.sort_by(|a, b| a.name.cmp(&b.name));

    all_entries
}

pub fn get_comment(app: &Arc<DesktopEntryData>) -> Option<String> {
    if let Some(path) = &app.path {
        let mut locale = current_locale::current_locale().ok();
        if let Some(_locale) = locale {
            // TODO: Temporary fix for the locale issue
            locale = Some(_locale.split_at(2).0.to_string());
        }
        let desktop_entry = DesktopEntry::from_path(path, Some(locale.as_slice()));

        if let Ok(entry) = desktop_entry {
            return Some(
                entry
                    .comment(locale.as_slice())
                    .unwrap_or_default()
                    .into_owned(),
            );
        }
    }

    None
}

pub async fn get_current_user() -> Result<User, zbus::Error> {
    let uid = users::get_current_uid() as u64;

    let conn = zbus::Connection::system().await?;
    let user = accounts_zbus::UserProxy::from_uid(&conn, uid).await?;

    // Fetch all fields concurrently
    let (username, user_realname, profile_picture, uid, user_home, user_shell) = tokio::join!(
        user.user_name(),
        user.real_name(),
        user.icon_file(),
        user.uid(),
        user.home_directory(),
        user.shell()
    );

    Ok(User {
        username: username?,
        user_realname: user_realname?,
        profile_picture: profile_picture?,
        uid: uid?,
        user_home: user_home?,
        user_shell: user_shell?,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub user_realname: String,
    pub profile_picture: String,
    pub uid: u64,
    pub user_home: String,
    pub user_shell: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ApplicationCategory {
    All,
    RecentlyUsed,
    Audio,
    Video,
    Development,
    Games,
    Graphics,
    Network,
    Office,
    Science,
    Settings,
    System,
    Utility,
}

impl ApplicationCategory {
    pub fn get_display_name(self) -> String {
        match self {
            ApplicationCategory::All => fl!("all-applications"),
            ApplicationCategory::RecentlyUsed => fl!("recently-used"),
            ApplicationCategory::Audio => fl!("audio"),
            ApplicationCategory::Video => fl!("video"),
            ApplicationCategory::Development => fl!("development"),
            ApplicationCategory::Games => fl!("games"),
            ApplicationCategory::Graphics => fl!("graphics"),
            ApplicationCategory::Network => fl!("network"),
            ApplicationCategory::Office => fl!("office"),
            ApplicationCategory::Science => fl!("science"),
            ApplicationCategory::Settings => fl!("settings"),
            ApplicationCategory::System => fl!("system"),
            ApplicationCategory::Utility => fl!("utility"),
        }
    }

    pub fn get_icon_name(self) -> &'static str {
        match self {
            ApplicationCategory::All => "open-menu-symbolic",
            ApplicationCategory::RecentlyUsed => "document-open-recent-symbolic",
            ApplicationCategory::Audio => "applications-audio-symbolic",
            ApplicationCategory::Video => "applications-video-symbolic",
            ApplicationCategory::Development => "applications-engineering-symbolic",
            ApplicationCategory::Games => "applications-games-symbolic",
            ApplicationCategory::Graphics => "applications-graphics-symbolic",
            ApplicationCategory::Network => "network-workgroup-symbolic",
            ApplicationCategory::Office => "applications-office-symbolic",
            ApplicationCategory::Science => "applications-science-symbolic",
            ApplicationCategory::Settings => "preferences-system-symbolic",
            ApplicationCategory::System => "applications-system-symbolic",
            ApplicationCategory::Utility => "applications-utilities-symbolic",
        }
    }

    pub fn get_mime_name(self) -> &'static str {
        match self {
            ApplicationCategory::All => "",
            ApplicationCategory::RecentlyUsed => "",
            ApplicationCategory::Audio => "Audio",
            ApplicationCategory::Video => "Video",
            ApplicationCategory::Development => "Development",
            ApplicationCategory::Games => "Game",
            ApplicationCategory::Graphics => "Graphics",
            ApplicationCategory::Network => "Network",
            ApplicationCategory::Office => "Office",
            ApplicationCategory::Science => "Science",
            ApplicationCategory::Settings => "Settings",
            ApplicationCategory::System => "System",
            ApplicationCategory::Utility => "Utility",
        }
    }
}

impl Display for ApplicationCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_mime_name())
    }
}
