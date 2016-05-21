pub mod spsc;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn with_spawn() {
    use std::thread;
    let (mut tx, mut rx) = spsc::channel(2, 0 as i32);
    let t = thread::spawn(move|| {
      for i in 1..4 {
        tx.put(|v| *v = i);
      }
    });
    t.join().unwrap();
    let sum = rx.iter().fold(0, |acc, num| acc + num);
    assert_eq!(sum, 5);
  }

  #[test]
  fn at_most_once() {
    let (mut tx, mut rx) = spsc::channel(20, 0 as i32);
    tx.put(|v| *v = 1);
    tx.put(|v| *v = 2);
    tx.put(|v| *v = 3);
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
    use std::thread;
    use spsc::IterRange;

    let (mut tx, mut rx) = spsc::channel(2, 0 as i32);
    let t = thread::spawn(move|| {
      for i in 1..4 {
        tx.put(|v| *v = i);
      }
    });
    t.join().unwrap();
    let i = rx.iter();
    let (from,to) = i.get_range();
    assert_eq!(from, 1);
    assert_eq!(to, 3);
    assert_eq!(Some(1),i.next_id());
  }
}
