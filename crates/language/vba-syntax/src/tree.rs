//! The lossless syntax tree.
//!
//! Rowan stores token text as `str`, but VBA source is bytes in a Windows
//! code page and need not be valid UTF-8, so the tree is a small arena of its
//! own. It owns the source bytes; every token is a span into them, and tokens
//! tile the source without gaps, so concatenating all token texts always
//! reproduces the input.

use std::fmt::Write as _;

use vba_lexer::TokenKind;
use vba_source::Span;

use crate::NodeKind;

/// Index of a node in a [`SyntaxTree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Child {
    Node(NodeId),
    Token(u32),
}

#[derive(Debug, Clone)]
struct NodeData {
    kind: NodeKind,
    span: Span,
    parent: Option<NodeId>,
    children: Vec<Child>,
    value: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
struct TokenData {
    kind: TokenKind,
    span: Span,
    parent: NodeId,
}

/// An immutable, lossless concrete syntax tree that owns its source bytes.
#[derive(Debug, Clone)]
pub struct SyntaxTree {
    source: Vec<u8>,
    nodes: Vec<NodeData>,
    tokens: Vec<TokenData>,
    root: NodeId,
}

impl SyntaxTree {
    /// The root node (always a [`NodeKind::Module`]).
    pub fn root(&self) -> SyntaxNode<'_> {
        SyntaxNode {
            tree: self,
            id: self.root,
        }
    }

    /// The source bytes the tree was built from.
    pub fn source(&self) -> &[u8] {
        &self.source
    }

    /// Reconstructs the source text from the tokens. Equal to
    /// [`SyntaxTree::source`] by construction; exposed for tests.
    pub fn text(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.source.len());
        for t in &self.tokens {
            out.extend_from_slice(&self.source[t.span.start as usize..t.span.end as usize]);
        }
        out
    }

    /// Looks up a node by id.
    pub fn node(&self, id: NodeId) -> SyntaxNode<'_> {
        SyntaxNode { tree: self, id }
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of tokens.
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }

    /// An indented dump of the tree for snapshot tests. Nodes print as
    /// `Kind@start..end`, tokens as `Kind@start..end "text"` (lossy UTF-8,
    /// escaped). Directive nodes also print their evaluated value.
    pub fn debug_dump(&self) -> String {
        let mut s = String::new();
        self.dump_node(self.root, 0, &mut s);
        s
    }

    fn dump_node(&self, id: NodeId, depth: usize, out: &mut String) {
        let n = &self.nodes[id.0 as usize];
        let _ = write!(
            out,
            "{:indent$}{:?}@{}..{}",
            "",
            n.kind,
            n.span.start,
            n.span.end,
            indent = depth * 2
        );
        if let Some(v) = n.value {
            let _ = write!(out, " value={v}");
        }
        out.push('\n');
        for c in &n.children {
            match *c {
                Child::Node(c) => self.dump_node(c, depth + 1, out),
                Child::Token(t) => {
                    let t = &self.tokens[t as usize];
                    let text = &self.source[t.span.start as usize..t.span.end as usize];
                    let _ = writeln!(
                        out,
                        "{:indent$}{:?}@{}..{} {:?}",
                        "",
                        t.kind,
                        t.span.start,
                        t.span.end,
                        String::from_utf8_lossy(text),
                        indent = (depth + 1) * 2
                    );
                }
            }
        }
    }
}

/// A borrowed handle to a node.
#[derive(Clone, Copy)]
pub struct SyntaxNode<'t> {
    tree: &'t SyntaxTree,
    id: NodeId,
}

impl std::fmt::Debug for SyntaxNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}@{:?}", self.kind(), self.span())
    }
}

impl PartialEq for SyntaxNode<'_> {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self.tree, o.tree) && self.id == o.id
    }
}
impl Eq for SyntaxNode<'_> {}

impl<'t> SyntaxNode<'t> {
    fn data(&self) -> &'t NodeData {
        &self.tree.nodes[self.id.0 as usize]
    }

    /// The node id.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// The tree this node belongs to.
    pub fn tree(&self) -> &'t SyntaxTree {
        self.tree
    }

    /// The node kind.
    pub fn kind(&self) -> NodeKind {
        self.data().kind
    }

    /// The byte span (empty for zero-width nodes).
    pub fn span(&self) -> Span {
        self.data().span
    }

    /// The exact source bytes of the node.
    pub fn text(&self) -> &'t [u8] {
        let s = self.data().span;
        &self.tree.source[s.start as usize..s.end as usize]
    }

    /// The evaluated value of a conditional-compilation directive
    /// (`CcIf`, `CcElseIf`): `Some` when the condition was evaluated.
    pub fn value(&self) -> Option<bool> {
        self.data().value
    }

    /// The parent node, `None` for the root.
    pub fn parent(&self) -> Option<SyntaxNode<'t>> {
        self.data().parent.map(|id| SyntaxNode {
            tree: self.tree,
            id,
        })
    }

    /// All children (nodes and tokens, trivia included) in source order.
    pub fn children(&self) -> impl Iterator<Item = SyntaxElement<'t>> + 't {
        let tree = self.tree;
        self.data().children.iter().map(move |c| match *c {
            Child::Node(id) => SyntaxElement::Node(SyntaxNode { tree, id }),
            Child::Token(i) => SyntaxElement::Token(SyntaxToken { tree, index: i }),
        })
    }

    /// Child nodes only.
    pub fn child_nodes(&self) -> impl Iterator<Item = SyntaxNode<'t>> + 't {
        self.children().filter_map(|e| e.into_node())
    }

    /// Child tokens only, including trivia.
    pub fn child_tokens(&self) -> impl Iterator<Item = SyntaxToken<'t>> + 't {
        self.children().filter_map(|e| e.into_token())
    }

    /// Child tokens that are not trivia.
    pub fn significant_tokens(&self) -> impl Iterator<Item = SyntaxToken<'t>> + 't {
        self.child_tokens().filter(|t| !t.kind().is_trivia())
    }

    /// The first child node of `kind`.
    pub fn child_node(&self, kind: NodeKind) -> Option<SyntaxNode<'t>> {
        self.child_nodes().find(|n| n.kind() == kind)
    }

    /// The first child token of `kind`.
    pub fn child_token(&self, kind: TokenKind) -> Option<SyntaxToken<'t>> {
        self.child_tokens().find(|t| t.kind() == kind)
    }

    /// Every token below this node in source order.
    pub fn descendant_tokens(&self) -> Vec<SyntaxToken<'t>> {
        let mut out = Vec::new();
        self.collect_tokens(&mut out);
        out
    }

    fn collect_tokens(&self, out: &mut Vec<SyntaxToken<'t>>) {
        for c in self.children() {
            match c {
                SyntaxElement::Token(t) => out.push(t),
                SyntaxElement::Node(n) => n.collect_tokens(out),
            }
        }
    }

    /// Every node below this node (excluding itself) in pre-order.
    pub fn descendants(&self) -> Vec<SyntaxNode<'t>> {
        let mut out = Vec::new();
        self.collect_nodes(&mut out);
        out
    }

    fn collect_nodes(&self, out: &mut Vec<SyntaxNode<'t>>) {
        for n in self.child_nodes() {
            out.push(n);
            n.collect_nodes(out);
        }
    }
}

/// A borrowed handle to a token.
#[derive(Clone, Copy)]
pub struct SyntaxToken<'t> {
    tree: &'t SyntaxTree,
    index: u32,
}

impl std::fmt::Debug for SyntaxToken<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}@{:?} {:?}",
            self.kind(),
            self.span(),
            String::from_utf8_lossy(self.text())
        )
    }
}

impl<'t> SyntaxToken<'t> {
    fn data(&self) -> &'t TokenData {
        &self.tree.tokens[self.index as usize]
    }

    /// The lexer token kind.
    pub fn kind(&self) -> TokenKind {
        self.data().kind
    }

    /// The byte span.
    pub fn span(&self) -> Span {
        self.data().span
    }

    /// The exact source bytes.
    pub fn text(&self) -> &'t [u8] {
        let s = self.data().span;
        &self.tree.source[s.start as usize..s.end as usize]
    }

    /// The parent node.
    pub fn parent(&self) -> SyntaxNode<'t> {
        SyntaxNode {
            tree: self.tree,
            id: self.data().parent,
        }
    }

    /// Whether the token is the given keyword.
    pub fn is_keyword(&self, k: vba_lexer::Keyword) -> bool {
        self.kind() == TokenKind::Keyword(k)
    }

    /// Whether the token text equals `word`, ignoring ASCII case.
    pub fn text_eq(&self, word: &str) -> bool {
        self.text().eq_ignore_ascii_case(word.as_bytes())
    }
}

/// A child of a node.
#[derive(Debug, Clone, Copy)]
pub enum SyntaxElement<'t> {
    /// An interior node.
    Node(SyntaxNode<'t>),
    /// A token.
    Token(SyntaxToken<'t>),
}

impl<'t> SyntaxElement<'t> {
    /// The node, if this is one.
    pub fn into_node(self) -> Option<SyntaxNode<'t>> {
        match self {
            SyntaxElement::Node(n) => Some(n),
            SyntaxElement::Token(_) => None,
        }
    }

    /// The token, if this is one.
    pub fn into_token(self) -> Option<SyntaxToken<'t>> {
        match self {
            SyntaxElement::Token(t) => Some(t),
            SyntaxElement::Node(_) => None,
        }
    }
}

// ---- builder -------------------------------------------------------------

struct Frame {
    kind: NodeKind,
    children: Vec<Child>,
}

/// A position in the children of the innermost open node.
#[derive(Clone, Copy)]
pub(crate) struct Checkpoint(usize);

pub(crate) struct Builder {
    source: Vec<u8>,
    nodes: Vec<NodeData>,
    tokens: Vec<TokenData>,
    frames: Vec<Frame>,
    pos: u32,
}

impl Builder {
    pub(crate) fn new(source: &[u8]) -> Self {
        Builder {
            source: source.to_vec(),
            nodes: Vec::new(),
            tokens: Vec::new(),
            frames: Vec::new(),
            pos: 0,
        }
    }

    pub(crate) fn start_node(&mut self, kind: NodeKind) {
        self.frames.push(Frame {
            kind,
            children: Vec::new(),
        });
    }

    pub(crate) fn checkpoint(&self) -> Checkpoint {
        Checkpoint(self.frames.last().map_or(0, |f| f.children.len()))
    }

    pub(crate) fn start_node_at(&mut self, cp: Checkpoint, kind: NodeKind) {
        let top = self.frames.last_mut().expect("open node");
        let children = top.children.split_off(cp.0);
        self.frames.push(Frame { kind, children });
    }

    pub(crate) fn token(&mut self, kind: TokenKind, span: Span) {
        debug_assert_eq!(span.start, self.pos);
        self.pos = span.end;
        let idx = self.tokens.len() as u32;
        self.tokens.push(TokenData {
            kind,
            span,
            parent: NodeId(0),
        });
        self.frames
            .last_mut()
            .expect("open node")
            .children
            .push(Child::Token(idx));
    }

    pub(crate) fn finish_node(&mut self) -> NodeId {
        let frame = self.frames.pop().expect("open node");
        let id = NodeId(self.nodes.len() as u32);
        let first = frame.children.first().map(|c| self.child_start(*c));
        let last = frame.children.last().map(|c| self.child_end(*c));
        let span = match (first, last) {
            (Some(s), Some(e)) => Span::new(s, e),
            _ => Span::empty(self.pos),
        };
        for c in &frame.children {
            match *c {
                Child::Node(n) => self.nodes[n.0 as usize].parent = Some(id),
                Child::Token(t) => self.tokens[t as usize].parent = id,
            }
        }
        self.nodes.push(NodeData {
            kind: frame.kind,
            span,
            parent: None,
            children: frame.children,
            value: None,
        });
        if let Some(top) = self.frames.last_mut() {
            top.children.push(Child::Node(id));
        }
        id
    }

    pub(crate) fn last_finished(&self) -> NodeId {
        NodeId(self.nodes.len() as u32 - 1)
    }

    pub(crate) fn set_value(&mut self, id: NodeId, v: Option<bool>) {
        self.nodes[id.0 as usize].value = v;
    }

    fn child_start(&self, c: Child) -> u32 {
        match c {
            Child::Node(n) => self.nodes[n.0 as usize].span.start,
            Child::Token(t) => self.tokens[t as usize].span.start,
        }
    }

    fn child_end(&self, c: Child) -> u32 {
        match c {
            Child::Node(n) => self.nodes[n.0 as usize].span.end,
            Child::Token(t) => self.tokens[t as usize].span.end,
        }
    }

    /// Finishes the (already closed) root node and returns the tree.
    pub(crate) fn build(mut self, root: NodeId) -> SyntaxTree {
        debug_assert!(self.frames.is_empty());
        self.nodes[root.0 as usize].parent = None;
        SyntaxTree {
            source: self.source,
            nodes: self.nodes,
            tokens: self.tokens,
            root,
        }
    }
}
