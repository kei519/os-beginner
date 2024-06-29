use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned, TokenStreamExt as _};
use syn::{parse_macro_input, spanned::Spanned, Error, FnArg, ItemFn};

extern crate proc_macro;

macro_rules! err {
    ($span:expr, $message:expr) => {
        Error::new($span.span(), $message).to_compile_error()
    };
    ($span:expr, $message:expr, $($args:expr),*) => {
        Error::new($span.spna(), format!($message, $($args),*)).to_compile_error();
    };
}

/// mikan-os 上で動作するアプリのメイン関数を表す。
/// メイン関数は `Args` を受け取り [i32] を返す。
///
/// この関数は [uefi-rs](https://github.com/rust-osdev/uefi-rs) の `entry` マクロを参考にしている。
#[proc_macro_attribute]
pub fn main(args: TokenStream, input: TokenStream) -> TokenStream {
    // エラー原因を一括でできるだけたくさん表示するために、エラーを保存する変数
    let mut errors = TokenStream2::new();

    // 属性は引数を受け付けない
    if !args.is_empty() {
        errors.append_all(err!(
            TokenStream2::from(args),
            "Entry attribute accepts no arguments"
        ));
    }

    let f = parse_macro_input!(input as ItemFn);

    if let Some(asyncness) = f.sig.asyncness {
        errors.append_all(err!(asyncness, "Entry should not be async"));
    }
    if let Some(constness) = f.sig.constness {
        errors.append_all(err!(constness, "Entry should not be const"));
    }
    if !f.sig.generics.params.is_empty() {
        errors.append_all(err!(f.sig.generics.params, "Entry should not be generic"));
    }

    // ここまでのエラーを一括で表示
    if !errors.is_empty() {
        return errors.into();
    }

    let signature_span = f.sig.span();

    let unsafety = &f.sig.unsafety;
    let fn_ident = &f.sig.ident;
    let fn_inputs_types = f.sig.inputs.iter().map(|arg| match arg {
        FnArg::Receiver(arg) => quote!(#arg),
        FnArg::Typed(arg) => {
            let ty = &arg.ty;
            quote!(#ty)
        }
    });
    let fn_output_type = &f.sig.output;

    // 関数を `fn(Args) -> i32` にキャストすることで、他の引数、戻り値の場合にエラーを出せる
    let fn_type_check = quote_spanned! {signature_span =>
        const _:
            #unsafety fn(::app_lib::args::Args) -> i32 =
            #fn_ident as #unsafety fn(#(#fn_inputs_types),*) #fn_output_type;
    };

    let result = quote! {
        #fn_type_check

        /// デフォルトのエントリーポイント
        #[no_mangle]
        extern "sysv64" fn _start(argc: i32, argv :*const *const ::core::ffi::c_char) -> ! {
            let args = unsafe { ::app_lib::args::Args::new(argc as usize, argv) };
            ::app_lib::exit(#fn_ident(args))
        }

        #f
    };
    result.into()
}
