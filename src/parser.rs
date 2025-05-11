use std::{collections::HashMap, hash::{DefaultHasher, Hash as _, Hasher as _}};

use crate::{
    block::{ListItem, NorgBlock},
    inline::{Attribute, NorgInline},
};

// pub type Markup = String;
pub type AnchorMap = HashMap<u64, AnchorDefinitionNode>;

#[derive(Debug, Clone)]
pub struct AnchorDefinitionNode {
    /// byte range
    pub range: Range,
    // TODO: change this to target::NorgLinkTarget when we start parsing target syntax from rust
    // parser
    pub target: String,
}

#[derive(Debug)]
pub struct NorgAST {
    pub anchors: AnchorMap,
    pub blocks: Vec<NorgBlock>,
}

impl Into<janetrs::JanetStruct<'_>> for AnchorDefinitionNode {
    fn into(self) -> janetrs::JanetStruct<'static> {
        janetrs::JanetStruct::builder(2)
            .put(
                janetrs::JanetKeyword::new("target"),
                janetrs::JanetString::from(self.target),
            )
            .put(janetrs::JanetKeyword::new("range"), self.range)
            .finalize()
    }
}

impl Into<janetrs::JanetStruct<'_>> for NorgAST {
    fn into(self) -> janetrs::JanetStruct<'static> {
        janetrs::JanetStruct::builder(2)
            .put(
                janetrs::JanetKeyword::new("anchors"),
                janetrs::Janet::table(
                    self.anchors
                        .into_iter()
                        .map(|(key, value)| {
                            // let value = janetrs::JanetString::from(value);
                            let value: janetrs::JanetStruct = value.into();
                            (key, value)
                        })
                        .collect(),
                ),
            )
            .put(
                janetrs::JanetKeyword::new("blocks"),
                janetrs::Janet::tuple(self.blocks.into_iter().collect()),
            )
            .finalize()
    }
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

impl From<tree_sitter::Range> for Range {
    fn from(value: tree_sitter::Range) -> Self {
        Self {
            start: value.start_byte,
            end: value.end_byte,
        }
    }
}

impl Into<janetrs::Janet> for Range {
    fn into(self) -> janetrs::Janet {
        janetrs::Janet::tuple(self.into())
    }
}

impl Into<janetrs::JanetTuple<'_>> for Range {
    fn into(self) -> janetrs::JanetTuple<'static> {
        janetrs::tuple![self.start, self.end]
    }
}

pub fn parse(text: &[u8]) -> NorgAST {
    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_norg::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("Error loading Norg parser");
    let tree = parser.parse(&text, None).unwrap();
    parse_tstree(&tree, text)
}

pub fn parse_tstree(tree: &tree_sitter::Tree, text: &[u8]) -> NorgAST {
    let root = tree.root_node();
    let mut anchors = HashMap::new();
    let blocks = tsnode_to_blocks(&mut anchors, root, text);
    NorgAST { anchors, blocks }
}

#[derive(Default)]
struct CarryoverScanner {
    tags: Vec<(String, Option<String>)>,
    attrs: Vec<Attribute>,
}

fn tsnode_to_blocks(anchors: &mut AnchorMap, node: tree_sitter::Node, text: &[u8]) -> Vec<NorgBlock> {
    let mut cursor = node.walk();
    let mut carryovers = CarryoverScanner::default();
    node.named_children(&mut cursor)
        .flat_map(|node| {
            let block = match node.kind() {
                "section" => {
                    let heading_node = node.child_by_field_name("heading").unwrap();
                    let prefix_count = heading_node
                        .child(0)
                        .expect("heading node should have at least one child")
                        .utf8_text(text)
                        .expect("heading prefix should be valid utf8 text")
                        .len();
                    let title = heading_node
                        .child_by_field_name("title")
                        .map(|node| tsnode_to_inlines(anchors, node, text));
                    let attrs = get_attributes_from_tsnode(heading_node, text).unwrap_or(vec![]);
                    Some(NorgBlock::Section {
                        attrs,
                        level: prefix_count as u16,
                        heading: title,
                        contents: tsnode_to_blocks(anchors, node, text),
                    })
                }
                "paragraph" => Some(NorgBlock::Paragraph {
                    attrs: std::mem::take(&mut carryovers.attrs),
                    inlines: tsnode_to_inlines(anchors, node, text),
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
                "carryover_attributes" => {
                    let attrs = get_attributes_from_tsnode(node, text).unwrap();
                    carryovers.attrs.extend(attrs);
                    None
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
                    carryovers.tags.push((name, raw_param));
                    None
                }
                "unordered_list" => {
                    let prefix_node = node.child(0).unwrap().child(0).unwrap();
                    let prefix_count = prefix_node.utf8_text(text).unwrap().len();
                    Some(NorgBlock::UnorderedList {
                        attrs: vec![],
                        level: prefix_count as u16,
                        items: {
                            let mut cursor = node.walk();
                            node.named_children(&mut cursor)
                                .map(|node| ListItem {
                                    attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                                    contents: tsnode_to_blocks(anchors, node, text),
                                })
                                .collect()
                        },
                    })
                }
                "ordered_list" => {
                    let prefix_node = node.child(0).unwrap().child(0).unwrap();
                    let prefix_count = prefix_node.utf8_text(text).unwrap().len();
                    Some(NorgBlock::OrderedList {
                        attrs: vec![],
                        level: prefix_count as u16,
                        items: {
                            let mut cursor = node.walk();
                            node.named_children(&mut cursor)
                                .map(|node| ListItem {
                                    attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                                    contents: tsnode_to_blocks(anchors, node, text),
                                })
                                .collect()
                        },
                    })
                }
                "quote" => {
                    let prefix_node = node.child(0).unwrap().child(0).unwrap();
                    let prefix_count = prefix_node.utf8_text(text).unwrap().len();
                    Some(NorgBlock::Quote {
                        attrs: vec![],
                        level: prefix_count as u16,
                        items: {
                            let mut cursor = node.walk();
                            node.named_children(&mut cursor)
                                .map(|node| ListItem {
                                    attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                                    contents: tsnode_to_blocks(anchors, node, text),
                                })
                                .collect()
                        },
                    })
                }
                "horizontal_line" => Some(NorgBlock::HorizontalLine { attrs: vec![] }),
                _ => None,
            };
            block.map(|block| {
                if carryovers.tags.len() > 0 {
                    let tags = std::mem::take(&mut carryovers.tags);
                    tags.into_iter()
                        .fold(block, |block, (name, params)| NorgBlock::CarryoverTag {
                            name,
                            params,
                            target: Box::new(block),
                        })
                } else {
                    block
                }
            })
        })
        .collect()
}

fn tsnode_to_inlines(anchors: &mut AnchorMap, node: tree_sitter::Node, text: &[u8]) -> Vec<NorgInline> {
    let mut cursor = node.walk();
    use NorgInline::*;
    node.named_children(&mut cursor)
        .map(|node| match node.kind() {
            "whitespace" => Some(Whitespace),
            "soft_break" => Some(SoftBreak),
            "hard_break" => Some(HardBreak),
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
                markup: tsnode_to_inlines(anchors, node, text),
            }),
            "italic" => Some(Italic {
                attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                markup: tsnode_to_inlines(anchors, node, text),
            }),
            "underline" => Some(Underline {
                attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                markup: tsnode_to_inlines(anchors, node, text),
            }),
            "strikethrough" => Some(Strikethrough {
                attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                markup: tsnode_to_inlines(anchors, node, text),
            }),
            "verbatim" => Some(Verbatim {
                attrs: get_attributes_from_tsnode(node, text).unwrap_or(vec![]),
                markup: tsnode_to_inlines(anchors, node, text),
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
                    .child_by_field_name("markup")
                    .map(|node| tsnode_to_inlines(anchors, node, text));
                let attrs = get_attributes_from_tsnode(node, text).unwrap_or(vec![]);
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
                let range: Range = node.range().into();
                let markup_node = node.child_by_field_name("markup").unwrap();
                let markup = tsnode_to_inlines(anchors, markup_node, text);
                let hash = {
                    let mut hasher = DefaultHasher::new();
                    markup.hash(&mut hasher);
                    hasher.finish()
                };
                if let Some(ref target) = target {
                    anchors.insert(
                        hash,
                        AnchorDefinitionNode {
                            range,
                            target: target.clone(),
                        },
                    );
                }
                let attrs = get_attributes_from_tsnode(node, text).unwrap_or(vec![]);
                Some(Anchor {
                    target,
                    hash,
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
    node.child_by_field_name("attributes").map(|attrs_node| {
        let mut cursor = attrs_node.walk();
        attrs_node
            .named_children(&mut cursor)
            .map(|attr_node| {
                let key = attr_node
                    .child_by_field_name("key")
                    .map(|node| node.utf8_text(text).unwrap());
                let val = attr_node
                    .child_by_field_name("value")
                    .map(|node| node.utf8_text(text).unwrap());
                match (key, val) {
                    (None, None) => Attribute::Blank,
                    (Some(key), None) => Attribute::Key(key.to_string()),
                    (Some(key), Some(val)) => Attribute::KeyValue(key.to_string(), val.to_string()),
                    _ => unreachable!(),
                }
            })
            .collect()
    })
}
