// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Mohammed Maruff (Esan Maruff) <esan@etamil.in>
//! Canonical XML, which is what an XML signature is actually over.
//!
//! ## Why this exists before any signing does
//!
//! An XML signature does not sign the bytes you received. Two documents that
//! any parser calls identical can differ in bytes — attribute order, `<a/>`
//! against `<a></a>`, which namespace declarations are written where, whether
//! the encoding wrote `>` or `&gt;`. So the signature is over a *canonical*
//! serialisation, and both ends have to produce the same one from the same
//! tree or nothing verifies.
//!
//! That makes canonicalization the part where being wrong is invisible. A
//! signature over a nearly-right canonical form verifies against itself all day
//! and fails against every other implementation in the world — which for an
//! Aadhaar response or an ISO 20022 payment is discovered in production, by the
//! counterparty. It is implemented here first, and on its own, so it can be
//! held against the worked examples in the specification rather than against
//! the rest of a signature stack that would hide it.
//!
//! ## What is implemented
//!
//! Canonical XML 1.0 (<https://www.w3.org/TR/xml-c14n>) and Exclusive XML
//! Canonicalization 1.0 (<https://www.w3.org/TR/xml-exc-c14n>), both without
//! comments and both over a whole element subtree.
//!
//! The difference is the one that matters in practice. Inclusive carries every
//! namespace in scope down into the signed subtree, so moving a signed element
//! into a document that declares namespaces differently breaks it. Exclusive
//! carries only the declarations the subtree actually uses, which is why a
//! signed assertion can be enveloped in somebody else's message at all — and it
//! is what SAML, and Aadhaar's responses, use.
//!
//! Not implemented: C14N 1.1, comment-preserving variants, and the
//! `InclusiveNamespaces` PrefixList that exclusive canonicalization allows.
//! Each changes the output, so each is refused by name rather than approximated.

use std::collections::BTreeMap;

/// Which canonical form to produce.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    /// Canonical XML 1.0. Every namespace in scope is rendered on the top
    /// element of the subtree.
    Inclusive,
    /// Exclusive XML Canonicalization 1.0. Only namespaces a given element
    /// actually uses are rendered on it.
    Exclusive,
}

/// Canonicalize `source`, or the element named by `id` inside it.
///
/// `id` is matched against an `Id`, `id` or `ID` attribute, which is how an
/// XML signature's `Reference URI="#..."` names what it covers.
pub fn canonicalize(source: &str, id: Option<&str>, form: Form) -> Result<String, String> {
    let document = roxmltree::Document::parse(source).map_err(|why| {
        format!(
            "XML படிக்க முடியவில்லை: {}  (the XML does not parse: {})",
            why, why
        )
    })?;

    let root = match id {
        None => document.root_element(),
        Some(wanted) => find_by_id(document.root_element(), wanted).ok_or_else(|| {
            format!(
                "'{}' என்ற அடையாளத்துடன் உறுப்பு இல்லை  (no element carries that id)",
                wanted
            )
        })?,
    };

    let mut out = String::new();
    // The rendered namespace context an ancestor would have left. Empty at the
    // top of the subtree, which is exactly what makes a signed fragment
    // independent of where it was found.
    let rendered: BTreeMap<String, String> = BTreeMap::new();
    write_element(root, form, &rendered, &mut out)?;
    Ok(out)
}

/// The first element with that local name: its canonical form and its text.
///
/// Verifying a signature means reading values back out of the document — the
/// SignedInfo to check the signature over, the DigestValue to compare against.
/// Local name rather than qualified, because the prefix a document chose for
/// the XML Signature namespace is its own business.
pub fn find_element(
    source: &str,
    local_name: &str,
    form: Form,
) -> Result<(String, String), String> {
    let document = roxmltree::Document::parse(source).map_err(|why| {
        format!(
            "XML படிக்க முடியவில்லை: {}  (the XML does not parse: {})",
            why, why
        )
    })?;
    let found = by_local_name(document.root_element(), local_name)
        .ok_or_else(|| format!("'{}' என்ற உறுப்பு இல்லை  (no element named that)", local_name))?;

    let mut canonical = String::new();
    write_element(found, form, &BTreeMap::new(), &mut canonical)?;

    let text: String = found
        .descendants()
        .filter(roxmltree::Node::is_text)
        .filter_map(|node| node.text())
        .collect();
    Ok((canonical, text))
}

fn by_local_name<'a>(
    node: roxmltree::Node<'a, 'a>,
    wanted: &str,
) -> Option<roxmltree::Node<'a, 'a>> {
    if node.tag_name().name() == wanted {
        return Some(node);
    }
    node.children()
        .filter(roxmltree::Node::is_element)
        .find_map(|child| by_local_name(child, wanted))
}

fn find_by_id<'a>(node: roxmltree::Node<'a, 'a>, wanted: &str) -> Option<roxmltree::Node<'a, 'a>> {
    for attribute in node.attributes() {
        let name = attribute.name();
        if (name == "Id" || name == "id" || name == "ID") && attribute.value() == wanted {
            return Some(node);
        }
    }
    node.children()
        .filter(roxmltree::Node::is_element)
        .find_map(|child| find_by_id(child, wanted))
}

fn write_element(
    element: roxmltree::Node,
    form: Form,
    inherited: &BTreeMap<String, String>,
    out: &mut String,
) -> Result<(), String> {
    let qname = qualified(element);

    // --- namespace declarations ---
    //
    // Sorted by prefix, with the default namespace first, because the
    // specification orders namespace nodes by prefix and the default one has
    // none. BTreeMap does that ordering on its own.
    let mut declare: BTreeMap<String, String> = BTreeMap::new();
    for namespace in element.namespaces() {
        let prefix = namespace.name().unwrap_or("").to_string();
        let uri = namespace.uri().to_string();

        let wanted = match form {
            // Inclusive renders everything in scope that an ancestor has not
            // already rendered with the same value.
            Form::Inclusive => true,
            // Exclusive renders only what this element visibly uses: its own
            // prefix, or the prefix of one of its attributes. This is the whole
            // difference, and the reason a signed subtree survives being moved.
            Form::Exclusive => visibly_used(element, &prefix),
        };
        if wanted && inherited.get(&prefix) != Some(&uri) {
            declare.insert(prefix, uri);
        }
    }

    out.push('<');
    out.push_str(&qname);
    for (prefix, uri) in &declare {
        if prefix.is_empty() {
            out.push_str(" xmlns=\"");
        } else {
            out.push_str(" xmlns:");
            out.push_str(prefix);
            out.push_str("=\"");
        }
        out.push_str(&escape_attribute(uri));
        out.push('"');
    }

    // --- attributes ---
    //
    // Sorted by namespace URI then local name, so an attribute with no
    // namespace sorts before any that has one.
    let mut attributes: Vec<(String, String, String)> = element
        .attributes()
        .map(|a| {
            (
                a.namespace().unwrap_or("").to_string(),
                a.name().to_string(),
                a.value().to_string(),
            )
        })
        .collect();
    attributes.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));

    for (uri, local, value) in &attributes {
        let shown = if uri.is_empty() {
            local.clone()
        } else {
            let prefix = element
                .lookup_prefix(uri.as_str())
                .ok_or_else(|| format!("'{}' க்கு முன்னொட்டு இல்லை  (no prefix for {})", uri, uri))?;
            format!("{}:{}", prefix, local)
        };
        out.push(' ');
        out.push_str(&shown);
        out.push_str("=\"");
        out.push_str(&escape_attribute(value));
        out.push('"');
    }
    out.push('>');

    // What this element leaves in scope for its children.
    let mut passed = inherited.clone();
    for (prefix, uri) in declare {
        passed.insert(prefix, uri);
    }

    for child in element.children() {
        if child.is_element() {
            write_element(child, form, &passed, out)?;
        } else if child.is_text() {
            out.push_str(&escape_text(child.text().unwrap_or("")));
        } else if child.is_pi() {
            // A processing instruction keeps its target and data, and an empty
            // data section leaves no trailing space.
            let pi = child.pi().expect("a pi node has pi data");
            out.push_str("<?");
            out.push_str(pi.target);
            if let Some(value) = pi.value {
                out.push(' ');
                out.push_str(value);
            }
            out.push_str("?>");
        }
        // Comments are dropped: both forms implemented here are the
        // without-comments variants, which is what a signature uses.
    }

    // Always the long form. `<a/>` and `<a></a>` are the same element and have
    // to canonicalize to the same bytes, so one of them has to win.
    out.push_str("</");
    out.push_str(&qname);
    out.push('>');
    Ok(())
}

/// Is `prefix` used by this element's own name or by one of its attributes?
///
/// "Visibly utilised" in the specification's words. An attribute's prefix
/// counts; a prefix used only by a descendant does not, because the descendant
/// will render it itself.
fn visibly_used(element: roxmltree::Node, prefix: &str) -> bool {
    let own = element
        .tag_name()
        .namespace()
        .and_then(|uri| element.lookup_prefix(uri));
    if own.unwrap_or("") == prefix {
        return true;
    }
    // An element with no namespace visibly uses the default declaration only
    // when that declaration is what puts it in no namespace.
    if prefix.is_empty() && element.tag_name().namespace().is_none() {
        return false;
    }
    element.attributes().any(|attribute| {
        attribute
            .namespace()
            .and_then(|uri| element.lookup_prefix(uri))
            .map(|found| found == prefix)
            .unwrap_or(false)
    })
}

/// Text content. `>` is escaped even though a parser does not require it,
/// because the specification says so and a signature is over bytes.
fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            other => out.push(other),
        }
    }
    out
}

/// Attribute values additionally escape the quote and the whitespace that an
/// ordinary parser would otherwise normalise away.
fn escape_attribute(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            other => out.push(other),
        }
    }
    out
}

/// The name as it is written: prefix and all.
///
/// `ExpandedName` gives the namespace and the local name, which is what a
/// parser cares about — but a canonical serialisation is bytes, and the bytes
/// carry the prefix the document chose.
fn qualified(element: roxmltree::Node) -> String {
    let local = element.tag_name().name();
    match element.tag_name().namespace() {
        None => local.to_string(),
        Some(uri) => match element.lookup_prefix(uri) {
            Some(prefix) if !prefix.is_empty() => format!("{}:{}", prefix, local),
            _ => local.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exc(source: &str) -> String {
        canonicalize(source, None, Form::Exclusive).unwrap()
    }
    fn inc(source: &str) -> String {
        canonicalize(source, None, Form::Inclusive).unwrap()
    }

    /// `<a/>` and `<a></a>` are the same element, so they have to become the
    /// same bytes. One of the two forms has to win and the specification picks
    /// the long one.
    #[test]
    fn an_empty_element_is_written_long() {
        assert_eq!(exc("<a/>"), "<a></a>");
        assert_eq!(exc("<a></a>"), "<a></a>");
    }

    /// Sorted by namespace URI then local name, so an attribute in no
    /// namespace sorts before any that has one — whatever order they arrived.
    #[test]
    fn attributes_come_out_in_the_specified_order() {
        assert_eq!(
            exc(r#"<a z="1" b="2" xmlns:p="http://p" p:c="3"/>"#),
            r#"<a xmlns:p="http://p" b="2" z="1" p:c="3"></a>"#
        );
    }

    /// `>` needs no escaping to parse, and is escaped anyway: a signature is
    /// over bytes, so both ends have to choose the same bytes.
    #[test]
    fn text_escapes_what_the_specification_says_to() {
        assert_eq!(
            exc("<a>1 &lt; 2 &amp; 3 > 4</a>"),
            "<a>1 &lt; 2 &amp; 3 &gt; 4</a>"
        );
    }

    /// An attribute value additionally escapes the quote and the whitespace an
    /// ordinary parser would normalise away.
    #[test]
    fn attribute_values_escape_whitespace() {
        assert_eq!(exc("<a b=\"x&#9;y\"/>"), r#"<a b="x&#x9;y"></a>"#);
        assert_eq!(
            exc("<a b=\"say &quot;no&quot;\"/>"),
            r#"<a b="say &quot;no&quot;"></a>"#
        );
    }

    /// Both forms here are the without-comments variants, which is what a
    /// signature uses.
    #[test]
    fn comments_are_dropped() {
        assert_eq!(exc("<a><!-- gone -->text</a>"), "<a>text</a>");
    }

    /// The difference between the two forms, and the reason exclusive exists:
    /// a declaration nothing in the subtree uses is not carried along, so the
    /// subtree can be moved into another document and still verify.
    #[test]
    fn exclusive_drops_a_namespace_nothing_uses() {
        assert_eq!(exc(r#"<a xmlns:unused="http://u">x</a>"#), "<a>x</a>");
        assert_eq!(
            inc(r#"<a xmlns:unused="http://u">x</a>"#),
            r#"<a xmlns:unused="http://u">x</a>"#
        );
    }

    /// The specification's own shape: canonicalizing an inner element, the
    /// ancestors' declarations do not come with it under exclusive, and the
    /// declaration a descendant needs is rendered on that descendant.
    #[test]
    fn exclusive_renders_each_declaration_where_it_is_used() {
        let document = r#"<n0:local xmlns:n0="http://a" xmlns:n3="ftp://x"><n1:elem2 xmlns:n1="http://b" Id="signed"><n3:stuff/></n1:elem2></n0:local>"#;
        assert_eq!(
            canonicalize(document, Some("signed"), Form::Exclusive).unwrap(),
            r#"<n1:elem2 xmlns:n1="http://b" Id="signed"><n3:stuff xmlns:n3="ftp://x"></n3:stuff></n1:elem2>"#
        );
    }

    /// Inclusive carries every namespace in scope onto the top of the subtree,
    /// which is exactly what makes a signed fragment depend on where it was.
    #[test]
    fn inclusive_carries_the_ancestors_declarations_in() {
        let document = r#"<n0:local xmlns:n0="http://a" xmlns:n3="ftp://x"><n1:elem2 xmlns:n1="http://b" Id="signed"><n3:stuff/></n1:elem2></n0:local>"#;
        let out = canonicalize(document, Some("signed"), Form::Inclusive).unwrap();
        assert!(out.contains(r#"xmlns:n0="http://a""#), "{}", out);
        assert!(out.contains(r#"xmlns:n3="ftp://x""#), "{}", out);
    }

    /// A Reference names what it covers by id, so finding the wrong element —
    /// or none — has to be an error rather than a signature over the document.
    #[test]
    fn an_unknown_id_is_refused() {
        assert!(canonicalize("<a Id=\"x\"/>", Some("y"), Form::Exclusive).is_err());
    }

    #[test]
    fn xml_that_does_not_parse_is_refused() {
        assert!(canonicalize("<a>", None, Form::Exclusive).is_err());
    }
}
