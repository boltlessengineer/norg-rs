use crate::Range;

#[derive(PartialEq)]
pub struct SyntaxNode(Repr);

#[derive(Debug, PartialEq)]
enum Repr {
    Leaf(LeafNode),
    Inner(InnerNode),
    Error(ErrorNode),
}

impl SyntaxNode {
    pub fn is_leaf(&self) -> bool {
        matches!(self.0, Repr::Leaf(_))
    }
    pub fn leaf(kind: SyntaxKind, range: Range) -> Self {
        Self(Repr::Leaf(LeafNode::new(kind, range)))
    }
    pub fn inner(kind: SyntaxKind, children: Vec<Self>) -> Self {
        // TODO: accept range and debug_assert that all children nodes fit inside that range
        Self(Repr::Inner(InnerNode::new(kind, children)))
    }
    pub fn error(range: Range, text: &str) -> Self {
        let text = String::from(text);
        Self(Repr::Error(ErrorNode { range, text }))
    }

    pub fn kind(&self) -> SyntaxKind {
        match &self.0 {
            Repr::Leaf(leaf) => leaf.kind,
            Repr::Inner(inner) => inner.kind,
            Repr::Error(_) => SyntaxKind::Error,
        }
    }

    pub fn range(&self) -> Range {
        match &self.0 {
            Repr::Leaf(leaf) => leaf.range,
            Repr::Inner(inner) => inner.range,
            Repr::Error(error) => error.range,
        }
    }
}

#[derive(PartialEq)]
pub struct LeafNode {
    pub kind: SyntaxKind,
    pub range: Range,
}

#[derive(PartialEq)]
pub struct InnerNode {
    pub kind: SyntaxKind,
    pub range: Range,
    pub children: Vec<SyntaxNode>,
}

#[derive(PartialEq)]
pub struct ErrorNode {
    pub text: String,
    pub range: Range,
}

impl LeafNode {
    fn new(kind: SyntaxKind, range: Range) -> Self {
        Self { kind, range }
    }
}

impl InnerNode {
    fn new(kind: SyntaxKind, children: Vec<SyntaxNode>) -> Self {
        let range = Range::new(
            children.first().unwrap().range().start,
            children.last().unwrap().range().end,
        );
        Self { kind, range, children }
    }
}

impl std::fmt::Debug for SyntaxNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.0 {
            Repr::Leaf(leaf) => leaf.fmt(f),
            Repr::Inner(inner) => inner.fmt(f),
            Repr::Error(node) => node.fmt(f),
        }
    }
}

impl std::fmt::Debug for LeafNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.kind, self.range)
    }
}

impl std::fmt::Debug for InnerNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.kind, self.range)?;
        if !self.children.is_empty() {
            f.write_str(" ")?;
            f.debug_list().entries(&self.children).finish()?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for ErrorNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error: {:?} : {}", self.range, self.text)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SyntaxKind {
    Document,

    Section,
    Heading(usize),
    BlankHeading(usize),
    Paragraph,
    BlankLine,
    HorizontalLine,

    UnorderedList,
    OrderedList,
    Quote,
    Null,

    UnorderedListItem,
    OrderedListItem,
    QuoteItem,
    NullItem,

    HeadingPrefix,

    UnorderedPrefix,
    OrderedPrefix,
    QuotePrefix,
    NullPrefix,

    /// #(...)
    CarryoverAttributes,
    /// #asdf
    CarryoverTag,
    /// .asdf
    InfirmTag,
    /// @asdf ... @end
    RangedTag,

    /// '#'
    CarryoverPrefix,
    /// '.'
    InfirmTagPrefix,
    /// "@"
    RangedTagPrefix,

    /// identifier for tags
    Identifier,
    Arguments,
    Argument,
    /// ';'
    ArgDelimiter,

    /// verbatim line content of ranged tag
    /// preceding whitespace is trimmed
    RangedTagLine,
    RangedTagLines,

    Word,
    Punctuation,
    // TODO: replace Punctuation with Special(char)
    Special(char),
    Escaped(char),
    Whitespace,
    SoftBreak,
    HardBreak,
    End,

    Bold,
    Italic,
    Underline,
    Strikethrough,
    Verbatim,

    BoldOpen,
    BoldClose,
    ItalicOpen,
    ItalicClose,
    UnderlineOpen,
    UnderlineClose,
    StrikethroughOpen,
    StrikethroughClose,
    VerbatimOpen,
    VerbatimClose,
    MarkupOpen,
    MarkupClose,
    DestinationOpen,
    DestinationClose,

    InlineTag,
    InlineTagPrefix,

    Attributes,
    /// `(`
    AttributesOpen,
    /// `)`
    AttributesClose,
    /// `;`
    AttributeDelimiter,
    Attribute,

    // {asdf}[asdf]
    Link,
    // [asdf]{asdf}
    Anchor,

    /// `[this]`
    Markup,
    /// `{this}`
    Destination,

    /// `:$name/path:* title`
    DestApplink,
    /// `* asdf:** asdf`
    DestScopedLink,
    /// `https://example.com`
    DestRawlink,

    /// `:`
    DestApplinkPrefix,
    /// `$name`
    DestApplinkWorkspace,
    /// `$`
    DestApplinkWorkspacePrefix,
    /// `name`
    DestApplinkWorkspaceName,
    /// `path`
    DestApplinkPath,

    /// `:`
    DestScopeDelimiter,
    DestScopeHeading,
    DestScopeWikiHeading,
    /// `*`
    DestScopeHeadingPrefix,
    /// `?`
    DestScopeWikiHeadingPrefix,
    /// `title`
    DestScopeText,

    Error,
}

pub struct LinkedNode<'a> {
    node: &'a SyntaxNode,
    parent: Option<Box<Self>>,
}

impl<'a> LinkedNode<'a> {
    pub fn new(root: &'a SyntaxNode) -> Self {
        Self { node: root, parent: None }
    }

    pub fn get(&self) -> &'a SyntaxNode {
        self.node
    }

    pub fn parent(&self) -> Option<&Self> {
        self.parent.as_deref()
    }
}
