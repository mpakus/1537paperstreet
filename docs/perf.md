# Performance measurements

## T-217: text highlights (2026-10-08)

Ordinary 100 KiB Markdown rendering, existing Criterion corpus, 20 samples,
one-second warm-up and two-second target measurement (Criterion extended it).
Same checkout and development profile for both runs:

| Measure | Before | After |
| --- | ---: | ---: |
| Estimate | 16.175 ms | 15.562 ms |
| Confidence interval | 16.107–16.239 ms | 15.533–15.598 ms |

No regression was observed; the apparent improvement is not a claimed
optimization. Normal previews have no additional source-map DOM nodes.
Temporary mappings are generated only when applying a highlight.
The final run includes continuous markers across source line breaks; the
earlier implementation measured 15.302 ms under the same conditions.

```sh
rtk cargo bench --profile dev -p ps-render --bench render -- \
  'render markdown/document/100 KiB' --save-baseline before-highlights \
  --sample-size 20 --measurement-time 2 --warm-up-time 1
rtk cargo bench --profile dev -p ps-render --bench render -- \
  'render markdown/document/100 KiB' --baseline before-highlights \
  --sample-size 20 --measurement-time 2 --warm-up-time 1
```

The release-profile benchmark could not compile on this host (`E0463`: could
not find `ts_rs_macros` while compiling `ts-rs`), including after rebuilding
those two packages. These numbers are a local before/after comparison, not
validation of the production performance budgets or highlight-action latency.
