use cb::*;

mod put_impl {
  use super::super::*;

  pub fn copy<T: Copy>(size: usize, count: usize, default: T) -> usize {
    let mut x = CircularBuffer::new(size);
    for _i in 0..count {
      x.put( |v| *v = Some(default));
    }
    x.iter().count()
  }

  pub fn clone<T: Clone>(size: usize, count: usize, default: T) -> usize {
    let mut x = CircularBuffer::new(size);
    for _i in 0..count {
      x.put( |v| *v = Some(default.clone()));
    }
    x.iter().count()
  }
}

#[test]
fn put_less_i32() {
  for i in 1..100 as usize {
    assert_eq!(i, put_impl::copy(i+1+(i/3),i,0 as i32));
  }
}

#[test]
fn put_full_str() {
  for i in 1..100 as usize {
    assert_eq!(i, put_impl::copy(i,i,"hello"));
  }
}

#[test]
fn put_overflow_box_string() {
  for i in 1..100 as usize {
    assert_eq!(i, put_impl::clone(i+1,i,String::from("hello")));
  }
}

#[test]
fn put_overflow_box_string_3() {
  use std::mem;
  let mut x = CircularBuffer::new(3);
  x.put(|v| {
    let mut other = Some(Box::new(String::from("foo")));
    mem::swap(&mut other, v);
    assert_eq!(true, other.is_none());
  });
  x.put(|v| {
    let mut other = Some(Box::new(String::from("bar")));
    mem::swap(&mut other, v);
    assert_eq!(true, other.is_none());
  });
  x.put(|v| {
    let mut other = Some(Box::new(String::from("baz")));
    mem::swap(&mut other, v);
    assert_eq!(true, other.is_none());
  });
  x.put(|v| {
    let mut other = Some(Box::new(String::from("faz")));
    mem::swap(&mut other, v);
    assert_eq!(true, other.is_none());
  });
  x.put(|v| {
    let mut other = Some(Box::new(String::from("foobar")));
    mem::swap(&mut other, v);
    assert_eq!(true, other.is_some());
  });
  assert_eq!(x.iter().count(), 3);
}

#[test]
fn iter_empty_i32_5() {
  let mut x = CircularBuffer::<i32>::new(5);
  assert_eq!(x.iter().count(), 0);
}

mod iter_impl {
  use super::super::*;

  pub fn copy<T: Copy>(size: usize, count: usize, iter: usize, default: T) -> usize {
    let mut x = CircularBuffer::new(size);
    for _i in 0..iter-1 {
      for _ii in 0..count {
        x.put( |v| *v = Some(default));
      }
      let _xi = x.iter().count();
    }
    for _i in 0..count {
      x.put( |v| *v = Some(default));
    }
    x.iter().count()
  }

  pub fn min_max(size: usize, count: i32, iter: usize, min: i32, max: i32) -> bool {
    let mut x = CircularBuffer::<i32>::new(size);
    let mut a = 0;
    for _i in 0..iter-1 {
      for _ii in 0..count {
        x.put( |v| *v = Some(a));
        a += 1;
      }
      let _xi = x.iter().count();
    }
    for _i in 0..count {
      x.put( |v| *v = Some(a));
      a += 1;
    }
    let mut r_min = min + max + a + 1;
    let mut r_max = min - count - 1;
    for i in x.iter() {
      if r_min > i { r_min = i; }
      if r_max < i { r_max = i; }
    }
    assert_eq!(r_min, min);
    assert_eq!(r_max, max);
    min == r_min && max == r_max
  }
}

#[test]
fn iter_less_i32() {
  for i in 1..100 as i32 {
    assert!(iter_impl::min_max((i+1) as usize, i, 13, (i*13)-i, (i*13)-1));
  }
}

#[test]
fn iter_less_str() {
  for i in 1..100 as usize {
    assert_eq!(i, iter_impl::copy(i+1,i,13,"hello"));
  }
}

#[test]
fn iter_less_str_7() {
  let mut x = CircularBuffer::<&str>::new(7);
  x.put(|v| { *v = Some("Hello") });
  x.put(|v| { *v = Some("World") });
  let c = x.iter().fold(String::new(), |mut acc,x| { acc.push_str(x); (acc) } );
  assert_eq!(c, "HelloWorld");
}

#[test]
fn iter_full_box_string() {
  let mut x = CircularBuffer::new(10);
  for _i in 0..10 {
    x.put(|v| {
      assert!( v.is_none() );
      *v = Some(Box::new(String::from("hello")));
    });
  }

  let mut c = 0;
  for i in x.iter() {
    assert_eq!(*i, "hello");
    c += 1;
  }
  assert_eq!(c, 10);
}

#[test]
fn iter_overflow_i32() {
  for i in 1..100 as i32 {
    let e_min = ((i+1)*17)-i;
    let e_max = ((i+1)*17)-1;
    assert!(iter_impl::min_max(i as usize, i+1, 17, e_min, e_max));
  }
}

#[test]
fn tmp_and_put_i32() {
  let mut x = CircularBuffer::new(1);
  x.put(|v| {
    assert!(v.is_none());
    *v = Some(1);
  });
  x.tmp(|v| assert!(v.is_none()) );
  x.put(|v| {
    assert!(v.is_none());
    *v = Some(2);
  });
  x.tmp(|v| assert!(v.is_some()) );
  x.put(|v| {
    assert!(v.is_some());
    assert_eq!(*v, Some(1));
    *v = Some(3);
  });
  x.tmp(|v| assert!(v.is_some()) );
}

// get_range_empty : Box<string> (17)
// get_range_less : ptr (4)
// get_range_full : Vec<i32> (6)
// get_range_overflow : f32 (8)
// get_range_repeat : &str (10)

// tmp_full
// tmp_overflow

// lossless
// put_none

#[test]
fn empty_buffer() {
  use cb::IterRange;

  let mut x = CircularBuffer::<i32>::new(1);
  {
    assert_eq!(x.iter().count(), 0);
  }
  {
    let i = x.iter();
    let (from,to) = i.get_range();
    assert_eq!(from,0);
    assert_eq!(to,0);
    assert_eq!(None, i.next_id());
  }
}

#[test]
fn add_one() {
  use cb::IterRange;
  let mut x = CircularBuffer::new(10);
  {
    let pos = x.put(|v| *v = Some(1));
    assert_eq!(pos, 0);
    let i = x.iter();
    let (from,to) = i.get_range();
    assert_eq!(from, 0);
    assert_eq!(to, 1);
    assert_eq!(Some(0), i.next_id());
  }
  {
    x.put(|v| *v = Some(2));
    x.put(|v| *v = Some(3));
    let mut i = x.iter();
    let (from, to) = i.get_range();
    assert_eq!(from, 1);
    assert_eq!(to, 3);
    assert_eq!(Some(1), i.next_id());
    i.next();
    let (from,to) = i.get_range();
    assert_eq!(from, 2);
    assert_eq!(to, 3);
    assert_eq!(Some(2), i.next_id());
  }
  {
    x.put(|v| *v = Some(4));
    let pos = x.put(|v| *v = Some(5));
    assert_eq!(pos, 4);
    let i = x.iter();
    assert_eq!(i.count, 2);
    assert_eq!(i.start, 3);
  }
}

#[test]
fn sum_available() {
  let mut x = CircularBuffer::new(4);
  x.put(|v| *v = Some(2));
  x.put(|v| *v = Some(4));
  x.put(|v| *v = Some(6));
  x.put(|v| *v = Some(8));
  x.put(|v| *v = Some(10));
  let sum = x.iter().take(3).fold(0, |acc, num| acc + num);
  assert_eq!(sum, 18);
}

#[test]
fn read_twice() {
  let mut x = CircularBuffer::new(2);
  x.put(|v| *v = Some(1));
  assert_eq!(x.iter().count(), 1);
  assert_eq!(x.iter().count(), 0);
  x.put(|v| *v = Some(2));
  x.put(|v| *v = Some(3));
  assert_eq!(x.iter().count(), 2);
  assert_eq!(x.iter().count(), 0);
}
