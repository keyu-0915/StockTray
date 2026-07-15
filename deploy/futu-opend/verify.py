import os
import sys

from futu import OpenQuoteContext, RET_OK


host = os.environ.get("FUTU_HOST", "futu-opend")
port = int(os.environ.get("FUTU_PORT", "32179"))
symbols = ["SH.000001", "SH.600519", "SZ.300750"]

quote = OpenQuoteContext(host=host, port=port)
try:
    result, data = quote.get_market_snapshot(symbols)
    if result != RET_OK:
        print(f"snapshot request failed: {data}", file=sys.stderr)
        raise SystemExit(1)

    data["change_rate_pct"] = (
        (data["last_price"] / data["prev_close_price"] - 1.0) * 100.0
    )
    columns = ["code", "name", "last_price", "change_rate_pct"]
    print(data.loc[:, columns].to_string(index=False))
finally:
    quote.close()
