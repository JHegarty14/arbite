use backtrace::Backtrace;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::panic::UnwindSafe;

struct Panic {
    backtrace: Backtrace,
    msg: String,
}

static mut PANIC: Option<Panic> = None;

pub fn handle_error<F>(f: F) -> proc_macro::TokenStream
where
    F: FnOnce() -> Result<TokenStream, TokenStream> + UnwindSafe,
{
    unsafe {
        PANIC = None;

        std::panic::set_hook(Box::new(|info| {
            PANIC = Some(Panic {
                backtrace: Backtrace::new(),
                msg: info.to_string(),
            });
        }));

        let result = std::panic::catch_unwind(|| f());
        let _ = std::panic::take_hook();

        if result.is_ok() {
            return match result.unwrap() {
                Ok(r) => r.into(),
                Err(r) => r.into(),
            };
        }
        if let Some(ref p) = PANIC {
            let msg = format!("sept_di panicked:\n{}\n{:#?}", p.msg, p.backtrace);
            return quote! {
                compile_error!(#msg);
            }
            .into();
        } else {
            std::panic::resume_unwind(result.err().unwrap())
        }
    }
}