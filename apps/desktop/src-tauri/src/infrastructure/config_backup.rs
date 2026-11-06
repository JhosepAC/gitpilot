use std::io;

#[cfg(windows)]
pub(super) async fn seal(token: &str, bytes: &[u8]) -> io::Result<Vec<u8>> {
    dpapi(token, bytes, true)
}

#[cfg(windows)]
pub(super) async fn open(token: &str, bytes: &[u8]) -> io::Result<Vec<u8>> {
    dpapi(token, bytes, false)
}

#[cfg(windows)]
fn dpapi(token: &str, bytes: &[u8], encrypt: bool) -> io::Result<Vec<u8>> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };

    let blob = |data: &[u8]| CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr().cast_mut(),
    };
    let input = blob(bytes);
    let entropy = blob(token.as_bytes());
    let mut output = CRYPT_INTEGER_BLOB::default();
    let success = unsafe {
        if encrypt {
            CryptProtectData(
                &input,
                null(),
                &entropy,
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                null_mut(),
                &entropy,
                null(),
                null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    let bytes = if output.cbData == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() }
    };
    if !encrypt && !output.pbData.is_null() {
        unsafe { std::ptr::write_bytes(output.pbData, 0, output.cbData as usize) };
    }
    unsafe { LocalFree(output.pbData.cast()) };
    Ok(bytes)
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use aes_gcm::{
        aead::{Aead, KeyInit, Payload},
        Aes256Gcm, Nonce,
    };
    use std::{process::Stdio, time::Duration};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        process::Command,
    };
    use zeroize::Zeroizing;

    fn unavailable() -> io::Error {
        io::Error::other("Protected backup is unavailable")
    }

    async fn secret_tool(
        operation: &str,
        token: &str,
        value: Option<&[u8]>,
    ) -> io::Result<Zeroizing<Vec<u8>>> {
        let mut command = Command::new("/usr/bin/secret-tool");
        command.arg(operation);
        if value.is_some() {
            command.arg("--label=GitPilot configuration backup");
        }
        let mut process = command
            .args(["application", "dev.gitpilot.desktop", "operation", token])
            .stdin(if value.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
        tokio::time::timeout(Duration::from_secs(15), async {
            if let Some(value) = value {
                let mut input = process.stdin.take().ok_or_else(unavailable)?;
                input.write_all(value).await?;
                input.shutdown().await?;
            }
            let mut result = Zeroizing::new(Vec::new());
            process
                .stdout
                .take()
                .ok_or_else(unavailable)?
                .take(100)
                .read_to_end(&mut result)
                .await?;
            if result.len() >= 100 || !process.wait().await?.success() {
                return Err(unavailable());
            }
            Ok(result)
        })
        .await
        .map_err(|_| unavailable())?
    }

    pub(super) async fn seal(token: &str, bytes: &[u8]) -> io::Result<Vec<u8>> {
        let mut key = Zeroizing::new([0; 32]);
        getrandom::fill(key.as_mut()).map_err(|_| unavailable())?;
        let encoded = Zeroizing::new(hex::encode(key.as_ref()));
        secret_tool("store", token, Some(encoded.as_bytes())).await?;
        let mut nonce = [0; 12];
        getrandom::fill(&mut nonce).map_err(|_| unavailable())?;
        let cipher = Aes256Gcm::new_from_slice(key.as_ref()).map_err(|_| unavailable())?;
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: bytes,
                    aad: token.as_bytes(),
                },
            )
            .map_err(|_| unavailable())?;
        Ok([nonce.as_slice(), ciphertext.as_slice()].concat())
    }

    pub(super) async fn open(token: &str, bytes: &[u8]) -> io::Result<Vec<u8>> {
        let encoded = secret_tool("lookup", token, None).await?;
        let key = encoded.strip_suffix(b"\n").unwrap_or(&encoded);
        let key = Zeroizing::new(hex::decode(key).map_err(|_| unavailable())?);
        let (nonce, ciphertext) = bytes.split_at_checked(12).ok_or_else(unavailable)?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| unavailable())?;
        cipher
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: ciphertext,
                    aad: token.as_bytes(),
                },
            )
            .map_err(|_| unavailable())
    }
}

#[cfg(target_os = "linux")]
pub(super) async fn seal(token: &str, bytes: &[u8]) -> io::Result<Vec<u8>> {
    linux::seal(token, bytes).await
}

#[cfg(target_os = "linux")]
pub(super) async fn open(token: &str, bytes: &[u8]) -> io::Result<Vec<u8>> {
    linux::open(token, bytes).await
}
