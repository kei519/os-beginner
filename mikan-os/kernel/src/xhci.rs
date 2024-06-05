use crate::{
    bitfield::BitField as _,
    interrupt::InterruptVector,
    log,
    logger::LogLevel,
    pci::{self, Device},
    sync::OnceMutex,
    usb::Controller,
};

pub static XHC: OnceMutex<Controller> = OnceMutex::new();

pub fn init() {
    let mut xhc_dev = None;
    {
        let devices = pci::DEVICES.read();

        // Intel 製を優先して xHC を探す
        let mut intel_found = false;

        for device in &*devices {
            if device.class_code().r#match(0x0c, 0x03, 0x30) {
                if intel_found {
                    continue;
                }

                if 0x8086 == device.read_vendor_id() {
                    intel_found = true;
                }
                xhc_dev = Some(*device);
            }
        }

        if xhc_dev.is_some() {
            let xhc_dev = xhc_dev.unwrap();
            log!(
                LogLevel::Info,
                "xHC has been found: {}.{}.{}",
                xhc_dev.bus(),
                xhc_dev.device(),
                xhc_dev.function()
            );
        }
    }
    let mut xhc_dev = xhc_dev.unwrap();

    let bsp_local_apic_id = (unsafe { *(0xfee0_0020 as *const u32) } >> 24) as u8;
    xhc_dev
        .configure_msi_fixed_destination(
            bsp_local_apic_id,
            pci::MSITriggerMode::Level,
            pci::MSIDeliverMode::Fixed,
            InterruptVector::XHCI as u8,
            0,
        )
        .unwrap();
    let xhc_dev = xhc_dev;

    // xHC の BAR から情報を得る
    let xhc_bar = xhc_dev.read_bar(0);
    log!(LogLevel::Debug, "ReadBar: {:#x?}", xhc_bar);
    let xhc_mmio_base = xhc_bar.unwrap().get_bits(4..) << 4;
    log!(LogLevel::Debug, "xHC mmio_base = {:08x}", xhc_mmio_base);

    let mut xhc = Controller::new(xhc_mmio_base);

    if xhc_dev.read_vendor_id() == 0x8086 {
        switch_ehci2xhci(&xhc_dev);
    }

    let result = xhc.initialize();
    log!(LogLevel::Debug, "xhc.initialize: {:?}", result);

    log!(LogLevel::Info, "xHC starting");
    xhc.run().unwrap();

    for i in 1..=xhc.max_ports() {
        let mut port = xhc.port_at(i);
        log!(
            LogLevel::Debug,
            "Port {}: IsConnected={}",
            i,
            port.is_connected()
        );

        if port.is_connected() {
            if let Err(err) = xhc.configure_port(&mut port) {
                log!(LogLevel::Error, "failed to configure port: {}", err);
                continue;
            }
        }
    }

    XHC.init(xhc);
}

fn switch_ehci2xhci(xhc_dev: &Device) {
    let mut intel_ehc_exist = false;
    let devices = pci::DEVICES.read();
    for device in &*devices {
        if device.class_code().r#match(0x0c, 0x03, 0x20) && device.read_vendor_id() == 0x8086 {
            intel_ehc_exist = true;
            break;
        }
    }
    if !intel_ehc_exist {
        return;
    }

    let superspeed_ports = xhc_dev.read_conf_reg(0xdc);
    xhc_dev.write_conf_reg(0xd8, superspeed_ports);
    let ehci2xhci_ports = xhc_dev.read_conf_reg(0xd4);
    xhc_dev.write_conf_reg(0xd0, ehci2xhci_ports);
    log!(
        LogLevel::Debug,
        "switch_ehci2xhci: SS = {:02x}, xHCI = {:02x}",
        superspeed_ports,
        ehci2xhci_ports
    );
}
