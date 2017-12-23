use std::collections::BinaryHeap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use std::cmp::Ordering;
use delayed::Delayed;

/// A concurrent unbounded blocking queue where each item can only be removed when its delay
/// expires.
///
/// The queue supports multiple producers and multiple consumers.
///
/// Items of the queue must implement the `Delayed` trait. In most situations you can just use
/// the helper struct `Delay` to wrap the values to be used by the queue.
///
/// If you implement the `Delayed` trait for your types, keep in mind that the `DelayQueue` assumes
/// that the `Instant` until which each item is delayed does not change while that item is
/// in the queue.
///
/// # Examples
///
/// Basic usage:
///
/// ```no_run
/// use delay_queue::{Delay, DelayQueue};
/// use std::time::{Duration, Instant};
///
/// let mut queue = DelayQueue::new();
/// queue.push(Delay::for_duration("2nd", Duration::from_secs(5)));
/// queue.push(Delay::until_instant("1st", Instant::now()));
///
/// println!("First pop: {}", queue.pop().value);
/// println!("Second pop: {}", queue.pop().value);
/// assert!(queue.is_empty());
/// ```
#[derive(Debug)]
pub struct DelayQueue<T: Delayed> {
    /// Points to the data that is shared between instances of the same queue (created by
    /// cloning a queue). Usually the different instances of a queue will live in different
    /// threads.
    shared_data: Arc<DelayQueueSharedData<T>>,
}

/// The underlying data of a queue.
///
/// When a `DelayQueue` is cloned, it's clone will point to the same `DelayQueueSharedData`.
/// This is done so a queue be used by different threads.
#[derive(Debug)]
struct DelayQueueSharedData<T: Delayed> {
    /// Mutex protected `BinaryHeap` that holds the items of the queue in the order that they
    /// should be popped.
    queue: Mutex<BinaryHeap<Entry<T>>>,

    /// Condition variable that signals when there is a new item at the head of the queue.
    condvar_new_head: Condvar,
}

impl<T: Delayed> DelayQueue<T> {
    /// Creates an empty `DelayQueue`.
    pub fn new() -> DelayQueue<T> {
        DelayQueue {
            shared_data: Arc::new(DelayQueueSharedData {
                queue: Mutex::new(BinaryHeap::new()),
                condvar_new_head: Condvar::new(),
            }),
        }
    }

    /// Pushes an item onto the queue.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use delay_queue::{Delay, DelayQueue};
    /// use std::time::Duration;
    ///
    /// let mut queue = DelayQueue::new();
    /// queue.push(Delay::for_duration("2nd", Duration::from_secs(5)));
    /// ```
    pub fn push(&mut self, item: T) {
        let mut queue = self.shared_data.queue.lock().unwrap();

        {
            let cur_head = queue.peek();
            if (cur_head == None)
                || (item.delayed_until() < cur_head.unwrap().delayed.delayed_until())
            {
                self.shared_data.condvar_new_head.notify_one();
            }
        }

        queue.push(Entry::new(item));
    }

    /// Pops the next item from the queue, blocking if necessary until an item is available and its
    /// delay has expired.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use delay_queue::{Delay, DelayQueue};
    /// use std::time::{Duration, Instant};
    ///
    /// let mut queue = DelayQueue::new();
    ///
    /// queue.push(Delay::until_instant("1st", Instant::now()));
    ///
    /// // The pop will not block, since the delay has expired.
    /// println!("First pop: {}", queue.pop().value);
    ///
    /// queue.push(Delay::for_duration("2nd", Duration::from_secs(5)));
    ///
    /// // The pop will block for approximately 5 seconds before returning the item.
    /// println!("Second pop: {}", queue.pop().value);
    /// ```
    pub fn pop(&mut self) -> T {
        let mut queue = self.shared_data.queue.lock().unwrap();

        loop {
            let now = Instant::now();

            let wait_duration = match queue.peek() {
                Some(elem) if elem.delayed.delayed_until() <= now => break,
                Some(elem) => elem.delayed.delayed_until() - now,
                None => Duration::from_secs(0),
            };

            queue = if wait_duration > Duration::from_secs(0) {
                self.shared_data
                    .condvar_new_head
                    .wait_timeout(queue, wait_duration)
                    .unwrap()
                    .0
            } else {
                self.shared_data.condvar_new_head.wait(queue).unwrap()
            };
        }

        if queue.len() > 1 {
            self.shared_data.condvar_new_head.notify_one();
        }

        queue.pop().unwrap().delayed
    }

    /// Checks if the queue is empty.
    pub fn is_empty(&self) -> bool {
        let queue = self.shared_data.queue.lock().unwrap();
        queue.is_empty()
    }
}

impl<T: Delayed> Clone for DelayQueue<T> {
    /// Returns a new `DelayQueue` that points to the same underlying data.
    ///
    /// This is needed to share a queue between different threads.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use delay_queue::{Delay, DelayQueue};
    /// use std::time::Duration;
    /// use std::thread;
    ///
    /// let mut queue = DelayQueue::new();
    ///
    /// queue.push(Delay::for_duration("1st", Duration::from_secs(1)));
    ///
    /// let mut cloned_queue = queue.clone();
    ///
    /// let handle = thread::spawn(move || {
    ///     println!("First pop: {}", cloned_queue.pop().value);
    ///     println!("Second pop: {}", cloned_queue.pop().value);
    /// });
    ///
    /// queue.push(Delay::for_duration("2nd", Duration::from_secs(2)));
    ///
    /// handle.join().unwrap();
    /// ```
    fn clone(&self) -> DelayQueue<T> {
        DelayQueue {
            shared_data: self.shared_data.clone(),
        }
    }
}


/// An entry in the `DelayQueue`.
///
/// Holds a `Delayed` item and implements an ordering based on delay `Instant`s of the items.
#[derive(Debug)]
struct Entry<T: Delayed> {
    delayed: T,
}

impl<T: Delayed> Entry<T> {
    fn new(delayed: T) -> Entry<T> {
        Entry { delayed }
    }
}

/// Implements ordering for `Entry`, so it can be used to correctly order elements in the
/// `BinaryHeap` of the `DelayQueue`.
///
/// Earlier entries have higher priority (should be popped first), so they are Greater that later
/// entries.
impl<T: Delayed> Ord for Entry<T> {
    fn cmp(&self, other: &Entry<T>) -> Ordering {
        other
            .delayed
            .delayed_until()
            .cmp(&self.delayed.delayed_until())
    }
}

impl<T: Delayed> PartialOrd for Entry<T> {
    fn partial_cmp(&self, other: &Entry<T>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Delayed> PartialEq for Entry<T> {
    fn eq(&self, other: &Entry<T>) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T: Delayed> Eq for Entry<T> {}


#[cfg(test)]
mod tests {
    extern crate timebomb;

    use self::timebomb::timeout_ms;
    use std::time::{Duration, Instant};
    use std::thread;
    use delayed::Delay;
    use super::{DelayQueue, Entry};

    #[test]
    fn entry_comparisons() {
        let delayed_one_hour = Entry::new(Delay::for_duration("abc", Duration::from_secs(3600)));
        let delayed_now = Entry::new(Delay::for_duration("def", Duration::from_secs(0)));

        assert_eq!(delayed_now, delayed_now);
        assert_ne!(delayed_now, delayed_one_hour);

        assert!(delayed_now > delayed_one_hour);
        assert!(delayed_one_hour < delayed_now);
        assert!(delayed_one_hour <= delayed_one_hour);
    }

    #[test]
    fn is_empty() {
        let mut queue = DelayQueue::new();

        assert!(queue.is_empty());

        queue.push(Delay::until_instant("1st", Instant::now()));

        assert!(!queue.is_empty());
        assert_eq!(queue.pop().value, "1st");
        assert!(queue.is_empty());
    }

    #[test]
    fn push_pop_single_thread() {
        let mut queue = DelayQueue::new();

        let delay1 = Delay::until_instant("1st", Instant::now());
        let delay2 = Delay::for_duration("2nd", Duration::from_millis(20));
        let delay3 = Delay::for_duration("3rd", Duration::from_millis(30));
        let delay4 = Delay::for_duration("4th", Duration::from_millis(40));

        queue.push(delay2);
        queue.push(delay4);
        queue.push(delay1);

        assert_eq!(queue.pop().value, "1st");
        assert_eq!(queue.pop().value, "2nd");

        queue.push(delay3);

        assert_eq!(queue.pop().value, "3rd");
        assert_eq!(queue.pop().value, "4th");

        assert!(queue.is_empty());
    }

    #[test]
    fn push_pop_different_thread() {
        let mut queue = DelayQueue::new();

        let delay1 = Delay::until_instant("1st", Instant::now());
        let delay2 = Delay::for_duration("2nd", Duration::from_millis(20));
        let delay3 = Delay::for_duration("3rd", Duration::from_millis(30));
        let delay4 = Delay::for_duration("4th", Duration::from_millis(40));

        queue.push(delay2);
        queue.push(delay3);
        queue.push(delay1);

        let mut cloned_queue = queue.clone();

        let handle = thread::spawn(move || {
            assert_eq!(cloned_queue.pop().value, "1st");
            assert_eq!(cloned_queue.pop().value, "2nd");
            assert_eq!(cloned_queue.pop().value, "3rd");
            assert_eq!(cloned_queue.pop().value, "4th");
            assert!(cloned_queue.is_empty());
        });

        queue.push(delay4);

        handle.join().unwrap();

        assert!(queue.is_empty());
    }

    #[test]
    fn pop_before_push() {
        timeout_ms(
            || {
                let mut queue: DelayQueue<Delay<&str>> = DelayQueue::new();

                let mut cloned_queue = queue.clone();

                let handle = thread::spawn(move || {
                    assert_eq!(cloned_queue.pop().value, "1st");
                    assert!(cloned_queue.is_empty());
                });

                thread::sleep(Duration::from_millis(100));
                queue.push(Delay::for_duration("1st", Duration::from_millis(10)));

                handle.join().unwrap();

                assert!(queue.is_empty());
            },
            1000,
        );
    }

    #[test]
    fn pop_two_before_push() {
        timeout_ms(
            || {
                let mut queue: DelayQueue<Delay<&str>> = DelayQueue::new();
                let mut handles = vec![];

                for _ in 0..3 {
                    let mut queue = queue.clone();
                    let handle = thread::spawn(move || {
                        let val = queue.pop().value;
                        if val == "3rd" {
                            assert!(queue.is_empty());
                        }
                    });
                    handles.push(handle);
                }

                thread::sleep(Duration::from_millis(100));
                queue.push(Delay::for_duration("1st", Duration::from_millis(10)));
                queue.push(Delay::for_duration("2nd", Duration::from_millis(20)));
                queue.push(Delay::for_duration("3rd", Duration::from_millis(30)));

                for handle in handles {
                    handle.join().unwrap();
                }

                assert!(queue.is_empty());
            },
            1000,
        );
    }

    #[test]
    fn push_higher_priority_while_waiting_to_pop() {
        timeout_ms(
            || {
                let mut queue: DelayQueue<Delay<&str>> = DelayQueue::new();

                let delay1 = Delay::until_instant("1st", Instant::now());
                let delay2 = Delay::for_duration("2nd", Duration::from_millis(100));

                let mut cloned_queue = queue.clone();

                let handle = thread::spawn(move || {
                    assert_eq!(cloned_queue.pop().value, "1st");
                    assert_eq!(cloned_queue.pop().value, "2nd");
                    assert!(cloned_queue.is_empty());
                });

                thread::sleep(Duration::from_millis(10));
                queue.push(delay2);
                thread::sleep(Duration::from_millis(10));
                queue.push(delay1);

                handle.join().unwrap();

                assert!(queue.is_empty());
            },
            1000,
        );
    }
}
