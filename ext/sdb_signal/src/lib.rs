mod buffer;

use lazy_static::lazy_static;
use libc::{
    c_char, pthread_create, pthread_kill, pthread_self, pthread_t, sigaction, sigemptyset,
    SA_RESTART, SA_SIGINFO, SIGPROF,
};
use magnus::{function, prelude::*, Error, Ruby};
use rb_sys::{
    rb_define_module, rb_define_singleton_method, rb_int2inum, rb_profile_frame_full_label,
    rb_profile_frame_path, rb_profile_frames, Qtrue, RARRAY_LEN, VALUE,
};
use std::collections::HashMap;
use std::mem::zeroed;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};
use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::buffer::*;

const MAX_STACK_DEPTH: usize = 2048;
const ONE_MILLISECOND_NS: u64 = 1_000_000; // 1ms in nanoseconds
const RING_BUFFER_SIZE: usize = 1024 * 1024; // 1M entries

// Lock-free ring buffer for storing profiling data
struct RingBuffer {
    data: Box<[AtomicUsize; RING_BUFFER_SIZE]>,
    write_pos: AtomicUsize,
}

impl RingBuffer {
    fn new() -> Self {
        let mut data = Vec::with_capacity(RING_BUFFER_SIZE);
        for _ in 0..RING_BUFFER_SIZE {
            data.push(AtomicUsize::new(0));
        }
        Self {
            data: data.into_boxed_slice().try_into().unwrap(),
            write_pos: AtomicUsize::new(0),
        }
    }

    fn push(&self, value: usize) {
        let pos = self.write_pos.fetch_add(1, Ordering::Relaxed) % RING_BUFFER_SIZE;
        self.data[pos].store(value, Ordering::Relaxed);
    }
}

lazy_static! {
    static ref PROFILING_BUFFER: RingBuffer = RingBuffer::new();
    static ref COUNTER: AtomicUsize = AtomicUsize::new(0);
    static ref SAMPLING_INTERVAL_NS: AtomicUsize = AtomicUsize::new(ONE_MILLISECOND_NS as usize);
}

struct SchedulerData {
    threads: Vec<pthread_t>,
    thread_to_value: HashMap<pthread_t, VALUE>,
}

lazy_static! {
    static ref SCHEDULER_DATA: RwLock<SchedulerData> = RwLock::new(SchedulerData {
        threads: vec![],
        thread_to_value: HashMap::new(),
    });
}

#[inline]
pub fn arvg_to_ptr(val: &[VALUE]) -> *const VALUE {
    val as *const [VALUE] as *const VALUE
}

fn get_counter_value() -> usize {
    COUNTER.load(Ordering::SeqCst)
}

// Temporary buffer for stack frames in the signal handler
static mut TEMP_BUFFER: [VALUE; MAX_STACK_DEPTH] = [0; MAX_STACK_DEPTH];
static mut TEMP_LINES: [i32; MAX_STACK_DEPTH] = [0; MAX_STACK_DEPTH];

// stack_scanner fetches all frames through `rb_profile_thread_frames`
// and stores them in the lock-free ring buffer
unsafe extern "C" fn stack_scanner(_: i32, _: *mut libc::siginfo_t, _: *mut libc::c_void) {
    COUNTER.fetch_add(1, Ordering::Relaxed);

    let frames_count = rb_profile_frames(
        0,
        MAX_STACK_DEPTH as i32,
        TEMP_BUFFER.as_mut_ptr(),
        TEMP_LINES.as_mut_ptr(),
    );

    // Store frame count
    PROFILING_BUFFER.push(frames_count as usize);

    // Store frames
    let mut j = 0;
    while j < frames_count {
        let frame = TEMP_BUFFER[j as usize];
        PROFILING_BUFFER.push(frame as usize);
        j += 1;
    }

    // Push separator
    PROFILING_BUFFER.push(usize::MAX);
}

fn setup_signal_handler() {
    unsafe {
        let mut sa: sigaction = zeroed();
        sa.sa_sigaction = stack_scanner as usize;
        sa.sa_flags = SA_RESTART | SA_SIGINFO;
        sigemptyset(&mut sa.sa_mask);
        sigaction(SIGPROF, &sa, std::ptr::null_mut());
    }
}

extern "C" fn scheduler_func(_: *mut libc::c_void) -> *mut libc::c_void {
    unsafe {
        loop {
            let interval = SAMPLING_INTERVAL_NS.load(Ordering::Relaxed) as u64;

            if interval >= ONE_MILLISECOND_NS {
                sleep(Duration::from_nanos(interval));
            } else {
                // For sub-millisecond intervals, use busy loop as on linux or macos, they can't sleep for less than 1ms
                let start = Instant::now();
                let target = start + Duration::from_nanos(interval);

                while Instant::now() < target {
                    std::hint::spin_loop();
                }
            }

            let data = SCHEDULER_DATA.read().unwrap();
            for thread in &data.threads {
                pthread_kill(*thread as pthread_t, SIGPROF);
            }
        }
    }
}

fn get_current_thread_id() -> pthread_t {
    unsafe { pthread_self() }
}

// start_scheduler creates a new thread through pthread_create which triggers stack scanning every millisecond.
// threads: the threads to scan
unsafe extern "C" fn start_scheduler(_module: VALUE) -> VALUE {
    unsafe {
        let mut thread: pthread_t = zeroed();

        pthread_create(
            &mut thread,
            ptr::null(),
            scheduler_func,
            std::ptr::null_mut(),
        );
    }

    Qtrue as VALUE
}

fn sleep_with_gvl() {
    sleep(Duration::from_secs(60 * 60));
}

fn print_counter() {
    println!("counter={}", get_counter_value());
}

unsafe extern "C" fn register_thread(_module: VALUE) -> VALUE {
    let thread = get_current_thread_id();
    if let Ok(mut data) = SCHEDULER_DATA.write() {
        println!("push thread={thread}");
        data.threads.push(thread);

        // Get the current Ruby thread VALUE
        let rb_thread = rb_sys::rb_thread_current();
        data.thread_to_value.insert(thread, rb_thread);
    }

    Qtrue as VALUE
}

fn set_sampling_interval(nanos: usize) {
    SAMPLING_INTERVAL_NS.store(nanos, Ordering::Relaxed);
}

fn get_sampling_interval() -> usize {
    SAMPLING_INTERVAL_NS.load(Ordering::Relaxed)
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("SdbSignal")?;
    module.define_singleton_method("setup_signal_handler", function!(setup_signal_handler, 0))?;
    module.define_singleton_method("sleep_with_gvl", function!(sleep_with_gvl, 0))?;
    module.define_singleton_method("print_counter", function!(print_counter, 0))?;
    module.define_singleton_method("set_sampling_interval", function!(set_sampling_interval, 1))?;
    module.define_singleton_method("get_sampling_interval", function!(get_sampling_interval, 0))?;

    unsafe {
        let m = rb_define_module("SdbSignal\0".as_ptr() as *const c_char);

        let start_scheduler_callback = std::mem::transmute::<
            unsafe extern "C" fn(VALUE) -> VALUE,
            unsafe extern "C" fn() -> VALUE,
        >(start_scheduler);

        rb_define_singleton_method(
            m,
            "start_scheduler\0".as_ptr() as _,
            Some(start_scheduler_callback),
            0,
        );

        let register_thread_callback = std::mem::transmute::<
            unsafe extern "C" fn(VALUE) -> VALUE,
            unsafe extern "C" fn() -> VALUE,
        >(register_thread);

        rb_define_singleton_method(
            m,
            "register_current_thread\0".as_ptr() as _,
            Some(register_thread_callback),
            0,
        );
    };

    Ok(())
}
