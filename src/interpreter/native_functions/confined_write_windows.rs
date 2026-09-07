//! Windows NT handle-relative publication. cap-std's Windows rename/link/remove
//! helpers reconstruct paths, so they must not be used at this boundary.
//! ABI contracts: Microsoft NtCreateFile (winternl.h), FILE_RENAME_INFORMATION
//! (ntifs.h), and FILE_DISPOSITION_INFORMATION (ntddk.h).
use cap_std::{ambient_authority, fs::Dir};
use std::ffi::{c_void, OsStr};
use std::fs::File;
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};

#[repr(C)]
struct UnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *mut u16,
}
#[repr(C)]
struct ObjectAttributes {
    length: u32,
    root: *mut c_void,
    name: *mut UnicodeString,
    attributes: u32,
    security_descriptor: *mut c_void,
    security_qos: *mut c_void,
}
#[repr(C)]
struct IoStatus {
    status: usize,
    information: usize,
}
#[repr(C)]
struct RenameInformation {
    replace: u8,
    root: *mut c_void,
    name_length: u32,
    name: [u16; 1],
}

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtCreateFile(
        handle: *mut *mut c_void,
        access: u32,
        attributes: *mut ObjectAttributes,
        status: *mut IoStatus,
        allocation: *mut i64,
        file_attributes: u32,
        share: u32,
        disposition: u32,
        options: u32,
        ea: *mut c_void,
        ea_length: u32,
    ) -> i32;
    fn NtSetInformationFile(
        handle: *mut c_void,
        status: *mut IoStatus,
        information: *mut c_void,
        length: u32,
        class: u32,
    ) -> i32;
    fn RtlNtStatusToDosError(status: i32) -> u32;
}

fn status_result(status: i32) -> io::Result<()> {
    if status >= 0 {
        Ok(())
    } else {
        // SAFETY: RtlNtStatusToDosError accepts any NTSTATUS value.
        Err(io::Error::from_raw_os_error(unsafe { RtlNtStatusToDosError(status) } as i32))
    }
}

fn open_at(
    parent: &File,
    name: &OsStr,
    access: u32,
    disposition: u32,
    directory: bool,
) -> io::Result<File> {
    let mut wide: Vec<u16> = name.encode_wide().collect();
    let bytes = u16::try_from(wide.len() * 2)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "component too long"))?;
    let mut unicode =
        UnicodeString { length: bytes, maximum_length: bytes, buffer: wide.as_mut_ptr() };
    let mut attrs = ObjectAttributes {
        length: std::mem::size_of::<ObjectAttributes>() as u32,
        root: parent.as_raw_handle(),
        name: &mut unicode,
        attributes: 0x40,
        security_descriptor: std::ptr::null_mut(),
        security_qos: std::ptr::null_mut(),
    };
    let mut status = IoStatus { status: 0, information: 0 };
    let mut handle = std::ptr::null_mut();
    // Synchronous + OPEN_REPARSE_POINT: open the entry, never its reparse target.
    let options = 0x20 | 0x0020_0000 | if directory { 1 } else { 0x40 };
    // SAFETY: all structures/buffers remain alive for the synchronous call. The
    // returned handle is adopted exactly once only after successful completion.
    let result = unsafe {
        NtCreateFile(
            &mut handle,
            access,
            &mut attrs,
            &mut status,
            std::ptr::null_mut(),
            if directory { 0x10 } else { 0x80 },
            7,
            disposition,
            options,
            std::ptr::null_mut(),
            0,
        )
    };
    status_result(result)?;
    Ok(unsafe { File::from_raw_handle(handle) })
}

fn publish(file: &File, directory: &File, name: &OsStr, overwrite: bool) -> io::Result<()> {
    let wide: Vec<u16> = name.encode_wide().collect();
    let length = std::mem::size_of::<RenameInformation>() + wide.len() * 2;
    // usize storage guarantees alignment for the variable-length NT structure.
    let mut storage = vec![
        0usize;
        length
            .max(std::mem::size_of::<RenameInformation>())
            .div_ceil(std::mem::size_of::<usize>())
    ];
    let info = storage.as_mut_ptr().cast::<RenameInformation>();
    let mut status = IoStatus { status: 0, information: 0 };
    // SAFETY: storage is aligned and large enough for the header and UTF-16 name.
    // RootDirectory is the retained destination handle, not a reconstructed path.
    let result = unsafe {
        (*info).replace = u8::from(overwrite);
        (*info).root = directory.as_raw_handle();
        (*info).name_length = (wide.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            wide.as_ptr(),
            std::ptr::addr_of_mut!((*info).name).cast::<u16>(),
            wide.len(),
        );
        NtSetInformationFile(file.as_raw_handle(), &mut status, info.cast(), length as u32, 10)
    };
    status_result(result)
}

fn discard(file: &File) -> io::Result<()> {
    let mut delete: u8 = 1;
    let mut status = IoStatus { status: 0, information: 0 };
    // SAFETY: FileDispositionInformation takes one BOOLEAN; this marks only the
    // opened temporary file for deletion when its last handle closes.
    status_result(unsafe {
        NtSetInformationFile(
            file.as_raw_handle(),
            &mut status,
            (&mut delete as *mut u8).cast(),
            1,
            13,
        )
    })
}

pub(super) fn write(
    root: &str,
    components: &[&OsStr],
    payload: &[u8],
    overwrite: bool,
    mut hook: impl FnMut(&str),
) -> Result<(), String> {
    let error = |stage: &str, e: io::Error| format!("write_file_atomic_beneath[{stage}]: {e}");
    let mut directory = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|e| error("root_open_failed", e))?
        .into_std_file();
    for component in &components[..components.len() - 1] {
        // FILE_OPEN_IF, directory traversal + attributes + synchronous access.
        let next = open_at(&directory, component, 0x0010_00a1, 3, true)
            .map_err(|e| error("component_rejected", e))?;
        let metadata = next.metadata().map_err(|e| error("component_metadata_failed", e))?;
        if !metadata.is_dir() || metadata.file_attributes() & 0x400 != 0 {
            return Err("write_file_atomic_beneath[component_rejected]: reparse parent".into());
        }
        directory = next;
    }
    hook("parent_opened");
    let temporary = format!(".kujo-beneath-{}.tmp", uuid::Uuid::new_v4());
    // GENERIC_WRITE | DELETE | SYNCHRONIZE | FILE_READ_ATTRIBUTES; FILE_CREATE.
    let mut file = open_at(&directory, OsStr::new(&temporary), 0x4011_0080, 2, false)
        .map_err(|e| error("temporary_create_failed", e))?;
    let result = (|| {
        file.write_all(payload).map_err(|e| error("write_failed", e))?;
        file.sync_all().map_err(|e| error("sync_failed", e))?;
        hook("before_publish");
        let target = components[components.len() - 1];
        match open_at(&directory, target, 0x0010_0080, 1, false) {
            Ok(existing) => {
                let metadata =
                    existing.metadata().map_err(|e| error("target_metadata_failed", e))?;
                if !metadata.is_file() || metadata.file_attributes() & 0x400 != 0 {
                    return Err(
                        "write_file_atomic_beneath[target_rejected]: not a regular file".into()
                    );
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => return Err(error("target_rejected", e)),
        }
        publish(&file, &directory, target, overwrite).map_err(|e| error("publish_failed", e))
    })();
    if let Err(original) = result {
        if let Err(cleanup) = discard(&file) {
            return Err(format!(
                "write_file_atomic_beneath[cleanup_failed]: {cleanup}; original: {original}"
            ));
        }
        return Err(original);
    }
    Ok(())
}
