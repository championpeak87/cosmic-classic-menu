use cosmic::app::Core;
use cosmic::cosmic_config::Config;
use cosmic::cosmic_theme::Spacing;
use cosmic::desktop::DesktopEntryData;
use cosmic::iced::{
    platform_specific::shell::commands::popup::{destroy_popup, get_popup},
    widget::{column, row},
    window::Id,
    Alignment, Length, Limits, Task,
};
use cosmic::iced_runtime::core::window;
use cosmic::iced_widget::button;
use cosmic::widget::menu::menu_button;
use cosmic::widget::{container, text_input, Column};
use cosmic::widget::{scrollable, text};
use cosmic::{theme, Element};
use freedesktop_desktop_entry::DesktopEntry;
use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::Debug;
use std::{process, env};
use std::sync::Arc;

use crate::logic::{available_categories, load_apps};
use crate::power_options::{lock, log_out, restart, shutdown, suspend};

const ID: &str = "com.championpeak87.cosmic-classic-menu";
const CONFIG_VERS: u64 = 1;
const POPUP_MAX_WIDTH: f32 = 400.0;
const POPUP_MIN_WIDTH: f32 = 400.0;
const POPUP_MAX_HEIGHT: f32 = 600.0;
const POPUP_MIN_HEIGHT: f32 = 600.0;

/// Holds the applet's state
pub struct Window {
    core: Core,
    config: Config,
    popup: Option<Id>,
    search_field: String,
    available_categories: HashSet<&'static str>,
    available_applications: Vec<Arc<DesktopEntryData>>,
    all_applications: Vec<Arc<DesktopEntryData>>,
    popup_type: PopupType,
}

/// Messages to be sent to the Libcosmic Update function
#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup(PopupType),
    PopupClosed(Id),
    SearchFieldInput(String),
    PowerOptionSelected(PowerAction),
    ApplicationSelected(Arc<DesktopEntryData>),
    CategorySelected(String),
    ShowConfig,
    OpenDiskManagement,
    OpenSystemConfig,
    OpenSystemMonitor,
    Zbus(Result<(), zbus::Error>),
}

#[derive(Clone, Debug)]
pub enum PowerAction {
    Shutdown,
    Logout,
    Lock,
    Reboot,
    Suspend,
}

impl PowerAction {
    fn perform(self) -> cosmic::iced::Task<cosmic::app::Message<Message>> {
        let msg = |m| cosmic::app::message::app(Message::Zbus(m));
        match self {
            PowerAction::Lock => cosmic::iced::Task::perform(lock(), msg),
            PowerAction::Logout => cosmic::iced::Task::perform(log_out(), msg),
            PowerAction::Reboot => cosmic::iced::Task::perform(restart(), msg),
            PowerAction::Shutdown => cosmic::iced::Task::perform(shutdown(), msg),
            PowerAction::Suspend => cosmic::iced::Task::perform(suspend(), msg),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PopupType {
    MainMenu,
    ContextMenu,
}

impl cosmic::Application for Window {
    type Executor = cosmic::executor::multi::Executor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Window, Task<cosmic::app::Message<Message>>) {
        // Set the start up state of the application using the above variables
        let all_apps = load_apps();

        let window = Window {
            core,
            config: Config::new(ID, CONFIG_VERS).unwrap(),
            popup: None,
            search_field: String::new(),
            available_applications: all_apps.clone(),
            available_categories: available_categories(),
            all_applications: all_apps,
            popup_type: PopupType::MainMenu,
        };

        // Return the state and no Task
        (window, Task::none())
    }

    // The function that is called when the applet is closed
    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    // Libcosmic's update function
    fn update(&mut self, message: Self::Message) -> Task<cosmic::app::Message<Self::Message>> {
        match message {
            Message::TogglePopup(popup_type) => {
                self.popup_type = popup_type;
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);

                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );

                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(POPUP_MAX_WIDTH)
                        .min_width(POPUP_MIN_WIDTH)
                        .min_height(POPUP_MIN_HEIGHT)
                        .max_height(POPUP_MAX_HEIGHT);

                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                // delete search field
                self.search_field = "".to_string();
                self.available_applications = self.all_applications.clone();

                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::SearchFieldInput(input) => {
                if input.is_empty() {
                    self.available_applications = self.all_applications.clone();
                } else {
                    self.available_applications = self
                        .all_applications
                        .iter()
                        .filter(|x| {
                            x.name
                                .to_lowercase()
                                .starts_with(input.to_lowercase().as_str())
                        })
                        .cloned()
                        .collect();
                }
                self.search_field = input;
            }
            Message::PowerOptionSelected(action) => {
                match action {
                    PowerAction::Logout => {
                        if let Err(_err) = process::Command::new("cosmic-osd").arg("log-out").spawn()
                        {
                            return PowerAction::Logout.perform();
                        }
                    }
                    PowerAction::Reboot => {
                        if let Err(_err) = process::Command::new("cosmic-osd").arg("restart").spawn()
                        {
                            return PowerAction::Reboot.perform();
                        }
                    }
                    PowerAction::Shutdown => {
                        if let Err(_err) =
                            process::Command::new("cosmic-osd").arg("shutdown").spawn()
                        {
                            return PowerAction::Shutdown.perform();
                        }
                    }
                    a => return a.perform(),
                };
            }
            Message::ApplicationSelected(app) => {
                let app_exec: String = app.exec.to_owned().unwrap();
                let env_vars: Vec<(String, String)> = env::vars().collect();
                let app_id: Option<String> = Some(app.id.clone());
                
                tokio::spawn(async move {
                    cosmic::desktop::spawn_desktop_exec(app_exec, env_vars, app_id.as_deref()).await;
                });

                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
            }
            Message::CategorySelected(category) => {
                // delete search field
                self.search_field = "".to_string();

                self.available_applications = load_apps()
                    .into_iter()
                    .filter(|app| app.categories.contains(&category))
                    .collect();
            }
            Message::ShowConfig => todo!("Configuration not yet implemented"),
            Message::OpenDiskManagement => todo!("Disk management not yet implemented"),
            Message::OpenSystemConfig => todo!("System config not yet implemented"),
            Message::OpenSystemMonitor => todo!("System monitor not yet implemented"),
            Message::Zbus(result) => {
                if let Err(e) = result {
                    eprintln!("cosmic-classic-menu ERROR: '{}'", e);
                }
            }
        }
        Task::none()
    }

    // Libcosmic's view function
    fn view(&self) -> Element<Self::Message> {
        self.core
            .applet
            .autosize_window(
                cosmic::widget::mouse_area(
                    cosmic::widget::button::custom(
                        row![
                            cosmic::widget::icon::from_name("com.system76.CosmicAppLibrary"),
                            text("Menu"),
                        ]
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::TogglePopup(PopupType::MainMenu))
                    .class(cosmic::theme::Button::AppletIcon),
                )
                .on_right_press(Message::TogglePopup(PopupType::ContextMenu)),
            )
            .into()
    }

    // Libcosmic's applet view_window function
    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        let Spacing {
            space_xxs,
            space_s,
            
            space_l,
            ..
        } = theme::active().cosmic().spacing;

        match self.popup_type {
            PopupType::MainMenu => {
                let power_menu = container(
                    row![
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "system-log-out-symbolic"
                        ))
                        .on_press(Message::PowerOptionSelected(PowerAction::Logout))
                        .icon_size(space_l)
                        .height(space_l)
                        .width(space_l),
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "system-suspend-symbolic"
                        ))
                        .on_press(Message::PowerOptionSelected(PowerAction::Suspend))
                        .icon_size(space_l)
                        .height(space_l)
                        .width(space_l),
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "system-lock-screen-symbolic"
                        ))
                        .on_press(Message::PowerOptionSelected(PowerAction::Lock))
                        .icon_size(space_l)
                        .height(space_l)
                        .width(space_l),
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "system-restart-symbolic"
                        ))
                        .on_press(Message::PowerOptionSelected(PowerAction::Reboot))
                        .icon_size(space_l)
                        .height(space_l)
                        .width(space_l),
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "system-shutdown-symbolic"
                        ))
                        .on_press(Message::PowerOptionSelected(PowerAction::Shutdown))
                        .icon_size(space_l)
                        .height(space_l)
                        .width(space_l),
                    ]
                    .padding([space_xxs, space_s])
                    .align_y(Alignment::Center),
                )
                .width(Length::Fill)
                .align_x(Alignment::End);

                let search_field = text_input("Search app", &self.search_field)
                    .on_input(Message::SearchFieldInput)
                    .padding([space_xxs, space_s]);

                let app_list: cosmic::widget::Column<Message> = self
                    .available_applications
                    .iter()
                    .fold(cosmic::widget::column(), |col, app| {
                        let comment = match &app.path {
                            Some(path) => {
                                let locale = current_locale::current_locale().ok();
                                let desktop_entry =
                                    DesktopEntry::from_path(path, Some(locale.as_slice()));

                                if let Ok(entry) = desktop_entry {
                                    match entry.comment(locale.as_slice()).unwrap_or(Cow::from(""))
                                    {
                                        Cow::Borrowed(x) => x.to_string(),
                                        Cow::Owned(x) => x,
                                    }
                                } else {
                                    "".to_string()
                                }
                            }
                            None => "".to_string(),
                        };
                        col.push(
                            cosmic::widget::button::custom(container(
                                row![
                                    app.icon
                                        .as_cosmic_icon()
                                        .width(Length::Fixed(space_l.into()))
                                        .height(Length::Fixed(space_l.into())),
                                    column![text(app.name.clone()), text(comment).size(8.0)]
                                        .padding([space_xxs, space_s])
                                ]
                                .align_y(Alignment::Center),
                            ))
                            .width(Length::Fill)
                            .on_press(Message::ApplicationSelected(app.clone())),
                        )
                        .width(Length::Fill)
                    });
                let places_list = self.available_categories.clone().into_iter().fold(
                    cosmic::widget::column(),
                    |col, category| {
                        col.push(
                            button(category)
                                .on_press(Message::CategorySelected(category.to_string()))
                                .width(Length::Fill),
                        )
                        .width(Length::Fill)
                    },
                );

                let menu_layout = column![
                    power_menu,
                    search_field,
                    cosmic::applet::padded_control(cosmic::widget::divider::horizontal::default())
                        .padding([space_xxs, space_s])
                        .width(Length::Fill),
                    row![
                        scrollable(app_list).width(Length::FillPortion(20)),
                        cosmic::applet::padded_control(cosmic::widget::divider::vertical::default())
                            .padding([space_xxs,space_xxs,space_s,space_s]),
                        scrollable(places_list).width(Length::FillPortion(10))
                    ]
                ]
                .padding([space_xxs, space_s]);

                return self
                    .core
                    .applet
                    .popup_container(menu_layout)
                    .max_height(POPUP_MAX_HEIGHT)
                    .min_height(POPUP_MAX_HEIGHT)
                    .into();
            }
            PopupType::ContextMenu => {
                let content = vec![
                    menu_button(vec![
                        row![cosmic::widget::text::body("Menu configuration"),]
                            .align_y(Alignment::Center)
                            .into(),
                    ])
                    .on_press(Message::ShowConfig)
                    .into(),
                    cosmic::applet::padded_control(cosmic::widget::divider::horizontal::default())
                        .padding([space_xxs, space_s])
                        .into(),
                    menu_button(vec![row![cosmic::widget::text::body("System Settings"),]
                        .align_y(Alignment::Center)
                        .into()])
                    .on_press(Message::OpenSystemConfig)
                    .into(),
                    menu_button(vec![row![cosmic::widget::text::body("System monitor"),]
                        .align_y(Alignment::Center)
                        .into()])
                    .on_press(Message::OpenSystemMonitor)
                    .into(),
                    menu_button(vec![row![cosmic::widget::text::body("Disks"),]
                        .align_y(Alignment::Center)
                        .into()])
                    .on_press(Message::OpenDiskManagement)
                    .into(),
                    menu_button(vec![row![cosmic::widget::text::body("Power options"),]
                        .align_y(Alignment::Center)
                        .into()])
                    .on_press(Message::ShowConfig)
                    .into(),
                ];

                return self
                    .core
                    .applet
                    .popup_container(Column::with_children(content).padding([space_xxs, space_s]))
                    .into();
            }
        }
    }
}
