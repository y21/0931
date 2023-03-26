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
    let time_ns1 = dur1.as_nanos() / iter_count / 3;
    let time_ns2 = dur2.as_nanos() / iter_count / 3;

    if time_ns1 == 0 {
        println!(
            "{:.5}ns",
            (dur1.as_nanos() as f64) / iter_count as f64 / 3.0
        );
    } else {
        println!("{:?}", time_ns1);
    }

    if time_ns2 == 0 {
        println!(
            "{:.5}ns",
            (dur2.as_nanos() as f64) / iter_count as f64 / 3.0
        );
    } else {
        println!("{:?}", time_ns2);
    }
}
