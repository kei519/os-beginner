/// __`$name` という名前で宣言する。
/// 引数の型は全て [u64]。
macro_rules! syscall {
    ($name:ident, $number:expr, $($args:ident),*) => {
        ::paste::paste! {
            extern "sysv64" {
                #[doc = concat!(
                    "システムコール: ",
                    stringify!($name),
                    "（",
                    stringify!($number),
                    "）",
                )]
                pub(crate) fn [<__ $name>]($($args: u64),*) -> SysResult;
            }

            ::core::arch::global_asm! {
                concat!{
                    ".global ", stringify!([<__ $name>]), "\n",
                    stringify!([<__ $name>]), ":\n",
                    // EAX にシステムコール番号を入れる
                    "    mov eax, ", $number,
                        r#"
    mov r10, rcx  # RCX に RIP が保存されるため、System-V ABI では R10 で渡す
    syscall
    ret
                        "#,
                }
            }
        }
    };
}

syscall!(log_string, 0x8000_0000, log_level, s);
syscall!(put_string, 0x8000_0001, fd, buf, len);
syscall!(exit, 0x8000_0002, code);
syscall!(open_window, 0x8000_0003, w, h, x, y, title);
syscall!(win_write_string, 0x8000_0004, layer_id, x, y, color, s);

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct SysResult {
    pub value: u64,
    pub error: i32,
}
