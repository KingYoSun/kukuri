use windows::{
    Data::Xml::Dom::XmlDocument,
    UI::Notifications::{ToastNotification, ToastNotificationManager},
    core::{HSTRING, h},
};

fn activation_uri(id: &str) -> Result<String, String> {
    if id.is_empty() || id.len() > 1024 || id.chars().any(char::is_control) {
        return Err("invalid OS notification activation ID".into());
    }
    // Windows canonicalizes a host-only protocol URI by adding the root slash.
    let mut uri = tauri::Url::parse("kukuri://notification/").expect("static notification URI");
    uri.query_pairs_mut().append_pair("id", id);
    Ok(uri.into())
}

fn document(
    id: &str,
    title: &str,
    body: Option<&str>,
    silent: bool,
) -> Result<XmlDocument, String> {
    let uri = activation_uri(id)?;
    let build = || -> windows::core::Result<XmlDocument> {
        let doc = XmlDocument::new()?;
        let root = doc.CreateElement(h!("toast"))?;
        root.SetAttribute(h!("activationType"), h!("protocol"))?;
        root.SetAttribute(h!("launch"), &HSTRING::from(uri.as_str()))?;
        let visual = doc.CreateElement(h!("visual"))?;
        let binding = doc.CreateElement(h!("binding"))?;
        binding.SetAttribute(h!("template"), h!("ToastGeneric"))?;
        for value in std::iter::once(title).chain(body) {
            let text = doc.CreateElement(h!("text"))?;
            text.AppendChild(&doc.CreateTextNode(&HSTRING::from(value))?)?;
            binding.AppendChild(&text)?;
        }
        visual.AppendChild(&binding)?;
        root.AppendChild(&visual)?;
        let audio = doc.CreateElement(h!("audio"))?;
        if silent {
            audio.SetAttribute(h!("silent"), h!("true"))?;
        } else {
            audio.SetAttribute(h!("src"), h!("ms-winsoundevent:Notification.Default"))?;
        }
        root.AppendChild(&audio)?;
        doc.AppendChild(&root)?;
        Ok(doc)
    };
    build().map_err(|error| error.to_string())
}

pub(super) fn show(
    app_id: &str,
    id: &str,
    title: &str,
    body: Option<&str>,
    silent: bool,
) -> Result<(), String> {
    let doc = document(id, title, body, silent)?;
    let toast =
        ToastNotification::CreateToastNotification(&doc).map_err(|error| error.to_string())?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))
        .and_then(|notifier| notifier.Show(&toast))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_uri_survives_windows_canonicalization() {
        let launch = activation_uri("notification:owner:reply:1").unwrap();
        let canonical = windows::Foundation::Uri::CreateUri(&HSTRING::from(launch.as_str()))
            .unwrap()
            .AbsoluteUri()
            .unwrap()
            .to_string();
        assert_eq!(canonical, launch);
    }

    #[test]
    fn protocol_activation_carries_only_the_encoded_notification_id() {
        let id = "notification:owner:reply:example&?\"通知";
        let doc = document(id, "Title <&>", Some("private preview <&>"), false).unwrap();
        let root = doc.DocumentElement().unwrap();
        assert_eq!(root.GetAttribute(h!("activationType")).unwrap(), "protocol");
        let launch = root.GetAttribute(h!("launch")).unwrap().to_string();
        let uri = tauri::Url::parse(&launch).unwrap();
        assert_eq!(uri.scheme(), "kukuri");
        assert_eq!(uri.host_str(), Some("notification"));
        assert_eq!(uri.path(), "/");
        assert_eq!(
            uri.query_pairs().collect::<Vec<_>>(),
            vec![("id".into(), id.into())]
        );
        assert!(!launch.contains("private preview"));
        let xml = doc.GetXml().unwrap().to_string();
        assert!(xml.contains("Title &lt;&amp;&gt;"));
        assert!(xml.contains("private preview &lt;&amp;&gt;"));
    }

    #[test]
    fn native_document_preserves_silent_and_absent_preview() {
        let doc = document("notification:owner:reply:1", "Reply", None, true).unwrap();
        assert_eq!(
            doc.GetElementsByTagName(h!("text"))
                .unwrap()
                .Length()
                .unwrap(),
            1
        );
        let xml = doc.GetXml().unwrap().to_string();
        assert!(xml.contains("silent=\"true\""));
        assert!(!xml.contains("ms-winsoundevent"));
    }

    #[test]
    fn invalid_activation_ids_are_rejected_before_os_dispatch() {
        for id in [
            String::new(),
            "x".repeat(1025),
            "x\0y".into(),
            "x\ny".into(),
        ] {
            assert!(activation_uri(&id).is_err());
        }
        assert!(activation_uri(&"x".repeat(1024)).is_ok());
    }
}
