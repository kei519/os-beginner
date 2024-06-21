#[repr(C)]
#[derive(Clone, Copy)]
pub struct Elf64Ehdr {
    pub ident: [u8; 16],
    pub r#type: u16,
    pub machine: u16,
    pub version: u32,
    pub entry: usize,
    pub phoff: u64,
    pub shoff: u64,
    pub flags: u32,
    pub ehsize: u16,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Elf64Phdr {
    pub r#type: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: usize,
    pub paddr: usize,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(unused)]
pub enum ProgramType {
    Null = 0,
    Load = 1,
    Dynamic = 2,
    Interp = 3,
    Note = 4,
    Shlib = 5,
    Phdr = 6,
    Tls = 7,
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(unused)]
pub struct Elf64Dyn {
    tag: i64,
    val: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(unused)]
pub enum DT {
    Null = 0,
    Rela = 7,
    Relasz = 8,
    Relaent = 9,
}

#[repr(C)]
#[derive(Clone, Copy)]
#[allow(unused)]
pub struct Elf64Rela {
    pub offset: u64,
    pub info: u32,
    pub addend: i32,
}
