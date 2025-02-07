use cocoa::appkit::{NSApplication, NSApplicationActivationPolicy};
use cocoa::base::nil;

pub fn init_platform() {
    unsafe {
        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
    }
}
