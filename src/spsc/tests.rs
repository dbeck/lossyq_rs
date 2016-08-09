use spsc;
use std::mem;
use std::thread;
use cb::IterRange;
use time;

#[test]
fn option_i32() {
  // test the interface supports i32 datatype
  let (mut tx, mut _rx) = spsc::channel(2);
  tx.put( |v| *v = Some(1) );
}

#[test]
fn string_literal() {
  // test the interface supports string literals
  let (mut tx, mut _rx) = spsc::channel(2);
  tx.put( |v| *v = Some("world") );
}

#[test]
fn boxed_string_obj() {
  // test the interface supports boxed strings
  let (mut tx, mut _rx) = spsc::channel(2);
  tx.put( |v| *v = Some(Box::new(String::from("world"))) );
}

#[test]
fn no_loss() {
  // how to prevent overwrite
  // passing 1_000_000 values without loss
  let (mut tx, mut rx) = spsc::channel::<i32>(2);
  let t = thread::spawn(move|| {
    for i in 0..1_000_000i32 {
      let mut x = Some(i);
      loop {
        tx.put(|v| mem::swap(v, &mut x));
        // check if we swapped out a valid value, if yes
        // we need to put it back in the next loop until we are
        // not replacing an empty one
        if x.is_none() { break; }
      }
    }
    let mut x : Option<i32> = None;
    loop {
      // check the writer tmp area is empty. if not we will need to
      // add it to the active items
      tx.tmp(|v| mem::swap(v, &mut x));
      if x.is_none() { break; }
      loop {
        tx.put(|v| mem::swap(v, &mut x));
        if x.is_none() { break; }
      }
    }
  });

  let started_at = time::precise_time_s();
  let mut recvd = vec![];
  loop {
    if recvd.len() == 1_000_000 { break; }
    for i in rx.iter() {
      recvd.push(i);
    }
    if (started_at+20.0) < time::precise_time_s() {
      break;
    }
  }
  assert_eq!(recvd.len(), 1_000_000);
  t.join().unwrap();
}

#[test]
fn with_spawn() {
  // put a few objects in one thread and get them in the main
  let (mut tx, mut rx) = spsc::channel(2);
  let t = thread::spawn(move|| {
    for i in 1..4 {
      tx.put(|v| *v = Some(i));
    }
  });
  t.join().unwrap();
  let sum = rx.iter().fold(0, |acc, num| acc + num);
  assert_eq!(sum, 5);
}

#[test]
fn at_most_once() {
  // when an iterator gets a range of objects, it removes all from
  // the queue
  let (mut tx, mut rx) = spsc::channel(20);
  tx.put(|v| *v = Some(1));
  tx.put(|v| *v = Some(2));
  tx.put(|v| *v = Some(3));
  {
    // first iterator processes the single element
    // assumes I process everything else in the iterator
    let mut it = rx.iter();
    assert_eq!(Some(1), it.next());
  }
  {
    // the second iterator gets nothing, since the first
    // iterator received the whole range no matter if it
    // has really called a next on them ot not
    let mut it = rx.iter();
    assert_eq!(None, it.next());
  }
}

#[test]
fn range() {
  // test how iterator range works. it allows testing
  // how many objects are in the iterator and also what their
  // ids are.

  let (mut tx, mut rx) = spsc::channel(2);
  let t = thread::spawn(move|| {
    for i in 1..4 {
      tx.put(|v| *v = Some(i));
    }
  });
  t.join().unwrap();
  let i = rx.iter();
  let (from,to) = i.get_range();
  assert_eq!(from, 1);
  assert_eq!(to, 3);
  assert_eq!(Some(1),i.next_id());
}
