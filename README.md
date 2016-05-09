# lossyq for Rust

`lossyq` is a single-publisher, single-subscriber queue with certain characteristics:

- at the creation of the queue, you need to decide how big the queue need to be
- the queue will be allocated at creation time with the specified amount of elements (see notes below)
- when adding an element, the put closure will receive a reference to the element to be updated, so no allocation needed to add new elements
- when adding elements, the updater won't ever be blocked. it doesn't care about if there was a reader to read the element
- the reader maintains the last position read, and it will read the elements up to the last written position
- if the reader is slower then the writer, then it may not see elements that was written in the meanwhile

# Example

```rust
extern crate lossyq;

fn main() {
  use std::thread;

  // create a very small channel of 2 elements
  let (mut tx, mut rx) = lossyq::spsc::channel(2, 0 as i32);
  let t = thread::spawn(move|| {
    for i in 1..4 {
      // the tx.put() receives a lambda function that in turn
      // gets a writable reference to the next element in the queue
      tx.put(|v| *v = i);
    }
  });
  t.join().unwrap();
  // the receiver receives an iterator which may be
  // further passed to other adapters
  let sum = rx.iter().fold(0, |acc, num| acc + num);

  // this should print 5 as the writer sent three items to the
  // queue: [1,2,3] and the first item got overwritten by the
  // last one
  println!("sum={}",sum);
}
```

## Putting an element

As in the example above the `put` function receives a closure that in turn receives a mutable reference to an element in the queue. This way we never need to allocate memory on insertion.

## Reading elements

When reading, the `iter` function receives an iterator that has a reference to all readable elements at the moment. If the writer writes more elements to the queue, the iterator will still be valid, only that it won't see the newly written elements. To see them, a new iterator needs to be created by a new `iter` call.

# Rationale

Let me emphasize the fact that the reader may lose updates. I believe this is not a problem, only a certain property to live with. Other queue implementations choose to, either make the queue larger when it becomes full, or block the writer until the reader processed some from the queue. I think all of these are valid choices and they have consequences. When we allocate more memory for the queue, we might obviously run out of it, then we go swapping and the whole system is cursed. The other choice is when we block the writer, the writer performance is limited by the reader.

I think there are scenarios where we are more interested in the latest data than dealing with the out of memory or blocking conditions. An example is heartbeats. We might not care about losing old heartbeats as long as we know the given component was alive few seconds ago.

The fact that this queue doesn't allocate memory, makes its performance predictable and probably fast. (I made no measurements whatsoever, on purpose.)

When you want to minimize the chance of losing items, you would need to choose a larger queue size. The right size depends on your application.

# Implementation notes

The heart of this queue is the `CircularBuffer` data structure. It uses atomic integer operations to make sure the writer and the reader can operate concurrently.

```rust
struct CircularBuffer<T : Copy> {
  seqno       : AtomicUsize,        // the ID of the last written item
  data        : Vec<T>,             // (2*n)+1 preallocated elements
  size        : usize,              // n

  buffer      : Vec<AtomicUsize>,   // (positions+seqno)[]
  read_priv   : Vec<usize>,         // positions belong to the reader
  write_tmp   : usize,              // temporary position where the writer writes first
  max_read    : usize,              // reader's last read seqno
}
```

## When writing

The `data` vector holds `2n+1` preallocated items. `n` items belong to the reader and `n+1` items belong to the writer. The ownership of who owns which elements are tracked by the `buffer`, `read_priv` and `write_tmp` members. The `buffer` vector represents the `CircularBuffer` where each element is composed of 16 bits of the `seqno` and the rest is a position to the `data` vector. The `write_tmp` element is also a position referring to the `data` vector. When the writer writes a new element:

- it writes to the `data` element pointed by `write_tmp`
- than it calculates `seqno` modulo `size`, which is the position in `buffer` which is going to be updated (new_pos)
- than `buffer[new_pos]` will be updated to hold `(write_tmp << 16) + (seqno % 0xffff)`
- finally `write_tmp` will be updated to the previous value of `buffer[old_pos] >> 16`
- (basically the positions of `write_tmp` and `buffer` will be swapped)

This design allows the writer to always write to a private area that is not touched by the reader and then it atomically swaps the `buffer[new_pos]` element over to the freshly written element. This allows writing without interfering with the reader.

## When reading

To read data one needs to obtain an iterator through the `iter()` function. This loops through the `buffer` in reverse order and atomically swaps the reader's own positions held by the `read_priv` vector with the position part of the `buffer` component. While looping it checks that the sequence number part of the `buffer` entry is the expected one. If not then it knows that the writer has flipped over, so the given element should be returned during the next iteration and it stops.

The result of this operation is that `read_priv` vector holds the pointers to the previously written elements and the reader gave its own elements to the writer in exchange, so the writer can write those, while the reader works with its own copies.

# License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2](LICENSE-APACHE) of your choice.
