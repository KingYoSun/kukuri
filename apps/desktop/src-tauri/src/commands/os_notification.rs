use tauri::{AppHandle, Emitter, Manager};

/// Event payload emitted to the frontend when an OS toast is clicked.
#[derive(Clone, serde::Serialize)]
struct ActivationPayload {
    notification_id: String,
}

/// Bring the main window forward and tell the frontend which notification was
/// activated. The frontend resolves the id back to a notification and opens the
/// target post via the existing in-app handler.
fn activate_main_window(app: &AppHandle, notification_id: String) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app.emit(
        "os-notification://activated",
        ActivationPayload { notification_id },
    );
}

/// Show an OS toast for an incoming notification. On platforms that support it,
/// clicking the toast focuses the window and emits `os-notification://activated`
/// so the frontend can open the related post.
///
/// This replaces the direct `plugin:notification|notify` invocation because the
/// `tauri-plugin-notification` desktop backend never reports click/activation
/// events back to the app (those are mobile-only).
#[tauri::command]
pub fn show_os_notification(
    app: AppHandle,
    id: String,
    title: String,
    body: Option<String>,
    silent: bool,
) -> Result<(), String> {
    show_platform_notification(app, id, title, body, silent)
}

#[cfg(windows)]
fn show_platform_notification(
    app: AppHandle,
    id: String,
    title: String,
    body: Option<String>,
    silent: bool,
) -> Result<(), String> {
    use tauri_winrt_notification::{Sound, Toast};

    // Use the same AUMID as the existing plugin path (the bundle identifier) so
    // the toast renders under the registered app identity.
    let app_id = app.config().identifier.clone();
    let mut toast = Toast::new(&app_id).title(&title);
    if let Some(body) = body.as_deref() {
        toast = toast.text1(body);
    }
    toast = toast.sound(if silent { None } else { Some(Sound::Default) });

    // The activation closure must be `Send + 'static`, so it owns a cloned
    // `AppHandle` and the notification id captured for this single toast.
    let activate_app = app.clone();
    toast = toast.on_activated(move |_action| {
        activate_main_window(&activate_app, id.clone());
        Ok(())
    });

    toast.show().map_err(|error| error.to_string())
}

#[cfg(target_os = "linux")]
fn show_platform_notification(
    app: AppHandle,
    id: String,
    title: String,
    body: Option<String>,
    _silent: bool,
) -> Result<(), String> {
    use notify_rust::Notification;

    let mut notification = Notification::new();
    notification.summary(&title);
    if let Some(body) = body.as_deref() {
        notification.body(body);
    }
    // "default" fires when the notification body itself is clicked.
    notification.action("default", "Open");

    let handle = notification.show().map_err(|error| error.to_string())?;

    // `wait_for_action` blocks until the notification is actioned or closed, so
    // run it off the command thread.
    let activate_app = app.clone();
    std::thread::spawn(move || {
        handle.wait_for_action(|action| {
            if action == "default" {
                activate_main_window(&activate_app, id);
            }
        });
    });
    Ok(())
}

#[cfg(target_os = "macos")]
fn show_platform_notification(
    _app: AppHandle,
    _id: String,
    title: String,
    body: Option<String>,
    _silent: bool,
) -> Result<(), String> {
    use notify_rust::Notification;

    // notify-rust routes through mac-notification-sys on macOS, whose action
    // callbacks require a signed application bundle. For now we only display the
    // toast; click-through activation can be added later via the UserNotifications
    // framework (UNUserNotificationCenter).
    let mut notification = Notification::new();
    notification.summary(&title);
    if let Some(body) = body.as_deref() {
        notification.body(body);
    }
    notification.show().map_err(|error| error.to_string())?;
    Ok(())
}
