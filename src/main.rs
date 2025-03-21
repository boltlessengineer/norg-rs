use tree_sitter::Parser;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NorgInline {
    Text(String),
    Special(String),
    Escape(char),
    Whitespace,
    SoftBreak,
    Bold(Vec<Self>),
    Italic(Vec<Self>),
    Underline(Vec<Self>),
    Strikethrough(Vec<Self>),
    // TODO: verbatim should have different content type
    Verbatim(Vec<Self>),
    Macro {
        name: String,
    },
    Link {
        target: String,
        markup: Option<Vec<Self>>,
    },
    Anchor {
        target: Option<String>,
        markup: Vec<Self>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NorgBlock {
    Section {
        level: u16,
        heading: Option<Vec<NorgInline>>,
        contents: Vec<NorgBlock>,
    },
    Paragraph(Vec<NorgInline>),
    Infirm {
        name: String,
        // TODO: change this to Vec<String>
        params: Option<String>,
    },
    RangedTag {
        name: String,
        params: Option<String>,
        content: Vec<String>,
    },
}

fn main() {
    let code = r#".image path/to/image.png
@code language
@end
{:asdf:}
"#;
    let mut parser = Parser::new();
    let language = tree_sitter_norg::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("Error loading Norg parser");
    let tree = parser.parse(code, None).unwrap();
    let root = tree.root_node();
    dbg!(root.to_sexp());
    let text = code.as_bytes();
    let ast = node_to_nodes(root, text);
    dbg!(ast);
}

fn node_to_nodes(node: tree_sitter::Node, text: &[u8]) -> Vec<NorgBlock> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .map(|n| node_to_node(n, text))
        .flatten()
        .collect()
}

fn node_to_node(node: tree_sitter::Node, text: &[u8]) -> Option<NorgBlock> {
    match node.kind() {
        "section" => {
            let heading_node = node.child_by_field_name("heading").unwrap();
            let prefix_node = heading_node.child(0).unwrap();
            let prefix_count = prefix_node
                .utf8_text(text)
                .unwrap()
                .len();
            let title_node = heading_node.child(1);
            let title = title_node.map(|node| node_to_inlines(node, text));
            Some(NorgBlock::Section {
                level: prefix_count as u16,
                heading: title,
                contents: node_to_nodes(node, text),
            })
        }
        "paragraph" => {
            let mut cursor = node.walk();
            let inlines = node
                .named_children(&mut cursor)
                .map(|node| node_to_inline(node, text))
                .flatten()
                .collect();
            Some(NorgBlock::Paragraph(inlines))
        }
        "infirm_tag" => {
            let name = node
                .child_by_field_name("name")
                .unwrap()
                .utf8_text(text)
                .unwrap()
                .to_string();
            let raw_param = node
                .child_by_field_name("param")
                .map(|node| {
                    node.utf8_text(text).unwrap().to_string()
                });
            Some(NorgBlock::Infirm {
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
                .map(|node| {
                    node.utf8_text(text).unwrap().to_string()
                });
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
        _ => None,
    }
}

fn node_to_inlines(node: tree_sitter::Node, text: &[u8]) -> Vec<NorgInline> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .map(|node| node_to_inline(node, text))
        .flatten()
        .collect()
}

fn node_to_inline(node: tree_sitter::Node, text: &[u8]) -> Option<NorgInline> {
    match node.kind() {
        "whitespace" => Some(NorgInline::Whitespace),
        "soft_break" => Some(NorgInline::SoftBreak),
        "word" => {
            let text = node.utf8_text(text).unwrap().to_string();
            Some(NorgInline::Text(text))
        }
        "punctuation" => {
            let text = node.utf8_text(text).unwrap().to_string();
            Some(NorgInline::Special(text))
        }
        "escape_sequence" => {
            let character = node.utf8_text(text).unwrap().chars().nth(1).unwrap();
            Some(NorgInline::Escape(character))
        }
        "bold" => Some(NorgInline::Bold(node_to_inlines(node, text))),
        "italic" => Some(NorgInline::Italic(node_to_inlines(node, text))),
        "underline" => Some(NorgInline::Underline(node_to_inlines(node, text))),
        "strikethrough" => Some(NorgInline::Strikethrough(node_to_inlines(node, text))),
        "verbatim" => Some(NorgInline::Verbatim(node_to_inlines(node, text))),
        "inline_macro" => {
            let name = node
                .child_by_field_name("name")
                .unwrap()
                .utf8_text(text)
                .unwrap()
                .to_string();
            Some(NorgInline::Macro { name })
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
                .map(|node| node_to_inlines(node, text));
            Some(NorgInline::Link { target, markup })
        }
        "anchor" => {
            let target = node
                .child_by_field_name("target")
                .map(|node| node.utf8_text(text).unwrap().to_string());
            let markup = node_to_inlines(node.child_by_field_name("description").unwrap(), text);
            Some(NorgInline::Anchor { target, markup })
        }
        _ => None,
    }
}
