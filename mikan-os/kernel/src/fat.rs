use core::{ffi::c_void, ptr};

use crate::util::OnceStatic;

pub static BOOT_VOLUME_IMAGE: OnceStatic<&'static BPB> = OnceStatic::new();

pub fn init(volume_image: *mut c_void) {
    BOOT_VOLUME_IMAGE.init(unsafe { &*(volume_image as *const BPB) });
}

pub fn get_sector_by_cluster<T>(cluster: u64, len: usize) -> &'static [T] {
    unsafe { &*ptr::slice_from_raw_parts(get_cluster_addr(cluster) as *const T, len) }
}

pub fn read_name(entry: &DirectoryEntry) -> (&[u8], &[u8]) {
    let base_len = entry
        .name
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, &b)| if b != 0x20 { Some(i + 1) } else { None })
        .unwrap_or(0);

    let ext_len = entry.name[8..]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, &b)| if b != 0x20 { Some(i + 1) } else { None })
        .unwrap_or(0);

    (&entry.name[..base_len], &entry.name[8..8 + ext_len])
}

#[repr(packed)]
pub struct BPB {
    jmp_bot: [u8; 3],
    oemname: [u8; 8],
    byts_per_sec: u16,
    sec_per_clus: u8,
    rsvd_sec_cnt: u16,
    num_fats: u8,
    root_ent_cnd: u16,
    tot_sec16: u16,
    media: u8,
    fat_sz16: u16,
    sec_per_trk: u16,
    num_heads: u16,
    hidd_sec: u32,
    tot_sec32: u32,
    fat_sz32: u32,
    ext_flags: u16,
    fsver: u16,
    root_clus: u32,
    fsinfo: u16,
    bk_boot_sec: u16,
    reserved: [u8; 12],
    drv_num: u8,
    reserved1: u8,
    boot_sig: u8,
    vol_id: u32,
    vol_lab: [u8; 11],
    fil_sys_type: u64,
}

impl BPB {
    pub fn jmp_bot(&self) -> [u8; 3] {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.jmp_bot)) }
    }
    pub fn oemname(&self) -> [u8; 8] {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.oemname)) }
    }
    pub fn byts_per_sec(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.byts_per_sec)) }
    }
    pub fn sec_per_clus(&self) -> u8 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.sec_per_clus)) }
    }
    pub fn rsvd_sec_cnt(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.rsvd_sec_cnt)) }
    }
    pub fn num_fats(&self) -> u8 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.num_fats)) }
    }
    pub fn root_ent_cnd(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.root_ent_cnd)) }
    }
    pub fn tot_sec16(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.tot_sec16)) }
    }
    pub fn media(&self) -> u8 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.media)) }
    }
    pub fn fat_sz16(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.fat_sz16)) }
    }
    pub fn sec_per_trk(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.sec_per_trk)) }
    }
    pub fn num_heads(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.num_heads)) }
    }
    pub fn hidd_sec(&self) -> u32 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.hidd_sec)) }
    }
    pub fn tot_sec32(&self) -> u32 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.tot_sec32)) }
    }
    pub fn fat_sz32(&self) -> u32 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.fat_sz32)) }
    }
    pub fn ext_flags(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.ext_flags)) }
    }
    pub fn fsver(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.fsver)) }
    }
    pub fn root_clus(&self) -> u32 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.root_clus)) }
    }
    pub fn fsinfo(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.fsinfo)) }
    }
    pub fn bk_boot_sec(&self) -> u16 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.bk_boot_sec)) }
    }
    pub fn reserved(&self) -> [u8; 12] {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.reserved)) }
    }
    pub fn drv_num(&self) -> u8 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.drv_num)) }
    }
    pub fn reserved1(&self) -> u8 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.reserved1)) }
    }
    pub fn boot_sig(&self) -> u8 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.boot_sig)) }
    }
    pub fn vol_id(&self) -> u32 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.vol_id)) }
    }
    pub fn vol_lab(&self) -> [u8; 11] {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.vol_lab)) }
    }
    pub fn fil_sys_type(&self) -> u64 {
        unsafe { ptr::read_unaligned(ptr::addr_of!(self.fil_sys_type)) }
    }

    pub fn as_ptr(&self) -> *const Self {
        self as *const _
    }
}

#[repr(C)]
pub struct DirectoryEntry {
    pub name: [u8; 11],
    pub attr: u8,
    pub nt_res: u8,
    pub crt_time_tenth: u8,
    pub crt_time: u16,
    pub crt_date: u16,
    pub lst_acc_date: u16,
    pub fst_clus_hl: u16,
    pub wrt_time: u16,
    pub wrt_date: u16,
    pub fst_clus_lo: u16,
    pub file_size: u32,
}

#[repr(u8)]
pub enum Attribute {
    ReadOnly = 0x01,
    Hidden = 0x02,
    System = 0x04,
    VolumeID = 0x08,
    Directory = 0x10,
    Archive = 0x20,
    LongName = 0x0f,
}

fn get_cluster_addr(cluster: u64) -> *const u32 {
    let image = BOOT_VOLUME_IMAGE.get();
    let sector_num = image.rsvd_sec_cnt() as u64
        + image.num_fats() as u64 * image.fat_sz32() as u64
        + (cluster - 2) * image.sec_per_clus() as u64;
    let offset = sector_num * image.byts_per_sec() as u64;

    unsafe { (image.as_ptr() as *const u32).byte_add(offset as usize) }
}
