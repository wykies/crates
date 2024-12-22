#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
package.edition = "2021"
[dependencies]
switch-db ={version = "*", path = "../crates/switch-db"}
anyhow = "1.0.94"
---

fn main() -> anyhow::Result<()> {
    switch_db::run()
}