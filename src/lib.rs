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
    let (from,to) = rx.iter().get_range();
    assert_eq!(from, 1);
    assert_eq!(to, 3);
  }

}
