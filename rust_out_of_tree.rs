//! A misc device that represents an in-memory file.

use core::pin::Pin;

use kernel::{
    bindings, c_str, device,
    fs::{File, Kiocb},
    iov::{IovIterDest, IovIterSource},
    miscdevice::{MiscDevice, MiscDeviceOptions, MiscDeviceRegistration},
    new_mutex,
    prelude::*,
    sync::{aref::ARef, Arc, Mutex},
};

// Helper functions

/// Set data for a device.
///
/// SAFETY: Must only be called once (i.e. there was no previous data).
unsafe fn dev_set_drv_data<T>(dev: &device::Device, data: &Arc<T>) {
    unsafe {
        let ptr = core::ptr::from_ref(dev).cast::<bindings::device>() as *mut _;
        bindings::dev_set_drvdata(ptr, core::ptr::from_ref(data) as *mut _);
    }
}

/// Get data of a device.
///
/// SAFETY: Must have the same generic type as [`dev_set_drv_data`].
unsafe fn dev_get_drv_data<T>(dev: &device::Device) -> Arc<T> {
    unsafe {
        let ptr = core::ptr::from_ref(dev).cast::<bindings::device>() as *mut _;
        let data = bindings::dev_get_drvdata(ptr) as *mut Arc<T>;
        Arc::clone(&*data)
    }
}

// Module code

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
    miscdev: MiscDeviceRegistration<RustMiscDevice>,
    #[pin]
    global: Arc<GlobalData>,
}

/// The data structure we will store "globally" for the device
/// to keep the buffer in memory accross open and close.
#[pin_data]
struct GlobalData {
    #[pin]
    data: Mutex<KVVec<u8>>,
}

impl kernel::InPlaceModule for RustMiscDeviceModule {
    fn init(_module: &'static ThisModule) -> impl PinInit<Self, Error> {
        pr_info!("Initialising Rust Misc Device Sample\n");

        // Create options
        let options = MiscDeviceOptions {
            name: c_str!("rust-misc-device"),
            mode: 0o766,
        };

        let global = Arc::try_pin_init(
            try_pin_init! {
                GlobalData {
                    data <- new_mutex!(KVVec::new())
                }
            },
            GFP_KERNEL,
        )
        .unwrap();

        let init = try_pin_init!(Self {
            miscdev <- MiscDeviceRegistration::register(options),
            global,
        });

        // Run post-initialization
        init.pin_chain(|v| v.post_init())
    }
}

impl RustMiscDeviceModule {
    /// Initializes driver data.
    fn post_init(self: Pin<&mut Self>) -> Result<()> {
        let dev = self.miscdev.device();
        unsafe {
            // Safe because called only once.
            dev_set_drv_data(dev, &self.global);
        }
        Ok(())
    }
}

// Misc-device implementation

#[pin_data(PinnedDrop)]
struct RustMiscDevice {
    #[pin]
    global: Arc<GlobalData>,
    dev: ARef<device::Device>,
}

#[vtable]
impl MiscDevice for RustMiscDevice {
    type Ptr = Pin<KBox<Self>>;

    fn open(_file: &File, misc: &MiscDeviceRegistration<Self>) -> Result<Pin<KBox<Self>>> {
        let dev = ARef::from(misc.device());

        dev_info!(dev, "Opening Rust Misc Device Sample\n");

        // Load the global driver data here
        let global: Arc<GlobalData> = unsafe { dev_get_drv_data(misc.device()) };
        KBox::try_pin_init(
            try_pin_init! {
                RustMiscDevice {
                    global,
                    dev,
                }
            },
            GFP_KERNEL,
        )
    }

    // Read the global buffer.
    fn read_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterDest<'_>) -> Result<usize> {
        let me = kiocb.file();
        dev_info!(me.dev, "Reading from Rust Misc Device Sample\n");

        let inner = me.global.data.lock();
        // Read the buffer contents, taking the file position into account.
        let read = iov.simple_read_from_buffer(kiocb.ki_pos_mut(), &inner)?;

        Ok(read)
    }

    // Allow writing to the global buffer.
    fn write_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterSource<'_>) -> Result<usize> {
        let me = kiocb.file();
        dev_info!(me.dev, "Writing to Rust Misc Device Sample\n");

        let mut inner = me.global.data.lock();

        // Replace buffer contents.
        inner.clear();
        let len = iov.copy_from_iter_vec(&mut inner, GFP_KERNEL)?;

        // Set position to zero so that future `read` calls will see the new contents.
        *kiocb.ki_pos_mut() = 0;

        Ok(len)
    }
}

// Print some information when device is dropped.
#[pinned_drop]
impl PinnedDrop for RustMiscDevice {
    fn drop(self: Pin<&mut Self>) {
        dev_info!(self.dev, "Exiting the Rust Misc Device Sample\n");
    }
}
