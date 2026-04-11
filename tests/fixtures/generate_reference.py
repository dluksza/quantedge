#!/usr/bin/env python3
"""Generate reference indicator values from Binance OHLCV CSV.

Uses talipp for indicator computation. Algorithms match TradingView / TA-Lib:
- SMA: arithmetic mean of last N closes
- EMA: SMA seed, then alpha = 2/(N+1)
- BB: SMA middle, population std dev (÷N), 2 sigma bands
- RSI: Wilder's smoothing (period changes / period seed)

Uses pandas_ta for VWAP (anchored, HLC3 source, daily reset).

Usage:
    1. pip install talipp pandas_ta
    2. Download raw data:
       curl -sL "https://data.binance.vision/data/spot/monthly/klines/BTCUSDT/1h/BTCUSDT-1h-2025-01.zip" -o /tmp/btcusdt.zip
       unzip /tmp/btcusdt.zip -d /tmp
    3. Run: python3 tests/fixtures/generate_reference.py /tmp/BTCUSDT-1h-2025-01.csv
    4. Verify: cargo test
"""

import csv
import os
import sys
from unittest.mock import MagicMock

import pandas as pd
# pandas_ta requires numba at import time but only uses it for JIT.
# Mock it so the script runs on environments without numba (e.g. Python 3.14).
sys.modules.setdefault("numba", MagicMock())
import pandas_ta as ta  # noqa: E402
from talipp.indicators import ADX, ATR, BB, CCI, CHOP, EMA, MACD, OBV, RSI, SMA, Stoch, StochRSI, SuperTrend, KeltnerChannels, DonchianChannels, Williams, Ichimoku
from talipp.ohlcv import OHLCV

PERIOD = 20
RSI_PERIOD = 14
ATR_PERIOD = 14
STOCH_PERIOD = 14
STOCH_SMOOTH = 3
STOCH_RSI_RSI_PERIOD = 14
STOCH_RSI_STOCH_PERIOD = 14
STOCH_RSI_K_SMOOTH = 3
STOCH_RSI_D_SMOOTH = 3
KC_MA_PERIOD = 20
KC_ATR_PERIOD = 10
KC_MULT = 1.5
DC_PERIOD = 20
ADX_PERIOD = 14
WILLR_PERIOD = 14
CCI_PERIOD = 20
CHOP_PERIOD = 14
SUPERTREND_ATR_PERIOD = 10
SUPERTREND_MULT = 3
ICHIMOKU_TENKAN = 9
ICHIMOKU_KIJUN = 26
ICHIMOKU_SENKOU_B = 52
ICHIMOKU_DISPLACEMENT = 26
OUTPUT_DIR = "tests/fixtures/data"


def read_binance_csv(path):
    """Read raw Binance kline CSV (no header, 12 columns)."""
    rows = []
    with open(path) as f:
        for row in csv.reader(f):
            rows.append(
                {
                    "open_time": int(row[0]),
                    "open": float(row[1]),
                    "high": float(row[2]),
                    "low": float(row[3]),
                    "close": float(row[4]),
                    "volume": float(row[5]),
                }
            )
    return rows


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    rows = read_binance_csv(sys.argv[1])
    closes = [r["close"] for r in rows]
    times = [r["open_time"] for r in rows]

    # OHLCV with header
    with open(f"{OUTPUT_DIR}/btcusdt-1h.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "open", "high", "low", "close", "volume"])
        for r in rows:
            w.writerow(
                [
                    r["open_time"],
                    r["open"],
                    r["high"],
                    r["low"],
                    r["close"],
                    r["volume"],
                ]
            )

    # Compute indicators via talipp
    # SMA
    sma = SMA(period=PERIOD, input_values=closes)
    with open(f"{OUTPUT_DIR}/sma-20-close.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "expected"])
        for i, val in enumerate(sma):
            if val is not None:
                w.writerow([times[i], f"{val:.10f}"])

    # EMA
    ema = EMA(period=PERIOD, input_values=closes)
    with open(f"{OUTPUT_DIR}/ema-20-close.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "expected"])
        for i, val in enumerate(ema):
            if val is not None:
                w.writerow([times[i], f"{val:.10f}"])

    # BB
    bb = BB(period=PERIOD, std_dev_mult=2.0, input_values=closes)
    with open(f"{OUTPUT_DIR}/bb-20-2-close.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "upper", "middle", "lower"])
        for i, val in enumerate(bb):
            if val is not None:
                w.writerow(
                    [
                        times[i],
                        f"{val.ub:.10f}",
                        f"{val.cb:.10f}",
                        f"{val.lb:.10f}",
                    ]
                )

    # RSI
    rsi = RSI(period=RSI_PERIOD, input_values=closes)
    with open(f"{OUTPUT_DIR}/rsi-14-close.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "expected"])
        for i, val in enumerate(rsi):
            if val is not None:
                w.writerow([times[i], f"{val:.10f}"])

    # MACD
    macd = MACD(
        fast_period=12,
        slow_period=26,
        signal_period=9,
        input_values=closes,
    )
    with open(f"{OUTPUT_DIR}/macd-12-26-9-close.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "macd", "signal", "histogram"])
        for i, val in enumerate(macd):
            if val is not None and val.signal is not None:
                w.writerow(
                    [
                        times[i],
                        f"{val.macd:.10f}",
                        f"{val.signal:.10f}",
                        f"{val.histogram:.10f}",
                    ]
                )

    # ATR
    ohlcv_bars = [OHLCV(r["open"], r["high"], r["low"], r["close"]) for r in rows]
    atr = ATR(period=ATR_PERIOD, input_values=ohlcv_bars)
    with open(f"{OUTPUT_DIR}/atr-14.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "expected"])
        for i, val in enumerate(atr):
            if val is not None:
                w.writerow([times[i], f"{val:.10f}"])

    # Stochastic Oscillator
    # talipp Stoch(period, smoothing_period) gives raw %K and %D = SMA(%K, smoothing_period).
    # This maps to Rust Stoch(length=14, k_smooth=1, d_smooth=3).
    stoch = Stoch(period=STOCH_PERIOD, smoothing_period=STOCH_SMOOTH, input_values=ohlcv_bars)
    with open(f"{OUTPUT_DIR}/stoch-14-1-3.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "k", "d"])
        for i, val in enumerate(stoch):
            if val is not None and val.d is not None:
                w.writerow(
                    [
                        times[i],
                        f"{val.k:.10f}",
                        f"{val.d:.10f}",
                    ]
                )

    # Stochastic RSI
    # talipp StochRSI(rsi_period, stoch_period, k_smoothing_period, d_smoothing_period)
    # uses SMA for smoothing by default. Output: k (%K), d (%D).
    # This maps to Rust StochRsi(rsi_length=14, stoch_length=14, k_smooth=3, d_smooth=3).
    stoch_rsi = StochRSI(
        rsi_period=STOCH_RSI_RSI_PERIOD,
        stoch_period=STOCH_RSI_STOCH_PERIOD,
        k_smoothing_period=STOCH_RSI_K_SMOOTH,
        d_smoothing_period=STOCH_RSI_D_SMOOTH,
        input_values=closes,
    )
    with open(f"{OUTPUT_DIR}/stoch-rsi-14-14-3-3-close.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "k", "d"])
        for i, val in enumerate(stoch_rsi):
            if val is not None and val.k is not None and val.d is not None:
                w.writerow(
                    [
                        times[i],
                        f"{val.k:.10f}",
                        f"{val.d:.10f}",
                    ]
                )

    # Keltner Channels
    # talipp KeltnerChannels(ma_period, atr_period, atr_mult_up, atr_mult_down)
    # uses EMA for centre line by default. Output: ub (upper), cb (central), lb (lower).
    # This maps to Rust Kc(length=20, atr_length=10, multiplier=1.5).
    kc = KeltnerChannels(
        ma_period=KC_MA_PERIOD,
        atr_period=KC_ATR_PERIOD,
        atr_mult_up=KC_MULT,
        atr_mult_down=KC_MULT,
        input_values=ohlcv_bars,
    )
    with open(f"{OUTPUT_DIR}/kc-20-10-1.5.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "upper", "middle", "lower"])
        for i, val in enumerate(kc):
            if val is not None:
                w.writerow(
                    [
                        times[i],
                        f"{val.ub:.10f}",
                        f"{val.cb:.10f}",
                        f"{val.lb:.10f}",
                    ]
                )

    # Donchian Channels
    # talipp DonchianChannels(period) takes OHLCV. Output: ub (upper), cb (central), lb (lower).
    # This maps to Rust Dc(length=20).
    dc = DonchianChannels(period=DC_PERIOD, input_values=ohlcv_bars)
    with open(f"{OUTPUT_DIR}/dc-20.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "upper", "middle", "lower"])
        for i, val in enumerate(dc):
            if val is not None:
                w.writerow(
                    [
                        times[i],
                        f"{val.ub:.10f}",
                        f"{val.cb:.10f}",
                        f"{val.lb:.10f}",
                    ]
                )

    # ADX
    # talipp ADX(di_period, adx_period) takes OHLCV. Output: adx, plus_di, minus_di.
    # This maps to Rust Adx(length=14).
    adx = ADX(di_period=ADX_PERIOD, adx_period=ADX_PERIOD, input_values=ohlcv_bars)
    with open(f"{OUTPUT_DIR}/adx-14.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "adx", "plus_di", "minus_di"])
        for i, val in enumerate(adx):
            if val is not None and val.adx is not None:
                w.writerow(
                    [
                        times[i],
                        f"{val.adx:.10f}",
                        f"{val.plus_di:.10f}",
                        f"{val.minus_di:.10f}",
                    ]
                )

    # Williams %R
    # talipp Williams(period) takes OHLCV. Output: single float.
    # This maps to Rust WillR(length=14).
    willr = Williams(period=WILLR_PERIOD, input_values=ohlcv_bars)
    with open(f"{OUTPUT_DIR}/willr-14.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "expected"])
        for i, val in enumerate(willr):
            if val is not None:
                w.writerow([times[i], f"{val:.10f}"])

    # CCI
    # talipp CCI(period) takes OHLCV. Uses typical price (HLC3) internally.
    # This maps to Rust Cci(length=20) with default HLC3 source.
    cci = CCI(period=CCI_PERIOD, input_values=ohlcv_bars)
    with open(f"{OUTPUT_DIR}/cci-20.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "expected"])
        for i, val in enumerate(cci):
            if val is not None:
                w.writerow([times[i], f"{val:.10f}"])

    # CHOP
    # talipp CHOP(period) takes OHLCV. Output: single float (0–100 scale).
    # This maps to Rust Chop(length=14).
    chop = CHOP(period=CHOP_PERIOD, input_values=ohlcv_bars)
    with open(f"{OUTPUT_DIR}/chop-14.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "expected"])
        for i, val in enumerate(chop):
            if val is not None:
                w.writerow([times[i], f"{val:.10f}"])

    # Supertrend
    # talipp SuperTrend(atr_period, mult) takes OHLCV.
    # Output: SuperTrendVal(value, trend). trend is Trend.UP or Trend.DOWN.
    # The first non-None value has value=0 (sentinel) — skip it.
    # This maps to Rust Supertrend(length=10, multiplier=3.0).
    from talipp.indicators.SuperTrend import Trend
    supertrend = SuperTrend(atr_period=SUPERTREND_ATR_PERIOD, mult=SUPERTREND_MULT, input_values=ohlcv_bars)
    with open(f"{OUTPUT_DIR}/supertrend-10-3.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "value", "is_bullish"])
        for i, val in enumerate(supertrend):
            if val is not None and val.value != 0:
                is_bullish = 1 if val.trend == Trend.UP else 0
                w.writerow([times[i], f"{val.value:.10f}", is_bullish])

    # Ichimoku Cloud
    # talipp Ichimoku params: tenkan_period, kijun_period, senkou_slow_period,
    # senkou_lookup_period (displacement), chikou_lag_period.
    # Output: conversion_line (tenkan), base_line (kijun),
    # cloud_leading_fast_line (senkou_a), cloud_leading_slow_line (senkou_b),
    # lagging_line (chikou - already displaced, we skip it).
    ichimoku = Ichimoku(
        tenkan_period=ICHIMOKU_TENKAN,
        kijun_period=ICHIMOKU_KIJUN,
        senkou_slow_period=ICHIMOKU_SENKOU_B,
        senkou_lookup_period=ICHIMOKU_DISPLACEMENT,
        chikou_lag_period=ICHIMOKU_DISPLACEMENT,
        input_values=ohlcv_bars,
    )
    with open(f"{OUTPUT_DIR}/ichimoku-9-26-52-26.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "tenkan", "kijun", "senkou_a", "senkou_b"])
        for i, val in enumerate(ichimoku):
            if (
                val is not None
                and val.conversion_line is not None
                and val.base_line is not None
                and val.cloud_leading_fast_line is not None
                and val.cloud_leading_slow_line is not None
            ):
                w.writerow(
                    [
                        times[i],
                        f"{val.conversion_line:.10f}",
                        f"{val.base_line:.10f}",
                        f"{val.cloud_leading_fast_line:.10f}",
                        f"{val.cloud_leading_slow_line:.10f}",
                    ]
                )

    # OBV
    # talipp OBV takes OHLCV. Output: single float (cumulative volume).
    obv = OBV(input_values=ohlcv_bars)
    with open(f"{OUTPUT_DIR}/obv-close.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "expected"])
        for i, val in enumerate(obv):
            if val is not None:
                w.writerow([times[i], f"{val:.10f}"])

    # VWAP (anchored daily)
    # pandas_ta.vwap(high, low, close, volume, anchor="D") uses HLC3 internally.
    # Requires a DatetimeIndex for anchor-based resets.
    # Timestamps are microseconds since epoch — convert to datetime.
    df = pd.DataFrame(rows)
    df["datetime"] = pd.to_datetime(df["open_time"], unit="us", utc=True)
    df.set_index("datetime", inplace=True)
    vwap_series = ta.vwap(df["high"], df["low"], df["close"], df["volume"], anchor="D")
    with open(f"{OUTPUT_DIR}/vwap-day-hlc3.csv", "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["open_time", "expected"])
        for i, val in enumerate(vwap_series):
            if pd.notna(val):
                w.writerow([times[i], f"{val:.10f}"])
    vwap_count = int(vwap_series.notna().sum())

    sma_count = sum(1 for v in sma if v is not None)
    ema_count = sum(1 for v in ema if v is not None)
    bb_count = sum(1 for v in bb if v is not None)
    rsi_count = sum(1 for v in rsi if v is not None)
    macd_count = sum(1 for v in macd if v is not None and v.signal is not None)
    atr_count = sum(1 for v in atr if v is not None)
    stoch_count = sum(1 for v in stoch if v is not None and v.d is not None)
    stoch_rsi_count = sum(1 for v in stoch_rsi if v is not None and v.k is not None and v.d is not None)
    kc_count = sum(1 for v in kc if v is not None)
    dc_count = sum(1 for v in dc if v is not None)
    adx_count = sum(1 for v in adx if v is not None and v.adx is not None)
    willr_count = sum(1 for v in willr if v is not None)
    cci_count = sum(1 for v in cci if v is not None)
    chop_count = sum(1 for v in chop if v is not None)
    supertrend_count = sum(1 for v in supertrend if v is not None and v.value != 0)
    ichimoku_count = sum(
        1
        for v in ichimoku
        if v is not None
        and v.conversion_line is not None
        and v.base_line is not None
        and v.cloud_leading_fast_line is not None
        and v.cloud_leading_slow_line is not None
    )
    obv_count = sum(1 for v in obv if v is not None)
    print(
        f"Generated {sma_count} SMA, "
        f"{ema_count} EMA, "
        f"{bb_count} BB, "
        f"{rsi_count} RSI, "
        f"{macd_count} MACD, "
        f"{atr_count} ATR, "
        f"{stoch_count} Stoch, "
        f"{stoch_rsi_count} StochRSI, "
        f"{kc_count} KC, "
        f"{dc_count} DC, "
        f"{adx_count} ADX, "
        f"{willr_count} WillR, "
        f"{cci_count} CCI, "
        f"{chop_count} CHOP, "
        f"{supertrend_count} Supertrend, "
        f"{ichimoku_count} Ichimoku, "
        f"{obv_count} OBV, "
        f"{vwap_count} VWAP reference values "
        f"from {len(rows)} OHLCV bars."
    )


if __name__ == "__main__":
    main()
