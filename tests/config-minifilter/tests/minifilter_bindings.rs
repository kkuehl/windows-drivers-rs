// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Tests that the `minifilter` API subset of `wdk-sys` generates the bindings
//! that a File System Minifilter driver needs to register itself with the
//! Filter Manager.
//!
//! The Filter Manager routines cannot be called outside of kernel-mode, so
//! these tests instead exercise the *contract* the generated bindings must
//! uphold. Struct layouts are already validated against `fltKernel.h` by the
//! layout assertions bindgen emits into `types.rs`, so they are deliberately
//! not re-asserted with hardcoded offsets here. What these tests pin down is
//! the set of properties bindgen configuration can silently break: that the
//! Filter Manager items are generated at all, that they are reachable at the
//! paths a consumer imports them from, and that the types line up such that a
//! registration can be built without any casts.

#[cfg(test)]
mod tests {
    use wdk_sys::{
        _FLT_POSTOP_CALLBACK_STATUS,
        _FLT_PREOP_CALLBACK_STATUS,
        FLT_FILE_NAME_INFORMATION,
        FLT_FILE_NAME_NORMALIZED,
        FLT_FILE_NAME_OPTIONS,
        FLT_FILE_NAME_QUERY_DEFAULT,
        FLT_FILTER_UNLOAD_FLAGS,
        FLT_OPERATION_REGISTRATION,
        FLT_POST_OPERATION_FLAGS,
        FLT_POSTOP_CALLBACK_STATUS,
        FLT_PREOP_CALLBACK_STATUS,
        FLT_REGISTRATION,
        FLT_REGISTRATION_FLAGS,
        FLT_REGISTRATION_VERSION,
        FLT_RELATED_OBJECTS,
        FLTFL_REGISTRATION_DO_NOT_SUPPORT_SERVICE_STOP,
        IRP_MJ_CREATE,
        IRP_MJ_MAXIMUM_FUNCTION,
        NTSTATUS,
        PCFLT_RELATED_OBJECTS,
        PFLT_CALLBACK_DATA,
        PFLT_FILTER_UNLOAD_CALLBACK,
        PFLT_POST_OPERATION_CALLBACK,
        PFLT_PRE_OPERATION_CALLBACK,
        PVOID,
        STATUS_SUCCESS,
        UCHAR,
        USHORT,
        minifilter::IRP_MJ_OPERATION_END,
    };

    /// `IRP_MJ_OPERATION_END` is defined in `fltKernel.h` as a macro with a
    /// cast in its expansion (`((UCHAR)0x80)`), which bindgen cannot
    /// translate, so `wdk-sys` ports it by hand. It must be typed as a
    /// [`UCHAR`] so that it can initialize
    /// `FLT_OPERATION_REGISTRATION::MajorFunction` directly, and
    /// it must not collide with a real `IRP_MJ_*` code or the Filter Manager
    /// would stop walking the operation table early.
    #[test]
    const fn irp_mj_operation_end_terminates_the_operation_table() {
        // `MajorFunction` is a `UCHAR`, so assigning without a cast only compiles if
        // the hand-ported constant has the same type. This is the regression
        // that a plain `u32` port would introduce.
        let terminator = FLT_OPERATION_REGISTRATION {
            MajorFunction: IRP_MJ_OPERATION_END,
            Flags: 0,
            PreOperation: None,
            PostOperation: None,
            Reserved1: core::ptr::null_mut(),
        };

        assert!(
            terminator.MajorFunction as u32 > IRP_MJ_MAXIMUM_FUNCTION,
            "the sentinel must not alias a real IRP_MJ_* code"
        );
    }

    /// The `IRP_MJ_*` codes are generated into the shared `constants.rs` as
    /// `u32`, but `FLT_OPERATION_REGISTRATION::MajorFunction` is a [`UCHAR`]. A
    /// minifilter therefore has to narrow them, which is only sound while every
    /// code the WDK defines is in range.
    #[test]
    fn every_irp_mj_code_fits_in_a_major_function() {
        for irp_mj_code in 0..=IRP_MJ_MAXIMUM_FUNCTION {
            assert!(
                UCHAR::try_from(irp_mj_code).is_ok(),
                "IRP_MJ code {irp_mj_code} does not fit in \
                 FLT_OPERATION_REGISTRATION::MajorFunction"
            );
        }
    }

    /// The Filter Manager rejects a registration whose `Size` does not match
    /// the `FLT_REGISTRATION` the driver compiled against, and
    /// `Size`/`Version` are [`USHORT`]s while `size_of` and the generated
    /// `FLT_REGISTRATION_VERSION` are wider. If either value stopped
    /// fitting, every minifilter built on these bindings would fail to
    /// load, so this catches it at test time instead.
    #[test]
    fn registration_size_and_version_fit_in_their_fields() {
        assert!(
            USHORT::try_from(size_of::<FLT_REGISTRATION>()).is_ok(),
            "size_of::<FLT_REGISTRATION>() does not fit in FLT_REGISTRATION::Size"
        );
        assert!(
            USHORT::try_from(FLT_REGISTRATION_VERSION).is_ok(),
            "FLT_REGISTRATION_VERSION does not fit in FLT_REGISTRATION::Version"
        );
    }

    /// Registration flags and file name options are bitmask constants, so they
    /// must be generated with the same integer type as the fields and arguments
    /// they feed, and must be combinable without a cast.
    #[test]
    const fn registration_flags_and_name_options_are_typed_for_their_fields() {
        let flags: FLT_REGISTRATION_FLAGS = FLTFL_REGISTRATION_DO_NOT_SUPPORT_SERVICE_STOP;
        let name_options: FLT_FILE_NAME_OPTIONS =
            FLT_FILE_NAME_NORMALIZED | FLT_FILE_NAME_QUERY_DEFAULT;

        assert!(flags != 0);
        assert!(name_options != FLT_FILE_NAME_NORMALIZED);
        assert!(name_options != FLT_FILE_NAME_QUERY_DEFAULT);
    }

    /// The callback status enumerations are returned by value from the
    /// pre/post-operation callbacks. Their exact discriminants come from
    /// `fltKernel.h` and are validated by bindgen, but they must be distinct
    /// and must be generated as the same type the callback typedefs return.
    #[test]
    const fn callback_statuses_are_distinct_and_correctly_typed() {
        let success_with_callback: FLT_PREOP_CALLBACK_STATUS =
            _FLT_PREOP_CALLBACK_STATUS::FLT_PREOP_SUCCESS_WITH_CALLBACK;
        let success_no_callback: FLT_PREOP_CALLBACK_STATUS =
            _FLT_PREOP_CALLBACK_STATUS::FLT_PREOP_SUCCESS_NO_CALLBACK;
        let pending: FLT_PREOP_CALLBACK_STATUS = _FLT_PREOP_CALLBACK_STATUS::FLT_PREOP_PENDING;
        let complete: FLT_PREOP_CALLBACK_STATUS = _FLT_PREOP_CALLBACK_STATUS::FLT_PREOP_COMPLETE;

        assert!(success_with_callback != success_no_callback);
        assert!(success_with_callback != pending);
        assert!(success_with_callback != complete);
        assert!(success_no_callback != pending);
        assert!(success_no_callback != complete);
        assert!(pending != complete);

        let finished: FLT_POSTOP_CALLBACK_STATUS =
            _FLT_POSTOP_CALLBACK_STATUS::FLT_POSTOP_FINISHED_PROCESSING;
        let more_processing: FLT_POSTOP_CALLBACK_STATUS =
            _FLT_POSTOP_CALLBACK_STATUS::FLT_POSTOP_MORE_PROCESSING_REQUIRED;

        assert!(finished != more_processing);
    }

    /// A minifilter's whole interaction with the Filter Manager goes through an
    /// `FLT_REGISTRATION` built at compile time, so it must be constructible
    /// from the generated items alone: `Default` for the callbacks a driver
    /// does not implement, a `'static` operation table, and no casts
    /// anywhere. This test fails to compile if bindgen stops deriving
    /// `Default`, drops a field, or changes a field's type.
    #[test]
    fn a_registration_is_constructible_from_generated_items_alone() {
        unsafe extern "C" fn pre_operation(
            _data: PFLT_CALLBACK_DATA,
            _flt_objects: PCFLT_RELATED_OBJECTS,
            _completion_context: *mut PVOID,
        ) -> FLT_PREOP_CALLBACK_STATUS {
            _FLT_PREOP_CALLBACK_STATUS::FLT_PREOP_SUCCESS_WITH_CALLBACK
        }

        unsafe extern "C" fn post_operation(
            _data: PFLT_CALLBACK_DATA,
            _flt_objects: PCFLT_RELATED_OBJECTS,
            _completion_context: PVOID,
            _flags: FLT_POST_OPERATION_FLAGS,
        ) -> FLT_POSTOP_CALLBACK_STATUS {
            _FLT_POSTOP_CALLBACK_STATUS::FLT_POSTOP_FINISHED_PROCESSING
        }

        unsafe extern "C" fn filter_unload(_flags: FLT_FILTER_UNLOAD_FLAGS) -> NTSTATUS {
            STATUS_SUCCESS
        }

        // The callback typedefs must carry their full argument lists, otherwise a
        // callback with the wrong signature would be accepted here and corrupt
        // the stack in kernel-mode.
        let pre_operation: PFLT_PRE_OPERATION_CALLBACK = Some(pre_operation);
        let post_operation: PFLT_POST_OPERATION_CALLBACK = Some(post_operation);
        let filter_unload: PFLT_FILTER_UNLOAD_CALLBACK = Some(filter_unload);

        // `FLT_OPERATION_REGISTRATION` holds a raw `Reserved1` pointer and so is not
        // `Sync`, which is why the sample driver wraps its table in a newtype
        // to put it in a `static`. Here a local suffices, since it outlives the
        // registration that borrows it.
        let operation_registration = [
            FLT_OPERATION_REGISTRATION {
                MajorFunction: UCHAR::try_from(IRP_MJ_CREATE)
                    .expect("IRP_MJ_CREATE should fit in MajorFunction"),
                Flags: 0,
                PreOperation: None,
                PostOperation: None,
                Reserved1: core::ptr::null_mut(),
            },
            FLT_OPERATION_REGISTRATION {
                MajorFunction: IRP_MJ_OPERATION_END,
                Flags: 0,
                PreOperation: None,
                PostOperation: None,
                Reserved1: core::ptr::null_mut(),
            },
        ];

        let registration = FLT_REGISTRATION {
            Size: USHORT::try_from(size_of::<FLT_REGISTRATION>())
                .expect("FLT_REGISTRATION should fit in FLT_REGISTRATION::Size"),
            Version: USHORT::try_from(FLT_REGISTRATION_VERSION)
                .expect("FLT_REGISTRATION_VERSION should fit in FLT_REGISTRATION::Version"),
            Flags: FLTFL_REGISTRATION_DO_NOT_SUPPORT_SERVICE_STOP,
            OperationRegistration: operation_registration.as_ptr(),
            FilterUnloadCallback: filter_unload,
            // Every callback a minifilter does not implement must be defaultable.
            ..FLT_REGISTRATION::default()
        };

        assert!(pre_operation.is_some());
        assert!(post_operation.is_some());
        assert!(registration.FilterUnloadCallback.is_some());
        assert!(!registration.OperationRegistration.is_null());
        assert!(
            registration.InstanceSetupCallback.is_none(),
            "FLT_REGISTRATION::default() should leave unimplemented callbacks null"
        );
    }

    /// The structures the Filter Manager passes into callbacks are only useful
    /// if their fields were generated too, since a minifilter reaches the
    /// filter, instance, and file objects for an operation through
    /// `FLT_RELATED_OBJECTS`, and the file name through
    /// `FLT_FILE_NAME_INFORMATION::Name`.
    #[test]
    fn callback_parameter_structs_expose_the_fields_a_minifilter_reads() {
        let related_objects = FLT_RELATED_OBJECTS::default();
        assert!(related_objects.Filter.is_null());
        assert!(related_objects.Instance.is_null());
        assert!(related_objects.Volume.is_null());
        assert!(related_objects.FileObject.is_null());

        let file_name_information = FLT_FILE_NAME_INFORMATION::default();
        assert!(file_name_information.Name.Buffer.is_null());
        assert_eq!(file_name_information.Name.Length, 0);
    }
}
