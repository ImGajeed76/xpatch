# Performance Analysis

> This is a copy of the original [benchmark](https://github.com/ImGajeed76/gdelta/blob/master/PERFORMANCE.md) of the gdelta package.

This document provides comprehensive performance analysis of gdelta and comparison with other delta compression
algorithms across synthetic benchmarks and real-world git repository workloads.

## Table of Contents

1. [Hardware & Methodology](#hardware--methodology)
2. [Executive Summary](#executive-summary)
3. [Synthetic Benchmark Results](#synthetic-benchmark-results)
4. [Real-World Git Repository Performance](#real-world-git-repository-performance)
5. [Algorithm Comparison](#algorithm-comparison)
6. [Use Case Recommendations](#use-case-recommendations)
7. [Scaling Behavior](#scaling-behavior)
8. [Running Your Own Benchmarks](#running-your-own-benchmarks)

---

## Hardware & Methodology

### Test System

```
CPU:    AMD Ryzen 7 7800X3D 8-Core Processor (16 threads)
RAM:    64 GB DDR5
OS:     Fedora Linux 42
Rust:   1.83+ with default optimizations
```

### Benchmark Methodologies

**1. Synthetic Benchmarks** (`cargo bench --bench comprehensive`)

- 15 data formats × 7 change patterns × 3 sizes = 315 test cases per algorithm
- Controlled modifications: deletes, appends, minor/moderate/major edits
- Metrics: compression ratio, encode/decode speed, verification
- Total: 2,097 tests across 7 algorithms

**2. Real-World Git Repository Analysis** (via xpatch)

- Actual file evolution across git history
- Tests every file version against previous versions
- mdn/content: 30,719 files, 1,225,740 comparisons
- tokio: 1,805 files, 133,728 comparisons
- Measures real version control workload performance

**Data sources:**

- Synthetic benchmarks: gdelta comprehensive suite
- Git benchmarks: xpatch test
  results ([mdn/content](https://github.com/ImGajeed76/xpatch/blob/master/test_results/report_20251211_204655.md), [tokio](https://github.com/ImGajeed76/xpatch/blob/master/test_results/report_20251211_205512.md))

### Tested Algorithms

| Algorithm       | Description                                      | Version |
|-----------------|--------------------------------------------------|---------|
| **gdelta**      | Pure delta (this implementation)                 | 0.2.1   |
| **gdelta_zstd** | gdelta + zstd compression (level 3)              | 0.2.1   |
| **gdelta_lz4**  | gdelta + lz4 compression                         | 0.2.1   |
| **xpatch**      | Multi-algorithm wrapper (uses gdelta internally) | 0.2.0   |
| **vcdiff**      | Standard VCDIFF format implementation            | 0.1.0   |
| **qbsdiff**     | Industry-standard bsdiff                         | 1.4.4   |
| **zstd_dict**   | Zstd with dictionary training                    | 0.13.3  |

---

## Executive Summary

### Synthetic Benchmark Results (2,097 tests)

| Algorithm   | Encode Speed | Decode Speed | Compression | Verification |
|-------------|--------------|--------------|-------------|--------------|
| gdelta      | 496 MB/s     | 4.4 GB/s     | 68% saved   | 100% ✅       |
| gdelta_lz4  | 430 MB/s     | 3.9 GB/s     | 71% saved   | 100% ✅       |
| gdelta_zstd | 305 MB/s     | 2.2 GB/s     | 75% saved   | 100% ✅       |
| xpatch      | 306 MB/s     | 2.3 GB/s     | 75% saved   | 100% ✅       |
| vcdiff      | 94 MB/s      | 2.6 GB/s     | 64% saved   | 100% ✅       |
| qbsdiff     | 22 MB/s      | 111 MB/s     | 84% saved   | 100% ✅       |
| zstd_dict   | 14 MB/s      | 16 MB/s      | 55% saved   | 100% ✅       |

### Git Repository Results (1.36M comparisons)

**mdn/content (306,435 comparisons):**

| Algorithm     | Compression | Encode Time | Decode Time |
|---------------|-------------|-------------|-------------|
| xpatch (tags) | 97.5% saved | 319 µs      | 1 µs        |
| xpatch (seq)  | 97.5% saved | 14 µs       | 1 µs        |
| gdelta        | 97.0% saved | 4 µs        | 0 µs        |
| vcdiff        | 95.6% saved | 49 µs       | 7 µs        |

**tokio (33,432 comparisons):**

| Algorithm     | Compression | Encode Time | Decode Time |
|---------------|-------------|-------------|-------------|
| xpatch (tags) | 97.9% saved | 306 µs      | 0 µs        |
| xpatch (seq)  | 95.6% saved | 24 µs       | 1 µs        |
| gdelta        | 94.5% saved | 5 µs        | 0 µs        |
| vcdiff        | 93.1% saved | 28 µs       | 4 µs        |

### Key Findings

**Synthetic workloads show:**

- gdelta: fastest encode/decode, competitive compression
- qbsdiff: best compression, slowest speed
- xpatch: balanced performance matching gdelta+zstd
- vcdiff: moderate speed, lowest compression

**Real-world git workloads show:**

- xpatch (tags mode): best compression by significant margin
- gdelta: fastest speed, competitive compression
- xpatch (sequential): balanced speed and compression
- vcdiff: moderate performance across metrics

**Workload matters:** Algorithm performance varies significantly between synthetic edits and real file evolution
patterns.

---

## Synthetic Benchmark Results

### Performance Summary

**Encoding Throughput:**

```
gdelta       ████████████████████ 496 MB/s
gdelta_lz4   ██████████████████   430 MB/s
xpatch       ███████████████      306 MB/s  
gdelta_zstd  ███████████████      305 MB/s
vcdiff       █████                 94 MB/s
qbsdiff      ████                  22 MB/s
zstd_dict    ███                   14 MB/s
```

**Decoding Throughput:**

```
gdelta       ████████████████████ 4.4 GB/s
gdelta_lz4   ███████████████████  3.9 GB/s
vcdiff       █████████████        2.6 GB/s
xpatch       ████████████         2.3 GB/s
gdelta_zstd  ███████████          2.2 GB/s
qbsdiff      ███                  111 MB/s
zstd_dict    █                     16 MB/s
```

**Compression Ratio:**

```
qbsdiff      ████████████████████ 84.4%
xpatch       ███████████████      74.6%
gdelta_zstd  ███████████████      74.6%
gdelta_lz4   ██████████████       70.6%
gdelta       █████████████        68.4%
vcdiff       ████████████         64.2%
zstd_dict    ███████████          55.0%
```

### Real-World Delta Sizes (2MB file, synthetic edits)

| Algorithm   | Delta Size | Saved | Speed Trade-off       |
|-------------|------------|-------|-----------------------|
| qbsdiff     | 312 KB     | 85%   | 22 MB/s (baseline)    |
| xpatch      | 544 KB     | 74%   | 14x faster (306 MB/s) |
| gdelta_zstd | 544 KB     | 74%   | 14x faster (305 MB/s) |
| gdelta_lz4  | 647 KB     | 69%   | 20x faster (430 MB/s) |
| gdelta      | 717 KB     | 66%   | 23x faster (496 MB/s) |
| vcdiff      | 774 KB     | 63%   | 4x faster (94 MB/s)   |
| zstd_dict   | 1039 KB    | 50%   | 0.6x speed (14 MB/s)  |

### Performance by Change Pattern

**Deletions (best case):**

- All algorithms: 98-100% saved
- gdelta: fastest at 36,651 efficiency score
- xpatch: competitive with automatic optimization

**Appends (excellent case):**

- All algorithms: 97-98% saved
- gdelta: 36,651 efficiency score
- xpatch: 4,913 efficiency score with automatic base selection

**Minor edits (typical case):**

- qbsdiff: 95% saved, slow (27ms)
- xpatch: 83% saved, fast (3ms)
- gdelta: 80% saved, fastest (2.3ms)

**Major rewrites (worst case):**

- qbsdiff: 38% saved
- All others: 0-15% saved
- Speed advantage of fast algorithms increases

---

## Real-World Git Repository Performance

### mdn/content Repository (30,719 files)

**Compression efficiency:**

- xpatch (tags): 97.5% saved (best)
- xpatch (sequential): 97.5% saved
- gdelta: 97.0% saved
- vcdiff: 95.6% saved (lowest)

**Speed characteristics:**

- gdelta: 4µs encode, 0µs decode (fastest)
- xpatch (sequential): 14µs encode, 1µs decode
- vcdiff: 49µs encode, 7µs decode
- xpatch (tags): 319µs encode, 1µs decode

**Tag optimization impact:**

- Average: 3.8% better compression with tags
- Median: 8.8% better compression with tags
- Average base distance: 1.1 commits back

### tokio Repository (1,805 files)

**Compression efficiency:**

- xpatch (tags): 97.9% saved (best, significant margin)
- xpatch (sequential): 95.6% saved
- gdelta: 94.5% saved
- vcdiff: 93.1% saved (lowest)

**Speed characteristics:**

- gdelta: 5µs encode, 0µs decode (fastest)
- xpatch (sequential): 24µs encode, 1µs decode
- vcdiff: 28µs encode, 4µs decode
- xpatch (tags): 306µs encode, 0µs decode

**Tag optimization impact:**

- Average: 53.3% better compression with tags
- Median: 88.7% better compression with tags
- Average base distance: 1.9 commits back

### Git Workload Analysis

**Key observations:**

1. **xpatch tag optimization highly effective** on actual file evolution:
    - mdn/content: 3.8% average improvement, 8.8% median
    - tokio: 53.3% average improvement, 88.7% median
    - Benefit varies by repository structure

2. **Compression ratios much better** than synthetic benchmarks:
    - Git: 93-98% saved across all algorithms
    - Synthetic: 55-84% saved across all algorithms
    - Real file changes are more incremental

3. **Speed differences compress at small scales:**
    - All algorithms complete in microseconds
    - gdelta maintains speed advantage (4-5µs)
    - xpatch tag overhead (300µs) still fast in absolute terms

4. **Median performance varies from average:**
    - Median deltas often smaller than average
    - Most changes are small, some are large
    - Algorithm choice impacts outliers differently

---

## Algorithm Comparison

### Efficiency Scores

**Formula:** (1 - Compression Ratio) × 1000 / Encode Time (ms)

Higher score = better compression per unit time

**Synthetic benchmarks:**

| Algorithm   | Efficiency | Category            |
|-------------|------------|---------------------|
| gdelta      | 419        | Speed-optimized     |
| gdelta_lz4  | 376        | Speed-optimized     |
| xpatch      | 282        | Balanced            |
| gdelta_zstd | 281        | Balanced            |
| vcdiff      | 74         | Balanced            |
| qbsdiff     | 23         | Compression-focused |
| zstd_dict   | 6          | Compression-focused |

**Git repository (mdn/content):**

| Algorithm     | Efficiency | Category            |
|---------------|------------|---------------------|
| gdelta        | 242,500    | Speed-optimized     |
| xpatch (seq)  | 69,643     | Speed-optimized     |
| vcdiff        | 19,510     | Balanced            |
| xpatch (tags) | 3,056      | Compression-focused |

### Verification Results

**All algorithms:** 100% verification success across all test suites

- Synthetic: 2,097/2,097 tests passed
- mdn/content: 306,435/306,435 comparisons verified
- tokio: 33,432/33,432 comparisons verified

**Total verified operations:** 1,567,764

---

## Use Case Recommendations

### By Workload Type

**Version Control Systems (git, svn, etc.):**

- **Best compression:** xpatch with tag optimization
    - 97.5-97.9% saved on real repos
    - Tag mode: best for complex histories with branches
    - Sequential mode: faster, competitive compression
- **Best speed:** gdelta
    - 94.5-97.0% saved on real repos
    - Sub-10µs latency
    - Simple, predictable performance

**Backup & Deduplication:**

- **Fast operation:** gdelta (496 MB/s) or gdelta_lz4 (430 MB/s)
- **Balanced:** xpatch or gdelta_zstd (305 MB/s, 75% saved)
- **Maximum compression:** qbsdiff (84% saved)

**Network Synchronization:**

- **Low bandwidth:** qbsdiff (84% saved) or xpatch (75% saved)
- **Low latency:** gdelta (4.4 GB/s decode) or gdelta_lz4 (3.9 GB/s decode)
- **Balanced:** gdelta_zstd (75% saved, 2.2 GB/s decode)

**Real-time Processing:**

- **Sub-millisecond:** gdelta (496 MB/s encode, 4.4 GB/s decode)
- **Near-realtime:** gdelta_lz4 (430 MB/s encode, 3.9 GB/s decode)

**Archive Storage:**

- **Space-critical:** qbsdiff (84% saved)
- **Retrieval matters:** xpatch (75% saved, 2.3 GB/s decode)

**Standards Compliance:**

- **RFC 3284:** vcdiff (64% saved, 94 MB/s encode)

### By Data Characteristics

**Highly similar versions (typical git changes):**

- All algorithms perform excellently (93-98% saved)
- Speed difference becomes primary factor
- xpatch tag optimization provides marginal gains

**Diverse changes (synthetic edits):**

- Compression varies widely (55-84% saved)
- Algorithm choice has larger impact
- qbsdiff provides significantly better compression

**Small files/chunks (< 64KB):**

- gdelta: >1 GiB/s throughput
- Compression overhead minimal

**Large files (> 512KB):**

- Compression benefits increase
- gdelta_zstd or xpatch recommended

**Structured text (code, configs, JSON):**

- All delta algorithms work well
- 68-84% saved (synthetic)
- 94-98% saved (real evolution)

**Binary data:**

- qbsdiff: 84% saved
- gdelta: 66% saved
- vcdiff: 64% saved

### Decision Matrix

| Priority     | Synthetic Workload | Git Repository           |
|--------------|--------------------|--------------------------|
| Speed        | gdelta (496 MB/s)  | gdelta (4µs)             |
| Compression  | qbsdiff (84%)      | xpatch_tags (97.9%)      |
| Balanced     | xpatch (75%, 306)  | xpatch_seq (95.6%, 24µs) |
| Standards    | vcdiff (RFC 3284)  | vcdiff (RFC 3284)        |
| Decode Speed | gdelta (4.4 GB/s)  | gdelta (0µs)             |

---

## Scaling Behavior

### Compression Ratio vs. File Size (Synthetic)

| Algorithm   | 16KB  | 256KB | 2MB   | Trend            |
|-------------|-------|-------|-------|------------------|
| gdelta      | 0.310 | 0.314 | 0.324 | ➡️ Stable (+5%)  |
| gdelta_lz4  | 0.290 | 0.291 | 0.301 | ➡️ Stable (+4%)  |
| gdelta_zstd | 0.258 | 0.251 | 0.255 | ➡️ Stable (-1%)  |
| xpatch      | 0.257 | 0.251 | 0.255 | ➡️ Stable (-1%)  |
| vcdiff      | 0.367 | 0.353 | 0.352 | ➡️ Stable (-4%)  |
| qbsdiff     | 0.180 | 0.145 | 0.143 | ⬇️ Better (-21%) |
| zstd_dict   | N/A   | 0.402 | 0.497 | ⬆️ Worse (+24%)  |

### Throughput vs. Chunk Size (gdelta)

| Size  | Encode      | Decode     |
|-------|-------------|------------|
| 16KB  | 1,080 MiB/s | 6.7 GiB/s  |
| 64KB  | 1,020 MiB/s | 8.4 GiB/s  |
| 128KB | 1,040 MiB/s | 10.6 GiB/s |
| 256KB | 371 MiB/s   | 2.0 GiB/s  |

**Optimal:** 16-128KB chunks for peak throughput

### Performance by Data Format (Synthetic)

Best compression ratios by format:

| Format      | Best Algorithm | Ratio | Second Best | Ratio |
|-------------|----------------|-------|-------------|-------|
| SQL dumps   | qbsdiff        | 0.141 | xpatch      | 0.221 |
| XML         | qbsdiff        | 0.142 | xpatch      | 0.229 |
| Logs        | qbsdiff        | 0.141 | xpatch      | 0.238 |
| JSON        | qbsdiff        | 0.141 | xpatch      | 0.224 |
| CSV         | qbsdiff        | 0.141 | xpatch      | 0.232 |
| HTML        | qbsdiff        | 0.142 | xpatch      | 0.241 |
| Source code | qbsdiff        | 0.143 | xpatch      | 0.216 |
| Compressed  | qbsdiff        | 0.199 | xpatch      | 0.326 |
| Binary      | qbsdiff        | 0.199 | xpatch      | 0.325 |

**Pattern:** qbsdiff leads compression, xpatch consistently second, gdelta variants competitive on speed.

---

## Running Your Own Benchmarks

### Synthetic Benchmarks

```bash
# Quick verification (~2-5 minutes)
cargo bench --bench simple

# Comprehensive suite (~1 hour)
cargo bench --bench comprehensive

# Custom filters
BENCH_FORMATS=json,csv cargo bench --bench comprehensive
BENCH_ALGOS=gdelta,xpatch cargo bench --bench comprehensive
BENCH_PATTERNS=minor_edit,append_1024 cargo bench --bench comprehensive

# Full mode (larger sample size)
BENCH_MODE=full cargo bench --bench comprehensive
```

Results saved to `target/benchmark_report_<timestamp>.md`

### Git Repository Benchmarks

Using xpatch's git benchmark tool:

_Please see the xpatch repository for more information._

---

## Conclusions

### Algorithm Characteristics Summary

**gdelta:**

- Fastest encode/decode across all workloads
- Competitive compression (68% synthetic, 94-97% git)
- Predictable, stable performance
- Best for: speed-critical applications, real-time processing

**gdelta_lz4:**

- Fast with improved compression vs raw gdelta
- 430 MB/s encode, 3.9 GB/s decode
- 71% saved (synthetic)
- Best for: balanced speed/compression needs

**gdelta_zstd:**

- Better compression than lz4, slower than raw
- 305 MB/s encode, 2.2 GB/s decode
- 75% saved (synthetic)
- Best for: production systems valuing both metrics

**xpatch:**

- Matches gdelta_zstd in synthetic benchmarks
- Superior compression in git repositories with tags
- Automatic algorithm selection
- Best for: version control, complex file histories

**vcdiff:**

- RFC 3284 standard compliance
- Moderate speed (94 MB/s encode)
- Lower compression than alternatives
- Best for: standards-required environments

**qbsdiff:**

- Best compression across synthetic workloads (84%)
- Significantly slower than alternatives (22 MB/s)
- Best for: bandwidth-critical, time-insensitive use cases

**zstd_dict:**

- Lowest compression among tested algorithms
- Slowest encode/decode
- Requires training data
- Limited applicability

### Workload-Specific Insights

**For synthetic/controlled edits:**

- Algorithm choice significantly impacts compression (55-84% range)
- Speed varies widely (14-496 MB/s)
- qbsdiff provides best compression
- gdelta provides best speed

**For git repository workloads:**

- All algorithms compress well (93-98% saved)
- Differences smaller but still meaningful
- xpatch tag optimization excels (up to 98% saved)
- Speed differences measured in microseconds

**Key takeaway:** Test with your actual data patterns. Synthetic benchmarks show capabilities, real-world workloads show
practical performance.

### Recommended Defaults

**Most users:** gdelta_zstd

- 305 MB/s encode, 2.2 GB/s decode
- 75% saved in synthetic tests
- Balanced performance

**Version control:** xpatch

- Excellent compression on real repos (95-98%)
- Tag mode for complex histories
- Sequential mode for speed

**Speed-critical:** gdelta

- 496 MB/s encode, 4.4 GB/s decode
- Competitive compression
- Simplest implementation

**Space-critical:** qbsdiff

- 84% saved (best compression)
- Accept 22x slower encoding

**Standards-required:** vcdiff

- RFC 3284 compliance
- Moderate performance

---

*Synthetic benchmarks: 2025-12-12, AMD Ryzen 7 7800X3D, 64GB RAM, Fedora Linux 42*

*Git benchmarks: 2025-12-11, AMD Ryzen 7 7800X3D, 64GB RAM, Fedora Linux 42, xpatch
v0.2.0 ([results](https://github.com/ImGajeed76/xpatch/tree/master/test_results))*
