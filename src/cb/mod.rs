use std::sync::atomic::{AtomicUsize, Ordering};

pub struct CircularBuffer<T> {
  seqno       : AtomicUsize,        // the ID of the last written item
  seqno_priv  : usize,
  data        : Vec<Option<T>>,     // (2*n)+1 preallocated elements
  size        : usize,              // n

  buffer      : Vec<AtomicUsize>,   // all positions
  read_priv   : Vec<usize>,         // positions belong to the reader
  write_tmp   : usize,              // temporary position where the writer writes first
  max_read    : usize,              // reader's last read seqno
}

pub struct CircularBufferIterator<'a, T: 'a> {
  data   : &'a mut [Option<T>],
  revpos : &'a [usize],
  start  : usize,
  count  : usize,
}

pub trait IterRange {
  fn get_range(&self) -> (usize, usize);
  fn next_id(&self) -> Option<usize>;
}

impl <T> CircularBuffer<T> {
  pub fn new(size : usize) -> CircularBuffer<T> {

    let mut size = size;

    // size cannot be zero, silently set to one
    if size == 0 { size = 1; }

    let mut ret = CircularBuffer {
      seqno       : AtomicUsize::new(0),
      seqno_priv  : 0,
      data        : Vec::with_capacity(2*size+1),
      size        : size,
      buffer      : Vec::with_capacity(size),
      read_priv   : Vec::with_capacity(size),
      write_tmp   : 0,
      max_read    : 0,
    };

    // make sure there is enough place and fill it with the
    // default value (1+2*size)
    ret.data.push(None);

    for i in 0..size {
      ret.buffer.push(AtomicUsize::new(((1+i)<<4)+1));
      ret.read_priv.push(1+size+i);
      // 2*size
      ret.data.push(None);
      ret.data.push(None);
    }

    ret
  }

  #[inline(always)]
  pub fn seqno(&self) -> usize {
    self.seqno.load(Ordering::Acquire) >> 4
  }

  #[inline(always)]
  pub fn put<F>(&mut self, setter: F) -> usize
    where F : FnMut(&mut Option<T>)
  {
    let mut setter = setter;

    // get a reference to the data
    let mut opt : Option<&mut Option<T>> = self.data.get_mut(self.write_tmp);

    // calculate writer flag position
    let mut serial  = self.seqno_priv;
    let seqno       = serial >> 4;
    let pos         = seqno % self.size;

    if pos == 0 { serial += 1; }

    // write the data to the temporary writer buffer
    match opt.as_mut() {
      Some(v) => setter(v),
      None => {
        // this cannot happen under normal circumstances so the panic is only
        // left here to trigger crash during testing
        panic!(format!("write_tmp: {} is invalid. size is {}, seqno is {}, serial is {}",
          self.write_tmp, self.size, seqno, serial));
      }
    }

    // get a reference to the writer flag
    match self.buffer.get_mut(pos) {
      Some(v) => {
        let new_flag : usize = (self.write_tmp << 4) | (serial & 0xf);
        let result : usize = (*v).swap(new_flag, Ordering::AcqRel);
        self.write_tmp = result >> 4;
      },
      None => {
        // this cannot happen under normal circumstances so the panic is only
        // left here to trigger crash during testing
        panic!(format!("pos: {} is invalid. size is {}, seqno is {}, serial is {}",pos, self.size, seqno, serial));
      }
    }

    // increase sequence number and return the old one
    self.seqno_priv = ((seqno+1) << 4) | (serial&0xf);
    self.seqno.swap(self.seqno_priv, Ordering::AcqRel) >> 4
  }

  pub fn tmp<F>(&mut self, setter: F)
    where F : FnMut(&mut Option<T>)
  {
    let mut setter = setter;

    // get a reference to the data
    let mut opt : Option<&mut Option<T>> = self.data.get_mut(self.write_tmp);

    // write the data to the temporary writer buffer
    match opt.as_mut() {
      Some(v) => setter(v),
      None => {}
    }
  }

  #[inline(always)]
  pub fn iter(&mut self) -> CircularBufferIterator<T> {

    let mut serial : usize = self.seqno.load(Ordering::Acquire);
    let mut seqno : usize  = serial >> 4;
    let mut count : usize = 0;
    let max_read : usize = self.max_read;
    self.max_read = seqno;

    loop {
      if count >= self.size || seqno <= max_read || seqno == 0 { break; }
      let pos = (seqno-1) % self.size;

      // this could be optimized to be iterator based
      match self.read_priv.get_mut(count) {
        Some(r) => {
          match self.buffer.get_mut(pos) {
            Some(v) => {
              let old_flag : usize = (*v).load(Ordering::Acquire);

              // turned over?
              if old_flag&0xf != serial&0xf {
                break;
              }

              // now try to swap out
              let old_pos  : usize = old_flag >> 4;
              let chk_flag : usize = (old_pos << 4) | (serial & 0xf);
              let new_flag : usize = (*r << 4) | (serial & 0xf);

              if chk_flag == (*v).compare_and_swap(chk_flag, new_flag, Ordering::AcqRel) {
                *r = old_pos;
                seqno -=1;
                count += 1;
              } else {
                break;
              }
            },
            None => {
              // this cannot happen under any normal circumstances so just ignore it, but won't continue
              panic!(format!("pos: {} is invalid. size is {}, count is {}, seqno is {}, serial is {}",pos, self.size, count, seqno, serial));
            }
          }
        },
        None => {
          // this cannot happen under any normal circumstances so just ignore it, but won't continue
          panic!(format!("count: {} is invalid. size is {}, pos is {}, seqno is {}, serial is {}",count, self.size, pos, seqno, serial));
        }
      }

      if pos == 0 { serial -= 1; }
    }

    CircularBufferIterator {
      data    : self.data.as_mut_slice(),
      revpos  : self.read_priv.as_slice(),
      start   : seqno,
      count   : count,
    }
  }
}

impl <'a, T: 'a> Iterator for CircularBufferIterator<'a, T> {
  type Item = T;

  #[inline(always)]
  fn next(&mut self) -> Option<T> {
    use std::mem;
    if self.count > 0 {
      self.count -= 1;
      self.start += 1;
      let pos : usize = self.revpos[self.count];
      let mut ret : Option<T> = None;
      mem::swap(&mut ret, &mut self.data[pos]);
      ret
    } else {
      None
    }
  }
}

impl <'a, T: 'a> IterRange for CircularBufferIterator<'a, T> {

  #[inline(always)]
  fn get_range(&self) -> (usize, usize) {
    (self.start, self.start+self.count)
  }

  #[inline(always)]
  fn next_id(&self) -> Option<usize> {
    if self.count > 0 {
      Some(self.start)
    } else {
      None
    }
  }
}

#[cfg(test)]
pub mod tests;
