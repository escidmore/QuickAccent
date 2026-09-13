use cocoa::appkit::{NSApp, NSApplication, NSImage, NSMenu, NSMenuItem, NSStatusBar};
use cocoa::base::{id, nil, selector};
use cocoa::foundation::NSString;
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::geometry::{CGPoint, CGSize};
use objc::runtime::Class;
use objc::{msg_send, sel, sel_impl};
use std::ffi::c_void;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    fn AXUIElementCopyAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementSetMessagingTimeout(element: CFTypeRef, timeout: f32) -> i32;
    fn AXValueGetTypeID() -> core_foundation::base::CFTypeID;
    fn AXValueGetValue(value: CFTypeRef, kind: u32, result: *mut c_void) -> bool;
}

fn ax_attribute(element: &CFType, name: &str) -> Option<CFType> {
    let name = CFString::new(name);
    let mut value = std::ptr::null();
    unsafe {
        if AXUIElementCopyAttributeValue(
            element.as_CFTypeRef(),
            name.as_concrete_TypeRef(),
            &mut value,
        ) != 0
        {
            return None;
        }
        Some(CFType::wrap_under_create_rule(value))
    }
}

/// PID of the app the user is typing in, via NSWorkspace. Not the system-wide
/// AX element: `AXFocusedApplication` on it fails outright with
/// kAXErrorCannotComplete on current macOS even for a trusted process, which
/// silently sent the picker to the primary display.
fn frontmost_pid() -> Option<i32> {
    unsafe {
        let workspace: id = msg_send![Class::get("NSWorkspace")?, sharedWorkspace];
        let app: id = msg_send![workspace, frontmostApplication];
        if app == nil {
            return None;
        }
        let pid: i32 = msg_send![app, processIdentifier];
        // Never anchor the overlay to ourselves.
        (pid != std::process::id() as i32).then_some(pid)
    }
}

/// Global top-left coordinates in logical points, matching iced/winit on macOS.
/// Query before opening the picker, while the user's app still has focus.
pub fn focused_window_rect() -> Option<(f32, f32, f32, f32)> {
    unsafe {
        let app = CFType::wrap_under_create_rule(AXUIElementCreateApplication(frontmost_pid()?));
        // A hung app must not stall the picker indefinitely. This runs on the
        // UI thread, outside the time-sensitive keyboard tap callback.
        AXUIElementSetMessagingTimeout(app.as_CFTypeRef(), 0.25);
        let window = ax_attribute(&app, "AXFocusedWindow")?;
        let position = ax_attribute(&window, "AXPosition")?;
        let size = ax_attribute(&window, "AXSize")?;
        if position.type_of() != AXValueGetTypeID() || size.type_of() != AXValueGetTypeID() {
            return None;
        }
        let mut point = CGPoint::new(0.0, 0.0);
        let mut dimensions = CGSize::new(0.0, 0.0);
        // kAXValueCGPointType = 1; kAXValueCGSizeType = 2.
        if !AXValueGetValue(position.as_CFTypeRef(), 1, &mut point as *mut _ as *mut c_void)
            || !AXValueGetValue(
                size.as_CFTypeRef(),
                2,
                &mut dimensions as *mut _ as *mut c_void,
            )
        {
            return None;
        }
        Some((
            point.x as f32,
            point.y as f32,
            dimensions.width as f32,
            dimensions.height as f32,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Manual probe — needs Accessibility trust for the test process:
    /// `cargo test focused_window_rect_probe -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn focused_window_rect_probe() {
        let rect = focused_window_rect();
        eprintln!("frontmost pid={:?} rect={rect:?}", frontmost_pid());
        let (_, _, w, h) = rect.expect("focused window rect (is the test process trusted?)");
        assert!(w > 0.0 && h > 0.0);
    }
}

pub fn setup_status_item() {
    unsafe {
        let app = NSApp();
        app.setActivationPolicy_(
            cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
        );

        let status_bar = NSStatusBar::systemStatusBar(nil);
        let status_item: id = msg_send![status_bar, statusItemWithLength: -1.0];

        // Menu bar icon. NSImage cannot decode SVG data — only asset-catalog
        // SVGs are supported — so this must be a raster image; an SVG here
        // yields a nil image and an invisible status item. The PNG is a
        // template (black + alpha), which macOS recolours to match light and
        // dark menu bars automatically.
        let icon_data = include_bytes!("../assets/menubar-template.png");
        let ns_data: id = msg_send![Class::get("NSData").unwrap(), dataWithBytes: icon_data.as_ptr() length: icon_data.len()];
        let ns_image: id = msg_send![NSImage::alloc(nil), initWithData: ns_data];

        let button: id = msg_send![status_item, button];
        if ns_image != nil {
            // 36px artwork drawn at 18pt → crisp on Retina.
            let size = cocoa::foundation::NSSize::new(18.0, 18.0);
            let _: () = msg_send![ns_image, setSize: size];
            let _: () = msg_send![ns_image, setTemplate: true];
            let _: () = msg_send![button, setImage: ns_image];
        } else {
            // Never leave an invisible status item behind.
            let fallback = NSString::alloc(nil).init_str("Q\u{304}");
            let _: () = msg_send![button, setTitle: fallback];
        }

        // Menu with Quit
        let menu = NSMenu::new(nil);
        let quit_title = NSString::alloc(nil).init_str("Quit QuickAccent");
        let q = NSString::alloc(nil).init_str("q");
        let quit_item: id = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            quit_title,
            selector("terminate:"),
            q,
        );
        menu.addItem_(quit_item);

        let _: () = msg_send![status_item, setMenu: menu];
    }
}
