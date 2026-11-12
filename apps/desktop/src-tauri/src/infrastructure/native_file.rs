use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_CONFIG: u64 = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct Stamp {
    pub(super) id: [u64; 2],
    pub(super) digest: [u8; 32],
    mode: u32,
    owner: [u32; 2],
    access: [u8; 32],
}

impl Stamp {
    pub(super) fn same_permissions(&self, other: &Self) -> bool {
        self.mode == other.mode && self.owner == other.owner && self.access == other.access
    }
}

pub(super) struct Snapshot {
    pub(super) stamp: Stamp,
    pub(super) bytes: Vec<u8>,
}

pub(super) fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn unsafe_path() -> io::Error {
    io::Error::other("Unsupported file or directory")
}

fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .mode(0o600);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options
}

#[cfg(unix)]
fn file_details(file: &File, directory: bool) -> io::Result<([u64; 2], u32, [u32; 2])> {
    use std::os::{fd::AsRawFd, unix::fs::MetadataExt};
    let info = file.metadata()?;
    let uid = unsafe { libc::geteuid() };
    if directory {
        if !info.is_dir() || info.uid() != uid || info.mode() & 0o022 != 0 {
            return Err(unsafe_path());
        }
    } else if !info.is_file()
        || info.nlink() != 1
        || info.uid() != uid
        || info.mode() & 0o7022 != 0
        || info.mode() & 0o200 == 0
        || unsafe { libc::flistxattr(file.as_raw_fd(), std::ptr::null_mut(), 0) } != 0
    {
        return Err(unsafe_path());
    }
    Ok((
        [info.dev(), info.ino()],
        info.mode() & 0o777,
        [info.uid(), info.gid()],
    ))
}

#[cfg(windows)]
fn file_details(file: &File, directory: bool) -> io::Result<([u64; 2], u32, [u32; 2])> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_COMPRESSED,
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_ENCRYPTED, FILE_ATTRIBUTE_READONLY,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_SPARSE_FILE,
    };
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let attributes = info.dwFileAttributes;
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || (attributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory
        || (!directory
            && (info.nNumberOfLinks != 1
                || attributes
                    & (FILE_ATTRIBUTE_READONLY
                        | FILE_ATTRIBUTE_COMPRESSED
                        | FILE_ATTRIBUTE_ENCRYPTED
                        | FILE_ATTRIBUTE_SPARSE_FILE)
                    != 0))
    {
        return Err(unsafe_path());
    }
    if !directory {
        check_windows_streams(file)?;
    }
    let index = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    Ok(([u64::from(info.dwVolumeSerialNumber), index], 0, [0, 0]))
}

#[cfg(windows)]
fn check_windows_streams(file: &File) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::ERROR_HANDLE_EOF,
        Storage::FileSystem::{FileStreamInfo, GetFileInformationByHandleEx, FILE_STREAM_INFO},
    };
    // Bound the query and reject extra streams rather than silently dropping metadata.
    let mut buffer = [0u64; 8192];
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileStreamInfo,
            buffer.as_mut_ptr().cast(),
            std::mem::size_of_val(&buffer) as u32,
        )
    } == 0
    {
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_HANDLE_EOF as i32) && file.metadata()?.len() == 0 {
            return Ok(());
        }
        return Err(error);
    }
    let stream = unsafe { &*buffer.as_ptr().cast::<FILE_STREAM_INFO>() };
    let default_name: Vec<u16> = "::$DATA".encode_utf16().collect();
    if stream.NextEntryOffset != 0 || stream.StreamNameLength as usize != default_name.len() * 2 {
        return Err(unsafe_path());
    }
    let name =
        unsafe { std::slice::from_raw_parts(stream.StreamName.as_ptr(), default_name.len()) };
    if name != default_name {
        return Err(unsafe_path());
    }
    Ok(())
}

fn directory(path: &Path) -> io::Result<File> {
    let actual = fs::canonicalize(path)?;
    #[cfg(windows)]
    let actual = actual
        .to_str()
        .and_then(|text| text.strip_prefix(r"\\?\"))
        .map_or(actual.clone(), PathBuf::from);
    if actual != path {
        return Err(unsafe_path());
    }
    let mut open = options();
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        };
        open.custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    }
    open.read(true).open(path)
}

pub(super) fn parent_id(path: &Path) -> io::Result<[u64; 2]> {
    let parent = path.parent().ok_or_else(unsafe_path)?;
    let handle = directory(parent)?;
    if !local_volume(parent, &handle) {
        return Err(unsafe_path());
    }
    Ok(file_details(&handle, true)?.0)
}

#[cfg(unix)]
fn local_volume(_: &Path, handle: &File) -> bool {
    use std::os::fd::AsRawFd;
    let mut info = std::mem::MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::fstatfs(handle.as_raw_fd(), info.as_mut_ptr()) } != 0 {
        return false;
    }
    let filesystem = unsafe { info.assume_init() }.f_type as u64;
    // ext, XFS, Btrfs and OverlayFS support the rename and sync protocol.
    matches!(filesystem, 0xef53 | 0x58465342 | 0x9123683e | 0x794c7630)
}

#[cfg(windows)]
fn local_volume(path: &Path, _: &File) -> bool {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumePathNameW};
    let name = wide(path.as_os_str());
    let mut root = [0u16; 260];
    let ok = unsafe { GetVolumePathNameW(name.as_ptr(), root.as_mut_ptr(), root.len() as u32) };
    ok != 0 && unsafe { GetDriveTypeW(root.as_ptr()) } == 3 // DRIVE_FIXED
}

#[cfg(windows)]
fn access_fingerprint(file: &File) -> io::Result<[u8; 32]> {
    use std::{os::windows::io::AsRawHandle, ptr::null_mut};
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
            GetLengthSid, ACL, DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION,
            OWNER_SECURITY_INFORMATION,
        },
    };
    let (mut owner, mut group, mut acl, mut descriptor) =
        (null_mut(), null_mut(), null_mut::<ACL>(), null_mut());
    let result = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            &mut group,
            &mut acl,
            null_mut(),
            &mut descriptor,
        )
    };
    if result != 0 {
        return Err(io::Error::from_raw_os_error(result as i32));
    }
    let mut hasher = Sha256::new();
    let valid = !owner.is_null() && !group.is_null() && !acl.is_null();
    if valid {
        unsafe {
            hasher.update(std::slice::from_raw_parts(
                owner.cast::<u8>(),
                GetLengthSid(owner) as usize,
            ));
            hasher.update(std::slice::from_raw_parts(
                group.cast::<u8>(),
                GetLengthSid(group) as usize,
            ));
            hasher.update(std::slice::from_raw_parts(
                acl.cast::<u8>(),
                usize::from((*acl).AclSize),
            ));
        }
    }
    unsafe { LocalFree(descriptor) };
    if valid {
        Ok(hasher.finalize().into())
    } else {
        Err(unsafe_path())
    }
}

pub(super) fn snapshot(path: &Path) -> io::Result<Option<Snapshot>> {
    let file = match options().read(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let (id, mode, owner) = file_details(&file, false)?;
    #[cfg(windows)]
    let access = access_fingerprint(&file)?;
    #[cfg(unix)]
    let access = [0; 32];
    let mut bytes = Vec::new();
    file.take(MAX_CONFIG + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_CONFIG {
        return Err(unsafe_path());
    }
    Ok(Some(Snapshot {
        stamp: Stamp {
            id,
            digest: digest(&bytes),
            mode,
            owner,
            access,
        },
        bytes,
    }))
}

pub(super) fn private_directory(path: &Path) -> io::Result<()> {
    parent_id(path)?;
    #[cfg(unix)]
    let builder = {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = fs::DirBuilder::new();
        builder.mode(0o700);
        builder
    };
    #[cfg(windows)]
    let builder = fs::DirBuilder::new();
    match builder.create(path) {
        Ok(()) => {
            sync_parent(path)?;
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    let (_, mode, _) = file_details(&directory(path)?, true)?;
    if mode & 0o077 != 0 {
        return Err(unsafe_path());
    }
    Ok(())
}

pub(super) fn app_lock(path: &Path) -> io::Result<File> {
    parent_id(path)?;
    let lock = options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    file_details(&lock, false)?;
    lock.try_lock().map_err(io::Error::from)?;
    Ok(lock)
}

pub(super) struct GitLock {
    path: PathBuf,
    token: String,
}

impl GitLock {
    fn path(target: &Path) -> PathBuf {
        let mut path = target.as_os_str().to_owned();
        path.push(".lock");
        path.into()
    }

    pub(super) fn clear_owned(target: &Path, token: &str) -> io::Result<()> {
        let path = Self::path(target);
        let marker = format!("GitPilot {token}\n");
        if let Some(contents) = snapshot(&path)? {
            if contents.bytes != marker.as_bytes() {
                return Err(io::Error::from(io::ErrorKind::AlreadyExists));
            }
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub(super) fn acquire(target: &Path, token: &str) -> io::Result<Self> {
        let path = Self::path(target);
        let mut file = options().create_new(true).write(true).open(&path)?;
        let lock = Self {
            path,
            token: token.into(),
        };
        writeln!(file, "GitPilot {token}")?;
        file.sync_all()?;
        Ok(lock)
    }
}

impl Drop for GitLock {
    fn drop(&mut self) {
        let marker = format!("GitPilot {}\n", self.token);
        if snapshot(&self.path)
            .is_ok_and(|file| file.is_some_and(|file| file.bytes == marker.as_bytes()))
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(super) fn create_immutable(path: &Path, bytes: &[u8]) -> io::Result<()> {
    match options().create_new(true).write(true).open(path) {
        Ok(mut file) => {
            file.write_all(bytes)?;
            file.sync_all()?;
            sync_parent(path)
        }
        Err(error)
            if error.kind() == io::ErrorKind::AlreadyExists
                && snapshot(path)?.is_some_and(|existing| existing.bytes == bytes) =>
        {
            Ok(())
        }
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().chain([0]).collect()
}

#[cfg(windows)]
fn restrict_stage(path: &Path) -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, SetFileSecurityW,
            DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
        },
    };
    let sddl = wide(std::ffi::OsStr::new("D:P(A;;FA;;;OW)(A;;FA;;;SY)"));
    let mut descriptor = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let name = wide(path.as_os_str());
    let result = unsafe {
        SetFileSecurityW(
            name.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        )
    };
    let error = (result == 0).then(io::Error::last_os_error);
    unsafe { LocalFree(descriptor) };
    error.map_or(Ok(()), Err)
}

pub(super) struct Stage {
    file: Option<File>,
    id: [u64; 2],
    pub(super) path: PathBuf,
}

impl Stage {
    pub(super) fn make_executable(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            self.file
                .as_ref()
                .ok_or_else(unsafe_path)?
                .set_permissions(fs::Permissions::from_mode(0o700))?;
        }
        Ok(())
    }
    pub(super) fn create(path: PathBuf, before: Option<&Stamp>) -> io::Result<Self> {
        let file = options()
            .create_new(true)
            .read(true)
            .write(true)
            .open(&path)?;
        let (id, _, owner) = file_details(&file, false)?;
        let stage = Self {
            file: Some(file),
            id,
            path,
        };
        #[cfg(windows)]
        {
            let _ = (before, owner);
            restrict_stage(&stage.path)?;
        }
        #[cfg(unix)]
        if let Some(before) = before {
            if owner != before.owner {
                return Err(unsafe_path());
            }
        }
        Ok(stage)
    }

    pub(super) fn id(&self) -> [u64; 2] {
        self.id
    }

    pub(super) fn write(&mut self, bytes: &[u8], before: Option<&Stamp>) -> io::Result<()> {
        let mut file = self.file.take().ok_or_else(unsafe_path)?;
        file.write_all(bytes)?;
        #[cfg(unix)]
        if let Some(before) = before {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(before.mode))?;
        }
        #[cfg(windows)]
        let _ = before;
        file.sync_all()
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        if let Ok(Some(file)) = snapshot(&self.path) {
            if file.stamp.id == self.id {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}

#[cfg(windows)]
fn preserve_access(stage: &Path, target: &Path) -> io::Result<()> {
    use std::{
        os::windows::{fs::MetadataExt, io::AsRawHandle},
        ptr::null_mut,
    };
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
            GetSecurityDescriptorControl, SetFileSecurityW, DACL_SECURITY_INFORMATION,
            GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
            PROTECTED_DACL_SECURITY_INFORMATION, SE_DACL_PROTECTED,
            UNPROTECTED_DACL_SECURITY_INFORMATION,
        },
        Storage::FileSystem::SetFileAttributesW,
    };
    let original = options().read(true).open(target)?;
    file_details(&original, false)?;
    let expected = access_fingerprint(&original)?;
    let information =
        OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
    let mut descriptor = null_mut();
    let result = unsafe {
        GetSecurityInfo(
            original.as_raw_handle(),
            SE_FILE_OBJECT,
            information,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            &mut descriptor,
        )
    };
    if result != 0 {
        return Err(io::Error::from_raw_os_error(result as i32));
    }
    let copied = (|| {
        let (mut control, mut revision) = (0, 0);
        if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let inheritance = if control & SE_DACL_PROTECTED != 0 {
            PROTECTED_DACL_SECURITY_INFORMATION
        } else {
            UNPROTECTED_DACL_SECURITY_INFORMATION
        };
        if unsafe {
            SetFileSecurityW(
                wide(stage.as_os_str()).as_ptr(),
                information | inheritance,
                descriptor,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        if unsafe {
            SetFileAttributesW(
                wide(stage.as_os_str()).as_ptr(),
                original.metadata()?.file_attributes(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        // Publish this verified descriptor without ReplaceFileW's inheritance merge.
        let staged = options().read(true).open(stage)?;
        file_details(&staged, false)?;
        if access_fingerprint(&staged)? != expected || access_fingerprint(&original)? != expected {
            return Err(unsafe_path());
        }
        Ok(())
    })();
    unsafe { LocalFree(descriptor) };
    copied
}

pub(super) fn replace(stage: &Path, target: &Path, existed: bool) -> io::Result<()> {
    #[cfg(unix)]
    {
        if existed {
            fs::rename(stage, target)?;
        } else {
            use std::{ffi::CString, os::unix::ffi::OsStrExt};
            let stage = CString::new(stage.as_os_str().as_bytes()).map_err(|_| unsafe_path())?;
            let target = CString::new(target.as_os_str().as_bytes()).map_err(|_| unsafe_path())?;
            if unsafe {
                libc::renameat2(
                    libc::AT_FDCWD,
                    stage.as_ptr(),
                    libc::AT_FDCWD,
                    target.as_ptr(),
                    libc::RENAME_NOREPLACE,
                )
            } != 0
            {
                return Err(io::Error::last_os_error());
            }
        }
    }
    #[cfg(windows)]
    {
        if stage.parent() != target.parent() {
            return Err(unsafe_path());
        }
        if existed {
            preserve_access(stage, target)?;
        }
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        // Staging is in the same directory; never allow cross-volume copy fallback.
        // ReplaceFileW can duplicate legacy inherited ACEs on Windows Server.
        let flags = MOVEFILE_WRITE_THROUGH
            | if existed {
                MOVEFILE_REPLACE_EXISTING
            } else {
                0
            };
        let result = unsafe {
            MoveFileExW(
                wide(stage.as_os_str()).as_ptr(),
                wide(target.as_os_str()).as_ptr(),
                flags,
            )
        };
        if result == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    sync_parent(target)
}

pub(super) fn sync_parent(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    directory(path.parent().ok_or_else(unsafe_path)?)?.sync_all()?;
    #[cfg(windows)]
    let _ = path;
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn security_description(path: &Path) -> String {
        use std::{os::windows::io::AsRawHandle, ptr::null_mut};
        use windows_sys::Win32::{
            Foundation::LocalFree,
            Security::{
                Authorization::{
                    ConvertSecurityDescriptorToStringSecurityDescriptorW, GetSecurityInfo,
                    SE_FILE_OBJECT,
                },
                DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
            },
        };
        let file = options().read(true).open(path).unwrap();
        let information =
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
        let mut descriptor = null_mut();
        assert_eq!(
            unsafe {
                GetSecurityInfo(
                    file.as_raw_handle(),
                    SE_FILE_OBJECT,
                    information,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    &mut descriptor,
                )
            },
            0
        );
        let mut text = null_mut();
        let mut length = 0;
        let result = unsafe {
            ConvertSecurityDescriptorToStringSecurityDescriptorW(
                descriptor,
                1,
                information,
                &mut text,
                &mut length,
            )
        };
        unsafe { LocalFree(descriptor) };
        assert_ne!(result, 0);
        let description =
            String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, length as usize) });
        unsafe { LocalFree(text.cast()) };
        description
    }

    #[test]
    fn replacement_preserves_explicit_windows_security_group() {
        check_replacement(false);
    }

    #[test]
    fn replacement_preserves_legacy_inherited_windows_acl() {
        check_replacement(true);
    }

    #[test]
    fn refuses_windows_configuration_with_alternate_streams() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("config");
        fs::write(&target, b"original").unwrap();
        let stream = directory.path().join("config:fixture");
        fs::write(&stream, b"metadata").unwrap();
        assert!(snapshot(&target).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"original");
        assert_eq!(fs::read(&stream).unwrap(), b"metadata");
    }

    fn check_replacement(legacy: bool) {
        use windows_sys::Win32::{
            Foundation::LocalFree,
            Security::{
                Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW,
                SetFileSecurityW, DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION,
            },
        };
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("config");
        fs::write(&target, b"original").unwrap();
        let group = wide(std::ffi::OsStr::new("G:WD"));
        let mut descriptor = std::ptr::null_mut();
        assert_ne!(
            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    group.as_ptr(),
                    1,
                    &mut descriptor,
                    std::ptr::null_mut(),
                )
            },
            0
        );
        let changed = unsafe {
            SetFileSecurityW(
                wide(target.as_os_str()).as_ptr(),
                GROUP_SECURITY_INFORMATION,
                descriptor,
            )
        };
        let error = io::Error::last_os_error();
        unsafe { LocalFree(descriptor) };
        assert_ne!(changed, 0, "{error}");
        if legacy {
            let sddl = security_description(&target)
                .trim_end_matches('\0')
                .replace("D:AI", "D:");
            let text = wide(std::ffi::OsStr::new(&sddl));
            let mut descriptor = std::ptr::null_mut();
            assert_ne!(
                unsafe {
                    ConvertStringSecurityDescriptorToSecurityDescriptorW(
                        text.as_ptr(),
                        1,
                        &mut descriptor,
                        std::ptr::null_mut(),
                    )
                },
                0
            );
            let result = unsafe {
                SetFileSecurityW(
                    wide(target.as_os_str()).as_ptr(),
                    DACL_SECURITY_INFORMATION,
                    descriptor,
                )
            };
            unsafe { LocalFree(descriptor) };
            assert_ne!(result, 0);
            assert!(!security_description(&target).contains("D:AI"));
        }
        let before = snapshot(&target).unwrap().unwrap().stamp;
        let description = security_description(&target);
        let mut stage = Stage::create(directory.path().join("stage"), Some(&before)).unwrap();
        stage.write(b"changed", Some(&before)).unwrap();
        replace(&stage.path, &target, true).unwrap();
        let after = snapshot(&target).unwrap().unwrap();
        assert_eq!(after.bytes, b"changed");
        assert!(
            before.same_permissions(&after.stamp),
            "Windows ownership/group/ACL changed: before={description:?}; after={:?}",
            security_description(&target)
        );
    }
}
