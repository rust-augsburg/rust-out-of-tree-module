// SPDX-License-Identifier: GPL-2.0
// SPDX-FileCopyrightText: Copyright (C) 2025 Collabora Ltd.

//! Rust USB driver sample.

use kernel::{
    device::{self, Core},
    dma,
    prelude::*,
    sync::aref::ARef,
    uapi, usb,
};

struct SampleDriver {
    _intf: ARef<usb::Interface>,
}

struct UsbMouse {
    name: [char; 128],
    phys: [char; 64],
    usbdev: usb::Device, // *mut
    dev: InputDevice,    // *mut
    irq: usb::Urb,       // *mut
    data: *mut i8,
    data_dma: dma::DmaAddress,
}

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;
const BTN_SIDE: u32 = 0x113;
const BTN_EXTRA: u32 = 0x114;
const REL_X: u32 = 0x00;
const REL_Y: u32 = 0x01;
const REL_HWHEEL: u32 = 0x06;

fn try_usb_mouse_irq(urb: *mut Urb) -> Result<()> {
    let mouse: *mut UsbMouse = urb.context;
    let data: *mut i8 = mouse.data;
    let dev: &InputDevice = &mouse.dev;
    let mut status = 0;

    match -urb.status {
        0 => {}
        uapi::ECONNRESET | uapi::ENOENT | uapi::ESHUTDOWN => {
            return Err(());
        }
        _ => {
            unreachable!();
            // goto resubmit;
        }
    }

    dev.input_report_key(BTN_LEFT, data[0] & 0x01);
    dev.input_report_key(BTN_RIGHT, data[0] & 0x02);
    // input_report_key(dev, BTN_RIGHT,  data[0] & 0x01);
    // input_report_key(dev, BTN_LEFT,   data[0] & 0x02);

    dev.input_report_key(BTN_MIDDLE, data[0] & 0x04);
    dev.input_report_key(BTN_SIDE, data[0] & 0x08);
    dev.input_report_key(BTN_EXTRA, data[0] & 0x10);

    dev.input_report_rel(REL_X, data[1]);
    dev.input_report_rel(REL_Y, data[2]);
    dev.input_report_rel(REL_WHEEL, data[3]);

    dev.sync();
}

fn usb_mouse_irq(urb: *mut Urb) -> Result<()> {
    /*
    if try_usb_mouse_irq(urb).is_err() {
        status = usb_submit_urb (urb, GFP_ATOMIC);
        if (status) {
            dev_err(&mouse->usbdev->dev,
                "can't resubmit intr, %s-%s/input0, status %d\n",
                mouse->usbdev->bus->bus_name,
                mouse->usbdev->devpath, status);
        }
    }
     */
}

fn usb_mouse_open(dev: *mut InputDevice) -> c_int {
    let mouse: *mut UsbMouse = input_get_drvdata(dev);

    mouse.irq.dev = mouse.usbdev;
    if usb_submit_urb(mouse.irq, GFP_KERNEL) {
        return -EIO;
    }

    return 0;
}

/*
fn usb_mouse_close(dev: *mut InputDev) {
    struct usb_mouse *mouse = input_get_drvdata(dev);

    usb_kill_urb(mouse->irq);
}

fn usb_mouse_probe(intf: *mut UsbInterface, id: *const UsbDeviceId) -> c_int {
    struct usb_device *dev = interface_to_usbdev(intf);
    struct usb_host_interface *interface;
    struct usb_endpoint_descriptor *endpoint;
    struct usb_mouse *mouse;
    struct input_dev *input_dev;
    int pipe, maxp;
    int error = -ENOMEM;

    interface = intf->cur_altsetting;

    if (interface->desc.bNumEndpoints != 1)
        return -ENODEV;

    endpoint = &interface->endpoint[0].desc;
    if (!usb_endpoint_is_int_in(endpoint))
        return -ENODEV;

    pipe = usb_rcvintpipe(dev, endpoint->bEndpointAddress);
    maxp = usb_maxpacket(dev, pipe);

    mouse = kzalloc(sizeof(struct usb_mouse), GFP_KERNEL);
    input_dev = input_allocate_device();
    if (!mouse || !input_dev)
        goto fail1;

    mouse->data = usb_alloc_coherent(dev, 8, GFP_KERNEL, &mouse->data_dma);
    if (!mouse->data)
        goto fail1;

    mouse->irq = usb_alloc_urb(0, GFP_KERNEL);
    if (!mouse->irq)
        goto fail2;

    mouse->usbdev = dev;
    mouse->dev = input_dev;

    if (dev->manufacturer)
        strscpy(mouse->name, dev->manufacturer, sizeof(mouse->name));

    if (dev->product) {
        if (dev->manufacturer)
            strlcat(mouse->name, " ", sizeof(mouse->name));
        strlcat(mouse->name, dev->product, sizeof(mouse->name));
    }

    if (!strlen(mouse->name))
        snprintf(mouse->name, sizeof(mouse->name),
             "USB HIDBP Mouse %04x:%04x",
             le16_to_cpu(dev->descriptor.idVendor),
             le16_to_cpu(dev->descriptor.idProduct));

    usb_make_path(dev, mouse->phys, sizeof(mouse->phys));
    strlcat(mouse->phys, "/input0", sizeof(mouse->phys));

    input_dev->name = mouse->name;
    input_dev->phys = mouse->phys;
    usb_to_input_id(dev, &input_dev->id);
    input_dev->dev.parent = &intf->dev;

    input_dev->evbit[0] = BIT_MASK(EV_KEY) | BIT_MASK(EV_REL);
    input_dev->keybit[BIT_WORD(BTN_MOUSE)] = BIT_MASK(BTN_LEFT) |
        BIT_MASK(BTN_RIGHT) | BIT_MASK(BTN_MIDDLE);
    input_dev->relbit[0] = BIT_MASK(REL_X) | BIT_MASK(REL_Y);
    input_dev->keybit[BIT_WORD(BTN_MOUSE)] |= BIT_MASK(BTN_SIDE) |
        BIT_MASK(BTN_EXTRA);
    input_dev->relbit[0] |= BIT_MASK(REL_WHEEL);

    input_set_drvdata(input_dev, mouse);

    input_dev->open = usb_mouse_open;
    input_dev->close = usb_mouse_close;

    usb_fill_int_urb(mouse->irq, dev, pipe, mouse->data,
             (maxp > 8 ? 8 : maxp),
             usb_mouse_irq, mouse, endpoint->bInterval);
    mouse->irq->transfer_dma = mouse->data_dma;
    mouse->irq->transfer_flags |= URB_NO_TRANSFER_DMA_MAP;

    error = input_register_device(mouse->dev);
    if (error)
        goto fail3;

    usb_set_intfdata(intf, mouse);
    return 0;

fail3:
    usb_free_urb(mouse->irq);
fail2:
    usb_free_coherent(dev, 8, mouse->data, mouse->data_dma);
fail1:
    input_free_device(input_dev);
    kfree(mouse);
    return error;
}

fn usb_mouse_disconnect(intf: *mut UsbInterface) {
    struct usb_mouse *mouse = usb_get_intfdata (intf);

    usb_set_intfdata(intf, NULL);
    if (mouse) {
        usb_kill_urb(mouse->irq);
        input_unregister_device(mouse->dev);
        usb_free_urb(mouse->irq);
        usb_free_coherent(interface_to_usbdev(intf), 8, mouse->data, mouse->data_dma);
        kfree(mouse);
    }
} */

impl usb::Driver for SampleDriver {
    type IdInfo = ();
    const ID_TABLE: usb::IdTable<Self::IdInfo> = &USB_TABLE;

    fn probe(
        intf: &usb::Interface<Core>,
        _id: &usb::DeviceId,
        _info: &Self::IdInfo,
    ) -> Result<Pin<KBox<Self>>> {
        let dev: &device::Device<Core> = intf.as_ref();
        dev_info!(dev, "Rust USB driver sample probed\n");

        let drvdata = KBox::new(Self { _intf: intf.into() }, GFP_KERNEL)?;
        Ok(drvdata.into())
    }

    fn disconnect(intf: &usb::Interface<Core>, _data: Pin<&Self>) {
        let dev: &device::Device<Core> = intf.as_ref();
        dev_info!(dev, "Rust USB driver sample disconnected\n");
    }
}

kernel::usb_device_table!(
    USB_TABLE,
    MODULE_USB_TABLE,
    <SampleDriver as usb::Driver>::IdInfo,
    [(usb::DeviceId::from_id(0x1234, 0x5678), ()),]
);

kernel::module_usb_driver! {
    type: SampleDriver,
    name: "rust_mouse",
    authors: ["Daniel Almeida", "Aaron Erhardt"],
    description: "Rust USB mouse driver sample",
    license: "GPL v2",
}
