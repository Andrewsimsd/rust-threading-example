use std::sync::atomic::{
    AtomicBool, AtomicIsize, AtomicPtr, AtomicUsize, Ordering,
};
use std::sync::Arc;
use std::thread;
use std::ptr;

fn main() {
    correct_atomic_usage();
    incorrect_atomic_usage();
}

fn correct_atomic_usage() {
    println!("=== ✅ Correct Usage ===");

    // Atomic counter across threads
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..4 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("Counter value (should be 4000): {}", counter.load(Ordering::Relaxed));

    // Atomic flag with proper ordering
    let flag = Arc::new(AtomicBool::new(false));

    // Writer thread
    let flag_writer = Arc::clone(&flag);
    let writer = thread::spawn(move || {
        // Do some writes here...
        flag_writer.store(true, Ordering::Release);
    });

    // Reader thread
    let flag_reader = Arc::clone(&flag);
    let reader = thread::spawn(move || {
        while !flag_reader.load(Ordering::Acquire) {
            std::hint::spin_loop();
        }
        println!("Flag observed as true with Acquire/Release ordering");
    });

    writer.join().unwrap();
    reader.join().unwrap();

    // Atomic pointer usage
    let x = Box::new(123);
    let atomic_ptr = AtomicPtr::new(Box::into_raw(x));

    unsafe {
        let raw = atomic_ptr.load(Ordering::SeqCst);
        println!("AtomicPtr loaded value: {}", *raw);

        // Clean up heap memory
        drop(Box::from_raw(raw));
    }
}

fn incorrect_atomic_usage() {
    println!("\n=== ❌ Incorrect Usage ===");

    // ❌ Using atomics with incorrect memory ordering
    let flag = Arc::new(AtomicBool::new(false));
    let data = Arc::new(AtomicUsize::new(0));

    // Writer thread
    let flag_writer = Arc::clone(&flag);
    let data_writer = Arc::clone(&data);
    let writer = thread::spawn(move || {
        data_writer.store(42, Ordering::Relaxed);    // No ordering guarantees
        flag_writer.store(true, Ordering::Relaxed);  // ❌ Wrong: should use Release
    });

    // Reader thread
    let flag_reader = Arc::clone(&flag);
    let data_reader = Arc::clone(&data);
    let reader = thread::spawn(move || {
        while !flag_reader.load(Ordering::Relaxed) { // ❌ Wrong: should use Acquire
            std::hint::spin_loop();
        }

        // ⚠️ May print 0 due to reordering (data store not visible yet)
        let loaded = data_reader.load(Ordering::Relaxed);
        println!("Loaded data (expect 42, but might get 0): {}", loaded);
    });

    writer.join().unwrap();
    reader.join().unwrap();

    // ❌ Using AtomicPtr without freeing old value
    let p = Box::new(99);
    let atomic_ptr = AtomicPtr::new(Box::into_raw(p));

    // Replacing pointer, leaking memory
    let new_ptr = Box::into_raw(Box::new(100));
    atomic_ptr.store(new_ptr, Ordering::SeqCst);

    // ❌ Leaked original pointer (99) — no drop was called
    println!("Leaking memory due to lost pointer in AtomicPtr");

    // Clean up only the final value
    unsafe {
        drop(Box::from_raw(atomic_ptr.load(Ordering::SeqCst)));
    }

    // ❌ Using atomics on non-atomic data
    let mut x = 0usize;
    let x_ptr = &x as *const usize as usize;
    let y = AtomicUsize::new(x_ptr);

    // ❌ This isn't safe unless pointer is valid and synchronized properly
    println!("Unsafe casted pointer used atomically: {}", y.load(Ordering::Relaxed));
}
