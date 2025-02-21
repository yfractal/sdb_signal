const ISEQS_BUFFER_SIZE: usize = 100_000;

pub struct Buffer {
  buffer: [u64; ISEQS_BUFFER_SIZE],
  buffer_size: usize,
  buffer_index: usize,
}

impl Buffer {
  pub fn new() -> Self {

      Buffer {
          buffer: [0; ISEQS_BUFFER_SIZE],
          buffer_size: ISEQS_BUFFER_SIZE,
          buffer_index: 0
      }
  }

  #[inline]
  pub fn push(&mut self, item: u64) {
      if self.buffer_index < self.buffer_size {
          self.buffer[self.buffer_index] = item;
          self.buffer_index += 1;
      } else {
          self.buffer_index = 0;
      }
  }

  #[inline]
  pub fn push_seperator(&mut self) {
      self.push(u64::MAX);
      self.push(u64::MAX);
  }
}
