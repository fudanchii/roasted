Roasted
---
![test](https://github.com/fudanchii/roasted/actions/workflows/test.yaml/badge.svg?branch=master)
[![codecov](https://codecov.io/gh/fudanchii/roasted/branch/master/graph/badge.svg?token=RLATEAP63J)](https://codecov.io/gh/fudanchii/roasted)

Beancount ledger parser, written in Rust.

This is a clean room implementation of interpreter for Beancount ledger file, from a text based double-book accounting system called Beancount.

Roasted has several different behavior compared to Beancount and **not** aimed to be always 100% compatible with Beancount.

Architecture
---

Roasted consisted of 3 components:

- libroasted
  ledger parser, backend for greenflake
- roasted-greenflake
  accounting package with plugins
- roasted-cli
  command line interface for your double book accounting.

License
---

Licensed under either of these:

- Apache License, Version 2.0, https://www.apache.org/licenses/LICENSE-2.0
- MIT license https://opensource.org/licenses/MIT

