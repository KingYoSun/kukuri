// 同意前に読むのはOSの表示言語だけ。AppHandle/Stateやruntime、DB、networkに依存しない。
#[tauri::command]
pub fn get_system_locales() -> Vec<String> {
    platform_locales()
}

#[cfg(windows)]
fn platform_locales() -> Vec<String> {
    use windows::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    use windows::core::PWSTR;

    let mut count = 0;
    let mut length = 0;
    // SAFETY: 1回目はnull buffer/長さ0で必要UTF-16長を取得する。
    if unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut length) }
        .is_err()
        || length == 0
    {
        return Vec::new();
    }
    let mut buffer = vec![0u16; length as usize];
    // SAFETY: OSが要求した長さの有効bufferを渡す。言語設定変更等による不足はErrへ倒す。
    if unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            Some(PWSTR(buffer.as_mut_ptr())),
            &mut length,
        )
    }
    .is_err()
    {
        return Vec::new();
    }
    buffer
        .split(|unit| *unit == 0)
        .take_while(|part| !part.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

#[cfg(target_os = "linux")]
fn platform_locales() -> Vec<String> {
    locales_from_environment(|key| std::env::var(key).ok())
}

#[cfg(not(any(windows, target_os = "linux")))]
fn platform_locales() -> Vec<String> {
    Vec::new()
}

#[cfg(any(target_os = "linux", test))]
fn locales_from_environment(mut read: impl FnMut(&str) -> Option<String>) -> Vec<String> {
    let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(&mut read)
        .find(|value| !value.trim().is_empty());
    // gettext同様、C/POSIXではLANGUAGEによる翻訳指定を使わない。
    if locale
        .as_deref()
        .is_some_and(|value| matches!(value, "C" | "POSIX"))
    {
        return vec!["en".into()];
    }
    let mut languages = read("LANGUAGE")
        .unwrap_or_default()
        .split(':')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    languages.extend(locale);
    languages
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(values: &[(&str, &str)]) -> Vec<String> {
        locales_from_environment(|key| {
            values
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| (*value).into())
        })
    }

    #[test]
    fn message_locale_precedence_and_language_priority_are_preserved() {
        assert_eq!(resolve(&[("LANG", "ja_JP.UTF-8")]), ["ja_JP.UTF-8"]);
        assert_eq!(
            resolve(&[("LANG", "en_US.UTF-8"), ("LC_MESSAGES", "ja_JP.UTF-8")]),
            ["ja_JP.UTF-8"]
        );
        assert_eq!(
            resolve(&[("LANG", "ja_JP.UTF-8"), ("LC_ALL", "zh_CN.UTF-8")]),
            ["zh_CN.UTF-8"]
        );
        assert_eq!(
            resolve(&[("LANGUAGE", "fr:ja:en"), ("LANG", "en_US.UTF-8")]),
            ["fr", "ja", "en", "en_US.UTF-8"]
        );
        assert_eq!(
            resolve(&[("LC_ALL", ""), ("LC_MESSAGES", ""), ("LANG", "ja_JP.UTF-8")]),
            ["ja_JP.UTF-8"]
        );
        assert_eq!(resolve(&[("LC_ALL", "C"), ("LANGUAGE", "ja")]), ["en"]);
        assert_eq!(
            resolve(&[("LC_MESSAGES", "POSIX"), ("LANGUAGE", "ja")]),
            ["en"]
        );
        assert_eq!(resolve(&[("LANGUAGE", ":ja::en:")]), ["ja", "en"]);
        assert!(resolve(&[]).is_empty());
    }

    #[test]
    fn system_locales_are_available_without_a_desktop_runtime() {
        let locales = get_system_locales();
        assert!(
            locales
                .iter()
                .all(|locale| !locale.is_empty() && !locale.contains('\0'))
        );
    }
}
