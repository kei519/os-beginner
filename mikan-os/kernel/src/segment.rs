use core::{
    alloc::{GlobalAlloc, Layout},
    mem::{self, size_of},
};

use crate::{
    asmfunc::{self, load_gdt},
    bitfield::BitField as _,
    interrupt::InterruptDescriptor,
    log,
    logger::LogLevel,
    memory_manager::{BYTES_PER_FRAME, GLOBAL},
    sync::Mutex,
    util::OnceStatic,
    x86_descriptor::{DescriptorType, DescriptorTypeEnum, SystemSegmentType},
};

static GDT: Mutex<[SegmentDescriptor; 7]> = Mutex::new([SegmentDescriptor::default(); 7]);
static TSS: OnceStatic<Tss> = OnceStatic::new();

pub const KERNEL_CS: u16 = 1 << 3;
pub const KERNEL_SS: u16 = 2 << 3;
pub const TSS_SEL: u16 = 5 << 3;

pub fn setup_segments() {
    let mut gdt = GDT.lock_wait();
    gdt[1] = SegmentDescriptor::code_segment(0, 0xfffff, false, true, false, 0);
    gdt[2] = SegmentDescriptor::data_segment(0, 0xfffff, false, true, true, 0);
    // sysret 時のセグメントの設定のされ方が変なため、上と逆転している
    gdt[3] = SegmentDescriptor::data_segment(0, 0xfffff, false, true, true, 3);
    gdt[4] = SegmentDescriptor::code_segment(0, 0xfffff, false, true, false, 3);
    load_gdt(
        (size_of::<SegmentDescriptor>() * gdt.len()) as u16 - 1,
        gdt.as_ptr() as u64,
    );

    // TSS の設定
    TSS.init(Tss::new(allocate_stack_area(8)).set_ist(
        InterruptDescriptor::IST_FOR_TIMER as _,
        allocate_stack_area(8),
    ));

    let [tss_first, tss_second] =
        SegmentDescriptor::tss(TSS.as_ref().as_ptr() as _, (mem::size_of::<Tss>() - 1) as _);
    gdt[5] = tss_first;
    gdt[6] = tss_second;

    // TR の設定
    asmfunc::load_tr(TSS_SEL);
}

pub fn init() {
    setup_segments();

    asmfunc::set_ds_all(0);
    asmfunc::set_cs_ss(KERNEL_CS, KERNEL_SS);
}

/// セグメントディスクリプタを表す。
#[derive(Debug, Default, Clone, Copy)]
#[repr(packed)]
pub struct SegmentDescriptor {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    etc_1: u8,
    etc_2: u8,
    base_high: u8,
}

impl SegmentDescriptor {
    /// システムセグメントを表すディスクリプタを作る。
    pub fn system_segment(
        base: u32,
        limit: u32,
        r#type: SystemSegmentType,
        descriptor_privilege_level: u8,
    ) -> Self {
        let base_low = base.get_bits(..16) as u16;
        let base_middle = base.get_bits(16..24) as u8;
        let base_high = base.get_bits(24..32) as u8;
        let limit_low = limit.get_bits(..16) as u16;
        let limit_high = limit.get_bits(16..20) as u8;

        let mut etc_1 = 0;
        etc_1.set_bits(..5, DescriptorType::system_segment(r#type).into());
        etc_1.set_bits(5..7, descriptor_privilege_level);
        etc_1.set_bit(7, true); // present

        let mut etc_2 = 0;
        etc_2.set_bits(..4, limit_high);
        etc_2.set_bit(6, true); // default_operation_size
        etc_2.set_bit(7, true); // granalarity

        Self {
            limit_low,
            base_low,
            base_middle,
            etc_1,
            etc_2,
            base_high,
        }
    }

    pub fn tss(base: u64, limit: u32) -> [Self; 2] {
        let first = Self::system_segment(base as u32, limit, SystemSegmentType::TSSAvailable, 0);
        let second = Self {
            limit_low: base.get_bits(32..48) as _,
            base_low: base.get_bits(48..) as _,
            ..Default::default()
        };
        [first, second]
    }

    /// コードセグメントを表すディスクリプタを作る。
    pub fn code_segment(
        base: u32,
        limit: u32,
        accesed: bool,
        readable: bool,
        conforming: bool,
        descriptor_privilege_level: u8,
    ) -> Self {
        let base_low = base.get_bits(..16) as u16;
        let base_middle = base.get_bits(16..24) as u8;
        let base_high = base.get_bits(24..32) as u8;
        let limit_low = limit.get_bits(..16) as u16;
        let limit_high = limit.get_bits(16..20) as u8;

        let mut etc_1 = 0;
        etc_1.set_bits(
            ..5,
            DescriptorType::code_data_segment(accesed, readable, conforming, true).into(),
        );
        etc_1.set_bits(5..7, descriptor_privilege_level);
        etc_1.set_bit(7, true); // present

        let mut etc_2 = 0;
        etc_2.set_bits(..4, limit_high);
        etc_2.set_bit(4, false); // available
        etc_2.set_bit(5, true); // long mode
        etc_2.set_bit(6, false); // default_operation_size should be 0 when long_mode = 1
        etc_2.set_bit(7, true); // granalarity

        Self {
            limit_low,
            base_low,
            base_middle,
            etc_1,
            etc_2,
            base_high,
        }
    }

    /// データセグメントを表すディスクリプタを作る。
    pub fn data_segment(
        base: u32,
        limit: u32,
        accesed: bool,
        writable: bool,
        direction: bool,
        descriptor_privilege_level: u8,
    ) -> Self {
        let base_low = base.get_bits(..16) as u16;
        let base_middle = base.get_bits(16..24) as u8;
        let base_high = base.get_bits(24..32) as u8;
        let limit_low = limit.get_bits(..16) as u16;
        let limit_high = limit.get_bits(16..20) as u8;

        let mut etc_1 = 0;
        etc_1.set_bits(
            ..5,
            DescriptorType::code_data_segment(accesed, writable, direction, false).into(),
        );
        etc_1.set_bits(5..7, descriptor_privilege_level);
        etc_1.set_bit(7, true); // present

        let mut etc_2 = 0;
        etc_2.set_bits(..4, limit_high);
        etc_2.set_bit(4, false); // available
        etc_2.set_bit(5, false); // long mode should be 0
        etc_2.set_bit(6, true); // default_operation_size
        etc_2.set_bit(7, true); // granalarity

        Self {
            limit_low,
            base_low,
            base_middle,
            etc_1,
            etc_2,
            base_high,
        }
    }

    /// ヌルディスクリプタを作る。
    pub const fn default() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            etc_1: 0,
            etc_2: 0,
            base_high: 0,
        }
    }

    /// リミットを取得する。
    pub fn limit(&self) -> u32 {
        (self.etc_2.get_bits(4..) as u32) << 16 | self.limit_low as u32
    }

    /// ベースを取得する。
    pub fn base(&self) -> u32 {
        (self.base_high as u32) << 24 | (self.base_middle as u32) << 16 | self.base_low as u32
    }

    /// 種類を [DescriptorType] として取得する。
    pub fn type_raw(&self) -> DescriptorType {
        self.etc_1.get_bits(..5).into()
    }

    /// 種類を [DescriptorTypeEnum] として取得する。
    pub fn r#type(&self) -> DescriptorTypeEnum {
        self.type_raw().get()
    }

    /// ディスクリプタが表しているのがシステムセグメントであるかを取得する。
    pub fn is_system_segment(&self) -> bool {
        self.type_raw().is_system_segment()
    }

    /// ディスクリプタが表しているのがコード・データセグメントであるかを取得する。
    pub fn is_code_data_segment(&self) -> bool {
        self.type_raw().is_code_data_segment()
    }

    /// DPL を取得する。
    pub fn descriptor_privilege_level(&self) -> u8 {
        self.etc_1.get_bits(5..7)
    }

    /// 有効かどうかを取得する。
    pub fn present(&self) -> bool {
        self.etc_1.get_bit(7)
    }

    /// available（プログラムが利用可能）を取得する。
    pub fn available(&self) -> bool {
        self.etc_2.get_bit(4)
    }

    /// ロングモードが有効かどうかを取得する。
    pub fn long_mode(&self) -> bool {
        self.etc_2.get_bit(5)
    }

    /// Default ビットを取得する。
    pub fn default_operation_size(&self) -> bool {
        self.etc_2.get_bit(6)
    }

    /// Granuality ビットを取得する。
    pub fn granalarity(&self) -> bool {
        self.etc_2.get_bit(7)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Tss {
    reseved0: u32,
    rsp0_low: u32,
    rsp0_high: u32,
    rsp1_low: u32,
    rsp1_high: u32,
    rsp2_low: u32,
    rsp2_high: u32,
    reserved1: u32,
    reserved2: u32,
    ists: [u32; 2 * 7],
    reserved3: u32,
    reserved4: u32,
    reserved5: u16,
    io_base: u16,
}

impl Tss {
    pub fn new(rsp0: u64) -> Self {
        Self {
            rsp0_low: rsp0 as _,
            rsp0_high: rsp0.get_bits(32..) as _,
            ..Default::default()
        }
    }

    pub fn set_ist(mut self, ist: usize, stack_addr: u64) -> Self {
        let index = ist - 1;
        if index * 2 >= self.ists.len() {
            self
        } else {
            self.ists[index * 2] = stack_addr as _;
            self.ists[index * 2 + 1] = stack_addr.get_bits(32..) as _;
            self
        }
    }

    pub fn as_ptr(&self) -> *const Self {
        self as _
    }
}

/// `num_4kframes` 分のページを確保し、その領域の最後のアドレスを返す。
fn allocate_stack_area(num_4kframes: usize) -> u64 {
    let stk = unsafe {
        GLOBAL.alloc(Layout::from_size_align_unchecked(
            num_4kframes * BYTES_PER_FRAME,
            16,
        ))
    };
    if stk.is_null() {
        log!(LogLevel::Error, "failed to alloacte area");
        asmfunc::halt();
    }

    (stk as usize + num_4kframes * BYTES_PER_FRAME) as _
}
