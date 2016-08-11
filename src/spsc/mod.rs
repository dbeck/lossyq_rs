pub mod noloss;

use std::cell::UnsafeCell;
use std::sync::Arc;
use super::cb::{CircularBuffer, CircularBufferIterator};

pub struct Sender<T> {
  inner: Arc<UnsafeCell<CircularBuffer<T>>>,
}

unsafe impl<T> Send for Sender<T> { }

pub struct Receiver<T> {
  inner: Arc<UnsafeCell<CircularBuffer<T>>>,
}

unsafe impl<T> Send for Receiver<T> { }

pub fn channel<T: Send>(size : usize) -> (Sender<T>, Receiver<T>) {
    let a = Arc::new(UnsafeCell::new(CircularBuffer::new(size)));
    (Sender::new(a.clone()), Receiver::new(a))
}

impl<T: Send> Sender<T> {
  fn new(inner: Arc<UnsafeCell<CircularBuffer<T>>>) -> Sender<T> {
    Sender { inner: inner, }
  }

  pub fn put<F>(&mut self, setter: F) -> usize
    where F : FnMut(&mut Option<T>)
  {
    unsafe { (*self.inner.get()).put(setter) }
  }

  pub fn tmp<F>(&mut self, setter: F)
    where F : FnMut(&mut Option<T>)
  {
    unsafe { (*self.inner.get()).tmp(setter) }
  }

  pub fn seqno(&self) -> usize
  {
    unsafe { (*self.inner.get()).seqno() }
  }
}

impl<T: Send> Receiver<T> {
  fn new(inner: Arc<UnsafeCell<CircularBuffer<T>>>) -> Receiver<T> {
    Receiver { inner: inner, }
  }

  pub fn iter(&mut self) -> CircularBufferIterator<T> {
    unsafe { (*self.inner.get()).iter() }
  }
}

#[cfg(test)]
pub mod tests;
