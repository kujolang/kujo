# Hetzner Benchmark Setup and Pricing for Kujo SSG

## Purpose
This document captures the recommended Hetzner-based environment for reproducible Kujo SSG benchmarking and fair cross-SSG comparisons.

Use this when Kujo is close to launch and you want stable, publishable benchmark results.

## Key Decision
Use one dedicated Hetzner instance for all benchmark runs in a campaign.

This gives:
- Consistent CPU, RAM, storage, and region per run
- Lower run-to-run noise than local laptop testing
- Fairer apples-to-apples comparison between Kujo and other SSGs

## Recommended Server Tier
Primary recommendation:
- Plan: CCX33 (General Purpose, dedicated vCPU)
- Specs: 8 vCPU, 32 GB RAM, 240 GB SSD
- Price: 62.99 EUR per month max, 0.1009 EUR per hour

Lower-cost reliable option:
- Plan: CCX23
- Specs: 4 vCPU, 16 GB RAM, 160 GB SSD
- Price: 31.99 EUR per month max, 0.0513 EUR per hour

Higher-headroom option:
- Plan: CCX43
- Specs: 16 vCPU, 64 GB RAM, 360 GB SSD
- Price: 125.49 EUR per month max, 0.2011 EUR per hour

## Current Hetzner Cloud Plan Pricing Snapshot (EUR, max/mo, excl. VAT)

### Cost-Optimized (shared resources)
- CX23: 4.49
- CAX11: 4.99
- CX33: 6.99
- CAX21: 8.49
- CX43: 12.49
- CAX31: 16.49
- CX53: 22.99
- CAX41: 31.99

### Regular Performance (shared resources)
- CPX11: 5.99
- CPX12: 8.49
- CPX21: 9.99
- CPX22: 8.49
- CPX31: 17.99
- CPX32: 14.49
- CPX41: 32.99
- CPX42: 25.99
- CPX51: 71.49
- CPX52: 36.99
- CPX62: 50.99

### General Purpose (dedicated vCPU)
- CCX13: 16.49
- CCX23: 31.99
- CCX33: 62.99
- CCX43: 125.49
- CCX53: 250.49
- CCX63: 374.99

## Add-On Pricing Snapshot
- Snapshots: 0.0143 EUR per GB per month
- Backups: starting at 0.8980 EUR per month
- Floating IPv6: 1.00 EUR per month

## Estimated Monthly Budget Bands
- Budget reliable: CCX23 plus snapshots/backups, about 32 to 40 EUR
- Recommended: CCX33 plus snapshots/backups, about 65 to 80 EUR
- High headroom: CCX43 plus snapshots/backups, about 130 to 155 EUR

## Benchmark Campaign Protocol
1. Fix one region and one instance type for the entire campaign.
2. Use the same OS image and package versions.
3. Run 5 warm-up runs and discard them.
4. Run at least 20 measured runs per mode.
5. Keep cold and warm/no-op metrics separate.
6. Report median, p90, min, max, and standard deviation.
7. Record full environment metadata in every report.

## Fairness Rules for Cross-SSG Comparisons
- Same host and hardware profile for all tools
- Same dataset and template complexity
- Same output scope (index, sitemap, feed where possible)
- Same filesystem location type (local SSD)
- Same iteration count and reporting statistics

## Suggested Execution Timing
Run this full benchmark campaign when Kujo is near launch:
- Your implementation is stable enough that numbers represent release reality
- You only need a short rented window to produce reproducible benchmark artifacts
- You can publish one coherent benchmark report with method and data

## Practical Notes
- Dedicated cloud is highly consistent, but still run multiple iterations and report distribution metrics.
- If you need maximum isolation, evaluate Hetzner dedicated root servers for a second validation pass.
- Keep this pricing section updated before publishing any public benchmark post.

## Source Context
This document was compiled from current Hetzner cloud plan pages and the project benchmark planning discussion.
