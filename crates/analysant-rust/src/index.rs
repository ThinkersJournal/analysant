// SPDX-License-Identifier: MIT OR Apache-2.0
//! The code index: what is defined, where each name is used, and where a name appears only in a comment.

use crate::cfgeval::holds_in_production;
use crate::comments::{comment_lines, words};
use crate::pack::RustPack;
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;
use std::collections::HashMap;
use syn::{Attribute, Fields, ImplItem, Item, Meta, TraitItem, Type, Visibility};

/// Production code or test code. A file under a test path, a `#[test]` item and an item whose `cfg`
/// does not hold in a production build are test code, and so is everything inside them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Context {
    Production,
    Test,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CodeItemKind {
    Fn,
    Struct,
    Enum,
    Variant,
    Trait,
    Const,
    Static,
    Type,
    Union,
    Field,
    Macro,
}

/// An index into `CodeIndex::items`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ItemId(pub usize);

#[derive(Clone, Debug)]
pub struct CodeItem {
    pub kind: CodeItemKind,
    /// The bare name: `resolve`.
    pub name: String,
    /// `Type::name` for a method, a variant or a field; the bare name otherwise.
    pub qualified: String,
    pub file: u32,
    pub line: u32,
    pub context: Context,
    pub public: bool,
    /// A method of `impl Trait for Type`: callable through the trait, so its callers cannot be seen.
    pub trait_impl: bool,
}

/// One occurrence of a name in code, other than its own definition and `use` lines.
#[derive(Clone, Debug)]
pub struct Use {
    pub file: u32,
    pub line: u32,
    pub context: Context,
    /// The item whose body holds the occurrence; `None` at the top of a file or in an `impl` header.
    pub enclosing: Option<ItemId>,
    /// Followed by an argument list: `name(`.
    pub call: bool,
    /// Preceded by a dot: `.name`.
    pub dot: bool,
}

/// A file `syn` could not parse. Its comments are still indexed; its items are not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseFailure {
    pub file: String,
    pub reason: String,
}

#[derive(Default)]
pub struct CodeIndex {
    pub(crate) files: Vec<String>,
    pub(crate) items: Vec<CodeItem>,
    pub(crate) by_name: HashMap<String, Vec<ItemId>>,
    pub(crate) uses: HashMap<String, Vec<Use>>,
    pub(crate) comments: HashMap<String, Vec<(u32, u32)>>,
    pub(crate) failures: Vec<ParseFailure>,
    pub(crate) pack: Option<RustPack>,
}

impl CodeIndex {
    pub fn new(pack: RustPack) -> Self {
        Self {
            pack: Some(pack),
            ..Self::default()
        }
    }

    pub(crate) fn pack(&self) -> &RustPack {
        self.pack
            .as_ref()
            .expect("a CodeIndex is built by CodeIndex::new")
    }
    /// The name `text` refers to in this tree's language, in the form `relate` takes, or `None` when
    /// it does not name a code item (see `RustPack::code_reference`).
    pub fn code_reference(&self, text: &str) -> Option<String> {
        self.pack().code_reference(text)
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
    pub fn items(&self) -> &[CodeItem] {
        &self.items
    }
    pub fn file_name(&self, id: u32) -> &str {
        &self.files[id as usize]
    }
    pub fn failures(&self) -> &[ParseFailure] {
        &self.failures
    }

    /// Index one source file. `rel_path` is relative to the code root and `/`-separated.
    /// A file that does not parse is recorded in `failures()`; it is never silently skipped.
    pub fn add_source(&mut self, rel_path: &str, text: &str) {
        let file = u32::try_from(self.files.len()).expect("fewer than 2^32 files");
        self.files.push(rel_path.to_string());
        for (line, comment) in comment_lines(text) {
            for w in words(&comment) {
                self.comments
                    .entry(w.to_string())
                    .or_default()
                    .push((file, line));
            }
        }
        let path_is_test = self.pack().is_test_path(rel_path);
        match syn::parse_file(text) {
            Ok(parsed) => {
                let base = if path_is_test {
                    Context::Test
                } else {
                    Context::Production
                };
                let mut w = Walker {
                    index: self,
                    file,
                    in_trait_impl: false,
                };
                w.items(&parsed.items, base, None);
            }
            Err(e) => self.failures.push(ParseFailure {
                file: rel_path.to_string(),
                reason: e.to_string(),
            }),
        }
    }

    pub(crate) fn add_item(&mut self, item: CodeItem) -> ItemId {
        let id = ItemId(self.items.len());
        self.by_name.entry(item.name.clone()).or_default().push(id);
        self.items.push(item);
        id
    }
}

struct Walker<'a> {
    index: &'a mut CodeIndex,
    file: u32,
    in_trait_impl: bool,
}

impl Walker<'_> {
    /// Context of an item given its attributes and the context it sits in.
    fn context(&self, attrs: &[Attribute], inherited: Context) -> Context {
        if inherited == Context::Test {
            return Context::Test;
        }
        let pack = self.index.pack();
        for a in attrs {
            let Some(name) = a.path().get_ident().map(ToString::to_string) else {
                // A path attribute such as `tokio::test`: its last segment names it.
                if let Some(last) = a.path().segments.last() {
                    if pack.is_test_attribute(&last.ident.to_string()) {
                        return Context::Test;
                    }
                }
                continue;
            };
            if pack.is_test_attribute(&name) {
                return Context::Test;
            }
            if pack.is_cfg_attribute(&name) {
                if let Meta::List(list) = &a.meta {
                    if !holds_in_production(pack, list.tokens.clone()) {
                        return Context::Test;
                    }
                }
            }
        }
        Context::Production
    }

    fn items(&mut self, items: &[Item], inherited: Context, enclosing: Option<ItemId>) {
        for item in items {
            self.item(item, inherited, enclosing);
        }
    }

    fn qual(&self, owner: &str, member: &str) -> String {
        format!("{owner}{}{member}", self.index.pack().path_separator())
    }

    fn define(
        &mut self,
        kind: CodeItemKind,
        name: &syn::Ident,
        qualified: String,
        context: Context,
        public: bool,
    ) -> ItemId {
        let line = u32::try_from(name.span().start().line).unwrap_or(u32::MAX);
        self.index.add_item(CodeItem {
            kind,
            name: name.to_string(),
            qualified,
            file: self.file,
            line,
            context,
            public,
            trait_impl: self.in_trait_impl,
        })
    }

    fn item(&mut self, item: &Item, inherited: Context, enclosing: Option<ItemId>) {
        match item {
            Item::Fn(f) => {
                let ctx = self.context(&f.attrs, inherited);
                let id = self.define(
                    CodeItemKind::Fn,
                    &f.sig.ident,
                    f.sig.ident.to_string(),
                    ctx,
                    is_public(&f.vis),
                );
                self.uses(
                    item.to_token_stream(),
                    &[f.sig.ident.to_string()],
                    ctx,
                    Some(id),
                );
            }
            Item::Struct(s) => {
                let ctx = self.context(&s.attrs, inherited);
                let sname = s.ident.to_string();
                let id = self.define(
                    CodeItemKind::Struct,
                    &s.ident,
                    sname.clone(),
                    ctx,
                    is_public(&s.vis),
                );
                let mut skip = vec![sname.clone()];
                if let Fields::Named(named) = &s.fields {
                    for f in &named.named {
                        if let Some(fname) = &f.ident {
                            let fctx = self.context(&f.attrs, ctx);
                            self.define(
                                CodeItemKind::Field,
                                fname,
                                self.qual(&sname, &fname.to_string()),
                                fctx,
                                is_public(&f.vis),
                            );
                            skip.push(fname.to_string());
                        }
                    }
                }
                self.uses(item.to_token_stream(), &skip, ctx, Some(id));
            }
            Item::Union(u) => {
                let ctx = self.context(&u.attrs, inherited);
                let id = self.define(
                    CodeItemKind::Union,
                    &u.ident,
                    u.ident.to_string(),
                    ctx,
                    is_public(&u.vis),
                );
                self.uses(
                    item.to_token_stream(),
                    &[u.ident.to_string()],
                    ctx,
                    Some(id),
                );
            }
            Item::Enum(e) => {
                let ctx = self.context(&e.attrs, inherited);
                let ename = e.ident.to_string();
                let id = self.define(
                    CodeItemKind::Enum,
                    &e.ident,
                    ename.clone(),
                    ctx,
                    is_public(&e.vis),
                );
                let mut skip = vec![ename.clone()];
                for v in &e.variants {
                    let vctx = self.context(&v.attrs, ctx);
                    self.define(
                        CodeItemKind::Variant,
                        &v.ident,
                        self.qual(&ename, &v.ident.to_string()),
                        vctx,
                        true,
                    );
                    skip.push(v.ident.to_string());
                    if let Fields::Named(named) = &v.fields {
                        for f in &named.named {
                            if let Some(fname) = &f.ident {
                                skip.push(fname.to_string());
                            }
                        }
                    }
                }
                self.uses(item.to_token_stream(), &skip, ctx, Some(id));
            }
            Item::Const(c) => {
                let ctx = self.context(&c.attrs, inherited);
                let id = self.define(
                    CodeItemKind::Const,
                    &c.ident,
                    c.ident.to_string(),
                    ctx,
                    is_public(&c.vis),
                );
                self.uses(
                    item.to_token_stream(),
                    &[c.ident.to_string()],
                    ctx,
                    Some(id),
                );
            }
            Item::Static(s) => {
                let ctx = self.context(&s.attrs, inherited);
                let id = self.define(
                    CodeItemKind::Static,
                    &s.ident,
                    s.ident.to_string(),
                    ctx,
                    is_public(&s.vis),
                );
                self.uses(
                    item.to_token_stream(),
                    &[s.ident.to_string()],
                    ctx,
                    Some(id),
                );
            }
            Item::Type(t) => {
                let ctx = self.context(&t.attrs, inherited);
                let id = self.define(
                    CodeItemKind::Type,
                    &t.ident,
                    t.ident.to_string(),
                    ctx,
                    is_public(&t.vis),
                );
                self.uses(
                    item.to_token_stream(),
                    &[t.ident.to_string()],
                    ctx,
                    Some(id),
                );
            }
            Item::Macro(m) => {
                // `macro_rules! name { ... }`: the name is a definition, the body is code.
                if let Some(name) = &m.ident {
                    let ctx = self.context(&m.attrs, inherited);
                    let id = self.define(CodeItemKind::Macro, name, name.to_string(), ctx, true);
                    self.uses(m.mac.tokens.clone(), &[], ctx, Some(id));
                } else {
                    let ctx = self.context(&m.attrs, inherited);
                    self.uses(m.mac.tokens.clone(), &[], ctx, enclosing);
                }
            }
            Item::Trait(t) => {
                let ctx = self.context(&t.attrs, inherited);
                let tname = t.ident.to_string();
                let id = self.define(
                    CodeItemKind::Trait,
                    &t.ident,
                    tname.clone(),
                    ctx,
                    is_public(&t.vis),
                );
                let mut skip = vec![tname.clone()];
                for ti in &t.items {
                    match ti {
                        TraitItem::Fn(f) => {
                            let fctx = self.context(&f.attrs, ctx);
                            let fid = self.define(
                                CodeItemKind::Fn,
                                &f.sig.ident,
                                self.qual(&tname, &f.sig.ident.to_string()),
                                fctx,
                                true,
                            );
                            self.uses(
                                ti.to_token_stream(),
                                &[f.sig.ident.to_string()],
                                fctx,
                                Some(fid),
                            );
                        }
                        TraitItem::Const(c) => skip.push(c.ident.to_string()),
                        TraitItem::Type(t) => skip.push(t.ident.to_string()),
                        _ => {}
                    }
                }
                // The trait's own header; its functions' tokens were counted above.
                let mut header = t.clone();
                header.items.clear();
                self.uses(header.to_token_stream(), &skip, ctx, Some(id));
            }
            Item::Impl(imp) => {
                let ctx = self.context(&imp.attrs, inherited);
                let self_name = type_name(&imp.self_ty);
                let saved = self.in_trait_impl;
                self.in_trait_impl = imp.trait_.is_some();
                let mut header = imp.clone();
                header.items.clear();
                self.uses(header.to_token_stream(), &[], ctx, enclosing);
                for ii in &imp.items {
                    match ii {
                        ImplItem::Fn(f) => {
                            let fctx = self.context(&f.attrs, ctx);
                            let fid = self.define(
                                CodeItemKind::Fn,
                                &f.sig.ident,
                                self.qual(&self_name, &f.sig.ident.to_string()),
                                fctx,
                                is_public(&f.vis),
                            );
                            self.uses(
                                ii.to_token_stream(),
                                &[f.sig.ident.to_string()],
                                fctx,
                                Some(fid),
                            );
                        }
                        ImplItem::Const(c) => {
                            let cctx = self.context(&c.attrs, ctx);
                            let cid = self.define(
                                CodeItemKind::Const,
                                &c.ident,
                                self.qual(&self_name, &c.ident.to_string()),
                                cctx,
                                is_public(&c.vis),
                            );
                            self.uses(
                                ii.to_token_stream(),
                                &[c.ident.to_string()],
                                cctx,
                                Some(cid),
                            );
                        }
                        other => self.uses(other.to_token_stream(), &[], ctx, enclosing),
                    }
                }
                self.in_trait_impl = saved;
            }
            Item::Mod(m) => {
                let ctx = self.context(&m.attrs, inherited);
                if let Some((_, content)) = &m.content {
                    self.items(content, ctx, enclosing);
                }
            }
            // `use` and `extern crate` import and re-export names: a re-export is not a use.
            _ => {}
        }
    }

    /// Record every identifier of `tokens` as a use, except one occurrence per name in `skip` (the
    /// item's own definition names).
    fn uses(
        &mut self,
        tokens: TokenStream,
        skip: &[String],
        ctx: Context,
        enclosing: Option<ItemId>,
    ) {
        let mut skip: Vec<String> = skip.to_vec();
        let mut flat: Vec<Flat> = Vec::new();
        flatten(tokens, &mut flat);
        for (i, f) in flat.iter().enumerate() {
            let Flat::Ident(name, line) = f else { continue };
            if let Some(pos) = skip.iter().position(|s| s == name) {
                skip.swap_remove(pos);
                continue;
            }
            let dot = i > 0 && matches!(flat[i - 1], Flat::Dot);
            let call = matches!(flat.get(i + 1), Some(Flat::Call));
            self.index.uses.entry(name.clone()).or_default().push(Use {
                file: self.file,
                line: *line,
                context: ctx,
                enclosing,
                call,
                dot,
            });
        }
    }
}

/// A token stream reduced to what the use rules read, in source order, groups expanded in place.
enum Flat {
    Ident(String, u32),
    Dot,
    /// An opening parenthesis: the identifier before it is called.
    Call,
    Other,
}

fn flatten(tokens: TokenStream, out: &mut Vec<Flat>) {
    for tt in tokens {
        match tt {
            TokenTree::Ident(id) => {
                let line = u32::try_from(id.span().start().line).unwrap_or(u32::MAX);
                out.push(Flat::Ident(id.to_string(), line));
            }
            TokenTree::Punct(p) if p.as_char() == '.' => out.push(Flat::Dot),
            TokenTree::Group(g) => {
                out.push(if g.delimiter() == Delimiter::Parenthesis {
                    Flat::Call
                } else {
                    Flat::Other
                });
                flatten(g.stream(), out);
            }
            _ => out.push(Flat::Other),
        }
    }
}

fn is_public(v: &Visibility) -> bool {
    !matches!(v, Visibility::Inherited)
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(p) => p
            .path
            .segments
            .last()
            .map_or_else(String::new, |s| s.ident.to_string()),
        Type::Reference(r) => type_name(&r.elem),
        _ => String::new(),
    }
}
