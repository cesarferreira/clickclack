use anyhow::Result;
use cocoa::base::{id, nil, NO};
use cocoa::foundation::{NSString, NSAutoreleasePool};
use cocoa::appkit::{NSStatusBar, NSMenu, NSMenuItem};
use objc::runtime::{Object, Class};
use objc::{msg_send, sel, sel_impl, class};
use objc::runtime::Sel;
use std::fs;
use std::path::PathBuf;
use log::{info, debug, error};
use std::io::{self, Read, Cursor};

const STATUS_ITEM_LENGTH: f64 = -1.0;

pub struct TrayIcon {
    status_item: id,
    menu: id,
    pool: id,
    target: id,
}

impl TrayIcon {
    pub fn new() -> Result<Self> {
        unsafe {
            let pool = NSAutoreleasePool::new(nil);
            
            let target = {
                let cls = register_menu_target_class();
                let target: id = msg_send![cls, new];
                target
            };
            
            let status_bar = NSStatusBar::systemStatusBar(nil);
            let status_item = status_bar.statusItemWithLength_(STATUS_ITEM_LENGTH);
            if status_item == nil {
                return Err(anyhow::anyhow!("Failed to create status item"));
            }

            // Set the title for the status item
            let title = NSString::alloc(nil).init_str("🎹");
            let _: () = msg_send![status_item, setTitle:title];
            let _: () = msg_send![title, release];

            // Create and retain the menu
            let menu = NSMenu::new(nil);
            if menu == nil {
                return Err(anyhow::anyhow!("Failed to create menu"));
            }
            let _: () = msg_send![menu, setAutoenablesItems: NO];
            let _: () = msg_send![menu, retain];

            // Store menu in target
            let _: () = msg_send![target, setMenu:menu];

            // Enable/Disable toggle
            let enabled = {
                let state = crate::APP_STATE.lock();
                state.enabled
            };
            add_menu_item(menu, "Enable Sound", "toggleSound", enabled, target);
            add_separator(menu);

            // Volume controls
            let current_volume = {
                let state = crate::APP_STATE.lock();
                state.volume
            };

            add_menu_item(menu, "Volume", "", false, target);
            add_menu_item(menu, "  100%", "setVolume100", (current_volume - 1.0).abs() < 0.01, target);
            add_menu_item(menu, "  75%", "setVolume75", (current_volume - 0.75).abs() < 0.01, target);
            add_menu_item(menu, "  50%", "setVolume50", (current_volume - 0.5).abs() < 0.01, target);
            add_menu_item(menu, "  25%", "setVolume25", (current_volume - 0.25).abs() < 0.01, target);
            add_separator(menu);

            // Keyboard profiles
            let current_profile = {
                let state = crate::APP_STATE.lock();
                state.keyboard_profile.clone()
            };

            add_menu_item(menu, "Keyboard Profile", "", false, target);
            
            // Read all keyboard profiles from the directory
            let keyboards_dir = get_assets_dir().join("keyboards");
            let profiles = fs::read_dir(&keyboards_dir)
                .unwrap_or_else(|_| panic!("Failed to read keyboards directory at {:?}", keyboards_dir))
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| !name.starts_with('.') && name != "test-profile")
                .collect::<Vec<_>>();

            for profile in &profiles {
                let display_name = profile.replace("-v1", "").replace('-', " ");
                add_menu_item(menu, &format!("  {}", display_name), &format!("setProfile_{}", profile), current_profile == *profile, target);
            }

            add_separator(menu);
            add_menu_item(menu, "Quit", "quit", false, target);

            // Set the menu to the status item and retain the status item
            let _: () = msg_send![status_item, setMenu:menu];
            let _: () = msg_send![status_item, retain];

            Ok(Self {
                status_item,
                menu,
                pool,
                target,
            })
        }
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.target, release];
            let _: () = msg_send![self.status_item, release];
            let _: () = msg_send![self.menu, release];
            let _: () = msg_send![self.pool, drain];
        }
    }
}

unsafe fn create_menu_item(title: &str, action: &str, checked: bool, target: id) -> id {
    info!("Creating menu item: '{}' with action: '{}'", title, action);
    let title = NSString::alloc(nil).init_str(title);
    
    // Convert the action string to a selector using runtime functions
    let selector = if action.is_empty() {
        info!("Empty action, creating menu item without selector");
        None
    } else {
        info!("Registering selector for action: {}", action);
        Some(Sel::register(action))
    };
    
    let item = NSMenuItem::alloc(nil);
    let empty_string = NSString::alloc(nil).init_str("");
    
    match selector {
        Some(sel) => {
            info!("Initializing menu item with selector");
            let _: () = msg_send![item,
                initWithTitle:title
                action:sel
                keyEquivalent:empty_string];
            let _: () = msg_send![item, setTarget:target];
        }
        None => {
            info!("Initializing menu item without selector");
            let _: () = msg_send![item,
                initWithTitle:title
                action:nil
                keyEquivalent:empty_string];
        }
    }
    
    if checked {
        info!("Setting menu item state to checked");
        let _: () = msg_send![item, setState: 1];
    }
    
    item
}

unsafe fn add_menu_item(menu: id, title: &str, action: &str, checked: bool, target: id) {
    let item = create_menu_item(title, action, checked, target);
    let _: () = msg_send![menu, addItem: item];
}

unsafe fn add_separator(menu: id) {
    let separator = NSMenuItem::separatorItem(nil);
    let _: () = msg_send![menu, addItem: separator];
}

unsafe fn register_menu_target_class() -> *const Class {
    let superclass = class!(NSObject);
    let mut decl = objc::declare::ClassDecl::new("MenuTarget", superclass).unwrap();

    // Add instance variables to store menu and current action data
    decl.add_ivar::<id>("menu");
    decl.add_ivar::<f32>("pending_volume");
    decl.add_ivar::<id>("pending_profile");

    extern "C" fn handle_action(_this: &Object, _sel: Sel) {
        println!("Action received: {:?}", _sel);
        
        let sel_name = unsafe {
            let name = objc::runtime::sel_getName(_sel);
            std::ffi::CStr::from_ptr(name).to_string_lossy().to_string()
        };
        println!("Selector name: {}", sel_name);

        match sel_name.as_str() {
            "toggleSound" => {
                println!("Toggle sound action detected");
                let mut state = crate::APP_STATE.lock();
                state.enabled = !state.enabled;
                println!("Sound {}", if state.enabled { "enabled" } else { "disabled" });
                if let Err(e) = state.save() {
                    println!("Failed to save configuration: {}", e);
                }
                
                unsafe {
                    if let Some(menu_item) = get_menu_item_for_action(_this, "toggleSound") {
                        println!("Found toggle menu item, updating state to {}", state.enabled);
                        let _: () = msg_send![menu_item, setState: if state.enabled { 1 } else { 0 }];
                    }
                }
            },
            "setVolume25" => handle_volume(_this, 0.25),
            "setVolume50" => handle_volume(_this, 0.5),
            "setVolume75" => handle_volume(_this, 0.75),
            "setVolume100" => handle_volume(_this, 1.0),
            sel_name if sel_name.starts_with("setProfile_") => {
                let profile = sel_name.strip_prefix("setProfile_").unwrap();
                handle_profile(_this, profile);
            },
            "quit" => {
                println!("Quit action detected");
                std::process::exit(0);
            },
            _ => println!("Unknown action: {}", sel_name)
        }
    }

    fn handle_volume(_this: &Object, volume: f32) {
        println!("Set volume action called with value: {}", volume);
        let mut state = crate::APP_STATE.lock();
        state.volume = volume;
        println!("Volume set to {}", volume);
        if let Err(e) = state.save() {
            println!("Failed to save configuration: {}", e);
        }
        
        unsafe {
            // Uncheck all volume items
            for name in &["setVolume25", "setVolume50", "setVolume75", "setVolume100"] {
                if let Some(menu_item) = get_menu_item_for_action(_this, name) {
                    println!("Unchecking volume {}", name);
                    let _: () = msg_send![menu_item, setState: 0];
                }
            }
            
            // Check the selected volume
            let action_name = match volume {
                0.25 => "setVolume25",
                0.50 => "setVolume50",
                0.75 => "setVolume75",
                1.0 => "setVolume100",
                _ => return
            };
            
            if let Some(menu_item) = get_menu_item_for_action(_this, action_name) {
                println!("Checking volume {}", action_name);
                let _: () = msg_send![menu_item, setState: 1];
            }
        }
    }

    fn handle_profile(_this: &Object, profile: &str) {
        println!("Set profile action called with value: {}", profile);
        let mut state = crate::APP_STATE.lock();
        state.keyboard_profile = profile.to_string();
        println!("Profile set to {}", profile);
        if let Err(e) = state.save() {
            println!("Failed to save configuration: {}", e);
        }
        
        unsafe {
            // Get all profile menu items and uncheck them
            let ptr = _this as *const _ as *mut Object;
            let menu: id = *(*ptr).get_ivar("menu");
            let count: usize = msg_send![menu, numberOfItems];
            
            // Uncheck all profile items
            for i in 0..count {
                let item: id = msg_send![menu, itemAtIndex:i];
                let action: Sel = msg_send![item, action];
                let action_name = objc::runtime::sel_getName(action);
                if let Ok(name) = std::ffi::CStr::from_ptr(action_name).to_str() {
                    if name.starts_with("setProfile_") {
                        let _: () = msg_send![item, setState:0];
                    }
                }
            }
            
            // Check the current profile
            let action_name = format!("setProfile_{}", profile);
            if let Some(menu_item) = get_menu_item_for_action(_this, &action_name) {
                println!("Checking profile {}", action_name);
                let _: () = msg_send![menu_item, setState:1];
            }
        }
    }

    extern "C" fn set_menu(_this: &Object, _sel: Sel, menu: id) {
        unsafe {
            println!("Set menu called");
            let ptr = _this as *const _ as *mut Object;
            (*ptr).set_ivar("menu", menu);
        }
    }

    unsafe {
        println!("Registering menu target class methods");
        decl.add_method(sel!(toggleSound), handle_action as extern "C" fn(&Object, Sel));
        decl.add_method(sel!(setVolume25), handle_action as extern "C" fn(&Object, Sel));
        decl.add_method(sel!(setVolume50), handle_action as extern "C" fn(&Object, Sel));
        decl.add_method(sel!(setVolume75), handle_action as extern "C" fn(&Object, Sel));
        decl.add_method(sel!(setVolume100), handle_action as extern "C" fn(&Object, Sel));
        
        // Register all profile methods
        let keyboards_dir = get_assets_dir().join("keyboards");
        let profiles = fs::read_dir(&keyboards_dir)
            .unwrap_or_else(|_| panic!("Failed to read keyboards directory at {:?}", keyboards_dir))
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| !name.starts_with('.') && name != "test-profile");

        for profile in profiles {
            let selector = format!("setProfile_{}", profile);
            decl.add_method(Sel::register(&selector), handle_action as extern "C" fn(&Object, Sel));
        }
        
        decl.add_method(sel!(quit), handle_action as extern "C" fn(&Object, Sel));
        decl.add_method(sel!(setMenu:), set_menu as extern "C" fn(&Object, Sel, id));
    }

    decl.register()
}

/// Helper function to get a menu item for a specific action
unsafe fn get_menu_item_for_action(target: &Object, action: &str) -> Option<id> {
    let ptr = target as *const _ as *mut Object;
    let menu: id = *(*ptr).get_ivar("menu");
    if menu == nil {
        println!("Menu is nil when searching for action: {}", action);
        return None;
    }
    
    let count: usize = msg_send![menu, numberOfItems];
    let sel = Sel::register(action);
    
    println!("Searching for menu item with action: {} among {} items", action, count);
    for i in 0..count {
        let item: id = msg_send![menu, itemAtIndex:i];
        let item_sel: Sel = msg_send![item, action];
        if item_sel == sel {
            println!("Found menu item for action: {}", action);
            return Some(item);
        }
    }
    
    println!("Could not find menu item for action: {}", action);
    None
}

pub fn ensure_assets_exist() -> std::io::Result<()> {
    info!("Starting asset initialization...");
    
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("clickclack");
    debug!("Config directory path: {:?}", config_dir);
    
    let keyboards_dir = config_dir.join("keyboards");
    debug!("Checking for keyboards directory at: {:?}", keyboards_dir);
    debug!("Keyboards directory exists: {}", keyboards_dir.exists());
    
    if !keyboards_dir.exists() {
        info!("Keyboards directory not found, downloading assets...");
        fs::create_dir_all(&config_dir)?;
        debug!("Created config directory at: {:?}", config_dir);
        
        // Download the zip file
        let url = "https://github.com/cesarferreira/clickclack/raw/refs/heads/main/assets/keyboards.zip";
        info!("Downloading assets from: {}", url);
        
        let response = match ureq::get(url).call() {
            Ok(resp) => resp,
            Err(e) => {
                error!("Failed to download assets: {}", e);
                return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
            }
        };
        debug!("Download response received");
        
        let mut bytes: Vec<u8> = Vec::new();
        match response.into_reader().read_to_end(&mut bytes) {
            Ok(_) => debug!("Downloaded {} bytes", bytes.len()),
            Err(e) => {
                error!("Failed to read response: {}", e);
                return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
            }
        }
        
        // Extract the zip file
        let cursor = Cursor::new(bytes);
        let mut archive = match zip::ZipArchive::new(cursor) {
            Ok(archive) => {
                debug!("Successfully created zip archive with {} files", archive.len());
                archive
            },
            Err(e) => {
                error!("Failed to create zip archive: {}", e);
                return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
            }
        };
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = config_dir.join(file.name());
            debug!("Extracting file to: {:?}", outpath);
            
            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
                debug!("Created directory: {:?}", outpath);
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p)?;
                    debug!("Created parent directory: {:?}", p);
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
                debug!("Extracted file: {:?}", outpath);
            }
        }
        
        info!("Assets downloaded and extracted successfully");
    } else {
        debug!("Keyboards directory already exists at: {:?}", keyboards_dir);
        
        // Check if the directory has content
        let entries = fs::read_dir(&keyboards_dir)?;
        let count = entries.count();
        debug!("Found {} entries in keyboards directory", count);
        
        if count == 0 {
            info!("Keyboards directory is empty, removing and recreating...");
            fs::remove_dir(&keyboards_dir)?;
            return ensure_assets_exist();
        }
    }
    
    info!("Asset initialization complete");
    Ok(())
}

fn get_assets_dir() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("clickclack");
    
    // Ensure assets exist (download if needed)
    if let Err(e) = ensure_assets_exist() {
        error!("Failed to ensure assets exist: {}", e);
    }
    
    config_dir
}
