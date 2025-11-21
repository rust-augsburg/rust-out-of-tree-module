// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2024 Google LLC.

//! Rust misc device sample.

use core::pin::Pin;

use kernel::{
    c_str,
    device::Device,
    fs::{File, Kiocb},
    iov::{IovIterDest, IovIterSource},
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    new_mutex,
    prelude::*,
    sync::{aref::ARef, Mutex},
};

module! {
    type: RustMiscDeviceModule,
    name: "rust_misc_device",
    authors: ["Lee Jones", "Aaron Erhardt"],
    description: "Rust misc device sample",
    license: "GPL",
}

#[pin_data]
struct RustMiscDeviceModule {
    #[pin]
    _miscdev: MiscDeviceRegistration<RustMiscDevice>,
}

impl kernel::InPlaceModule for RustMiscDeviceModule {
    fn init(_module: &'static ThisModule) -> impl PinInit<Self, Error> {
        pr_info!("Initialising Rust Misc Device Sample\n");

        let options = MiscDeviceOptions {
            name: c_str!("rust-misc-device"),
            mode: 0o766,
        };

        try_pin_init!(Self {
            _miscdev <- MiscDeviceRegistration::register(options),
        })
    }
}

#[pin_data(PinnedDrop)]
struct RustMiscDevice {
    // TODO: Protected buffer, similar to Vec<u8> in userspace
    // #[pin]
    // inner: BUFFER_TYPE
    dev: ARef<Device>,
}

#[vtable]
impl MiscDevice for RustMiscDevice {
    type Ptr = Pin<KBox<Self>>;

    fn open(_file: &File, misc: &MiscDeviceRegistration<Self>) -> Result<Pin<KBox<Self>>> {
        let dev = ARef::from(misc.device());

        dev_info!(dev, "Opening Rust Misc Device Sample\n");

        KBox::try_pin_init(
            try_pin_init! {
                RustMiscDevice {
                    inner <- todo!("BUFFER")
                    dev: dev,
                }
            },
            GFP_KERNEL,
        )
    }

    fn read_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterDest<'_>) -> Result<usize> {
        let me = kiocb.file();
        dev_info!(me.dev, "Reading from Rust Misc Device Sample\n");

        // TODO: Lock

        // TODO: Read the buffer contents, taking the file position into account.

        Ok(read)
    }

    fn write_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterSource<'_>) -> Result<usize> {
        let me = kiocb.file();
        dev_info!(me.dev, "Writing to Rust Misc Device Sample\n");

        let mut inner = me.inner.lock();

        // TODO: Replace buffer contents.

        // TODO: Set position to zero so that future `read` calls will see the new contents.

        Ok(len)
    }
}

#[pinned_drop]
impl PinnedDrop for RustMiscDevice {
    fn drop(self: Pin<&mut Self>) {
        dev_info!(self.dev, "Exiting the Rust Misc Device Sample\n");
    }
}

// HINT: Useful methods:
// - iov.simple_read_from_buffer
// - iov.copy_from_iter_vec
