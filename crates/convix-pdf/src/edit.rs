//! Structure-level PDF tools with lopdf: merge, split / extract pages, compress.

use convix_core::{Error, Result};
use lopdf::{Dictionary, Document, Object, ObjectId, SaveOptions};
use std::{collections::BTreeMap, path::Path};

pub fn load(path: &Path) -> Result<Document> {
    let doc = Document::load(path).map_err(|e| {
        let text = e.to_string().to_lowercase();
        if text.contains("password") || text.contains("decrypt") || text.contains("encrypt") {
            Error::Failed("ce PDF est protégé par un mot de passe".into())
        } else {
            Error::Failed(format!("PDF illisible ou endommagé ({e})"))
        }
    })?;
    if doc.get_pages().is_empty() {
        return Err(Error::Failed("ce PDF ne contient aucune page".into()));
    }
    Ok(doc)
}

/// Smallest file: object streams + compressed cross-reference (PDF 1.5, read everywhere).
pub fn to_bytes(doc: &mut Document) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let options = SaveOptions { use_object_streams: true, use_xref_streams: true, ..Default::default() };
    doc.save_with_options(&mut out, options)?;
    Ok(out)
}

/// Lossless clean-up: unused objects dropped, every stream compressed.
pub fn tidy(doc: &mut Document) {
    doc.prune_objects();
    doc.delete_zero_length_streams();
    doc.compress();
    doc.renumber_objects();
}

/// Attributes a page may inherit from its parents in the page tree.
const INHERITABLE: &[&[u8]] = &[b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

/// Copy inherited attributes into each page, so pages can move to another page tree.
fn flatten_inherited(doc: &mut Document) {
    let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for page_id in pages {
        let mut missing: Vec<(Vec<u8>, Object)> = Vec::new();
        let Ok(page) = doc.get_dictionary(page_id) else { continue };
        for key in INHERITABLE {
            if page.has(key) {
                continue;
            }
            // Walk up the Parent chain (bounded: malformed files can loop).
            let mut parent = page.get(b"Parent").and_then(Object::as_reference).ok();
            for _ in 0..32 {
                let Some(id) = parent else { break };
                let Ok(node) = doc.get_dictionary(id) else { break };
                if let Ok(value) = node.get(key) {
                    missing.push((key.to_vec(), value.clone()));
                    break;
                }
                parent = node.get(b"Parent").and_then(Object::as_reference).ok();
            }
        }
        if let Ok(page) = doc.get_dictionary_mut(page_id) {
            for (key, value) in missing {
                page.set(key, value);
            }
        }
    }
}

/// Merge documents in order into one.
pub fn merge(mut docs: Vec<Document>) -> Result<Document> {
    let mut merged = Document::with_version("1.7");
    let mut max_id = 1;
    let mut page_ids: Vec<ObjectId> = Vec::new();
    let mut objects: BTreeMap<ObjectId, Object> = BTreeMap::new();

    for doc in &mut docs {
        flatten_inherited(doc);
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;
        page_ids.extend(doc.get_pages().into_values());
        objects.extend(std::mem::take(&mut doc.objects));
    }

    let pages_id = (max_id, 0);
    let catalog_id = (max_id + 1, 0);
    let skip_types: &[&[u8]] = &[b"Catalog", b"Pages", b"Outlines", b"Outline"];
    for (id, object) in objects {
        let kind = object.as_dict().ok().and_then(|d| d.get(b"Type").ok()).and_then(|t| t.as_name().ok());
        if kind.is_some_and(|k| skip_types.contains(&k)) {
            continue; // replaced by our own tree; bookmarks would point to the old tree
        }
        merged.objects.insert(id, object);
    }
    for id in &page_ids {
        if let Ok(page) = merged.get_dictionary_mut(*id) {
            page.set("Parent", pages_id);
        }
    }

    let mut pages = Dictionary::new();
    pages.set("Type", "Pages");
    pages.set("Count", page_ids.len() as i64);
    pages.set("Kids", page_ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>());
    merged.objects.insert(pages_id, Object::Dictionary(pages));

    let mut catalog = Dictionary::new();
    catalog.set("Type", "Catalog");
    catalog.set("Pages", pages_id);
    merged.objects.insert(catalog_id, Object::Dictionary(catalog));
    merged.trailer.set("Root", catalog_id);
    merged.max_id = max_id + 1;

    tidy(&mut merged);
    Ok(merged)
}

/// Prepares a document for [`subset`]: inherited page attributes are copied into each page.
/// Call once, then take as many subsets as needed.
pub fn prepare_for_subsets(doc: &mut Document) {
    flatten_inherited(doc);
}

/// A new document with only `keep` (1-based page numbers), **in that order**.
///
/// Only the objects those pages use are copied (contents, fonts, images…), never the whole
/// file: splitting a 300-page PDF no longer clones it 300 times.
/// `doc` must have gone through [`prepare_for_subsets`].
pub fn subset(doc: &Document, keep: &[u32]) -> Document {
    let numbers = doc.get_pages();
    let page_ids: Vec<ObjectId> = keep.iter().filter_map(|n| numbers.get(n).copied()).collect();
    let all_pages: std::collections::HashSet<ObjectId> = numbers.values().copied().collect();

    // Everything reachable from the kept pages, without climbing to the page tree (Parent)
    // or jumping to pages we drop (links, bookmarks pointing elsewhere).
    let mut needed: std::collections::BTreeSet<ObjectId> = page_ids.iter().copied().collect();
    let mut todo: Vec<ObjectId> = page_ids.clone();
    while let Some(id) = todo.pop() {
        let Ok(object) = doc.get_object(id) else { continue };
        let mut refs = Vec::new();
        collect_refs(object, &mut refs);
        for r in refs {
            if all_pages.contains(&r) && !page_ids.contains(&r) {
                continue;
            }
            if needed.insert(r) {
                todo.push(r);
            }
        }
    }

    let mut out = Document::with_version(doc.version.as_str());
    for id in &needed {
        if let Ok(object) = doc.get_object(*id) {
            out.objects.insert(*id, object.clone());
        }
    }
    let max_id = doc.max_id;
    let (pages_id, catalog_id) = ((max_id + 1, 0), (max_id + 2, 0));
    for id in &page_ids {
        if let Ok(page) = out.get_dictionary_mut(*id) {
            page.set("Parent", pages_id);
        }
    }
    let mut pages = Dictionary::new();
    pages.set("Type", "Pages");
    pages.set("Count", page_ids.len() as i64);
    pages.set("Kids", page_ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>());
    out.objects.insert(pages_id, Object::Dictionary(pages));
    let mut catalog = Dictionary::new();
    catalog.set("Type", "Catalog");
    catalog.set("Pages", pages_id);
    out.objects.insert(catalog_id, Object::Dictionary(catalog));
    out.trailer.set("Root", catalog_id);
    out.max_id = max_id + 2;

    tidy(&mut out);
    out
}

/// References inside an object. A page's link back to the page tree (`/Parent` of a
/// `/Page` or `/Pages` node) is not followed: it would pull in every other page.
/// Other `/Parent` links (form fields, popups) are kept.
fn collect_refs(object: &Object, out: &mut Vec<ObjectId>) {
    let dict_refs = |d: &Dictionary, out: &mut Vec<ObjectId>| {
        let is_tree_node = matches!(d.get(b"Type").and_then(Object::as_name), Ok(b"Page" | b"Pages"));
        for (k, v) in d.iter() {
            if !(is_tree_node && k.as_slice() == b"Parent") {
                collect_refs(v, out);
            }
        }
    };
    match object {
        Object::Reference(id) => out.push(*id),
        Object::Array(items) => items.iter().for_each(|o| collect_refs(o, out)),
        Object::Dictionary(d) => dict_refs(d, out),
        Object::Stream(s) => dict_refs(&s.dict, out),
        _ => {}
    }
}
