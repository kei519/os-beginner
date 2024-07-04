/// __`$name` という名前で宣言する。
/// 引数の型は全て [u64]。
macro_rules! syscall {
    ($name:ident, $number:expr) => {
        syscall!($name, $number,);
    };
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
syscall!(
    win_write_string,
    0x8000_0004,
    layer_id_flags,
    x,
    y,
    color,
    s
);
syscall!(
    win_fill_rectangle,
    0x8000_0005,
    layer_id_flags,
    x,
    y,
    w,
    h,
    color
);
syscall!(get_current_tick, 0x8000_0006);
syscall!(win_redraw, 0x8000_0007, layer_id_flags);
syscall!(
    win_draw_line,
    0x8000_0008,
    layer_id_flags,
    x0,
    y0,
    x1,
    y1,
    color
);
syscall!(close_window, 0x8000_0009, layer_id);
syscall!(read_event, 0x8000_000a, events, len);
syscall!(create_timer, 0x8000_000b, mode, timer_value, timeout_ms);
syscall!(open_file, 0x8000_000c, path, flags);
syscall!(read_file, 0x8000_000d, fd, buf, count);

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct SysResult {
    pub value: u64,
    pub error: i32,
}
