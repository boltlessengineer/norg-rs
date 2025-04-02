use crate::{
    block::{ListItem, NorgBlock},
    inline::{Attribute, NorgInline},
};

pub fn parse(text: &[u8]) -> Vec<NorgBlock> {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_norg::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("Error loading Norg parser");
    let tree = parser.parse(&text, None).unwrap();
    let root = tree.root_node();
    dbg!(root.to_sexp());
    tsnode_to_blocks(root, text)
}

fn tsnode_to_blocks(node: tree_sitter::Node, text: &[u8]) -> Vec<NorgBlock> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .map(|node| match node.kind() {
            "section" => {
                let heading_node = node.child_by_field_name("heading").unwrap();
                let prefix_node = heading_node.child(0).unwrap();
                let prefix_count = prefix_node.utf8_text(text).unwrap().len();
                let title_node = heading_node.child(1);
                let title = title_node.map(|node| tsnode_to_inlines(node, text));
                Some(NorgBlock::Section {
                    attrs: None,
                    level: prefix_count as u16,
                    heading: title,
                    contents: tsnode_to_blocks(node, text),
                })
            }
            "paragraph" => Some(NorgBlock::Paragraph {
                attrs: None,
                inlines: tsnode_to_inlines(node, text),
            }),
            "infirm_tag" => {
                let name = node
                    .child_by_field_name("name")
                    .unwrap()
                    .utf8_text(text)
                    .unwrap()
                    .to_string();
                let raw_param = node
                    .child_by_field_name("param")
                    .map(|node| node.utf8_text(text).unwrap().to_string());
                Some(NorgBlock::InfirmTag {
                    name,
                    params: raw_param,
                })
            }
            "ranged_tag" => {
                let name = node
                    .child_by_field_name("name")
                    .unwrap()
                    .utf8_text(text)
                    .unwrap()
                    .to_string();
                let raw_param = node
                    .child_by_field_name("param")
                    .map(|node| node.utf8_text(text).unwrap().to_string());
                let mut cursor = node.walk();
                let lines = node
                    .children_by_field_name("line", &mut cursor)
                    .map(|node| node.utf8_text(text).unwrap().to_string())
                    .collect();
                Some(NorgBlock::RangedTag {
                    name,
                    params: raw_param,
                    content: lines,
                })
            }
            "carryover_tag" => {
                let name = node
                    .child_by_field_name("name")
                    .unwrap()
                    .utf8_text(text)
                    .unwrap()
                    .to_string();
                let raw_param = node
                    .child_by_field_name("param")
                    .map(|node| node.utf8_text(text).unwrap().to_string());
                Some(NorgBlock::CarryoverTag {
                    name,
                    params: raw_param,
                })
            }
            "unordered_list" => {
                let prefix_node = node.child(0).unwrap().child(0).unwrap();
                let prefix_count = prefix_node.utf8_text(text).unwrap().len();
                Some(NorgBlock::UnorderedList {
                    attrs: None,
                    level: prefix_count as u16,
                    items: {
                        let mut cursor = node.walk();
                        node.named_children(&mut cursor)
                            .map(|node| ListItem {
                                params: None,
                                contents: tsnode_to_blocks(node, text),
                            })
                            .collect()
                    },
                })
            }
            "ordered_list" => {
                let prefix_node = node.child(0).unwrap().child(0).unwrap();
                let prefix_count = prefix_node.utf8_text(text).unwrap().len();
                Some(NorgBlock::OrderedList {
                    attrs: None,
                    level: prefix_count as u16,
                    items: {
                        let mut cursor = node.walk();
                        node.named_children(&mut cursor)
                            .map(|node| ListItem {
                                params: None,
                                contents: tsnode_to_blocks(node, text),
                            })
                            .collect()
                    },
                })
            }
            "quote" => {
                let prefix_node = node.child(0).unwrap().child(0).unwrap();
                let prefix_count = prefix_node.utf8_text(text).unwrap().len();
                Some(NorgBlock::Quote {
                    attrs: None,
                    level: prefix_count as u16,
                    items: {
                        let mut cursor = node.walk();
                        node.named_children(&mut cursor)
                            .map(|node| ListItem {
                                params: None,
                                contents: tsnode_to_blocks(node, text),
                            })
                            .collect()
                    },
                })
            }
            _ => None,
        })
        .flatten()
        .collect()
}

fn tsnode_to_inlines(node: tree_sitter::Node, text: &[u8]) -> Vec<NorgInline> {
    let mut cursor = node.walk();
    use NorgInline::*;
    node.named_children(&mut cursor)
        .map(|node| match node.kind() {
            "whitespace" => Some(Whitespace),
            "soft_break" => Some(SoftBreak),
            "word" => {
                let text = node.utf8_text(text).unwrap().to_string();
                Some(Text(text))
            }
            "punctuation" => {
                let text = node.utf8_text(text).unwrap().to_string();
                Some(Special(text))
            }
            "escape_sequence" => {
                let character = node.utf8_text(text).unwrap().chars().nth(1).unwrap();
                Some(Escape(character))
            }
            // TODO: add attributes
            "bold" => Some(Bold {
                attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                markup: tsnode_to_inlines(node, text),
            }),
            "italic" => Some(Italic {
                attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                markup: tsnode_to_inlines(node, text),
            }),
            "underline" => Some(Underline {
                attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                markup: tsnode_to_inlines(node, text),
            }),
            "strikethrough" => Some(Strikethrough {
                attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                markup: tsnode_to_inlines(node, text),
            }),
            "verbatim" => Some(Verbatim {
                attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                markup: tsnode_to_inlines(node, text),
            }),
            "inline_macro" => {
                let name = node
                    .child_by_field_name("name")
                    .unwrap()
                    .utf8_text(text)
                    .unwrap()
                    .to_string();
                let attrs: Option<Vec<String>> =
                    node.child_by_field_name("attributes").map(|attrs| {
                        let mut cursor = attrs.walk();
                        attrs
                            .named_children(&mut cursor)
                            .map(|attr| attr.utf8_text(text).unwrap().to_string())
                            .collect()
                    });
                Some(Macro { name, attrs })
            }
            "link" => {
                let target = node
                    .child_by_field_name("target")
                    .unwrap()
                    .utf8_text(text)
                    .unwrap()
                    .to_string();
                let markup = node
                    .child_by_field_name("description")
                    .map(|node| tsnode_to_inlines(node, text));
                let attrs = node
                    .child_by_field_name("attributes")
                    .map(|attrs_node| {
                        let mut cursor = attrs_node.walk();
                        attrs_node
                            .named_children(&mut cursor)
                            .map(|attr_node| {
                                let key = attr_node
                                    .child_by_field_name("key")
                                    .map(|node| node.utf8_text(text).unwrap())
                                    .unwrap();
                                let val = attr_node
                                    .child_by_field_name("value")
                                    .map(|node| node.utf8_text(text).unwrap());
                                if let Some(val) = val {
                                    Attribute::KeyValue(key.to_string(), val.to_string())
                                } else {
                                    Attribute::Key(key.to_string())
                                }
                            })
                            .collect()
                    })
                    .unwrap_or(vec![]);
                Some(Link {
                    target,
                    markup,
                    attrs,
                })
            }
            "anchor" => {
                let target = node
                    .child_by_field_name("target")
                    .map(|node| node.utf8_text(text).unwrap().to_string());
                let markup =
                    tsnode_to_inlines(node.child_by_field_name("description").unwrap(), text);
                let attrs = node
                    .child_by_field_name("attributes")
                    .map(|attrs_node| {
                        let mut cursor = attrs_node.walk();
                        attrs_node
                            .named_children(&mut cursor)
                            .map(|attr_node| {
                                let key = attr_node
                                    .child_by_field_name("key")
                                    .map(|node| node.utf8_text(text).unwrap())
                                    .unwrap();
                                let val = attr_node
                                    .child_by_field_name("value")
                                    .map(|node| node.utf8_text(text).unwrap());
                                // let s = attr_node.utf8_text(text).unwrap();
                                // let mut parts = s.splitn(2, char::is_whitespace);
                                // let key = parts.next().unwrap();
                                if let Some(val) = val {
                                    Attribute::KeyValue(key.to_string(), val.to_string())
                                } else {
                                    Attribute::Key(key.to_string())
                                }
                            })
                            .collect()
                    })
                    .unwrap_or(vec![]);
                Some(Anchor {
                    target,
                    markup,
                    attrs,
                })
            }
            _ => None,
        })
        .flatten()
        .collect()
}

fn get_attributes_from_tsnode(node: tree_sitter::Node, text: &[u8]) -> Option<Vec<Attribute>> {
    node
        .child_by_field_name("attributes")
        .map(|attrs_node| {
            let mut cursor = attrs_node.walk();
            attrs_node
                .named_children(&mut cursor)
                .map(|attr_node| {
                    let key = attr_node
                        .child_by_field_name("key")
                        .map(|node| node.utf8_text(text).unwrap())
                        .unwrap();
                    let val = attr_node
                        .child_by_field_name("value")
                        .map(|node| node.utf8_text(text).unwrap());
                    // let s = attr_node.utf8_text(text).unwrap();
                    // let mut parts = s.splitn(2, char::is_whitespace);
                    // let key = parts.next().unwrap();
                    if let Some(val) = val {
                        Attribute::KeyValue(key.to_string(), val.to_string())
                    } else {
                        Attribute::Key(key.to_string())
                    }
                })
                .collect()
        })
}
