//! WebKitGTK's default file chooser title does not follow the document language.
//! Keep the HTML File API and its selection boundary; only own the native chooser UI.
//! #965: the accept filter stays the default, and an "all files" filter lets the user pick an
//! unsupported file so the frontend can explain why it is rejected instead of a silent chooser.

use gtk::prelude::*;
use tauri::{Manager, Runtime};
use webkit2gtk::{FileChooserRequestExt, WebViewExt, gio, glib};

struct DialogCopy {
    title: String,
    cancel: String,
    filter: String,
    all_files: String,
}

fn dialog_copy(locale: &str) -> DialogCopy {
    // Use the same bundled resources as the frontend, without a second translation table.
    let resource = match locale {
        "ja" => include_str!("../../src/i18n/locales/ja/common.json"),
        "zh-CN" => include_str!("../../src/i18n/locales/zh-CN/common.json"),
        _ => include_str!("../../src/i18n/locales/en/common.json"),
    };
    let common: serde_json::Value = serde_json::from_str(resource).expect("bundled common locale");
    DialogCopy {
        filter: common["composer"]["supportedFiles"]
            .as_str()
            .expect("file filter label")
            .to_owned(),
        all_files: common["composer"]["allFiles"]
            .as_str()
            .expect("all files filter label")
            .to_owned(),
        title: common["composer"]["chooseFiles"]
            .as_str()
            .expect("file chooser title")
            .to_owned(),
        cancel: common["actions"]["cancel"]
            .as_str()
            .expect("cancel label")
            .to_owned(),
    }
}

pub fn install<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    window.with_webview(|platform| {
        platform.inner().connect_run_file_chooser(|view, request| {
            let request = request.clone();
            let weak_view = view.downgrade();
            // Read at activation, including language changes since the previous dialog.
            view.evaluate_javascript(
                "document.documentElement.lang",
                None,
                None,
                None::<&gio::Cancellable>,
                move |result| {
                    let Some(view) = weak_view.upgrade() else {
                        request.cancel();
                        return;
                    };
                    let Ok(locale) = result else {
                        request.cancel();
                        return;
                    };
                    let copy = dialog_copy(&locale.to_string());
                    let parent = view
                        .toplevel()
                        .and_then(|widget| widget.downcast::<gtk::Window>().ok());
                    let dialog = gtk::FileChooserNative::new(
                        Some(&copy.title),
                        parent.as_ref(),
                        gtk::FileChooserAction::Open,
                        Some(&copy.title),
                        Some(&copy.cancel),
                    );
                    dialog.set_modal(true);
                    dialog.set_select_multiple(request.selects_multiple());
                    if let Some(filter) = request.mime_types_filter() {
                        filter.set_name(Some(&copy.filter));
                        dialog.add_filter(filter.clone());
                        // Unsupported files stay selectable here on purpose: the frontend
                        // rejects them by MIME type before reading and shows the reason.
                        let all_files = gtk::FileFilter::new();
                        all_files.set_name(Some(&copy.all_files));
                        all_files.add_pattern("*");
                        dialog.add_filter(all_files);
                        dialog.set_filter(&filter);
                    }
                    for file in request.selected_files() {
                        let _ = dialog.select_filename(std::path::Path::new(file.as_str()));
                    }
                    let cancel_dialog = dialog.downgrade();
                    let destroyed = view.connect_destroy(move |_| {
                        if let Some(dialog) = cancel_dialog.upgrade() {
                            dialog.emit_by_name::<()>("response", &[&gtk::ResponseType::Cancel]);
                        }
                    });
                    let cancel_dialog = dialog.downgrade();
                    let navigated = view.connect_load_changed(move |_, event| {
                        if event == webkit2gtk::LoadEvent::Started {
                            if let Some(dialog) = cancel_dialog.upgrade() {
                                dialog
                                    .emit_by_name::<()>("response", &[&gtk::ResponseType::Cancel]);
                            }
                        }
                    });
                    let weak_view = view.downgrade();
                    glib::MainContext::default().spawn_local(async move {
                        let response = dialog.run_future().await;
                        let alive = if let Some(view) = weak_view.upgrade() {
                            view.disconnect(destroyed);
                            view.disconnect(navigated);
                            true
                        } else {
                            false
                        };
                        if alive && response == gtk::ResponseType::Accept {
                            let files = dialog.filenames();
                            let paths: Option<Vec<&str>> =
                                files.iter().map(|file| file.to_str()).collect();
                            if let Some(paths) = paths.filter(|paths| !paths.is_empty()) {
                                // Only this native chooser's results reach WebKit. No path IPC,
                                // filesystem permission, Rust file read, or upload is introduced.
                                request.select_files(&paths);
                            } else {
                                request.cancel();
                            }
                        } else {
                            request.cancel();
                        }
                        dialog.destroy();
                    });
                },
            );
            true
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_titles_follow_supported_app_locales_and_fallback() {
        for (locale, title, cancel) in [
            ("ja", "ファイルを選択", "キャンセル"),
            ("en", "Choose files", "Cancel"),
            ("zh-CN", "选择文件", "取消"),
            ("invalid", "Choose files", "Cancel"),
        ] {
            let copy = dialog_copy(locale);
            assert_eq!(copy.title, title);
            assert_eq!(copy.cancel, cancel);
        }
    }

    // #965: the default filter names what is supported, and "all files" is offered next to it.
    #[test]
    fn native_filters_name_supported_media_and_offer_all_files() {
        for (locale, filter, all_files) in [
            ("ja", "画像と動画", "すべてのファイル"),
            ("en", "Images and videos", "All files"),
            ("zh-CN", "图片和视频", "所有文件"),
            ("invalid", "Images and videos", "All files"),
        ] {
            let copy = dialog_copy(locale);
            assert_eq!(copy.filter, filter);
            assert_eq!(copy.all_files, all_files);
        }
    }
}
