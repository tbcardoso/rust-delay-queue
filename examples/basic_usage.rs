extern crate delay_queue;

use std::time::{Duration, Instant};
use std::thread;
use delay_queue::{Delay, DelayQueue};

fn main() {
    let queue: DelayQueue<Delay<&str>> = DelayQueue::new();

    // Clone the queue and move it to the consumer thread
    let mut consumer_queue = queue.clone();
    let consumer_handle = thread::spawn(move || {
        // The pop() will block until an item is available and its delay has expired
        println!("First pop: {}", consumer_queue.pop().value); // Prints "First pop: now"
        println!("Second pop: {}", consumer_queue.pop().value); // Prints "Second pop: 3s"
    });

    // Clone the queue and move it to the producer thread
    let mut producer_queue = queue.clone();
    let producer_handle = thread::spawn(move || {
        // This item can only be popped after 3 seconds have passed
        producer_queue.push(Delay::for_duration("3s", Duration::from_secs(3)));

        // This item can be popped immediately
        producer_queue.push(Delay::until_instant("now", Instant::now()));
    });

    consumer_handle.join().unwrap();
    producer_handle.join().unwrap();

    assert!(queue.is_empty());
}
