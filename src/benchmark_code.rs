#![allow(unused)]
use std::hint::black_box;
use std::hint::black_box as bb;
use std::time::Duration;
use std::time::Instant;

fn main() {
    let t1 = || { /*{{TEST1}}*/ };
    let t2 = || { /*{{TEST2}}*/ };

    let b = Instant::now();
    std::hint::black_box(t1());
    let b = b.elapsed();

    let iter_count = match b.as_nanos() {
        0..=1000 => 1000000,          // 0ns-1µs
        1001..=1_000_000 => 1000,     // 1µs-1ms
        1_000_001..=50_000_000 => 20, // 1ms-50ms
        _ => 1,                       // 50ms+
    };

    let run1 = || {
        let b = Instant::now();
        for _ in 0..iter_count {
            std::hint::black_box(t1());
        }
        b.elapsed()
    };

    let run2 = || {
        let b = Instant::now();
        for _ in 0..iter_count {
            std::hint::black_box(t2());
        }
        b.elapsed()
    };

    let mut dur1 = Duration::ZERO;
    let mut dur2 = Duration::ZERO;
    for _ in 0..3 {
        dur1 += run1();
        dur2 += run2();
    }
    println!("Test 1: {:?}", dur1 / iter_count / 3);
    println!("Test 2: {:?}", dur2 / iter_count / 3);
}
