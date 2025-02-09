use anyhow::Result;
use tray_icon::{
    menu::{Menu, MenuItem, MenuEvent},
    TrayIconBuilder, Icon,
};
use std::sync::Arc;
use std::thread;
use log::info;

const TRAY_ICON: &[u8] = include_bytes!("../../assets/icon.png");

pub struct TrayIcon {
    _tray_icon: tray_icon::TrayIcon,
    _menu: Arc<Menu>,
    _items: Vec<(MenuItem, Option<f32>)>,
}

impl TrayIcon {
    pub fn new() -> Result<Self> {
        let menu = Menu::new();
        let mut items = Vec::new();
        
        // Create menu items with static strings
        let menu_configs = [
            ("Enable Sound", None),
            ("Volume: 100%", Some(1.0)),
            ("Volume: 75%", Some(0.75)),
            ("Volume: 50%", Some(0.5)),
            ("Volume: 25%", Some(0.25)),
            ("Frequency: High (800 Hz)", Some(800.0)),
            ("Frequency: Medium (500 Hz)", Some(500.0)),
            ("Frequency: Low (300 Hz)", Some(300.0)),
            ("Decay: Fast", Some(50.0)),
            ("Decay: Medium", Some(30.0)),
            ("Decay: Slow", Some(15.0)),
            ("Quit", None),
        ];

        // Create menu items and store their IDs
        let mut menu_ids = Vec::new();
        for &(label, value) in menu_configs.iter() {
            let item = MenuItem::new(String::from(label), true, None);
            let id = item.id().0.clone();
            menu.append(&item)?;
            items.push((item, value));
            menu_ids.push((id, value));
        }

        // Create the tray icon
        let icon = load_icon()?;
        let menu = Arc::new(menu);
        let menu_ids = Arc::new(menu_ids);

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu.as_ref().clone()))
            .with_icon(icon)
            .with_tooltip("ClickClack - Mechanical Keyboard Simulator")
            .build()?;

        // Spawn a thread to handle menu events
        let menu_ids_clone = Arc::clone(&menu_ids);
        thread::spawn(move || {
            let events = MenuEvent::receiver();
            for event in events {
                let mut app_state = crate::APP_STATE.lock();
                
                if let Some((_, value)) = menu_ids_clone.iter().find(|(id, _)| *id == event.id.0) {
                    match value {
                        None => {
                            // Handle toggle and quit
                            if event.id.0 == menu_ids_clone[0].0 {
                                app_state.enabled = !app_state.enabled;
                                info!("Sound {}", if app_state.enabled { "enabled" } else { "disabled" });
                            } else {
                                std::process::exit(0);
                            }
                        },
                        Some(value) => {
                            // Handle numeric values
                            if value > &100.0 {
                                app_state.frequency = *value;
                            } else if value > &1.0 {
                                app_state.decay = *value;
                            } else {
                                app_state.volume = *value;
                            }
                        }
                    }
                }
            }
        });

        Ok(Self {
            _tray_icon: tray_icon,
            _menu: menu,
            _items: items,
        })
    }
}

fn load_icon() -> Result<Icon> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory_with_format(TRAY_ICON, image::ImageFormat::Png)?.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    Ok(Icon::from_rgba(icon_rgba, icon_width, icon_height)?)
}