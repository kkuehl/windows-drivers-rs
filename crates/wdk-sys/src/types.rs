// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

pub use bindings::*;

#[allow(missing_docs)]
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[rustversion::attr(
    any(
        all(not(nightly), since(1.88)),
        all(nightly, since(2025-04-25)),
    ),
    allow(unnecessary_transmutes)
)]
#[allow(unsafe_op_in_unsafe_fn)]
#[allow(clippy::cast_lossless)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::doc_markdown)]
#[allow(clippy::default_trait_access)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[rustversion::attr(
    any(
        all(not(nightly), before(1.74)),
        all(nightly, before(2023-09-13)),
    ),
    allow(clippy::incorrect_clone_impl_on_copy_type)
)]
#[rustversion::attr(
    any(
        all(not(nightly), since(1.74)),
        all(nightly, since(2023-09-13)),
    ),
    allow(clippy::non_canonical_clone_impl)
)]
#[allow(clippy::missing_const_for_fn)]
#[allow(clippy::missing_safety_doc)]
#[allow(clippy::module_name_repetitions)]
#[allow(clippy::multiple_unsafe_ops_per_block)]
#[allow(clippy::must_use_candidate)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[allow(clippy::ptr_as_ptr)]
#[allow(clippy::ptr_offset_with_cast)]
#[rustversion::attr(
    any(
        all(not(nightly), since(1.77)),
        all(nightly, since(2024-01-11)),
    ),
    allow(clippy::pub_underscore_fields)
)]
#[rustversion::attr(
    any(
        all(not(nightly), since(1.78)),
        all(nightly, since(2024-02-09)),
    ),
    allow(clippy::ref_as_ptr)
)]
#[allow(clippy::semicolon_if_nothing_returned)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
#[allow(clippy::transmute_ptr_to_ptr)]
#[allow(clippy::undocumented_unsafe_blocks)]
#[allow(clippy::unnecessary_cast)]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::used_underscore_binding)]
#[allow(clippy::useless_transmute)]
#[allow(clippy::use_self)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/types.rs"));
}

// MDL (Memory Descriptor List) types from wdm.h
//
// These types are used by the MDL functions in ntddk.rs. The MDL structure
// itself
/// Memory Descriptor List (MDL) structure.
///
/// An MDL describes a buffer in memory by dividing it into physical pages.
/// This is the kernel's mechanism for safely accessing user-mode memory.
///
/// # Layout
///
/// This structure matches the Windows DDK `_MDL` definition from `wdm.h`.
/// While Microsoft documentation says drivers should not access MDL fields
/// directly, the `MmGetSystemAddressForMdlSafe` macro requires reading
/// `MdlFlags` and `MappedSystemVa`, so we expose the full structure.
#[repr(C)]
pub struct MDL {
    /// Pointer to the next MDL in a chain.
    pub Next: PMDL,
    /// Size of this MDL in bytes.
    pub Size: SHORT,
    /// MDL flags (see MDL_* constants).
    pub MdlFlags: USHORT,
    /// Process that owns the buffer (NULL for kernel buffers).
    pub Process: *mut core::ffi::c_void,
    /// Kernel virtual address if mapped, NULL otherwise.
    pub MappedSystemVa: PVOID,
    /// Starting virtual address of the buffer.
    pub StartVa: PVOID,
    /// Number of bytes in the buffer.
    pub ByteCount: ULONG,
    /// Offset from StartVa to the beginning of the buffer.
    pub ByteOffset: ULONG,
}

/// Pointer to an MDL.
pub type PMDL = *mut MDL;

/// Specifies the type of access for which pages should be locked.
///
/// Used with `MmProbeAndLockPages`.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LOCK_OPERATION {
    /// Lock pages for read access.
    IoReadAccess = 0,
    /// Lock pages for write access.
    IoWriteAccess = 1,
    /// Lock pages for both read and write access.
    IoModifyAccess = 2,
}

// Page priority constants for MmGetSystemAddressForMdlSafe
/// Normal page priority (typical case).
pub const NORMAL_PAGE_PRIORITY: ULONG = 16;
/// Memory caching types for `MmMapLockedPagesSpecifyCache`.
///
/// These values control how the mapped pages are cached in the processor's
/// TLBs.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MEMORY_CACHING_TYPE {
    /// Non-cached memory.
    MmNonCached = 0,
    /// Cached memory (typical case).
    MmCached = 1,
    /// Write-combined memory (for frame buffers).
    MmWriteCombined = 2,
    /// Hardware coherent cached memory.
    MmHardwareCoherentCached = 3,
    /// Non-cached unordered memory.
    MmNonCachedUnordered = 4,
    /// USB cached memory.
    MmUSBCached = 5,
    /// Maximum value (for validation).
    MmMaximumCacheType = 6,
}

/// Information class for ZwQueryValueKey.
///
/// Specifies the type of information to be returned about a registry key value.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KEY_VALUE_INFORMATION_CLASS {
    /// Returns KEY_VALUE_BASIC_INFORMATION.
    KeyValueBasicInformation = 0,
    /// Returns KEY_VALUE_FULL_INFORMATION.
    KeyValueFullInformation = 1,
    /// Returns KEY_VALUE_PARTIAL_INFORMATION (most commonly used).
    KeyValuePartialInformation = 2,
    /// Returns KEY_VALUE_FULL_INFORMATION_ALIGN64.
    KeyValueFullInformationAlign64 = 3,
    /// Returns KEY_VALUE_PARTIAL_INFORMATION_ALIGN64.
    KeyValuePartialInformationAlign64 = 4,
    /// Returns KEY_VALUE_LAYER_INFORMATION.
    KeyValueLayerInformation = 5,
    /// Maximum value.
    MaxKeyValueInfoClass = 6,
}

/// File information class for ZwQueryInformationFile.
///
/// Specifies the type of information to query about a file object.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FILE_INFORMATION_CLASS {
    // THIS TYPE SHADOWS A GENERATED BINDING OF THE SAME NAME. Know that before editing it.
    //
    // `types.rs:4` is `pub use bindings::*`, and bindgen emits its own `FILE_INFORMATION_CLASS` --
    // a `::core::ffi::c_int` alias beside a `_FILE_INFORMATION_CLASS` module of 84 constants, which
    // is its constified-enum style. An explicit item beats a glob import in Rust name resolution, so
    // `wdk_sys::FILE_INFORMATION_CLASS` means THIS enum, not that alias.
    //
    // That shadowing is what made the truncation below dangerous rather than merely untidy. The
    // generated alias is a bare `c_int` with no validity invariant, so converting any integer to it
    // is sound; this enum has exactly 84 valid bit patterns, so a `transmute` of a value with no
    // variant is undefined behaviour. A caller reading `wdk_sys::FILE_INFORMATION_CLASS` cannot tell
    // which of the two it got -- and `mimic-drv` transmuted into it using the generated constant
    // `_FILE_INFORMATION_CLASS::FileKnownFolderInformation` (76), which is correct as an integer and
    // was invalid as this enum.
    //
    // Kept rather than deleted because a real enum is stronger typing than `c_int`: a caller cannot
    // pass an arbitrary integer. That holds only while the enum is COMPLETE and DENSE over the
    // header's range, which is now pinned by `file_information_class_enum_matches_the_wdk_header` in
    // `tests/config-minifilter/tests/minifilter_bindings.rs`. If it is ever shortened again, delete
    // it instead and let the generated alias through.
    //
    // TRANSCRIBED FROM THE WDK HEADER, and it must stay that way.
    //
    // Source: `km/wdm.h:7834-7946`, WDK 10.0.26100.0 -- `FileDirectoryInformation = 1` followed by
    // implicit increments for 83 more entries, so entry N has value N. Generated by reading that
    // range, never by editing the previous list, because a value written from the same source as the
    // thing it checks pins the error instead of catching it. `constants.rs:172-185` records this
    // crate being bitten by exactly that once already.
    //
    // WHAT WAS WRONG BEFORE, because it was UB and not merely incomplete. The previous list stopped
    // at `FileShortNameInformation = 40`, declared `FileMaximumInformation = 41`, and had a HOLE at
    // 30. Three consequences, the third being the dangerous one:
    //
    //   * 30 is `FileCompletionInformation`, missing outright.
    //   * 41 is `FileIoCompletionNotificationInformation`, a real and different class. So the name
    //     `FileMaximumInformation` was bound to a queryable class and any bound check written
    //     against it was off by 43.
    //   * 42..=83 did not exist at all. A `transmute` of any of them -- and
    //     `FileKnownFolderInformation` (76) is one this driver genuinely needs -- produced a value
    //     with no corresponding variant, which for a `#[repr(i32)]` enum is undefined behaviour
    //     rather than a wrong answer.
    /// `FileDirectoryInformation` - value 1.
    FileDirectoryInformation = 1,
    /// `FileFullDirectoryInformation` - value 2.
    FileFullDirectoryInformation = 2,
    /// `FileBothDirectoryInformation` - value 3.
    FileBothDirectoryInformation = 3,
    /// `FileBasicInformation` - value 4.
    FileBasicInformation = 4,
    /// `FileStandardInformation` - value 5.
    FileStandardInformation = 5,
    /// `FileInternalInformation` - value 6.
    FileInternalInformation = 6,
    /// `FileEaInformation` - value 7.
    FileEaInformation = 7,
    /// `FileAccessInformation` - value 8.
    FileAccessInformation = 8,
    /// `FileNameInformation` - value 9.
    FileNameInformation = 9,
    /// `FileRenameInformation` - value 10.
    FileRenameInformation = 10,
    /// `FileLinkInformation` - value 11.
    FileLinkInformation = 11,
    /// `FileNamesInformation` - value 12.
    FileNamesInformation = 12,
    /// `FileDispositionInformation` - value 13.
    FileDispositionInformation = 13,
    /// `FilePositionInformation` - value 14.
    FilePositionInformation = 14,
    /// `FileFullEaInformation` - value 15.
    FileFullEaInformation = 15,
    /// `FileModeInformation` - value 16.
    FileModeInformation = 16,
    /// `FileAlignmentInformation` - value 17.
    FileAlignmentInformation = 17,
    /// `FileAllInformation` - value 18.
    FileAllInformation = 18,
    /// `FileAllocationInformation` - value 19.
    FileAllocationInformation = 19,
    /// `FileEndOfFileInformation` - value 20.
    FileEndOfFileInformation = 20,
    /// `FileAlternateNameInformation` - value 21.
    FileAlternateNameInformation = 21,
    /// `FileStreamInformation` - value 22.
    FileStreamInformation = 22,
    /// `FilePipeInformation` - value 23.
    FilePipeInformation = 23,
    /// `FilePipeLocalInformation` - value 24.
    FilePipeLocalInformation = 24,
    /// `FilePipeRemoteInformation` - value 25.
    FilePipeRemoteInformation = 25,
    /// `FileMailslotQueryInformation` - value 26.
    FileMailslotQueryInformation = 26,
    /// `FileMailslotSetInformation` - value 27.
    FileMailslotSetInformation = 27,
    /// `FileCompressionInformation` - value 28.
    FileCompressionInformation = 28,
    /// `FileObjectIdInformation` - value 29.
    FileObjectIdInformation = 29,
    /// `FileCompletionInformation` - value 30.
    FileCompletionInformation = 30,
    /// `FileMoveClusterInformation` - value 31.
    FileMoveClusterInformation = 31,
    /// `FileQuotaInformation` - value 32.
    FileQuotaInformation = 32,
    /// `FileReparsePointInformation` - value 33.
    FileReparsePointInformation = 33,
    /// `FileNetworkOpenInformation` - value 34.
    FileNetworkOpenInformation = 34,
    /// `FileAttributeTagInformation` - value 35.
    FileAttributeTagInformation = 35,
    /// `FileTrackingInformation` - value 36.
    FileTrackingInformation = 36,
    /// `FileIdBothDirectoryInformation` - value 37.
    FileIdBothDirectoryInformation = 37,
    /// `FileIdFullDirectoryInformation` - value 38.
    FileIdFullDirectoryInformation = 38,
    /// `FileValidDataLengthInformation` - value 39.
    FileValidDataLengthInformation = 39,
    /// `FileShortNameInformation` - value 40.
    FileShortNameInformation = 40,
    /// `FileIoCompletionNotificationInformation` - value 41.
    FileIoCompletionNotificationInformation = 41,
    /// `FileIoStatusBlockRangeInformation` - value 42.
    FileIoStatusBlockRangeInformation = 42,
    /// `FileIoPriorityHintInformation` - value 43.
    FileIoPriorityHintInformation = 43,
    /// `FileSfioReserveInformation` - value 44.
    FileSfioReserveInformation = 44,
    /// `FileSfioVolumeInformation` - value 45.
    FileSfioVolumeInformation = 45,
    /// `FileHardLinkInformation` - value 46.
    FileHardLinkInformation = 46,
    /// `FileProcessIdsUsingFileInformation` - value 47.
    FileProcessIdsUsingFileInformation = 47,
    /// `FileNormalizedNameInformation` - value 48.
    FileNormalizedNameInformation = 48,
    /// `FileNetworkPhysicalNameInformation` - value 49.
    FileNetworkPhysicalNameInformation = 49,
    /// `FileIdGlobalTxDirectoryInformation` - value 50.
    FileIdGlobalTxDirectoryInformation = 50,
    /// `FileIsRemoteDeviceInformation` - value 51.
    FileIsRemoteDeviceInformation = 51,
    /// `FileUnusedInformation` - value 52.
    FileUnusedInformation = 52,
    /// `FileNumaNodeInformation` - value 53.
    FileNumaNodeInformation = 53,
    /// `FileStandardLinkInformation` - value 54.
    FileStandardLinkInformation = 54,
    /// `FileRemoteProtocolInformation` - value 55.
    FileRemoteProtocolInformation = 55,
    /// `FileRenameInformationBypassAccessCheck` - value 56.
    FileRenameInformationBypassAccessCheck = 56,
    /// `FileLinkInformationBypassAccessCheck` - value 57.
    FileLinkInformationBypassAccessCheck = 57,
    /// `FileVolumeNameInformation` - value 58.
    FileVolumeNameInformation = 58,
    /// `FileIdInformation` - value 59.
    FileIdInformation = 59,
    /// `FileIdExtdDirectoryInformation` - value 60.
    FileIdExtdDirectoryInformation = 60,
    /// `FileReplaceCompletionInformation` - value 61.
    FileReplaceCompletionInformation = 61,
    /// `FileHardLinkFullIdInformation` - value 62.
    FileHardLinkFullIdInformation = 62,
    /// `FileIdExtdBothDirectoryInformation` - value 63.
    FileIdExtdBothDirectoryInformation = 63,
    /// `FileDispositionInformationEx` - value 64.
    FileDispositionInformationEx = 64,
    /// `FileRenameInformationEx` - value 65.
    FileRenameInformationEx = 65,
    /// `FileRenameInformationExBypassAccessCheck` - value 66.
    FileRenameInformationExBypassAccessCheck = 66,
    /// `FileDesiredStorageClassInformation` - value 67.
    FileDesiredStorageClassInformation = 67,
    /// `FileStatInformation` - value 68.
    FileStatInformation = 68,
    /// `FileMemoryPartitionInformation` - value 69.
    FileMemoryPartitionInformation = 69,
    /// `FileStatLxInformation` - value 70.
    FileStatLxInformation = 70,
    /// `FileCaseSensitiveInformation` - value 71.
    FileCaseSensitiveInformation = 71,
    /// `FileLinkInformationEx` - value 72.
    FileLinkInformationEx = 72,
    /// `FileLinkInformationExBypassAccessCheck` - value 73.
    FileLinkInformationExBypassAccessCheck = 73,
    /// `FileStorageReserveIdInformation` - value 74.
    FileStorageReserveIdInformation = 74,
    /// `FileCaseSensitiveInformationForceAccessCheck` - value 75.
    FileCaseSensitiveInformationForceAccessCheck = 75,
    /// `FileKnownFolderInformation` - value 76.
    FileKnownFolderInformation = 76,
    /// `FileStatBasicInformation` - value 77.
    FileStatBasicInformation = 77,
    /// `FileId64ExtdDirectoryInformation` - value 78.
    FileId64ExtdDirectoryInformation = 78,
    /// `FileId64ExtdBothDirectoryInformation` - value 79.
    FileId64ExtdBothDirectoryInformation = 79,
    /// `FileIdAllExtdDirectoryInformation` - value 80.
    FileIdAllExtdDirectoryInformation = 80,
    /// `FileIdAllExtdBothDirectoryInformation` - value 81.
    FileIdAllExtdBothDirectoryInformation = 81,
    /// `FileStreamReservationInformation` - value 82.
    FileStreamReservationInformation = 82,
    /// `FileMupProviderInfo` - value 83.
    FileMupProviderInfo = 83,
    /// One past the last valid class. Not a queryable class.
    FileMaximumInformation = 84,
}

/// Type of file to create with IoCreateFileSpecifyDeviceObjectHint.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CREATE_FILE_TYPE {
    /// Regular file (most common).
    CreateFileTypeNone = 0,
    /// Named pipe.
    CreateFileTypeNamedPipe = 1,
    /// Mailslot.
    CreateFileTypeMailslot = 2,
}

/// FILE_NAME_INFORMATION structure for ZwQueryInformationFile.
///
/// Returns the full file name when querying FileAlternateNameInformation.
#[repr(C)]
#[allow(non_snake_case)]
pub struct FILE_NAME_INFORMATION {
    /// Length of the file name in bytes (not including null terminator).
    pub FileNameLength: ULONG,
    /// File name as a WCHAR array (not null-terminated).
    /// This field is variable-length; the actual length is specified by FileNameLength.
    pub FileName: [WCHAR; 1],
}

// File information structures from ntifs.h
//
// These structures are passed in IRP_MJ_SET_INFORMATION operations to rename,
// hard-link, or change file attributes. They're not generated by bindgen
// because they're defined in ntifs.h (user-mode headers) but used in
// kernel-mode minifilters.

/// Information for renaming a file.
///
/// This structure is passed in the `InfoBuffer` parameter of an
/// `IRP_MJ_SET_INFORMATION` operation when the `FileInformationClass` is
/// `FILE_RENAME_INFORMATION` or related classes.
///
/// # Layout
///
/// Matches the Windows DDK `_FILE_RENAME_INFORMATION` definition from
/// `ntifs.h`.
///
/// # See also
///
/// - [FILE_RENAME_INFORMATION (MSDN)](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information)
#[repr(C)]
#[allow(non_snake_case)]
pub struct FILE_RENAME_INFO {
    /// If `TRUE`, replace the target file if it exists.
    pub ReplaceIfExists: BOOLEAN,
    /// Optional handle to the root directory for relative paths. NULL for
    /// absolute paths.
    pub RootDirectory: HANDLE,
    /// Length of the `FileName` field in bytes (not including null terminator).
    pub FileNameLength: ULONG,
    /// The new file name as a WCHAR array (not null-terminated).
    /// This field is variable-length; the actual length is specified by
    /// `FileNameLength`.
    pub FileName: [WCHAR; 1],
}

/// Information for creating a hard link to a file.
///
/// This structure is passed in the `InfoBuffer` parameter of an
/// `IRP_MJ_SET_INFORMATION` operation when the `FileInformationClass` is
/// `FILE_LINK_INFORMATION` or related classes.
///
/// # Layout
///
/// Matches the Windows DDK `_FILE_LINK_INFORMATION` definition from `ntifs.h`.
///
/// # See also
///
/// - [FILE_LINK_INFORMATION (MSDN)](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_link_information)
#[repr(C)]
#[allow(non_snake_case)]
pub struct FILE_LINK_INFO {
    /// If `TRUE`, replace the target file if it exists.
    pub ReplaceIfExists: BOOLEAN,
    /// Optional handle to the root directory for relative paths. NULL for
    /// absolute paths.
    pub RootDirectory: HANDLE,
    /// Length of the `FileName` field in bytes (not including null terminator).
    pub FileNameLength: ULONG,
    /// The link file name as a WCHAR array (not null-terminated).
    /// This field is variable-length; the actual length is specified by
    /// `FileNameLength`.
    pub FileName: [WCHAR; 1],
}

/// PE Data Directory entry.
///
/// Describes the location and size of a data directory (exports, imports, etc.)
/// in the PE image.
///
/// # Layout
///
/// Matches the Windows SDK `_IMAGE_DATA_DIRECTORY` definition from `winnt.h`.
///
/// # See also
///
/// - [IMAGE_DATA_DIRECTORY (MSDN)](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-image_data_directory)
#[repr(C)]
#[allow(non_snake_case)]
pub struct IMAGE_DATA_DIRECTORY {
    pub VirtualAddress: DWORD,
    pub Size: DWORD,
}

/// PE File Header.
///
/// Contains machine type, section count, timestamp, and other file-level
/// information for a PE image.
///
/// # Layout
///
/// Matches the Windows SDK `_IMAGE_FILE_HEADER` definition from `winnt.h`.
///
/// # See also
///
/// - [IMAGE_FILE_HEADER (MSDN)](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-image_file_header)
#[repr(C)]
#[allow(non_snake_case)]
pub struct IMAGE_FILE_HEADER {
    pub Machine: WORD,
    pub NumberOfSections: WORD,
    pub TimeDateStamp: DWORD,
    pub PointerToSymbolTable: DWORD,
    pub NumberOfSymbols: DWORD,
    pub SizeOfOptionalHeader: WORD,
    pub Characteristics: WORD,
}

/// PE Optional Header (64-bit).
///
/// Part of the PE NT headers structure, contains Windows-specific fields
/// and the subsystem indicator.
///
/// # Layout
///
/// Matches the Windows SDK `_IMAGE_OPTIONAL_HEADER64` definition from `winnt.h`.
///
/// # See also
///
/// - [IMAGE_OPTIONAL_HEADER64 (MSDN)](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-image_optional_header64)
#[repr(C)]
#[allow(non_snake_case)]
pub struct IMAGE_OPTIONAL_HEADER64 {
    pub Magic: WORD,
    pub MajorLinkerVersion: BYTE,
    pub MinorLinkerVersion: BYTE,
    pub SizeOfCode: DWORD,
    pub SizeOfInitializedData: DWORD,
    pub SizeOfUninitializedData: DWORD,
    pub AddressOfEntryPoint: DWORD,
    pub BaseOfCode: DWORD,
    pub ImageBase: ULONGLONG,
    pub SectionAlignment: DWORD,
    pub FileAlignment: DWORD,
    pub MajorOperatingSystemVersion: WORD,
    pub MinorOperatingSystemVersion: WORD,
    pub MajorImageVersion: WORD,
    pub MinorImageVersion: WORD,
    pub MajorSubsystemVersion: WORD,
    pub MinorSubsystemVersion: WORD,
    pub Win32VersionValue: DWORD,
    pub SizeOfImage: DWORD,
    pub SizeOfHeaders: DWORD,
    pub CheckSum: DWORD,
    /// Subsystem required to run this image (e.g., IMAGE_SUBSYSTEM_NATIVE for
    /// drivers).
    pub Subsystem: WORD,
    pub DllCharacteristics: WORD,
    pub SizeOfStackReserve: ULONGLONG,
    pub SizeOfStackCommit: ULONGLONG,
    pub SizeOfHeapReserve: ULONGLONG,
    pub SizeOfHeapCommit: ULONGLONG,
    pub LoaderFlags: DWORD,
    pub NumberOfRvaAndSizes: DWORD,
    pub DataDirectory: [IMAGE_DATA_DIRECTORY; 16],
}

/// PE NT Headers (64-bit).
///
/// Contains the PE signature, file header, and optional header for a 64-bit
/// executable image.
///
/// # Layout
///
/// Matches the Windows SDK `_IMAGE_NT_HEADERS64` definition from `winnt.h`.
///
/// # See also
///
/// - [IMAGE_NT_HEADERS64 (MSDN)](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-image_nt_headers64)
#[repr(C)]
#[allow(non_snake_case)]
pub struct IMAGE_NT_HEADERS64 {
    pub Signature: DWORD,
    pub FileHeader: IMAGE_FILE_HEADER,
    pub OptionalHeader: IMAGE_OPTIONAL_HEADER64,
}

/// Pointer to IMAGE_NT_HEADERS64.
pub type PIMAGE_NT_HEADERS64 = *mut IMAGE_NT_HEADERS64;
