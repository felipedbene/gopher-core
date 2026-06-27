//! The daemon-agnostic menu model and the geomyidae `.gph` serializer.
//!
//! A menu is a `Vec<Entry>`: info lines and links. [`render_menu_index`] turns it
//! into a geomyidae `.gph` index. To target a different daemon (Gophernicus
//! `gophermap`, raw RFC-1436), that one function is all you rewrite.

/// Gopher item type for a link. Daemon-agnostic; serialized per-daemon in
/// [`render_menu_index`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Text,   // gopher type 0
    Menu,   // gopher type 1
    Search, // gopher type 7 -- index-search server; client prompts, re-sends `selector\tquery`
    Url,    // gopher type h -- external link, selector is `URL:<addr>`
    Bin,    // gopher type 9 -- binary download
}

/// One line of a menu: either an info line (not selectable) or a link.
///
/// `host`/`port` are normally `None` — the serializer then emits geomyidae's own
/// host/port placeholder tokens, so the tree stays address-agnostic. They are
/// `Some` only for cross-server links (a hub link to a sibling hole), which must
/// advertise a concrete address the client dials directly. Set them with
/// [`Entry::with_host`].
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Info(String),
    Link {
        kind: ItemKind,
        display: String,
        selector: String,
        host: Option<String>,
        port: Option<u16>,
    },
}

impl Entry {
    /// Stamp a concrete `host`/`port` onto a link (the unified cross-server /
    /// host-override constructor — replaces the old `link_remote`). A no-op on an
    /// info line.
    pub fn with_host(self, host: impl Into<String>, port: u16) -> Entry {
        match self {
            Entry::Link {
                kind,
                display,
                selector,
                ..
            } => Entry::Link {
                kind,
                display,
                selector,
                host: Some(host.into()),
                port: Some(port),
            },
            info => info,
        }
    }
}

/// An info (non-selectable) line.
pub fn info(s: impl Into<String>) -> Entry {
    Entry::Info(s.into())
}

/// A link served from this tree (host/port default to the daemon's own; override
/// with [`Entry::with_host`]).
pub fn link(kind: ItemKind, display: impl Into<String>, selector: impl Into<String>) -> Entry {
    Entry::Link {
        kind,
        display: display.into(),
        selector: selector.into(),
        host: None,
        port: None,
    }
}

/// Serialize a daemon-agnostic menu ([`Entry`] list) into a geomyidae `.gph`
/// index.
///
/// Format (confirmed against geomyidae(8) and the phd implementation): a link is
/// `[<type>|<name>|<selector>|server|port]`; geomyidae substitutes the literal
/// tokens `server`/`port` with its own host/port at serve time, so the files stay
/// host/port-agnostic. A link carrying an explicit `host`/`port` (a cross-server
/// hub link) emits those concrete values instead. Any line not starting with `[`
/// is an info (i) line.
pub fn render_menu_index(entries: &[Entry]) -> String {
    let mut out = String::new();
    for e in entries {
        match e {
            Entry::Info(s) => {
                // Info text that happens to start with '[' would be mis-parsed as
                // a link; a leading space keeps it an info line.
                if s.starts_with('[') {
                    out.push(' ');
                }
                out.push_str(s);
                out.push('\n');
            }
            Entry::Link {
                kind,
                display,
                selector,
                host,
                port,
            } => {
                let t = match kind {
                    ItemKind::Text => '0',
                    ItemKind::Menu => '1',
                    ItemKind::Search => '7',
                    ItemKind::Url => 'h',
                    ItemKind::Bin => '9',
                };
                // `None` -> the literal placeholder tokens geomyidae fills in; an
                // explicit host/port (a cross-server hub link) emits as-is.
                let server = host.as_deref().unwrap_or("server");
                let port_col = match port {
                    Some(p) => p.to_string(),
                    None => "port".to_string(),
                };
                out.push_str(&format!(
                    "[{t}|{}|{}|{}|{}]\n",
                    gph_escape(display),
                    gph_escape(selector),
                    gph_escape(server),
                    port_col,
                ));
            }
        }
    }
    out
}

/// Escape the `.gph` field separator `|` within a field (geomyidae uses `\|`).
fn gph_escape(s: &str) -> String {
    s.replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_index_renders_geomyidae_gph() {
        let entries = vec![
            info("  a phlog"),
            link(ItemKind::Menu, "Posts", "/posts"),
            link(ItemKind::Text, "About", "/about.txt"),
            // a cross-server hub link advertises a concrete host/port
            link(ItemKind::Menu, "Live CTA trains", "/").with_host("gopher.debene.dev", 70),
        ];
        let gph = render_menu_index(&entries);

        assert!(gph.contains("  a phlog\n"));
        assert!(gph.contains("[1|Posts|/posts|server|port]\n"));
        assert!(gph.contains("[0|About|/about.txt|server|port]\n"));
        assert!(gph.contains("[1|Live CTA trains|/|gopher.debene.dev|70]\n"));
        assert!(!gph.contains('\t'));
    }

    #[test]
    fn search_item_renders_type_7() {
        // A type-7 index-search item (e.g. a dcgi the client sends a query to).
        let gph = render_menu_index(&[link(ItemKind::Search, "Ask the deck", "/draw.dcgi")]);
        assert_eq!(gph, "[7|Ask the deck|/draw.dcgi|server|port]\n");
    }

    #[test]
    fn info_line_starting_with_bracket_is_escaped() {
        let gph = render_menu_index(&[info("[not a link]")]);
        assert_eq!(gph, " [not a link]\n");
    }

    #[test]
    fn pipe_in_field_is_escaped() {
        let gph = render_menu_index(&[link(ItemKind::Text, "a|b", "/x")]);
        assert!(gph.contains("[0|a\\|b|/x|server|port]\n"));
    }

    #[test]
    fn with_host_is_noop_on_info() {
        assert_eq!(info("x").with_host("h", 70), info("x"));
    }
}
