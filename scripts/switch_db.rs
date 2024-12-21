#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
package.edition = "2021"
[dependencies]
switch-db-lib ={version = "*", path = "../crates/switch-db-lib"}
anyhow = "1.0.94"
---

fn main() -> anyhow::Result<()> {
    switch_db_lib::run()
}