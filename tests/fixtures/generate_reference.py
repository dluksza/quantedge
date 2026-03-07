#!/usr/bin/env python3
"""Generate reference indicator values from Binance OHLCV CSV.

Uses talipp for indicator computation. Algorithms match TradingView / TA-Lib:
- SMA: arithmetic mean of last N closes
- EMA: SMA seed, then alpha = 2/(N+1)
- BB: SMA middle, population std dev (÷N), 2 sigma bands
- RSI: Wilder's smoothing (period changes / period seed)

Usage:
    1. pip install talipp
    2. Download raw data:
       curl -sL "https://data.binance.vision/data/spot/monthly/klines/BTCUSDT/1h/BTCUSDT-1h-2025-01.zip" -o /tmp/btcusdt.zip
       unzip /tmp/btcusdt.zip -d /tmp
    3. Run: python3 tests/fixtures/generate_reference.py /tmp/BTCUSDT-1h-2025-01.csv
    4. Verify: cargo test
"""

import csv
import os
import sys

from talipp.indicators import ATR, BB, EMA, MACD, RSI, SMA
from talipp.ohlcv import OHLCV

PERIOD = 20
RSI_PERIOD = 14
ATR_PERIOD = 14
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

    sma_count = sum(1 for v in sma if v is not None)
    ema_count = sum(1 for v in ema if v is not None)
    bb_count = sum(1 for v in bb if v is not None)
    rsi_count = sum(1 for v in rsi if v is not None)
    macd_count = sum(1 for v in macd if v is not None and v.signal is not None)
    atr_count = sum(1 for v in atr if v is not None)
    print(
        f"Generated {sma_count} SMA, "
        f"{ema_count} EMA, "
        f"{bb_count} BB, "
        f"{rsi_count} RSI, "
        f"{macd_count} MACD, "
        f"{atr_count} ATR reference values "
        f"from {len(rows)} OHLCV bars."
    )


if __name__ == "__main__":
    main()
