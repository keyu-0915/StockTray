# StockTray v0.2.20

This release makes market-style samples independently updateable without requiring a new installer for every sample adjustment.

Style labels, concept boards, sample weights, coverage limits, constituent refresh cadence, and complete offline fallback samples now live in a signed market definition. The client validates every remote definition, stages it during the session, and only switches samples during the final call-auction window. Invalid or unavailable updates leave the last known good definition untouched, and previously signed definitions can be selected for rollback.

The middle style now includes innovation-drug and broader healthcare directions alongside robotics, commercial aerospace, and games. Market snapshots and intraday evidence retain the definition version used for calculation, while the analysis page shows whether the active definition is embedded or remotely signed.

A dedicated GitHub Actions workflow validates, signs, and publishes immutable market-data assets separately from application releases. After installing v0.2.20, future sample-definition changes can be delivered through that channel without repackaging StockTray.
