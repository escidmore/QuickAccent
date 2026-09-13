use cocoa::appkit::{NSApp, NSApplication, NSImage, NSMenu, NSMenuItem, NSStatusBar};
use cocoa::base::{id, nil, selector};
use cocoa::foundation::{NSRect, NSString};
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::geometry::{CGPoint, CGSize};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::c_void;
use std::sync::OnceLock;

/// Corner radius of the picker's glass backdrop, in points.
const OVERLAY_CORNER_RADIUS: f64 = 14.0;

/// Put a Liquid Glass backdrop (macOS 26+ `NSGlassEffectView`; a blur
/// `NSVisualEffectView` before that) under the picker's content. `ns_view` is
/// winit's content view and must stay the window's `contentView` — winit casts
/// it back to its own type — so the backdrop is added as a sibling ordered
/// below it and iced paints a transparent background over it.
pub fn attach_glass_backdrop(ns_view: *mut c_void, dark: bool) {
    unsafe {
        let view = ns_view as id;
        let window: id = msg_send![view, window];
        log::debug!("attaching glass backdrop: view={view:?} window={window:?} dark={dark}");
        if window == nil {
            return;
        }
        set_window_appearance(window, dark);
        let _: () = msg_send![window, setOpaque: false];
        let clear: id = msg_send![class!(NSColor), clearColor];
        let _: () = msg_send![window, setBackgroundColor: clear];
        // The Metal layer must not fill its transparent pixels.
        let layer: id = msg_send![view, layer];
        if layer != nil {
            let _: () = msg_send![layer, setOpaque: false];
        }

        let frame: NSRect = msg_send![view, frame];
        let backdrop: id = match Class::get("NSGlassEffectView") {
            Some(glass) => {
                let v: id = msg_send![glass, alloc];
                let v: id = msg_send![v, initWithFrame: frame];
                let _: () = msg_send![v, setCornerRadius: OVERLAY_CORNER_RADIUS];
                v
            }
            None => {
                let v: id = msg_send![class!(NSVisualEffectView), alloc];
                let v: id = msg_send![v, initWithFrame: frame];
                let _: () = msg_send![v, setMaterial: 13i64]; // NSVisualEffectMaterialHUDWindow
                let _: () = msg_send![v, setBlendingMode: 0i64]; // behindWindow
                let _: () = msg_send![v, setState: 1i64]; // active
                let _: () = msg_send![v, setWantsLayer: true];
                let layer: id = msg_send![v, layer];
                let _: () = msg_send![layer, setCornerRadius: OVERLAY_CORNER_RADIUS];
                let _: () = msg_send![layer, setMasksToBounds: true];
                v
            }
        };
        let _: () = msg_send![backdrop, setAutoresizingMask: 18u64]; // width | height sizable
        let superview: id = msg_send![view, superview];
        if superview != nil {
            let _: () = msg_send![superview, addSubview: backdrop positioned: -1i64 relativeTo: view]; // NSWindowBelow
        }
    }
}

/// Pin a window (and so its glass backdrop and title bar) to the light or dark
/// appearance, so a forced theme does not put dark text on light glass.
fn set_window_appearance(window: id, dark: bool) {
    unsafe {
        let name = NSString::alloc(nil).init_str(if dark {
            "NSAppearanceNameDarkAqua"
        } else {
            "NSAppearanceNameAqua"
        });
        let appearance: id = msg_send![class!(NSAppearance), appearanceNamed: name];
        if appearance != nil {
            let _: () = msg_send![window, setAppearance: appearance];
        }
    }
}

/// Apply the resolved theme to a window that already exists (settings).
pub fn apply_window_appearance(ns_view: *mut c_void, dark: bool) {
    unsafe {
        let window: id = msg_send![ns_view as id, window];
        if window != nil {
            set_window_appearance(window, dark);
        }
    }
}

/// Whether the system appearance is currently dark, so the picker's text and
/// chips stay readable on glass that adapts to whatever is behind it.
pub fn is_dark_appearance() -> bool {
    unsafe {
        let appearance: id = msg_send![NSApp(), effectiveAppearance];
        if appearance == nil {
            return false;
        }
        let name: id = msg_send![appearance, name];
        if name == nil {
            return false;
        }
        let cstr: *const std::os::raw::c_char = msg_send![name, UTF8String];
        if cstr.is_null() {
            return false;
        }
        let name = std::ffi::CStr::from_ptr(cstr).to_string_lossy();
        log::debug!("system appearance: {name}");
        name.contains("Dark")
    }
}

/// Bring the app forward so a freshly opened settings window is key. An
/// accessory app (no Dock icon) is never activated by macOS on its own.
pub fn activate_app() {
    unsafe {
        let _: () = msg_send![NSApp(), activateIgnoringOtherApps: true];
    }
}

/// Target object for the status-bar menu: `openSettings:` hands the click to
/// the iced app. Registered once; the instance lives as long as the menu.
fn menu_target() -> id {
    static TARGET: OnceLock<usize> = OnceLock::new();
    *TARGET.get_or_init(|| unsafe {
        extern "C" fn open_settings(_this: &Object, _sel: Sel, _sender: id) {
            crate::app::request(crate::app::UiEvent::OpenSettings);
        }
        let mut decl = ClassDecl::new("QAMenuTarget", class!(NSObject)).expect("QAMenuTarget");
        decl.add_method(
            sel!(openSettings:),
            open_settings as extern "C" fn(&Object, Sel, id),
        );
        let cls = decl.register();
        let obj: id = msg_send![cls, new];
        obj as usize
    }) as id
}

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

        // Menu: Settings… / Quit
        let menu = NSMenu::new(nil);
        let settings_title = NSString::alloc(nil).init_str("Settings\u{2026}");
        let comma = NSString::alloc(nil).init_str(",");
        let settings_item: id = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            settings_title,
            sel!(openSettings:),
            comma,
        );
        let _: () = msg_send![settings_item, setTarget: menu_target()];
        menu.addItem_(settings_item);
        menu.addItem_(NSMenuItem::separatorItem(nil));

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
