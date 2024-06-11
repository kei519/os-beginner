use core::mem;

use crate::{
    asmfunc,
    bitfield::BitField as _,
    message::MessageType,
    sync::Mutex,
    task, timer,
    x86_descriptor::{self, DescriptorType, SystemSegmentType},
};

static IDT: Mutex<[InterruptDescriptor; 256]> =
    Mutex::new([InterruptDescriptor::const_default(); 256]);

pub fn init() {
    let cs = asmfunc::get_cs();
    {
        let mut idt = IDT.lock_wait();
        idt[InterruptVector::XHCI as usize].set_idt_entry(
            InterruptDescriptorAttribute::new(
                x86_descriptor::SystemSegmentType::InterruptGate,
                0,
                true,
            ),
            int_handler_xhci,
            cs,
        );
        idt[InterruptVector::LAPICTimer as usize].set_idt_entry(
            InterruptDescriptorAttribute::new(
                x86_descriptor::SystemSegmentType::InterruptGate,
                0,
                true,
            ),
            int_handler_lapic_timer,
            cs,
        );
        asmfunc::load_idt(
            (mem::size_of::<InterruptDescriptor>() * idt.len()) as u16 - 1,
            idt.as_ptr() as u64,
        )
    }
}

#[custom_attribute::interrupt]
fn int_handler_xhci(_frame: &InterruptFrame) {
    // メインタスクが 1 で登録されるので必ず存在するはず
    task::send_message(1, MessageType::InterruptXHCI.into()).unwrap();
    notify_end_of_interrupt();
}

#[custom_attribute::interrupt]
fn int_handler_lapic_timer(_frame: &InterruptFrame) {
    timer::lapic_timer_on_interrupt();
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct InterruptDescriptorAttribute {
    etc_1: u8,
    etc_2: u8,
}

impl InterruptDescriptorAttribute {
    pub const fn const_default() -> Self {
        Self { etc_1: 0, etc_2: 0 }
    }

    pub fn new(r#type: SystemSegmentType, descriptor_privilege_level: u8, present: bool) -> Self {
        let mut etc_2 = 0;
        etc_2.set_bits(..4, DescriptorType::system_segment(r#type).into());
        etc_2.set_bits(5..7, descriptor_privilege_level);
        etc_2.set_bit(7, present);
        Self { etc_1: 0, etc_2 }
    }

    pub fn interrupt_stack_table(&self) -> u8 {
        self.etc_1 & 0x07
    }

    pub fn r#type(&self) -> DescriptorType {
        DescriptorType::from(self.etc_2 & 0x0f)
    }

    pub fn descriptor_privilege_level(&self) -> u8 {
        (self.etc_2 >> 5) & 0x03
    }

    pub fn present(&self) -> bool {
        (self.etc_2 >> 7) & 0x01 == 1
    }
}

impl Default for InterruptDescriptorAttribute {
    fn default() -> Self {
        Self::const_default()
    }
}

#[repr(packed)]
#[derive(Default, Clone, Copy)]
pub struct InterruptDescriptor {
    offset_low: u16,
    segment_selector: u16,
    attr: InterruptDescriptorAttribute,
    offset_middle: u16,
    offset_high: u32,
    _reserved: u32,
}

impl InterruptDescriptor {
    pub const fn const_default() -> Self {
        Self {
            offset_low: 0,
            segment_selector: 0,
            attr: InterruptDescriptorAttribute::const_default(),
            offset_middle: 0,
            offset_high: 0,
            _reserved: 0,
        }
    }

    pub fn set_idt_entry(
        &mut self,
        attr: InterruptDescriptorAttribute,
        entry: unsafe extern "C" fn(),
        segment_selector: u16,
    ) {
        let offset = entry as *const fn() as u64;
        self.attr = attr;
        self.offset_low = offset as u16;
        self.offset_middle = (offset >> 16) as u16;
        self.offset_high = (offset >> 32) as u32;
        self.segment_selector = segment_selector;
    }
}

pub fn notify_end_of_interrupt() {
    let end_of_interrupt = 0xfee0_00b0 as *mut u32;
    unsafe {
        end_of_interrupt.write_volatile(0);
    }
}

pub enum InterruptVector {
    XHCI = 0x40,
    LAPICTimer = 0x41,
}

pub struct InterruptFrame {
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

impl InterruptFrame {
    pub fn rip(&self) -> u64 {
        self.rip
    }

    pub fn cs(&self) -> u64 {
        self.cs
    }

    pub fn rflags(&self) -> u64 {
        self.rflags
    }

    pub fn rsp(&self) -> u64 {
        self.rsp
    }

    pub fn ss(&self) -> u64 {
        self.ss
    }
}
