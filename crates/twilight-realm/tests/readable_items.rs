//! Readable items (#290): a quest item with `pages` answers its item query with a `page_text` id
//! and Parchment, and a page text query with the chain of pages from that id on. The items here
//! are test-only, added through `Realm::with_items`.

mod common;

use benilla_protocol::SessionEvent;
use common::*;

const TWO_PAGES: u32 = 990101;
const NO_PAGES: u32 = 990102;

const TEST_ITEMS: &str = r#"
item = [
  { entry = 990101, name = "Test Letter", quality = 1, item_level = 1, class = 12, display_id = 7089, max_count = 1, pages = ["First page.", "Second page."] },
  { entry = 990102, name = "Test Pebble", quality = 1, item_level = 1, class = 12, display_id = 7089, max_count = 1 },
]
"#;

fn realm() -> Realm {
    Realm::new().with_items(TEST_ITEMS)
}

fn pages(realm: &mut Realm, page_id: u32) -> Vec<(u32, String, u32)> {
    realm.handle(RealmCommand::PageTextQuery { page_id });
    drain(realm)
        .into_iter()
        .filter_map(|e| match e {
            SessionEvent::PageText {
                page_id,
                text,
                next_page_id,
            } => Some((page_id, text, next_page_id)),
            _ => None,
        })
        .collect()
}

fn page_fields(realm: &mut Realm, entry: u32) -> (u32, u32) {
    realm.handle(RealmCommand::ItemQuery { entry });
    drain(realm)
        .into_iter()
        .find_map(|e| match e {
            SessionEvent::ItemTemplate {
                info: Some(info), ..
            } => Some((info.page_text, info.page_material)),
            _ => None,
        })
        .expect("the item answers")
}

#[test]
fn an_item_with_pages_answers_its_first_page_id_and_parchment() {
    let mut realm = realm();
    assert_eq!(page_fields(&mut realm, TWO_PAGES), (TWO_PAGES * 16, 1));
}

#[test]
fn a_page_text_query_answers_the_chain_in_order() {
    let mut realm = realm();
    let first = TWO_PAGES * 16;
    assert_eq!(
        pages(&mut realm, first),
        vec![
            (first, "First page.".to_string(), first + 1),
            (first + 1, "Second page.".to_string(), 0),
        ]
    );
    assert_eq!(
        pages(&mut realm, first + 1),
        vec![(first + 1, "Second page.".to_string(), 0)]
    );
}

#[test]
fn an_item_without_pages_is_not_readable() {
    let mut realm = realm();
    assert_eq!(page_fields(&mut realm, NO_PAGES), (0, 0));
    assert!(pages(&mut realm, NO_PAGES * 16).is_empty());
}

#[test]
fn an_unknown_page_id_answers_nothing() {
    let mut realm = realm();
    assert!(pages(&mut realm, TWO_PAGES * 16 + 2).is_empty());
    assert!(pages(&mut realm, 5).is_empty());
}
