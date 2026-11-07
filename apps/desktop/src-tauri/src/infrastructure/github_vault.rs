use std::path::Path;

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::github::GithubError;

pub(super) struct Vault {
    namespace: String,
}

impl Vault {
    pub(super) fn new(directory: &Path) -> Self {
        let digest = Sha256::digest(directory.as_os_str().as_encoded_bytes());
        Self {
            namespace: hex::encode(&digest[..16]),
        }
    }

    fn target(&self, reference: &str) -> Result<String, GithubError> {
        if reference.len() != 48 || !reference.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(GithubError::Storage);
        }
        Ok(format!("GitPilot/GitHub/{}/{reference}", self.namespace))
    }

    pub(super) async fn write(&self, reference: &str, value: &[u8]) -> Result<(), GithubError> {
        let target = self.target(reference)?;
        if value.len() > 5000 {
            return Err(GithubError::Protocol);
        }
        #[cfg(windows)]
        {
            windows::write(&target, value)
        }
        #[cfg(target_os = "linux")]
        {
            self.secret_tool("store", &target, Some(value))
                .await
                .map(|_| ())
        }
    }

    pub(super) async fn read(&self, reference: &str) -> Result<Zeroizing<Vec<u8>>, GithubError> {
        let target = self.target(reference)?;
        #[cfg(windows)]
        {
            windows::read(&target)
        }
        #[cfg(target_os = "linux")]
        {
            self.secret_tool("lookup", &target, None).await
        }
    }

    pub(super) async fn delete(&self, reference: &str) -> Result<(), GithubError> {
        let target = self.target(reference)?;
        #[cfg(windows)]
        {
            windows::delete(&target)
        }
        #[cfg(target_os = "linux")]
        {
            let _ = self.secret_tool("clear", &target, None).await;
            let remaining = self.secret_tool("search", &target, None).await?;
            remaining
                .is_empty()
                .then_some(())
                .ok_or(GithubError::SecureStore)
        }
    }

    #[cfg(target_os = "linux")]
    async fn secret_tool(
        &self,
        operation: &str,
        target: &str,
        value: Option<&[u8]>,
    ) -> Result<Zeroizing<Vec<u8>>, GithubError> {
        use tokio::process::Command;
        let mut command = Command::new("/usr/bin/secret-tool");
        command.arg(operation);
        if operation == "search" {
            command.arg("--all");
        }
        if value.is_some() {
            command.arg("--label=GitPilot GitHub account");
        }
        command.args(["application", "dev.gitpilot.desktop", "credential", target]);
        let (status, bytes) =
            super::git::output_with_input(&mut command, std::time::Duration::from_secs(15), value)
                .await
                .map_err(|_| GithubError::SecureStore)?;
        let mut bytes = Zeroizing::new(bytes);
        if !status.success() || bytes.len() > 5000 {
            return Err(GithubError::SecureStore);
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        Ok(bytes)
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ptr::{null_mut, write_bytes};
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_NOT_FOUND},
        Security::Credentials::{
            CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
            CRED_TYPE_GENERIC,
        },
    };

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(Some(0)).collect()
    }

    pub(super) fn write(target: &str, bytes: &[u8]) -> Result<(), GithubError> {
        let name = wide(target);
        let user = wide("GitPilot GitHub");
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: name.as_ptr().cast_mut(),
            CredentialBlobSize: bytes.len() as u32,
            CredentialBlob: bytes.as_ptr().cast_mut(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            UserName: user.as_ptr().cast_mut(),
            ..Default::default()
        };
        if unsafe { CredWriteW(&credential, 0) } == 0 {
            return Err(GithubError::SecureStore);
        }
        Ok(())
    }

    pub(super) fn read(target: &str) -> Result<Zeroizing<Vec<u8>>, GithubError> {
        let name = wide(target);
        let mut credential = null_mut();
        if unsafe { CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) } == 0 {
            return Err(GithubError::SecureStore);
        }
        let credential_ref = unsafe { &*credential };
        let result = if credential_ref.CredentialBlobSize > 5000
            || credential_ref.CredentialBlob.is_null()
        {
            Err(GithubError::SecureStore)
        } else {
            Ok(Zeroizing::new(
                unsafe {
                    std::slice::from_raw_parts(
                        credential_ref.CredentialBlob,
                        credential_ref.CredentialBlobSize as usize,
                    )
                }
                .to_vec(),
            ))
        };
        if !credential_ref.CredentialBlob.is_null() {
            unsafe {
                write_bytes(
                    credential_ref.CredentialBlob,
                    0,
                    credential_ref.CredentialBlobSize as usize,
                )
            };
        }
        unsafe { CredFree(credential.cast()) };
        result
    }

    pub(super) fn delete(target: &str) -> Result<(), GithubError> {
        let name = wide(target);
        if unsafe { CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0
            && unsafe { GetLastError() } != ERROR_NOT_FOUND
        {
            return Err(GithubError::SecureStore);
        }
        Ok(())
    }
}
