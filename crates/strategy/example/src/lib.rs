mod e01_ema_cross_forming;
mod e02_htf_filtered_cross;
mod e03_ema_cross_closed;
mod e04_bollinger_breakout;

pub use crate::e01_ema_cross_forming::EmaCrossingFormingSignalGenerator;
pub use crate::e02_htf_filtered_cross::HtfFilteredEmaCrossSignalGenerator;
pub use crate::e03_ema_cross_closed::EmaCrossingClosedSignalGenerator;
pub use crate::e04_bollinger_breakout::BollingerBreakoutSignalGenerator;
