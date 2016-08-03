pub mod cb;
pub mod spsc;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn option_i32() {
    let (mut tx, mut _rx) = spsc::channel(2);
    tx.put( |v| *v = Some(1) );
  }

  #[test]
  fn string_literal() {
    let (mut tx, mut _rx) = spsc::channel(2);
    tx.put( |v| *v = Some("world") );
  }

  #[test]
  fn boxed_string_obj() {
    let (mut tx, mut _rx) = spsc::channel(2);
    tx.put( |v| *v = Some(String::from("world")) );
  }

  #[test]
  fn with_spawn() {
    use std::thread;
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
    use std::thread;
    use cb::IterRange;

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
}
