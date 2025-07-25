// SPDX-License-Identifier: GPL-3.0-only

use cached::{cached_key, Cached, UnboundCache};
use cosmic::app::{Core, Task};
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::iced::Subscription;
use cosmic::iced::{
    platform_specific::shell::commands::popup::{destroy_popup, get_popup},
    widget::{column, row},
    window::Id,
    Alignment,
};
use cosmic::{Application, Element};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashMap;
use std::process;

use crate::applet_button::AppletButton;
use crate::applet_menu::AppletMenu;
use crate::config::{AppletButtonStyle, CosmicClassicMenuConfig, RecentApplication};
use crate::fl;
use crate::logic::apps::{desktop_files, load_apps, ApplicationCategory, Event, User, APPS_CACHE};
use crate::model::application_entry::ApplicationEntry;

/// This is the struct that represents your application.
/// It is used to define the data that will be used by your application.
#[derive(Default)]
pub struct CosmicClassicMenu {
    /// Application state which is managed by the COSMIC runtime.
    pub core: Core,
    /// The popup id.
    popup: Option<Id>,
    /// The configuration that is used to store the application settings.
    pub config: CosmicClassicMenuConfig,
    /// The search field that is used to filter the applications.
    pub search_field: String,
    /// The list of available applications that are displayed in the menu.
    pub available_applications: Vec<ApplicationEntry>,
    /// The popup type that is used to determine which popup to display.
    pub popup_type: PopupType,
    /// The selected category that is used to filter the applications.
    pub selected_category: Option<ApplicationCategory>,
    /// Currently logged user
    pub current_user: Option<User>,
}

/// This is the enum that contains all the possible variants that your application will need to transmit messages.
/// This is used to communicate between the different parts of your application.
/// If your application does not need to send messages, you can use an empty enum or `()`.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup(PopupType),
    PopupClosed(Id),
    SearchFieldInput(String),
    PowerOptionSelected(PowerAction),
    ApplicationSelected(ApplicationEntry),
    CategorySelected(ApplicationCategory),
    LaunchTool(SystemTool),
    Zbus(Result<(), zbus::Error>),
    UpdateLoggedUser(Result<User, zbus::Error>),
    FileEvent(Event)
}

#[derive(Clone, Debug)]
pub enum SystemTool {
    SystemSettings,
    SystemMonitor,
    DiskManagement,
}

impl SystemTool {
    fn perform(&self) {
        match self {
            SystemTool::SystemSettings => {
                if let Err(_) = std::process::Command::new("cosmic-settings").spawn() {
                    eprintln!("COSMIC Settings cannot be launched!");
                }
            }
            SystemTool::SystemMonitor => {
                if let Err(_) = std::process::Command::new("gnome-system-monitor").spawn() {
                    eprintln!("GNOME System Monitor cannot be launched!");
                }
            }
            SystemTool::DiskManagement => {
                if let Err(_) = std::process::Command::new("gnome-disks").spawn() {
                    eprintln!("GNOME Disks cannot be launched!");
                }
            }
        }
    }
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
    fn perform(self) -> cosmic::iced::Task<cosmic::Action<Message>> {
        let msg = |m| cosmic::Action::App(Message::Zbus(m));
        match self {
            PowerAction::Lock => cosmic::iced::Task::perform(crate::power_options::lock(), msg),
            PowerAction::Logout => {
                cosmic::iced::Task::perform(crate::power_options::log_out(), msg)
            }
            PowerAction::Reboot => {
                cosmic::iced::Task::perform(crate::power_options::restart(), msg)
            }
            PowerAction::Shutdown => {
                cosmic::iced::Task::perform(crate::power_options::shutdown(), msg)
            }
            PowerAction::Suspend => {
                cosmic::iced::Task::perform(crate::power_options::suspend(), msg)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PopupType {
    MainMenu,
    ContextMenu,
}

impl Default for PopupType {
    fn default() -> Self {
        PopupType::MainMenu
    }
}

/// Implement the `Application` trait for your application.
/// This is where you define the behavior of your application.
///
/// The `Application` trait requires you to define the following types and constants:
/// - `Executor` is the async executor that will be used to run your application's commands.
/// - `Flags` is the data that your application needs to use before it starts.
/// - `Message` is the enum that contains all the possible variants that your application will need to transmit messages.
/// - `APP_ID` is the unique identifier of your application.
impl Application for CosmicClassicMenu {
    type Executor = cosmic::executor::multi::Executor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.championpeak87.CosmicClassicMenu";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// This is the entry point of your application, it is where you initialize your application.
    ///
    /// Any work that needs to be done before the application starts should be done here.
    ///
    /// - `core` is used to passed on for you by libcosmic to use in the core of your own application.
    /// - `flags` is used to pass in any data that your application needs to use before it starts.
    /// - `Task` type is used to send messages to your application. `Task::none()` can be used to send no messages to your application.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let window = CosmicClassicMenu {
            core,
            popup: None,
            search_field: "".to_owned(),
            available_applications: crate::logic::apps::load_apps(),
            popup_type: PopupType::MainMenu,
            selected_category: Some(ApplicationCategory::ALL),
            config: CosmicClassicMenuConfig::config(),
            current_user: None,
        };

        // fetch current user asynchronously
        let fetch_current_user_task =
            Task::perform(crate::logic::apps::get_current_user(), |result| {
                cosmic::Action::App(Message::UpdateLoggedUser(result))
            });

        (window, Task::batch(vec![fetch_current_user_task]))
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    /// This is the main view of your application, it is the root of your widget tree.
    ///
    /// The `Element` type is used to represent the visual elements of your application,
    /// it has a `Message` associated with it, which dictates what type of message it can send.
    ///
    /// To get a better sense of which widgets are available, check out the `widget` module.
    fn view(&self) -> Element<Self::Message> {
        let applet_button_style = &self.config.applet_button_style;
        let panel_type = &self.core.applet.panel_type;
        let size = &self.core.applet.size;

        match applet_button_style {
            AppletButtonStyle::IconOnly => AppletButton::view_icon_only(&self),
            AppletButtonStyle::LabelOnly => AppletButton::view_label_only(&self),
            AppletButtonStyle::IconAndLabel => AppletButton::view_icon_and_label(&self),
            AppletButtonStyle::Auto => match panel_type {
                cosmic::applet::PanelType::Panel => match size {
                    cosmic::applet::Size::Hardcoded(hardcoded_size) => {
                        if hardcoded_size.0
                            < cosmic::applet::cosmic_panel_config::PanelSize::M
                                .get_applet_icon_size(false) as u16
                        {
                            AppletButton::view_label_only(&self)
                        } else {
                            AppletButton::view_icon_only(&self)
                        }
                    }
                    cosmic::applet::Size::PanelSize(panel_size) => match panel_size {
                        cosmic::applet::cosmic_panel_config::PanelSize::XS
                        | cosmic::applet::cosmic_panel_config::PanelSize::S => {
                            AppletButton::view_label_only(&self)
                        }
                        cosmic::applet::cosmic_panel_config::PanelSize::M
                        | cosmic::applet::cosmic_panel_config::PanelSize::L
                        | cosmic::applet::cosmic_panel_config::PanelSize::XL
                        | cosmic::applet::cosmic_panel_config::PanelSize::Custom(_) => {
                            AppletButton::view_icon_only(&self)
                        }
                    },
                },
                cosmic::applet::PanelType::Dock | cosmic::applet::PanelType::Other(_) => {
                    AppletButton::view_icon_only(&self)
                }
            },
        }
    }

    fn view_window(&self, _id: Id) -> Element<Self::Message> {
        match self.popup_type {
            PopupType::MainMenu => self.view_main_menu(),
            PopupType::ContextMenu => self.view_context_menu(),
        }
    }

    /// Application messages are handled here. The application state can be modified based on
    /// what message was received. Tasks may be returned for asynchronous execution on a
    /// background thread managed by the application's executor.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::TogglePopup(popup_type) => self.toggle_popup(popup_type),
            Message::PopupClosed(id) => self.close_popup(id),
            Message::SearchFieldInput(input) => self.update_search_field(&input),
            Message::PowerOptionSelected(action) => self.perform_power_action(action),
            Message::ApplicationSelected(app) => self.launch_application(app),
            Message::CategorySelected(category) => self.select_category(category),
            Message::LaunchTool(tool) => self.launch_tool(tool),
            Message::Zbus(result) => self.handle_zbus_result(result),
            Message::UpdateLoggedUser(user) => {
                self.current_user = user.ok();
                Task::none()
            },
            Message::FileEvent(event) => self.handle_event(event),
        }
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }


    fn subscription(&self) -> Subscription<Message> {
        desktop_files(Id::unique()).map(Message::FileEvent)
    }
}

impl CosmicClassicMenu {
    pub fn handle_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Changed => {
                // Invalidate the cache
                APPS_CACHE.lock().unwrap().cache_reset();
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn toggle_popup(&mut self, popup_type: PopupType) -> Task<Message> {
        self.popup_type = popup_type;
        if self.popup_type == PopupType::MainMenu {
            self.available_applications = crate::logic::apps::load_apps();
        }

        if let Some(p) = self.popup.take() {
            destroy_popup(p)
        } else {
            let new_id = Id::unique();
            self.popup.replace(new_id);

            let popup_settings = self.core.applet.get_popup_settings(
                self.core.main_window_id().unwrap(),
                new_id,
                None,
                None,
                None,
            );

            get_popup(popup_settings)
        }
    }

    fn close_popup(&mut self, id: Id) -> Task<Message> {
        self.search_field.clear();
        self.selected_category = Some(ApplicationCategory::ALL);
        self.available_applications = Vec::new();

        if self.popup.as_ref() == Some(&id) {
            self.popup = None;
        }

        Task::none()
    }

    fn update_search_field(&mut self, input: &str) -> Task<Message> {
        self.selected_category = None;
        let matcher = SkimMatcherV2::default();

        if input.is_empty() {
            self.available_applications = crate::logic::apps::load_apps();
            self.selected_category = Some(ApplicationCategory::ALL);
        } else {
            self.available_applications = crate::logic::apps::load_apps()
                .iter()
                .filter(|app| matcher.fuzzy_match(&app.name, input).is_some())
                .cloned()
                .collect();
        }
        self.search_field = input.to_string();

        Task::none()
    }

    fn perform_power_action(&mut self, action: PowerAction) -> Task<Message> {
        match action {
            PowerAction::Logout => {
                if let Err(_) = process::Command::new("cosmic-osd").arg("log-out").spawn() {
                    return action.perform();
                }
            }
            PowerAction::Reboot => {
                if let Err(_) = process::Command::new("cosmic-osd").arg("restart").spawn() {
                    return action.perform();
                }
            }
            PowerAction::Shutdown => {
                if let Err(_) = process::Command::new("cosmic-osd").arg("shutdown").spawn() {
                    return action.perform();
                }
            }
            _ => return action.perform(),
        };

        if let Some(p) = self.popup.take() {
            return destroy_popup(p);
        }

        Task::none()
    }

    fn launch_application(&mut self, app: ApplicationEntry) -> Task<Message> {
        let app_exec = app.exec.clone().unwrap();
        let env_vars: Vec<(String, String)> = std::env::vars().collect();
        let app_id = Some(app.id.clone());
        let is_terminal = app.is_terminal;

        tokio::spawn(async move {
            cosmic::desktop::spawn_desktop_exec(app_exec, env_vars, app_id.as_deref(), is_terminal)
                .await;
        });

        self.update_recent_applications(app);

        if let Some(p) = self.popup.take() {
            return destroy_popup(p);
        }
        Task::none()
    }

    fn update_recent_applications(&mut self, app: ApplicationEntry) {
        let current_recent_application = self
            .config
            .recent_applications
            .iter_mut()
            .find(|x| x.app_id == app.id);
        if let Some(recent_app) = current_recent_application {
            if recent_app.launch_count < u32::MAX {
                recent_app.launch_count += 1;
            }
        } else {
            self.config.recent_applications.push(RecentApplication {
                app_id: app.id.clone(),
                launch_count: 1,
            });
        }

        self.config
            .write_entry(CosmicClassicMenuConfig::config_handler().as_ref().unwrap())
            .expect("Failed to write recent applications config");
    }

    fn select_category(&mut self, category: ApplicationCategory) -> Task<Message> {
        self.search_field.clear();
        self.selected_category = Some(category.clone());

        if category == ApplicationCategory::ALL {
            self.available_applications = crate::logic::apps::load_apps();
        } else if category == ApplicationCategory::RECENTLY_USED {
            self.available_applications = self.get_recent_applications();
        } else {
            self.available_applications = crate::logic::apps::load_apps()
                .iter()
                .filter(|app| app.category.contains(&category.mime_name.to_string()))
                .cloned()
                .collect();
        }

        Task::none()
    }

    fn get_recent_applications(&self) -> Vec<ApplicationEntry> {
        let recent_applications: &Vec<RecentApplication> = &self.config.recent_applications;
        let all_applications_entries: HashMap<String, ApplicationEntry> =
            crate::logic::apps::load_apps()
                .into_iter()
                .map(|app| (app.id.clone(), app))
                .collect();

        // recent_applications.sort_by(|a, b| b.launch_count.cmp(&a.launch_count));
        recent_applications
            .iter()
            .filter_map(|app| all_applications_entries.get(&app.app_id).cloned())
            .collect()
    }

    fn launch_tool(&mut self, tool: SystemTool) -> Task<Message> {
        tool.perform();
        if let Some(p) = self.popup.take() {
            return destroy_popup(p);
        }
        Task::none()
    }

    fn handle_zbus_result(&self, result: Result<(), zbus::Error>) -> Task<Message> {
        if let Err(e) = result {
            eprintln!("cosmic-classic-menu ERROR: '{}'", e);
        }

        Task::none()
    }

    fn view_main_menu(&self) -> Element<Message> {
        // TODO: Implement grid view
        AppletMenu::view_main_menu_list(&self)
    }

    fn view_context_menu(&self) -> Element<Message> {
        let context_menu = column![
            cosmic::applet::menu_button(
                row![cosmic::widget::text::body(fl!("settings-label")),].align_y(Alignment::Center)
            )
            .class(cosmic::theme::Button::AppletMenu)
            .on_press(Message::LaunchTool(SystemTool::SystemSettings)),
            cosmic::applet::menu_button(
                row![cosmic::widget::text::body(fl!("system-monitor-label")),]
                    .align_y(Alignment::Center)
            )
            .class(cosmic::theme::Button::AppletMenu)
            .on_press(Message::LaunchTool(SystemTool::SystemMonitor)),
            cosmic::applet::menu_button(
                row![cosmic::widget::text::body(fl!("disks-label")),].align_y(Alignment::Center)
            )
            .class(cosmic::theme::Button::AppletMenu)
            .on_press(Message::LaunchTool(SystemTool::DiskManagement)),
        ];

        self.core.applet.popup_container(context_menu).into()
    }
}
