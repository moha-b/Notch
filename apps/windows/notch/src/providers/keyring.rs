use super::Failure;

#[cfg(windows)]
fn target() -> Vec<u16> {
    format!("{}.ollama", crate::identity::NAMESPACE)
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

#[cfg(windows)]
pub fn read() -> Result<String, Failure> {
    use windows::{core::PCWSTR, Win32::Security::Credentials::*};
    let target = target();
    let mut credential = std::ptr::null_mut();
    unsafe {
        CredReadW(
            PCWSTR(target.as_ptr()),
            CRED_TYPE_GENERIC,
            0,
            &mut credential,
        )
        .map_err(|_| Failure::NeedsAuth)?;
        let stored = &*credential;
        let decoded = if stored.CredentialBlobSize == 0 {
            Err(Failure::NeedsAuth)
        } else {
            String::from_utf8(
                std::slice::from_raw_parts(
                    stored.CredentialBlob,
                    stored.CredentialBlobSize as usize,
                )
                .to_vec(),
            )
            .map_err(|_| Failure::Invalid("Stored Ollama key is invalid."))
        };
        CredFree(credential.cast());
        decoded
    }
}

#[cfg(windows)]
pub fn store(key: &str) -> Result<(), String> {
    use windows::{core::PWSTR, Win32::Security::Credentials::*};
    if key.is_empty() || key.len() > 2560 {
        return Err("Ollama key must contain 1–2560 bytes.".into());
    }
    let mut target = target();
    let mut bytes = key.as_bytes().to_vec();
    let credential = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target.as_mut_ptr()),
        CredentialBlobSize: bytes.len() as u32,
        CredentialBlob: bytes.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        ..Default::default()
    };
    let saved = unsafe { CredWriteW(&credential, 0) }
        .map_err(|_| "Could not save the Ollama key in Credential Manager.".into());
    bytes.fill(0);
    saved
}

#[cfg(windows)]
pub fn delete() -> Result<(), String> {
    use windows::{core::PCWSTR, Win32::Security::Credentials::*};
    let target = target();
    unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, 0) }
        .map_err(|_| "Could not remove the Ollama key from Credential Manager.".into())
}

#[cfg(not(windows))]
pub fn read() -> Result<String, Failure> {
    Err(Failure::Unavailable(
        "Windows Credential Manager is required.".into(),
    ))
}
#[cfg(not(windows))]
pub fn store(_: &str) -> Result<(), String> {
    Err("Windows Credential Manager is required.".into())
}
#[cfg(not(windows))]
pub fn delete() -> Result<(), String> {
    Err("Windows Credential Manager is required.".into())
}
