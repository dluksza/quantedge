use std::hint::cold_path;

use crate::Price;

#[derive(Clone, Debug)]
pub(crate) struct EmaCore {
    length: usize,
    value: Price,
    previous_value: Price,
    current_price: Price,
    sma_sum: f64,
    seen_bars: usize,
    converged: bool,
    alpha: f64,
    length_reciprocal: f64,
}

impl EmaCore {
    pub(crate) fn bars_to_converge(length: usize) -> usize {
        3 * (length + 1)
    }

    pub(crate) fn new(length: usize) -> Self {
        #[allow(clippy::cast_precision_loss)]
        let alpha = 2.0 / (length + 1) as f64;

        Self::with_alpha(length, alpha)
    }

    pub(crate) fn with_alpha(length: usize, alpha: f64) -> Self {
        Self {
            length,
            value: 0.0,
            current_price: 0.0,
            previous_value: 0.0,
            seen_bars: 0,
            sma_sum: 0.0,
            converged: false,
            alpha,
            #[allow(clippy::cast_precision_loss)]
            length_reciprocal: 1.0 / length as f64,
        }
    }

    pub(crate) fn push(&mut self, price: Price) -> Option<Price> {
        if self.converged {
            if self.seen_bars == self.length {
                self.seen_bars = self.length + 1;
            }

            self.previous_value = self.value;
            self.value = self
                .alpha
                .mul_add(price - self.previous_value, self.previous_value);
            return Some(self.value);
        }

        cold_path();

        if self.seen_bars < self.length {
            self.current_price = price;
            self.sma_sum += price;
            self.seen_bars += 1;

            if self.seen_bars == self.length {
                self.value = self.sma_sum * self.length_reciprocal;
            } else {
                return None;
            }
        } else {
            self.seen_bars += 1;
            self.previous_value = self.value;

            self.value = self
                .alpha
                .mul_add(price - self.previous_value, self.previous_value);
        }

        self.converged = self.seen_bars >= self.length;

        self.value()
    }

    pub(crate) fn replace(&mut self, price: Price) -> Option<Price> {
        if self.seen_bars <= self.length {
            cold_path();
            self.sma_sum = self.sma_sum - self.current_price + price;
            self.current_price = price;

            if self.seen_bars == self.length {
                self.value = self.sma_sum * self.length_reciprocal;
            } else {
                return None;
            }
        } else {
            self.value = self
                .alpha
                .mul_add(price - self.previous_value, self.previous_value);
        }

        self.value()
    }

    pub(crate) fn value(&self) -> Option<Price> {
        if self.converged {
            Some(self.value)
        } else {
            cold_path();
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_ema(length: usize) -> EmaCore {
        EmaCore::new(length)
    }

    mod seeding {
        use super::*;

        #[test]
        fn none_before_seed_complete() {
            let mut ema = raw_ema(3);
            assert_eq!(ema.push(10.0), None);
            assert_eq!(ema.push(20.0), None);
        }

        #[test]
        fn first_value_is_sma() {
            let mut ema = raw_ema(3);
            ema.push(10.0);
            ema.push(20.0);
            // (10 + 20 + 30) / 3 = 20.0
            assert_eq!(ema.push(30.0), Some(20.0));
        }
        #[test]
        fn length_one_immediate() {
            let mut ema = raw_ema(1);
            assert_eq!(ema.push(42.0), Some(42.0));
        }
    }

    mod steady_state {
        use super::*;

        #[test]
        fn applies_ema_after_seed() {
            // EMA(3): α = 2/4 = 0.5
            let mut ema = raw_ema(3);
            ema.push(2.0);
            ema.push(4.0);
            ema.push(6.0); // seed = 4.0
            // EMA = 8 * 0.5 + 4.0 * 0.5 = 6.0
            assert_eq!(ema.push(8.0), Some(6.0));
        }

        #[test]
        fn continues_computation() {
            // EMA(3): α = 0.5
            let mut ema = raw_ema(3);
            ema.push(2.0);
            ema.push(4.0);
            ema.push(6.0); // 4.0
            ema.push(8.0); // 6.0
            // 10 * 0.5 + 6.0 * 0.5 = 8.0
            assert_eq!(ema.push(10.0), Some(8.0));
        }
        #[test]
        fn constant_input_stays_constant() {
            let mut ema = raw_ema(3);
            for _ in 0..20 {
                ema.push(50.0);
            }
            assert_eq!(ema.push(50.0), Some(50.0));
        }
    }

    mod convergence {
        use super::*;

        #[test]
        fn emits_at_seed_without_enforcement() {
            let mut ema = raw_ema(3);
            ema.push(10.0);
            ema.push(20.0);
            assert!(ema.push(30.0).is_some());
        }
    }

    mod replace {
        use super::*;

        #[test]
        fn none_during_seed() {
            let mut ema = raw_ema(3);
            ema.push(10.0);
            ema.push(20.0);
            // Replace 2nd bar: still accumulating
            assert_eq!(ema.replace(25.0), None);
        }

        #[test]
        fn seed_replace_adjusts_sum() {
            let mut ema = raw_ema(3);
            ema.push(10.0);
            ema.push(20.0);
            ema.replace(25.0); // replace 2nd bar
            // SMA = (10 + 25 + 30) / 3 = 21.666...
            let val = ema.push(30.0).unwrap();
            let expected = (10.0 + 25.0 + 30.0) / 3.0;
            assert!((val - expected).abs() < 1e-10);
        }
        #[test]
        fn seed_bar_replace_recomputes_sma() {
            let mut ema = raw_ema(3);
            ema.push(10.0);
            ema.push(20.0);
            assert_eq!(ema.push(30.0), Some(20.0)); // seed
            // Replace seed bar with 60
            // SMA = (10 + 20 + 60) / 3 = 30.0
            assert_eq!(ema.replace(60.0), Some(30.0));
        }

        #[test]
        fn seed_bar_replace_uses_sma_not_ema() {
            // Values where SMA != EMA-with-prev=0
            let mut ema = raw_ema(3);
            ema.push(10.0);
            ema.push(20.0);
            ema.push(30.0); // seed = 20.0
            // Replace with 45: SMA = (10 + 20 + 45) / 3 = 25.0
            // A buggy EMA path would give: α * 45 = 0.5 * 45 = 22.5
            assert_eq!(ema.replace(45.0), Some(25.0));
        }

        #[test]
        fn multiple_replaces_during_seed() {
            let mut ema = raw_ema(3);
            ema.push(10.0);
            ema.push(20.0);
            ema.replace(22.0);
            ema.replace(25.0); // final value for bar 2
            // SMA = (10 + 25 + 30) / 3 = 21.666...
            let val = ema.push(30.0).unwrap();
            let expected = (10.0 + 25.0 + 30.0) / 3.0;
            assert!((val - expected).abs() < 1e-10);
        }
        #[test]
        fn steady_state_recomputes_from_previous() {
            // EMA(3): α = 0.5
            let mut ema = raw_ema(3);
            ema.push(2.0);
            ema.push(4.0);
            ema.push(6.0); // seed = 4.0
            ema.push(8.0); // EMA = 6.0

            // Replace: 12 * 0.5 + 4.0 * 0.5 = 8.0
            // previous should still be 4.0 (the seed value)
            assert_eq!(ema.replace(12.0), Some(8.0));
        }
        #[test]
        fn multiple_replaces_stable() {
            let mut ema = raw_ema(3);
            ema.push(2.0);
            ema.push(4.0);
            ema.push(6.0); // 4.0
            ema.push(8.0); // 6.0

            ema.replace(10.0);
            ema.replace(12.0);
            // All replaces use same previous (4.0)
            // 12 * 0.5 + 4.0 * 0.5 = 8.0
            assert_eq!(ema.replace(12.0), Some(8.0));
        }

        #[test]
        fn push_after_replace_uses_replaced_value() {
            let mut ema = raw_ema(3);
            ema.push(2.0);
            ema.push(4.0);
            ema.push(6.0); // seed = 4.0
            ema.push(8.0); // 6.0
            ema.replace(10.0); // 10*0.5 + 4*0.5 = 7.0

            // 12 * 0.5 + 7.0 * 0.5 = 9.5
            assert_eq!(ema.push(12.0), Some(9.5));
        } // Next push: previous = 7.0 (replaced value)
        #[test]
        fn replace_matches_clean_computation() {
            // Repainted path
            let mut repainted = raw_ema(3);
            repainted.push(2.0);
            repainted.push(4.0);
            repainted.push(6.0);
            repainted.push(8.0);
            repainted.replace(10.0); // repaint
            let val = repainted.push(12.0);

            // Clean path (as if bar 4 was always 10)
            let mut clean = raw_ema(3);
            clean.push(2.0);
            clean.push(4.0);
            clean.push(6.0);
            clean.push(10.0);
            let expected = clean.push(12.0);

            assert_eq!(val, expected);
        }
    }
}
