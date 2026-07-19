# gopher-core

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE) ![Rust](https://img.shields.io/badge/Rust-std--only-orange.svg)

The daemon-agnostic gopher mechanics shared by
[`gopher-cta`](https://github.com/felipedbene/gopher-cta) (live CTA trains over
gopher) and [`gopher-blog`](https://github.com/felipedbene/gopher-blog) (the
debene.dev phlog). Extracted from the spine both daemons had copied verbatim, once
the [`Entry`] `host`/`port` API settled.

Deliberately tiny and **std-only** — no serde, no external deps:

- **Menu model** — `Entry` / `ItemKind` and the `info` / `link` builders;
  `Entry::with_host` stamps a concrete host/port on a cross-server link.
  `ItemKind` covers `Text` (0), `Menu` (1), `Search` (7, index-search),
  `Url` (h, external link) and `Bin` (9, binary download).
- **Serializer** — `render_menu_index`: an `Entry` list -> a geomyidae `.gph`
  index. (Targeting another daemon = rewrite only this.)
- **Publish primitives** — `new_snapshot` (`out-<ts>/`), `write_files`,
  `flip_current` (atomic symlink rename), `gc(out, keep)` (keep newest `keep` +
  whatever `current` resolves to), and the `publish` convenience.

## Use

```toml
gopher-core = { git = "https://github.com/felipedbene/gopher-core", tag = "v0.2.0" }
```

```rust
use gopher_core::{info, link, ItemKind, render_menu_index, publish};

let menu = vec![
    info("  my hole"),
    link(ItemKind::Menu, "Posts", "/posts"),
    link(ItemKind::Menu, "Other hole", "/").with_host("other.example", 70),
];
let gph = render_menu_index(&menu);            // -> geomyidae .gph
// publish(out_dir, &[(path, bytes)], keep)    // atomic snapshot + flip + gc
```

Each consumer pins its own tag and bumps when it chooses, so a core change never
auto-propagates into a live build.

## License

MIT — see [LICENSE](LICENSE).

---
### Part of the gopher constellation
**Servers & tools:** [gopher-core](https://github.com/felipedbene/gopher-core) · [gopher-cta](https://github.com/felipedbene/gopher-cta) · [gopher-blog](https://github.com/felipedbene/gopher-blog) · [gopher-askthedeck](https://github.com/felipedbene/gopher-askthedeck) · [gopher-spot](https://github.com/felipedbene/gopher-spot) · [the-economist-epub](https://github.com/felipedbene/the-economist-epub)
**Clients:** [casquinha](https://github.com/felipedbene/casquinha) (Mac OS 9) · [detoca](https://github.com/felipedbene/detoca) (OS X 10.6) · [degelato](https://github.com/felipedbene/degelato) (OS X 10.5 PPC) · [deburrow](https://github.com/felipedbene/deburrow) (Android)
**Protocol notes:** [fhb](https://github.com/felipedbene/fhb)
---
