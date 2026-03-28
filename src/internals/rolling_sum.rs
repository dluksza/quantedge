use crate::internals::RingBuffer;

#[derive(Clone, Debug)]
pub(crate) struct RollingSum {
    buffer: RingBuffer,
    sum: f64,
}

impl RollingSum {
    pub(crate) fn new(length: usize) -> Self {
        Self {
            buffer: RingBuffer::new(length),
            sum: 0.0,
        }
    }

    pub(crate) fn push(&mut self, value: f64) -> Option<f64> {
        if let Some(old) = self.buffer.push(value) {
            self.sum = self.sum - old + value;

            Some(self.sum)
        } else {
            self.sum += value;

            None
        }
    }

    pub(crate) fn replace(&mut self, value: f64) -> Option<f64> {
        let old = self.buffer.replace(value);
        self.sum = self.sum - old + value;

        self.sum()
    }

    #[must_use]
    pub(crate) fn sum(&self) -> Option<f64> {
        if self.buffer.is_ready() {
            Some(self.sum)
        } else {
            None
        }
    }
}
