use crate::Price;

#[derive(Clone, Debug)]
pub(crate) struct RingBuffer {
    buffer: Vec<Price>,
    head: usize,
    tail: usize,
    len: usize,
}

impl RingBuffer {
    #[must_use]
    pub(crate) fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    #[inline]
    pub(crate) fn is_ready(&self) -> bool {
        self.len == self.buffer.len()
    }

    #[inline]
    pub(crate) fn push(&mut self, value: Price) -> Option<Price> {
        if self.is_ready() {
            let old = self.buffer[self.head];

            self.buffer[self.head] = value;

            self.tail = self.head;
            self.head += 1;
            if self.head == self.buffer.len() {
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

    #[inline]
    pub(crate) fn find_value_and_index<F>(&self, should_replace: F) -> (Price, usize)
    where
        F: Fn(Price, Price) -> bool,
    {
        let mut index = 0;
        let mut found = self.buffer[0];

        for i in 1..self.len {
            if should_replace(found, self.buffer[i]) {
                index = i;
                found = self.buffer[i];
            }
        }

        (
            found,
            (self.tail + self.buffer.len() - index) % self.buffer.len(),
        )
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

    mod find_value_and_index {
        use super::RingBuffer;

        #[test]
        fn find_indexed() {
            let mut rb = RingBuffer::new(3);
            rb.push(4.0);
            rb.push(2.0);
            rb.push(5.0);

            assert_eq!(rb.find_value_and_index(|a, b| a > b), (2.0, 1));
            assert_eq!(rb.find_value_and_index(|a, b| a < b), (5.0, 0));
        }

        #[test]
        fn find_indexed_after_wrap() {
            let mut rb = RingBuffer::new(3);
            rb.push(1.0);
            rb.push(2.0);
            rb.push(3.0);
            // Wrap: evict 1, buffer = [4, 2, 3], head=1, tail=0
            rb.push(4.0);

            // Max is 4.0 at tail (position 0 = newest)
            assert_eq!(rb.find_value_and_index(|a, b| a < b), (4.0, 0));
            // Min is 2.0 at buffer[1], one back from oldest
            assert_eq!(rb.find_value_and_index(|a, b| a > b), (2.0, 2));
        }

        #[test]
        fn find_indexed_after_multiple_wraps() {
            let mut rb = RingBuffer::new(3);
            for v in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 10.0] {
                rb.push(v);
            }
            // Buffer = [10, 5, 6] (logical oldest→newest: 5, 6, 10)
            // tail=0, head=1
            assert_eq!(rb.find_value_and_index(|a, b| a < b), (10.0, 0));
            assert_eq!(rb.find_value_and_index(|a, b| a > b), (5.0, 2));
        }

        #[test]
        fn find_indexed_all_equal() {
            let mut rb = RingBuffer::new(3);
            rb.push(7.0);
            rb.push(7.0);
            rb.push(7.0);

            // No predicate matches; stays at index 0 (oldest = position 2)
            let (val, pos) = rb.find_value_and_index(|a, b| a < b);
            assert_eq!(val, 7.0);
            assert_eq!(pos, 2);
        }
    }
}
