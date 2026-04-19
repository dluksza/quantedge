use std::f64;

use crate::internals::RingBuffer;

#[derive(Clone, Debug)]
pub(crate) struct RollingMaxMin {
    oldest_pos: usize,
    highs: RingBuffer,
    lows: RingBuffer,
    max_val: f64,
    max_pos: usize,
    max_pos_prev: usize,
    min_val: f64,
    min_pos: usize,
    min_pos_prev: usize,
    forming_max: f64,
    forming_min: f64,
}

impl RollingMaxMin {
    pub(crate) fn new(length: usize) -> Self {
        Self {
            oldest_pos: length - 1,
            highs: RingBuffer::new(length),
            lows: RingBuffer::new(length),
            max_val: -1.0,
            max_pos: 0,
            max_pos_prev: 0,
            min_val: f64::MAX,
            min_pos: 0,
            min_pos_prev: 0,
            forming_max: -1.0,
            forming_min: f64::MAX,
        }
    }

    pub(crate) fn push(&mut self, value: f64) -> Option<(f64, f64)> {
        if self.max_val < 0.0 {
            self.max_val = value;
        }
        if self.min_val >= f64::MAX {
            self.min_val = value;
        }

        self.highs.push(value);

        if self.forming_max > self.max_val {
            self.max_val = self.forming_max;
            self.max_pos = 1;
        } else if self.max_pos == self.oldest_pos {
            (self.max_val, self.max_pos) = self.find_max();
        } else {
            self.max_pos += 1;
        }
        self.forming_max = value;
        self.max_pos_prev = self.max_pos;

        self.lows.push(value);

        if self.forming_min < self.min_val {
            self.min_val = self.forming_min;
            self.min_pos = 1;
        } else if self.min_pos == self.oldest_pos {
            (self.min_val, self.min_pos) = self.find_min();
        } else {
            self.min_pos += 1;
        }
        self.forming_min = value;
        self.min_pos_prev = self.min_pos;

        self.value()
    }

    pub(crate) fn replace(&mut self, value: f64) -> Option<(f64, f64)> {
        self.highs.replace(value);
        self.lows.replace(value);

        self.forming_max = value;
        if self.forming_max >= self.max_val {
            self.max_pos = 0;
        } else if self.max_pos_prev == 0 {
            (self.max_val, self.max_pos) = self.find_max();
            self.max_pos_prev = self.max_pos;
        } else {
            self.max_pos = self.max_pos_prev;
        }

        self.forming_min = value;
        if self.forming_min <= self.min_val {
            self.min_pos = 0;
        } else if self.min_pos_prev == 0 {
            (self.min_val, self.min_pos) = self.find_min();
            self.min_pos_prev = self.min_pos;
        } else {
            self.min_pos = self.min_pos_prev;
        }

        self.value()
    }

    fn find_max(&self) -> (f64, usize) {
        self.highs
            .find_value_and_index(|found, candidate| found < candidate)
    }

    fn find_min(&self) -> (f64, usize) {
        self.lows
            .find_value_and_index(|found, candidate| found > candidate)
    }

    fn value(&self) -> Option<(f64, f64)> {
        if self.highs.is_ready() {
            Some((self.max(), self.min()))
        } else {
            None
        }
    }

    fn max(&self) -> f64 {
        self.max_val.max(self.forming_max)
    }

    fn min(&self) -> f64 {
        self.min_val.min(self.forming_min)
    }
}
