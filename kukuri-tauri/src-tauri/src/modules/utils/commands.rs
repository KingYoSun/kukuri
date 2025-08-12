use nostr_sdk::prelude::*;

/// 16進数の公開鍵をnpub（Bech32形式）に変換
#[tauri::command]
pub fn pubkey_to_npub(pubkey: String) -> Result<String, String> {
    let public_key = PublicKey::from_hex(&pubkey)
        .map_err(|e| format!("無効な公開鍵: {e}"))?;
    
    let npub = public_key.to_bech32()
        .map_err(|e| format!("Bech32変換エラー: {e}"))?;
    
    Ok(npub)
}

/// npub（Bech32形式）を16進数の公開鍵に変換
#[tauri::command]
pub fn npub_to_pubkey(npub: String) -> Result<String, String> {
    let public_key = PublicKey::from_bech32(&npub)
        .map_err(|e| format!("無効なnpub: {e}"))?;
    
    Ok(public_key.to_hex())
}