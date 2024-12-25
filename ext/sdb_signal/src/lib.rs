use libc::{sigaction, sigemptyset, SA_RESTART, SA_SIGINFO, SIGPROF};
use magnus::{function, prelude::*, Error, Ruby};
use std::mem::zeroed;

extern "C" fn signal_handler(_: i32, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    // Your signal handling logic here
}

fn setup_signal_handler() {
    unsafe {
        let mut sa: sigaction = zeroed();
        sa.sa_sigaction = signal_handler as usize;
        sa.sa_flags = SA_RESTART | SA_SIGINFO;
        sigemptyset(&mut sa.sa_mask);
        sigaction(SIGPROF, &sa, std::ptr::null_mut());
    }
}

fn hello(subject: String) -> String {
    format!("Hello from Rust, {subject}!")
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("SdbSignal")?;
    module.define_singleton_method("hello", function!(hello, 1))?;
    module.define_singleton_method("setup_signal_handler", function!(setup_signal_handler, 0))?;

    Ok(())
}
