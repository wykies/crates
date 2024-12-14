# Wykies Open Source Rust Crates

<!-- TODO 4: Add Cargo Semver Checks to CI at Workspace level (There is supposed to be a workspace flag) -->

These are creates we decided to share are they were not the differentiating part of applications that we develop.
They vary in levels of maturity and speed of development is you are interested in a particular one reach and and we can probably publish stable version to crates.io.

Brief points to be aware of when looking into any creates in this repo:

- Feature flags on crates are documented with comments in their respective `Cargo.toml` files.
- Servers built using this framework need to enable the desired encryption options for the sqlx crate (See [sqlx readme](https://github.com/launchbadge/sqlx?tab=readme-ov-file#install) and [Demo chat server](crates/chat-app-server/Cargo.toml) for an example).
- The plugins are treated as first party code. There is not security separation. If that is needed do NOT give them access to the same database you use for the rest of your application. It was more designed for them to be able to be reused not to be sandboxed. Also pay attention to what routes they are adding to your application.

<!-- TODO 5 Document what tables each plugin uses (probably in their lib.rs, bonus points if it's automated so it stays updated) -->

## License

All code in this repository is dual-licensed under either:

- Apache License, Version 2.0
- MIT license

at your option.
This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are very good reasons to include both as noted in
this [issue](https://github.com/bevyengine/bevy/issues/2373) on [Bevy](https://bevyengine.org)'s repo.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

## Credits

- Chat App Icon: Designed by Freepik <http://www.freepik.com/>
- WebSocket security based on a reading a [heroku](https://devcenter.heroku.com/articles/websocket-security) article that that cites the original source as [Armin Ronacher](https://lucumr.pocoo.org/2012/9/24/websockets-101/).
- egui chat client borrowed inspiration from <egui.rs> and the [WebSockets](https://github.com/rerun-io/ewebsock) example app.
- Server design in based on the Book and accompanying code for [Zero To Production In Rust](https://www.zero2prod.com/)
- Design of the chat server started from the example in the [Actix Web Examples](https://github.com/actix/examples/tree/master/websockets).
