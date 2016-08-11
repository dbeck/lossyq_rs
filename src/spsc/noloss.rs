use super::Sender;
use std::mem;

pub trait Overflow {
  type Input : Send;
  fn overflow(&mut self, val : &mut Option<Self::Input>);
}

pub enum PourResult {
  Poured,
  Overflowed
}

pub fn pour<T: Send>(value: &mut Option<T>,
                     destination: &mut Sender<T>,
                     overflow: &mut Overflow<Input=T>)
    -> (PourResult, usize) {
  //
  let result = destination.put(|old_value| mem::swap(value, old_value));

  // the old_value should have been None, thus after the swap
  // value is to be None

  if value.is_some() {
    panic!("value unexpectedly overwritten");
  }

  // check the content of the write buffer, and if there is anything
  // save it in the overflow buffer

  let mut none : Option<T> = None;
  destination.tmp(|write_tmp| mem::swap(&mut none, write_tmp));
  if none.is_some() {
    overflow.overflow(&mut none);
    (PourResult::Overflowed, result)
  } else {
    (PourResult::Poured, result)
  }
}
