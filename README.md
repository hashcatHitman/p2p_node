<!--
SPDX-FileCopyrightText: Copyright © 2025 hashcatHitman

SPDX-License-Identifier: Apache-2.0 OR MIT
-->

# Peer-to-peer Node

[![unsafe forbidden]][safety dance] [![dependency badge]][deps.rs] [![CI status]][CI workflow] [![CodeQL]][CodeQL workflow]

---

A peer-to-peer node for the system we created in my Distributed Computing class.

## Getting Started

You'll need to install Rust and its package manager, Cargo, on your system.
Please refer to the official [recommended Rust installation method] for your
system.

You should also have some version of git installed. You can refer to the
[Git documentation] if you need help with that.

Clone the repository and navigate inside it:

```bash
git clone https://github.com/hashcatHitman/p2p_node.git
cd p2p_node
```

If you'd like to read the documentation, the recommended way to do so is with:

```bash
cargo doc --document-private-items --open
```

Which will open the documentation in your browser.

To build the project, you can do:

```bash
cargo build --profile release --locked
```

Cargo will download the dependencies and compile the project. It will probably
be located at `./target/release/p2p_node` or
`./target/release/p2p_node.exe`, depending on your system.

## MSRV Policy

<!-- Adapted from Arti's MSRV policy -->

Our current Minimum Supported Rust Version (MSRV) is 1.93.1.

We may increase the patch level of the MSRV on any release.

Otherwise, we will not increase MSRV on PATCH releases, though our dependencies
might.

We won't increase MSRV just because we can: we'll only do so when we have a
reason. (We don't guarantee that you'll agree with our reasoning; only that
it will exist.)

[unsafe forbidden]: https://img.shields.io/badge/unsafe-forbidden-success.svg
[safety dance]: https://github.com/rust-secure-code/safety-dance/

[dependency badge]: https://deps.rs/repo/github/hashcatHitman/p2p_node/status.svg
[deps.rs]: https://deps.rs/repo/github/hashcatHitman/p2p_node

[CI status]: https://github.com/hashcatHitman/p2p_node/actions/workflows/ci.yml/badge.svg
[CI workflow]: https://github.com/hashcatHitman/p2p_node/actions/workflows/ci.yml

[CodeQL]: https://github.com/hashcatHitman/p2p_node/actions/workflows/github-code-scanning/codeql/badge.svg
[CodeQL workflow]: https://github.com/hashcatHitman/p2p_node/actions/workflows/github-code-scanning/codeql

[recommended Rust installation method]: https://www.rust-lang.org/tools/install

[Git documentation]: https://git-scm.com/book/en/v2/Getting-Started-Installing-Git
