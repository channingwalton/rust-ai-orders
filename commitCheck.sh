#!/bin/bash

cargo clean && cargo fmt && cargo clippy -- -D warnings && cargo test -- --include-ignored
