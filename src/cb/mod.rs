use std::sync::atomic::{AtomicUsize, Ordering};

pub struct CircularBuffer<T> {
  seqno       : AtomicUsize,        // the ID of the last written item
  data        : Vec<Option<T>>,     // (2*n)+1 preallocated elements
  size        : usize,              // n
  modulo      : usize,              // makes the buffer indices unique

  buffer      : Vec<AtomicUsize>,   // (positions+seqno)[]
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
    let mut modulo = size+1;

    // find a smaller modulo number
    for i in 3..16 {
      if size % i != 0 {
        modulo = i;
        break;
      }
    }

    let mut ret = CircularBuffer {
      seqno      : AtomicUsize::new(0),
      data       : vec![],
      size       : size,
      modulo     : modulo,
      buffer     : vec![],
      read_priv  : vec![],
      write_tmp  : 0,
      max_read   : 0,
    };

    // make sure there is enough place and fill it with the
    // default value (1+2*size)
    ret.data.push(None);

    for i in 0..size {
      ret.buffer.push(AtomicUsize::new((1+i) << 4));
      ret.read_priv.push(1+size+i);
      // 2*size
      ret.data.push(None);
      ret.data.push(None);
    }

    ret
  }

  pub fn put<F>(&mut self, setter: F) -> usize
    where F : FnMut(&mut Option<T>)
  {
    let mut setter = setter;

    // get a reference to the data
    let mut opt : Option<&mut Option<T>> = self.data.get_mut(self.write_tmp);

    // write the data to the temporary writer buffer
    match opt.as_mut() {
      Some(v) => setter(v),
      None => {
        // this cannot happen under any normal circumstances, so I just silently ignore it
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
        let mut old_pos  : usize = old_flag >> 4;
        let new_flag     : usize = (self.write_tmp << 4) + ((seqno%self.modulo) & 0xf);

        loop {
          let result = (*v).compare_and_swap(old_flag,
                                             new_flag,
                                             Ordering::SeqCst);
          if result == old_flag {
            self.write_tmp = old_pos;
            break;
          } else {
            old_flag = result;
            old_pos  = old_flag >> 4;
          };
        };
      },
      None => {
        // this cannot happen under normal circumstances so just ignore it
      }
    }

    // increase sequence number
    self.seqno.fetch_add(1, Ordering::SeqCst)
  }

  pub fn iter(&mut self) -> CircularBufferIterator<T> {
    let mut seqno : usize = self.seqno.load(Ordering::SeqCst);
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
              let old_flag : usize = (*v).load(Ordering::SeqCst);
              let old_pos  : usize = old_flag >> 4;
              let old_mod  : usize = old_flag & 0xf;
              let chk_flag : usize = (old_pos << 4) + (((seqno-1)%self.modulo) & 0xf);
              let new_flag : usize = (*r << 4) + old_mod;

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
      data    : self.data.as_mut_slice(),
      revpos  : self.read_priv.as_slice(),
      start   : seqno,
      count   : count,
    }
  }
}

impl <'_, T: '_> Iterator for CircularBufferIterator<'_, T> {
  type Item = T;

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

impl <'_, T: '_> IterRange for CircularBufferIterator<'_, T> {
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

#[cfg(test)]
pub mod tests;
