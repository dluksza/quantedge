use crate::Price;

#[derive(Clone, Debug)]
pub(crate) struct RingBuffer {
    buffer: Vec<Price>,
    head: usize,
    tail: usize,
    len: usize,
    capacity: usize,
}

impl RingBuffer {
    #[must_use]
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            head: 0,
            tail: 0,
            len: 0,
            capacity,
        }
    }

    #[inline]
    pub(crate) fn is_ready(&self) -> bool {
        self.len == self.capacity
    }

    #[inline]
    pub(crate) fn push(&mut self, value: Price) -> Option<Price> {
        if self.is_ready() {
            let old = self.buffer[self.head];

            self.buffer[self.head] = value;

            self.tail = self.head;
            self.head += 1;
            if self.head == self.capacity {
                self.head = 0;
            }

            Some(old)
        } else {
            self.buffer[self.len] = value;
            self.tail = self.len;
            self.len += 1;

            None
        }
    }

    #[inline]
    pub(crate) fn replace(&mut self, value: Price) -> Price {
        let old = self.buffer[self.tail];

        self.buffer[self.tail] = value;

        old
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::RingBuffer;

    #[test]
    fn filling_returns_none() {
        let mut rb = RingBuffer::new(3);
        assert_eq!(rb.push(1.0), None);
        assert_eq!(rb.push(2.0), None);
        assert_eq!(rb.push(3.0), None);
        assert!(rb.is_ready());
    }

    #[test]
    fn full_evicts_oldest() {
        let mut rb = RingBuffer::new(3);
        rb.push(1.0);
        rb.push(2.0);
        rb.push(3.0);
        assert_eq!(rb.push(4.0), Some(1.0));
        assert_eq!(rb.push(5.0), Some(2.0));
        assert_eq!(rb.push(6.0), Some(3.0));
    }

    #[test]
    fn replace_swaps_latest() {
        let mut rb = RingBuffer::new(3);
        rb.push(1.0);
        rb.push(2.0);
        assert_eq!(rb.replace(9.0), 2.0);
        // Next push should still work correctly
        rb.push(3.0);
        assert_eq!(rb.push(4.0), Some(1.0));
        assert_eq!(rb.push(5.0), Some(9.0)); // replaced value
    }

    #[test]
    fn replace_when_full() {
        let mut rb = RingBuffer::new(2);
        rb.push(1.0);
        rb.push(2.0);
        assert_eq!(rb.replace(9.0), 2.0);
        assert_eq!(rb.push(3.0), Some(1.0));
        assert_eq!(rb.push(4.0), Some(9.0));
    }

    #[test]
    fn capacity_one() {
        let mut rb = RingBuffer::new(1);
        assert_eq!(rb.push(1.0), None);
        assert!(rb.is_ready());
        assert_eq!(rb.push(2.0), Some(1.0));
        assert_eq!(rb.replace(9.0), 2.0);
        assert_eq!(rb.push(3.0), Some(9.0));
    }

    #[test]
    fn wrap_around_correctness() {
        let mut rb = RingBuffer::new(2);
        rb.push(1.0);
        rb.push(2.0);
        rb.push(3.0); // evicts 1, head wraps
        rb.push(4.0); // evicts 2
        rb.push(5.0); // evicts 3, head wraps again
        assert_eq!(rb.push(6.0), Some(4.0));
    }
}
