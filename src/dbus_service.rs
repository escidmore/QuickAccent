//! Session-bus API the GNOME Shell panel menu calls (Settings… / Quit).

use std::sync::OnceLock;

use zbus::interface;

struct Service;

#[interface(name = "io.github.victormasson.QuickAccent")]
impl Service {
    fn open_settings(&self) {
        crate::app::request(crate::app::UiEvent::OpenSettings);
    }

    fn quit(&self) {
        crate::app::request(crate::app::UiEvent::Quit);
    }
}

/// Export the well-known name on a background thread. The connection is
/// kept alive for the process lifetime.
pub fn start() {
    std::thread::Builder::new()
        .name("quickaccent-dbus".into())
        .spawn(serve)
        .ok();
}

fn serve() {
    static CONN: OnceLock<zbus::blocking::Connection> = OnceLock::new();
    let conn = match zbus::blocking::Connection::session() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[QuickAccent] D-Bus session: {e}");
            return;
        }
    };
    if let Err(e) = conn
        .object_server()
        .at("/io/github/victormasson/QuickAccent", Service)
    {
        eprintln!("[QuickAccent] D-Bus export: {e}");
        return;
    }
    if let Err(e) = conn.request_name("io.github.victormasson.QuickAccent") {
        eprintln!("[QuickAccent] D-Bus name: {e}");
        return;
    }
    let _ = CONN.set(conn);
    std::thread::park();
}
