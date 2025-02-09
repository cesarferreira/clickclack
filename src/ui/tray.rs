use anyhow::Result;
use cocoa::base::{id, nil, YES, NO};
use cocoa::foundation::{NSString, NSAutoreleasePool, NSSize};
use cocoa::appkit::{NSStatusBar, NSMenu, NSMenuItem};
use objc::runtime::{Object, Class};
use objc::{msg_send, sel, sel_impl, class};
use objc::runtime::Sel;
use std::fs;
use log::{info, error};

const VOLUME_LEVELS: &[(f32, &str)] = &[
    (1.0, "100%"),
    (0.75, "75%"),
    (0.5, "50%"),
    (0.25, "25%"),
];
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
            add_menu_item(menu, "Enable Sound", "toggle:", enabled, target);
            add_separator(menu);

            // Volume controls
            let current_volume = {
                let state = crate::APP_STATE.lock();
                state.volume
            };

            add_menu_item(menu, "Volume", "", false, target);
            for &(level, label) in VOLUME_LEVELS {
                let checked = (level - current_volume).abs() < 0.01;
                add_menu_item(menu, &format!("  {}", label), &format!("setVolume:{}", level), checked, target);
            }
            add_separator(menu);

            // Keyboard profiles
            let current_profile = {
                let state = crate::APP_STATE.lock();
                state.keyboard_profile.clone()
            };

            add_menu_item(menu, "Keyboard Profile", "", false, target);
            let profiles = fs::read_dir("assets/keyboards")?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    if entry.file_type().ok()?.is_dir() {
                        entry.file_name().into_string().ok()
                    } else {
                        None
                    }
                })
                .filter(|name| !name.starts_with('.'))
                .collect::<Vec<_>>();

            for profile in profiles {
                let checked = profile == current_profile;
                add_menu_item(menu, &format!("  {}", profile), &format!("setProfile:{}", profile), checked, target);
            }

            add_separator(menu);
            add_menu_item(menu, "Quit", "quit:", false, target);

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
    let title = NSString::alloc(nil).init_str(title);
    
    // Convert the action string to a selector using runtime functions
    let selector = if action.is_empty() {
        None
    } else {
        Some(Sel::register(action))
    };
    
    let item = NSMenuItem::alloc(nil);
    let empty_string = NSString::alloc(nil).init_str("");
    
    match selector {
        Some(sel) => {
            let _: () = msg_send![item,
                initWithTitle:title
                action:sel
                keyEquivalent:empty_string];
            let _: () = msg_send![item, setTarget:target];
        }
        None => {
            let _: () = msg_send![item,
                initWithTitle:title
                action:nil
                keyEquivalent:empty_string];
        }
    }
    
    if checked {
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

    // Add instance variable to store menu
    decl.add_ivar::<id>("menu");

    extern "C" fn toggle(this: &Object, _: Sel, _cmd: Sel) {
        let mut state = crate::APP_STATE.lock();
        state.enabled = !state.enabled;
        info!("Sound {}", if state.enabled { "enabled" } else { "disabled" });
        if let Err(e) = state.save() {
            error!("Failed to save configuration: {}", e);
        }
        
        // Update menu item state
        unsafe {
            if let Some(menu_item) = get_menu_item_for_action(this, "toggle:") {
                let _: () = msg_send![menu_item, setState: if state.enabled { 1 } else { 0 }];
            }
        }
    }

    extern "C" fn set_volume(this: &Object, _: Sel, volume: f32) {
        let mut state = crate::APP_STATE.lock();
        state.volume = volume;
        info!("Volume set to {}", volume);
        if let Err(e) = state.save() {
            error!("Failed to save configuration: {}", e);
        }
        
        // Update menu item states
        unsafe {
            // Uncheck all volume items first
            for &(level, _) in VOLUME_LEVELS {
                if let Some(menu_item) = get_menu_item_for_action(this, &format!("setVolume:{}", level)) {
                    let _: () = msg_send![menu_item, setState: 0];
                }
            }
            
            // Check the selected volume
            if let Some(menu_item) = get_menu_item_for_action(this, &format!("setVolume:{}", volume)) {
                let _: () = msg_send![menu_item, setState: 1];
            }
        }
    }

    extern "C" fn set_profile(this: &Object, _: Sel, profile: id) {
        unsafe {
            let string_ref: id = msg_send![profile, description];
            let bytes: *const u8 = msg_send![string_ref, UTF8String];
            let len: usize = msg_send![string_ref, lengthOfBytesUsingEncoding:4];
            let profile_string = std::str::from_utf8(std::slice::from_raw_parts(bytes, len))
                .unwrap_or("")
                .to_string();
            
            let mut state = crate::APP_STATE.lock();
            
            // Uncheck all profile items first
            let profiles = fs::read_dir("assets/keyboards")
                .unwrap_or_else(|_| panic!("Failed to read keyboards directory"))
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| !name.starts_with('.'));
                
            for profile_name in profiles {
                if let Some(menu_item) = get_menu_item_for_action(this, &format!("setProfile:{}", profile_name)) {
                    let _: () = msg_send![menu_item, setState: 0];
                }
            }
            
            // Check the selected profile
            if let Some(menu_item) = get_menu_item_for_action(this, &format!("setProfile:{}", profile_string)) {
                let _: () = msg_send![menu_item, setState: 1];
            }
            
            state.keyboard_profile = profile_string.clone();
            info!("Keyboard profile set to {}", profile_string);
            if let Err(e) = state.save() {
                error!("Failed to save configuration: {}", e);
            }
        }
    }

    extern "C" fn quit(this: &Object, _: Sel, _cmd: Sel) {
        std::process::exit(0);
    }

    // Add method to set menu
    extern "C" fn set_menu(this: &Object, _: Sel, menu: id) {
        unsafe {
            let ptr = this as *const _ as *mut Object;
            (*ptr).set_ivar("menu", menu);
        }
    }

    unsafe {
        decl.add_method(sel!(toggle:), toggle as extern "C" fn(&Object, Sel, Sel));
        decl.add_method(sel!(setVolume:), set_volume as extern "C" fn(&Object, Sel, f32));
        decl.add_method(sel!(setProfile:), set_profile as extern "C" fn(&Object, Sel, id));
        decl.add_method(sel!(quit:), quit as extern "C" fn(&Object, Sel, Sel));
        decl.add_method(sel!(setMenu:), set_menu as extern "C" fn(&Object, Sel, id));
    }

    decl.register()
}

/// Helper function to get a menu item for a specific action
unsafe fn get_menu_item_for_action(target: &Object, action: &str) -> Option<id> {
    let ptr = target as *const _ as *mut Object;
    let menu: id = *(*ptr).get_ivar("menu");
    if menu == nil {
        return None;
    }
    
    let count: usize = msg_send![menu, numberOfItems];
    let sel = Sel::register(action);
    
    for i in 0..count {
        let item: id = msg_send![menu, itemAtIndex:i];
        let item_sel: Sel = msg_send![item, action];
        if item_sel == sel {
            return Some(item);
        }
    }
    
    None
}
