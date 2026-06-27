//! `gopher-core` — the daemon-agnostic gopher mechanics shared by `gopher-cta`
//! and `gopher-blog`.
//!
//! This is the spine that was copied between the two daemons (under
//! `EXTRACTION CANDIDATE` markers) until [`Entry`]'s `host`/`port` API settled.
//! It is deliberately tiny and **std-only** — no serde, no external deps:
//!
//! - the [menu model][Entry] (`Entry`/`ItemKind` + the `info`/`link` builders),
//! - the [`render_menu_index`] `Entry` -> geomyidae `.gph` serializer, and
//! - the atomic file-publish primitives ([`new_snapshot`], [`flip_current`],
//!   [`gc`], and the [`publish`] convenience).
//!
//! Domain logic (train maps, markdown, page layout) stays in the consumers.

mod menu;
mod publish;

pub use menu::{info, link, render_menu_index, Entry, ItemKind};
pub use publish::{flip_current, gc, new_snapshot, publish, write_files, TreeFile};
