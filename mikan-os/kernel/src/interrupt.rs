use core::{arch::global_asm, mem};

use crate::{
    asmfunc,
    bitfield::BitField as _,
    font::{self, write_string},
    frame_buffer::FrameBuffer,
    graphics::{PixelColor, Vector2D},
    layer::SCREEN,
    message::MessageType,
    paging::handle_page_fault,
    segment::KERNEL_CS,
    sync::Mutex,
    task,
    x86_descriptor::{DescriptorType, SystemSegmentType},
};

static IDT: Mutex<[InterruptDescriptor; 256]> =
    Mutex::new([InterruptDescriptor::const_default(); 256]);

pub fn init() {
    let mut idt = IDT.lock_wait();
    idt[InterruptVector::LAPICTimer as usize].set_idt_entry(
        int_handler_lapic_timer,
        KERNEL_CS,
        InterruptDescriptor::IST_FOR_TIMER,
    );

    let mut set_idt_entry = |irq: usize, handler| idt[irq].set_idt_entry(handler, KERNEL_CS, 0);

    set_idt_entry(InterruptVector::XHCI as _, int_handler_xhci);
    set_idt_entry(0, int_handler_de);
    set_idt_entry(1, int_handler_db);
    set_idt_entry(3, int_handler_bp);
    set_idt_entry(4, int_handler_of);
    set_idt_entry(5, int_handler_br);
    set_idt_entry(6, int_handler_ud);
    set_idt_entry(7, int_handler_nm);
    set_idt_entry(8, int_handler_df);
    set_idt_entry(10, int_handler_ts);
    set_idt_entry(11, int_handler_np);
    set_idt_entry(12, int_handler_ss);
    set_idt_entry(13, int_handler_gp);
    set_idt_entry(14, int_handler_pf);
    set_idt_entry(16, int_handler_mf);
    set_idt_entry(17, int_handler_ac);
    set_idt_entry(18, int_handler_mc);
    set_idt_entry(19, int_handler_xm);
    set_idt_entry(20, int_handler_ve);
    asmfunc::load_idt(
        (mem::size_of::<InterruptDescriptor>() * idt.len()) as u16 - 1,
        idt.as_ptr() as u64,
    )
}

fn kill_app(frame: &InterruptFrame) {
    // CPU 例外の原因がアプリの場合はアプリを落とすに留める
    let cpl = frame.cs & 0x3;
    if cpl != 3 {
        return;
    }

    let task = task::current_task();
    asmfunc::sti();

    const SIGSEGV: i32 = 11;
    asmfunc::exit_app(*task.os_stack_ptr(), 128 + SIGSEGV);
}

/// エラーコード付きのデフォルトの割り込みハンドラを定義する。
/// 割り込みハンドラ名は `int_handler_$arg` になる。（ただし `$arg` は全て小文字にされる）
macro_rules! fault_handler_with_error {
    ($fault_name:ident) => {
        ::paste::paste! {
            #[::custom_attribute::interrupt]
            fn [<int_handler_ $fault_name:lower>](
                frame: &$crate::interrupt::InterruptFrame,
                error_code: u64
            ) {
                kill_app(frame);
                $crate::interrupt::print_frame(
                    frame,
                    concat!("#", ::core::stringify!([< $fault_name:upper >])),
                );
                let mut screen = $crate::layer::SCREEN.lock_wait();
                let writer = &mut *screen;
                $crate::font::write_string(
                    writer,
                    Vector2D::new(500, 16 * 4),
                    b"ERR",
                    &$crate::graphics::PixelColor::new(0, 0, 0),
                );
                $crate::interrupt::print_hex(
                    error_code,
                    16,
                    Vector2D::new(500 + 8 * 4, 16 * 4),
                    writer);
                $crate::asmfunc::halt();
            }
        }
    };
}

/// エラーコードなしのデフォルトの割り込みハンドラを定義する。
/// 割り込みハンドラ名は `int_handler_$arg` になる。（ただし `$arg` は全て小文字にされる）
macro_rules! fault_handler_no_error {
    ($fault_name:ident) => {
        ::paste::paste! {
            #[::custom_attribute::interrupt]
            fn [<int_handler_ $fault_name:lower>](frame: &$crate::interrupt::InterruptFrame) {
                kill_app(frame);
                $crate::interrupt::print_frame(
                    frame,
                    concat!("#", ::core::stringify!([< $fault_name:upper >])),
                );
                $crate::asmfunc::halt();
            }
        }
    };
}

fault_handler_no_error!(DE);
fault_handler_no_error!(DB);
fault_handler_no_error!(BP);
fault_handler_no_error!(OF);
fault_handler_no_error!(BR);
fault_handler_no_error!(UD);
fault_handler_no_error!(NM);
fault_handler_with_error!(DF);
fault_handler_with_error!(TS);
fault_handler_with_error!(NP);
fault_handler_with_error!(SS);
fault_handler_with_error!(GP);
fault_handler_no_error!(MF);
fault_handler_with_error!(AC);
fault_handler_no_error!(MC);
fault_handler_no_error!(XM);
fault_handler_no_error!(VE);

#[custom_attribute::interrupt]
fn int_handler_xhci(_frame: &InterruptFrame) {
    // メインタスクが 1 で登録されるので必ず存在するはず
    task::send_message(1, MessageType::InterruptXHCI.into()).unwrap();
    notify_end_of_interrupt();
}

#[custom_attribute::interrupt]
fn int_handler_pf(frame: &InterruptFrame, error_code: u64) {
    let cr2 = asmfunc::get_cr2();
    if handle_page_fault(error_code, cr2).is_ok() {
        return;
    }
    kill_app(frame);
    print_frame(frame, "#PF");
    let mut screen = SCREEN.lock_wait();
    let writer = &mut *screen;
    write_string(
        writer,
        Vector2D::new(500, 16 * 4),
        b"ERR",
        &PixelColor::new(0, 0, 0),
    );
    print_hex(error_code, 16, Vector2D::new(500, 16 * 4), writer);
    asmfunc::halt();
}

extern "C" {
    fn int_handler_lapic_timer();
}

global_asm! {r#"
.global int_handler_lapic_timer
int_handler_lapic_timer: # int_handler_lapic_timer()
    push rbp
    mov rbp, rsp

    # スタック上に TaskContex 型の構造を構築する
    sub rsp, 512
    fxsave [rsp]
    push r15
    push r14
    push r13
    push r12
    push r11
    push r10
    push r9
    push r8
    push qword ptr [rbp]        # RBP
    push qword ptr [rbp + 0x20] # RSP
    push rsi
    push rdi
    push rdx
    push rcx
    push rbx
    push rax

    mov ax, gs
    mov bx, fs
    mov rcx, cr3

    push rax          # GS
    push rbx          # FS
    push [rbp + 0x28] # SS
    push [rbp + 0x10] # CS

    push rbp          # reserved1
    push [rbp + 0x18] # RFLAGS
    push [rbp + 0x08] # RIP
    push rcx          # CR3

    mov rdi, rsp
    call lapic_timer_on_interrupt

    add rsp, 8 * 8 # CR3 から GS までを無視
    pop rax
    pop rbx
    pop rcx
    pop rdx
    pop rdi
    pop rsi
    add rsp, 8 * 2 # RSP, RBP を無視
    pop r8
    pop r9
    pop r10
    pop r11
    pop r12
    pop r13
    pop r14
    pop r15
    fxrstor [rsp]

    mov rsp, rbp
    pop rbp
    iretq
"#}

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

    pub fn new(r#type: SystemSegmentType, descriptor_privilege_level: u8, ist: u8) -> Self {
        let mut etc_1 = 0;
        etc_1.set_bits(..3, ist);

        let mut etc_2 = 0;
        etc_2.set_bits(..4, DescriptorType::system_segment(r#type).into());
        etc_2.set_bits(5..7, descriptor_privilege_level);
        etc_2.set_bit(7, true); // present

        Self { etc_1, etc_2 }
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
    /// タイマー割り込みの IST（Interrupt Stack Table）。
    pub const IST_FOR_TIMER: u8 = 1;

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

    pub fn set_idt_entry(&mut self, entry: unsafe extern "C" fn(), segment_selector: u16, ist: u8) {
        let attr = InterruptDescriptorAttribute::new(SystemSegmentType::InterruptGate, 0, ist);
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

pub fn print_hex(value: u64, width: i32, pos: Vector2D<i32>, screen: &mut FrameBuffer) {
    for i in 0..width {
        let x = (value >> (4 * (width - i - 1) as u64)).get_bits(..4) as u8;
        let x = x + if x >= 10 { b'a' - 10 } else { b'0' };
        font::write_ascii(
            screen,
            pos + Vector2D::new(8 * i, 0),
            x,
            &PixelColor::new(0, 0, 0),
        );
    }
}

#[allow(clippy::erasing_op)]
#[allow(clippy::identity_op)]
pub fn print_frame(frame: &InterruptFrame, exp_name: &str) {
    let mut screen = SCREEN.lock_wait();
    let writer = &mut *screen;
    font::write_string(
        writer,
        Vector2D::new(500, 16 * 0),
        exp_name.as_bytes(),
        &PixelColor::new(0, 0, 0),
    );

    font::write_string(
        writer,
        Vector2D::new(500, 16 * 1),
        b"CS:RIP",
        &PixelColor::new(0, 0, 0),
    );
    print_hex(frame.cs(), 4, Vector2D::new(500 + 8 * 7, 16 * 1), writer);
    print_hex(frame.rip(), 16, Vector2D::new(500 + 8 * 12, 16 * 1), writer);

    font::write_string(
        writer,
        Vector2D::new(500, 16 * 2),
        b"RFLAGS",
        &PixelColor::new(0, 0, 0),
    );
    print_hex(
        frame.rflags(),
        16,
        Vector2D::new(500 + 8 * 7, 16 * 2),
        writer,
    );

    font::write_string(
        writer,
        Vector2D::new(500, 16 * 3),
        b"SS:RSP",
        &PixelColor::new(0, 0, 0),
    );
    print_hex(frame.ss(), 4, Vector2D::new(500 + 8 * 7, 16 * 3), writer);
    print_hex(frame.rsp(), 16, Vector2D::new(500 + 8 * 12, 16 * 3), writer);
}
