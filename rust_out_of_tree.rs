// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2024 Google LLC.

//! Rust misc device sample.

use core::pin::Pin;

use kernel::{
    c_str,
    device::Device,
    fs::{File, Kiocb},
    iov::IovIterDest,
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    prelude::*,
    str::CString,
    sync::aref::ARef,
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
                    dev: dev,
                }
            },
            GFP_KERNEL,
        )
    }

    fn read_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterDest<'_>) -> Result<usize> {
        let me = kiocb.file();
        dev_info!(me.dev, "Reading from Rust Misc Device Sample\n");

        let num = 42;
        let info = CString::try_from_fmt(fmt!("Hello world! My num is {num}."))?;
        let read = iov.simple_read_from_buffer(kiocb.ki_pos_mut(), info.to_bytes())?;

        Ok(read)
    }
}

#[pinned_drop]
impl PinnedDrop for RustMiscDevice {
    fn drop(self: Pin<&mut Self>) {
        dev_info!(self.dev, "Exiting the Rust Misc Device Sample\n");
    }
}
