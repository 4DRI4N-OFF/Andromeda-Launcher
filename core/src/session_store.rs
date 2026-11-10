//! Per-user DPAPI-protected session. Never writes cleartext tokens.
use crate::auth::{AuthError, Profile, Secret, Session};
use std::path::Path;
#[derive(serde::Serialize, serde::Deserialize)]
struct Stored {
    schema: u32,
    client_id: String,
    profile: Profile,
    refresh_token: String,
}
#[cfg(target_os = "windows")]
fn crypt(input: &[u8], protect: bool) -> Result<Vec<u8>, AuthError> {
    use std::ffi::c_void;
    #[repr(C)]
    struct Blob {
        size: u32,
        data: *mut u8,
    }
    #[link(name = "crypt32")]
    unsafe extern "system" {
        fn CryptProtectData(
            input: *const Blob,
            description: *const u16,
            entropy: *const Blob,
            reserved: *mut c_void,
            prompt: *mut c_void,
            flags: u32,
            output: *mut Blob,
        ) -> i32;
        fn CryptUnprotectData(
            input: *const Blob,
            description: *mut *mut u16,
            entropy: *const Blob,
            reserved: *mut c_void,
            prompt: *mut c_void,
            flags: u32,
            output: *mut Blob,
        ) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LocalFree(memory: *mut c_void) -> *mut c_void;
    }
    if input.len() > 64 * 1024 {
        return Err(AuthError::Storage);
    }
    let source = Blob {
        size: input.len() as u32,
        data: input.as_ptr() as *mut u8,
    };
    let entropy_bytes = b"Andromeda/session/v1";
    let entropy = Blob {
        size: entropy_bytes.len() as u32,
        data: entropy_bytes.as_ptr() as *mut u8,
    };
    let mut dest = Blob {
        size: 0,
        data: std::ptr::null_mut(),
    };
    // DPAPI current-user scope, UI forbidden; LocalFree owns the returned buffer.
    let ok = unsafe {
        if protect {
            CryptProtectData(
                &source,
                std::ptr::null(),
                &entropy,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut dest,
            )
        } else {
            CryptUnprotectData(
                &source,
                std::ptr::null_mut(),
                &entropy,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1,
                &mut dest,
            )
        }
    };
    if ok == 0 {
        return Err(AuthError::Storage);
    }
    if dest.data.is_null() || dest.size > 128 * 1024 {
        if !dest.data.is_null() {
            unsafe {
                LocalFree(dest.data.cast());
            }
        }
        return Err(AuthError::Storage);
    }
    let bytes = unsafe { std::slice::from_raw_parts(dest.data, dest.size as usize).to_vec() };
    unsafe {
        LocalFree(dest.data.cast());
    }
    Ok(bytes)
}
#[cfg(not(target_os = "windows"))]
fn crypt(_input: &[u8], _protect: bool) -> Result<Vec<u8>, AuthError> {
    Err(AuthError::Storage)
}
pub fn save(path: &Path, session: &Session) -> Result<(), AuthError> {
    let data = Stored {
        schema: 1,
        client_id: crate::auth::CLIENT_ID.into(),
        profile: session.profile.clone(),
        refresh_token: session.refresh_token.expose().into(),
    };
    let clear = serde_json::to_vec(&data).map_err(|_| AuthError::Storage)?;
    let protected = crypt(&clear, true)?;
    let parent = path.parent().ok_or(AuthError::Storage)?;
    std::fs::create_dir_all(parent).map_err(|_| AuthError::Storage)?;
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    use std::io::Write;
    let mut file = options.open(&temp).map_err(|_| AuthError::Storage)?;
    let result = (|| {
        file.write_all(&protected)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|_| AuthError::Storage)
}
pub fn refresh(path: &Path, http: &dyn crate::auth::Transport) -> Result<Session, AuthError> {
    let meta = std::fs::symlink_metadata(path).map_err(|_| AuthError::Storage)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 128 * 1024 {
        return Err(AuthError::Storage);
    }
    let protected = std::fs::read(path).map_err(|_| AuthError::Storage)?;
    let clear = crypt(&protected, false)?;
    let stored: Stored = serde_json::from_slice(&clear).map_err(|_| AuthError::Storage)?;
    if stored.schema != 1 || stored.client_id != crate::auth::CLIENT_ID {
        return Err(AuthError::Storage);
    }
    let session = crate::auth::refresh(http, &Secret::new(stored.refresh_token))?;
    if session.profile.id != stored.profile.id {
        return Err(AuthError::InvalidSession);
    }
    save(path, &session)?;
    Ok(session)
}
pub fn remove(path: &Path) -> Result<(), AuthError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(AuthError::Storage),
    }
}
#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    #[test]
    fn dpapi_roundtrip_is_not_plaintext() {
        let text = b"test-only-token";
        let sealed = crypt(text, true).unwrap();
        assert!(!sealed.windows(text.len()).any(|w| w == text));
        assert_eq!(crypt(&sealed, false).unwrap(), text);
        assert!(crypt(b"not-protected", false).is_err());
    }
}
