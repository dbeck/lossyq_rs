use std::sync::atomic::{AtomicUsize, Ordering};

struct CircularBuffer<T : Copy> {
  seqno       : AtomicUsize,        // the ID of the last written item
  data        : Vec<T>,             // (2*n)+1 preallocated elements
  size        : usize,              // n

  buffer      : Vec<AtomicUsize>,   // (positions+seqno)[]
  read_priv   : Vec<usize>,         // positions belong to the reader
  write_tmp   : usize,              // temporary position where the writer writes first
  max_read    : usize,              // reader's last read seqno
}

pub struct CircularBufferIterator<'a, T: 'a + Copy> {
  data   : &'a [T],
  revpos : &'a [usize],
  start  : usize,
  count  : usize,
}

pub trait IterRange {
  fn get_range(&self) -> (usize, usize);
  fn next_id(&self) -> Option<usize>;
}

impl <T : Copy> CircularBuffer<T> {
  fn new(size : usize, default_value : T) -> CircularBuffer<T> {

    let mut size = size;

    // size of multiples of 64k won't interact well with the writer's flags
    if size%(64*1024) == 0 || size == 0 { size += 1; }

    let mut ret = CircularBuffer {
      seqno      : AtomicUsize::new(0),
      data       : vec![],
      size       : size,
      buffer     : vec![],
      read_priv  : vec![],
      write_tmp  : 0,
      max_read   : 0,
    };

    // make sure there is enough place and fill it with the
    // default value
    ret.data.resize((size*2)+1, default_value);

    for i in 0..size {
      ret.buffer.push(AtomicUsize::new((1+i) << 16));
      ret.read_priv.push(1+size+i);
    }

    ret
  }

  fn put<F>(&mut self, setter: F) -> usize
    where F : FnMut(&mut T)
  {
    let mut setter = setter;

    // get a reference to the data
    let mut opt : Option<&mut T> = self.data.get_mut(self.write_tmp);

    // write the data to the temporary writer buffer
    match opt.as_mut() {
      Some(v) => setter(v),
      None => {
        // this cannot happen under any normal circumstances, so I just silently ignore it
        println!("dodgy thing 1");
        return self.seqno.load(Ordering::SeqCst);
      }
    }

    // calculate writer flag position
    let seqno  = self.seqno.load(Ordering::SeqCst);
    let pos    = seqno % self.size;

    // get a reference to the writer flag
    match self.buffer.get_mut(pos) {
      Some(v) => {
        let mut old_flag : usize = (*v).load(Ordering::SeqCst);
        let mut old_pos  : usize = old_flag >> 16;
        let new_flag     : usize = (self.write_tmp << 16) + (seqno & 0xffff);

        loop {
          let result = (*v).compare_and_swap(old_flag,
                                             new_flag,
                                             Ordering::SeqCst);
          if result == old_flag {
            self.write_tmp = old_pos;
            break;
          } else {
            old_flag = result;
            old_pos  = old_flag >> 16;
          };
        };
      },
      None => {
        // this cannot happen under normal circumstances so just ignore it
        println!("dodgy thing 2");
      }
    }

    // increase sequence number
    self.seqno.fetch_add(1, Ordering::SeqCst)
  }

  fn iter(&mut self) -> CircularBufferIterator<T> {
    let mut seqno : usize = self.seqno.load(Ordering::SeqCst);
    let mut count : usize = 0;
    let max_read : usize = self.max_read;
    self.max_read = seqno;

    loop {
      if count >= self.size || seqno <= max_read || seqno == 0 { break; }
      let pos = (seqno-1) % self.size;

      match self.read_priv.get_mut(count) {
        Some(r) => {
          match self.buffer.get_mut(pos) {
            Some(v) => {
              let old_flag : usize = (*v).load(Ordering::SeqCst);
              let old_pos  : usize = old_flag >> 16;
              let old_seq  : usize = old_flag & 0xffff;
              let chk_flag : usize = (old_pos << 16) + ((seqno-1) & 0xffff);
              let new_flag : usize = (*r << 16) + (old_seq & 0xffff);

              if chk_flag == (*v).compare_and_swap(chk_flag, new_flag, Ordering::SeqCst) {
                *r = old_pos;
                seqno -=1;
                count += 1;
              } else {
                break;
              }
            },
            None => {
              // this cannot happen under any normal circumstances so just ignore it, but won't continue
              println!("dodgy thing 3");
              break;
            }
          }
        },
        None => {
          // this cannot happen under any normal circumstances so just ignore it, but won't continue
          println!("dodgy thing 4");
          break;
        }
      }
    }

    CircularBufferIterator {
      data    : self.data.as_slice(),
      revpos  : self.read_priv.as_slice(),
      start   : seqno,
      count   : count,
    }
  }
}

impl <'_, T: '_ + Copy> Iterator for CircularBufferIterator<'_, T> {
  type Item = T;

  fn next(&mut self) -> Option<T> {
    if self.count > 0 {
      self.count -= 1;
      self.start += 1;
      let pos : usize = self.revpos[self.count];
      Some(self.data[pos])
    } else {
      None
    }
  }
}

impl <'_, T: '_ + Copy> IterRange for CircularBufferIterator<'_, T> {
  fn get_range(&self) -> (usize, usize) {
    (self.start, self.start+self.count)
  }
  fn next_id(&self) -> Option<usize> {
    if self.count > 0 {
      Some(self.start)
    } else {
      None
    }
  }
}

use std::cell::UnsafeCell;
use std::sync::Arc;

pub struct Sender<T: Copy> {
  inner: Arc<UnsafeCell<CircularBuffer<T>>>,
}

unsafe impl<T: Copy> Send for Sender<T> { }

pub struct Receiver<T: Copy> {
  inner: Arc<UnsafeCell<CircularBuffer<T>>>,
}

unsafe impl<T: Copy> Send for Receiver<T> { }

pub fn channel<T: Copy + Send>(size : usize,
                               default_value : T) -> (Sender<T>, Receiver<T>) {
    let a = Arc::new(UnsafeCell::new(CircularBuffer::new(size, default_value)));
    (Sender::new(a.clone()), Receiver::new(a))
}

impl<T: Copy + Send> Sender<T> {
  fn new(inner: Arc<UnsafeCell<CircularBuffer<T>>>) -> Sender<T> {
    Sender { inner: inner, }
  }

  pub fn put<F>(&mut self, setter: F) -> usize
    where F : FnMut(&mut T)
  {
    unsafe { (*self.inner.get()).put(setter) }
  }
}

impl<T: Copy + Send> Receiver<T> {
  fn new(inner: Arc<UnsafeCell<CircularBuffer<T>>>) -> Receiver<T> {
    Receiver { inner: inner, }
  }

  pub fn iter(&mut self) -> CircularBufferIterator<T> {
    unsafe { (*self.inner.get()).iter() }
  }
}

#[cfg(test)]
mod tests {
  use super::CircularBuffer;

  #[test]
  fn empty_buffer() {
    use spsc::IterRange;

    let mut x = CircularBuffer::new(1, 0 as i32);
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
    use spsc::IterRange;
    let mut x = CircularBuffer::new(10, 0 as i32);
    {
      let pos = x.put(|v| *v = 1);
      assert_eq!(pos, 0);
      let i = x.iter();
      let (from,to) = i.get_range();
      assert_eq!(from, 0);
      assert_eq!(to, 1);
      assert_eq!(Some(0), i.next_id());
    }
    {
      x.put(|v| *v = 2);
      x.put(|v| *v = 3);
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
      x.put(|v| *v = 4);
      let pos = x.put(|v| *v = 5);
      assert_eq!(pos, 4);
      let i = x.iter();
      assert_eq!(i.count, 2);
      assert_eq!(i.start, 3);
    }
  }

  #[test]
  fn sum_available() {
    let mut x = CircularBuffer::new(4, 0 as i32);
    x.put(|v| *v = 2);
    x.put(|v| *v = 4);
    x.put(|v| *v = 6);
    x.put(|v| *v = 8);
    x.put(|v| *v = 10);
    let sum = x.iter().take(3).fold(0, |acc, num| acc + num);
    assert_eq!(sum, 18);
  }

  #[test]
  fn overload_buffer() {
    let mut x = CircularBuffer::new(2, 0 as i32);
    x.put(|v| *v = 1);
    x.put(|v| *v = 2);
    x.put(|v| *v = 3);
    assert_eq!(x.iter().count(), 2);
  }

  #[test]
  fn read_twice() {
    let mut x = CircularBuffer::new(2, 0 as i32);
    x.put(|v| *v = 1);
    assert_eq!(x.iter().count(), 1);
    assert_eq!(x.iter().count(), 0);
    x.put(|v| *v = 2);
    x.put(|v| *v = 3);
    assert_eq!(x.iter().count(), 2);
    assert_eq!(x.iter().count(), 0);
  }
}
