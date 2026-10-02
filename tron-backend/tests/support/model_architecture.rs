use quote::ToTokens;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
};
use syn::{
    Attribute, Item, Meta,
    visit::{self, Visit},
    visit_mut::{self, VisitMut},
};

#[derive(Clone)]
struct Edge {
    path: PathBuf,
    test_only: bool,
}
struct Source {
    production: String,
    dispatch: String,
    edges: Vec<Edge>,
    test_only: bool,
}

// Unknown feature/platform predicates are deliberately kept. Only a predicate
// that requires `test` can establish that an item has no production presence.
fn requires_test(meta: &Meta) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") => {
            use syn::parse::Parser;
            let Ok(children) =
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())
            else {
                return false;
            };
            if list.path.is_ident("all") {
                children.iter().any(requires_test)
            } else {
                !children.is_empty() && children.iter().all(requires_test)
            }
        }
        _ => false,
    }
}
fn test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<Meta>()
                .is_ok_and(|meta| requires_test(&meta))
    })
}
fn attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(x) => &x.attrs,
        Item::Enum(x) => &x.attrs,
        Item::ExternCrate(x) => &x.attrs,
        Item::Fn(x) => &x.attrs,
        Item::ForeignMod(x) => &x.attrs,
        Item::Impl(x) => &x.attrs,
        Item::Macro(x) => &x.attrs,
        Item::Mod(x) => &x.attrs,
        Item::Static(x) => &x.attrs,
        Item::Struct(x) => &x.attrs,
        Item::Trait(x) => &x.attrs,
        Item::TraitAlias(x) => &x.attrs,
        Item::Type(x) => &x.attrs,
        Item::Union(x) => &x.attrs,
        Item::Use(x) => &x.attrs,
        _ => &[],
    }
}
fn module_dir(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap();
    if matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("lib.rs" | "main.rs" | "mod.rs")
    ) {
        parent.into()
    } else {
        parent.join(path.file_stem().unwrap())
    }
}
fn normalized(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}
struct Edges<'a> {
    origin: &'a Path,
    module_dir: PathBuf,
    inline_depth: usize,
    test_only: bool,
    edges: Vec<Edge>,
}
impl<'ast> Visit<'ast> for Edges<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        let previous = self.test_only;
        self.test_only |= test_only(attrs(item));
        visit::visit_item(self, item);
        self.test_only = previous;
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if let Some((_, items)) = &item.content {
            let previous = self.module_dir.clone();
            self.module_dir.push(item.ident.to_string());
            self.inline_depth += 1;
            for item in items {
                self.visit_item(item);
            }
            self.inline_depth -= 1;
            self.module_dir = previous;
        } else {
            let explicit = item.attrs.iter().find_map(|attr| {
                if !attr.path().is_ident("path") {
                    return None;
                }
                let Meta::NameValue(value) = &attr.meta else {
                    return None;
                };
                let syn::Expr::Lit(value) = &value.value else {
                    return None;
                };
                let syn::Lit::Str(value) = &value.lit else {
                    return None;
                };
                Some(value.value())
            });
            let path = if let Some(explicit) = explicit {
                if self.inline_depth == 0 {
                    self.origin.parent().unwrap().join(explicit)
                } else {
                    self.module_dir.join(explicit)
                }
            } else {
                let path = self.module_dir.join(format!("{}.rs", item.ident));
                if path.exists() {
                    path
                } else {
                    self.module_dir.join(item.ident.to_string()).join("mod.rs")
                }
            };
            self.edges.push(Edge {
                path: normalized(path),
                test_only: self.test_only,
            });
        }
    }
    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        if item.path.is_ident("include")
            && let Ok(path) = item.parse_body::<syn::LitStr>()
        {
            self.edges.push(Edge {
                path: normalized(self.origin.parent().unwrap().join(path.value())),
                test_only: self.test_only,
            });
        }
        visit::visit_macro(self, item);
    }
    fn visit_stmt_macro(&mut self, item: &'ast syn::StmtMacro) {
        let previous = self.test_only;
        self.test_only |= test_only(&item.attrs);
        visit::visit_stmt_macro(self, item);
        self.test_only = previous;
    }
}

struct Production;
impl VisitMut for Production {
    fn visit_file_mut(&mut self, file: &mut syn::File) {
        file.items.retain(|item| !test_only(attrs(item)));
        visit_mut::visit_file_mut(self, file);
    }
    fn visit_item_mod_mut(&mut self, item: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut item.content {
            items.retain(|item| !test_only(attrs(item)));
        }
        visit_mut::visit_item_mod_mut(self, item);
    }
    fn visit_item_impl_mut(&mut self, item: &mut syn::ItemImpl) {
        item.items.retain(|item| {
            !test_only(match item {
                syn::ImplItem::Const(x) => &x.attrs,
                syn::ImplItem::Fn(x) => &x.attrs,
                syn::ImplItem::Type(x) => &x.attrs,
                syn::ImplItem::Macro(x) => &x.attrs,
                _ => &[],
            })
        });
        visit_mut::visit_item_impl_mut(self, item);
    }
    fn visit_item_trait_mut(&mut self, item: &mut syn::ItemTrait) {
        item.items.retain(|item| {
            !test_only(match item {
                syn::TraitItem::Const(x) => &x.attrs,
                syn::TraitItem::Fn(x) => &x.attrs,
                syn::TraitItem::Type(x) => &x.attrs,
                syn::TraitItem::Macro(x) => &x.attrs,
                _ => &[],
            })
        });
        visit_mut::visit_item_trait_mut(self, item);
    }
    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        block.stmts.retain(|stmt| {
            !test_only(match stmt {
                syn::Stmt::Item(item) => attrs(item),
                syn::Stmt::Local(local) => &local.attrs,
                syn::Stmt::Macro(item) => &item.attrs,
                _ => &[],
            })
        });
        visit_mut::visit_block_mut(self, block);
    }
}

fn compact(tokens: &impl ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .split_whitespace()
        .collect()
}

// This projection recognizes the default, fail-closed model port plumbing by
// its complete AST body, not its source path. Network/retired-transport guards
// continue to inspect the unmodified production projection.
struct ControlledForwarding;
impl VisitMut for ControlledForwarding {
    fn visit_item_trait_mut(&mut self, item: &mut syn::ItemTrait) {
        if item.ident == "ModelClient" {
            let once: syn::Block = syn::parse_quote! {
                { let _=(config,request); Box::pin(async {Err("single-attempt model invocation must be explicitly implemented".into())}) }
            };
            let fail_closed = item.items.iter().any(|method| {
                matches!(method, syn::TraitItem::Fn(method)
                    if method.sig.ident == "generate_once"
                    && method.default.as_ref().is_some_and(|body| compact(body) == compact(&once)))
            });
            if fail_closed {
                let native: syn::Block = syn::parse_quote! {
                    { match sink {Some(sink)=>self.generate_stream(config,product,sink),None=>self.generate_once(config,product)} }
                };
                let stream: syn::Block = syn::parse_quote! {
                    { Box::pin(async move {
                        let response = self.generate_once(config, request).await?;
                        let output = response.output_text();
                        if response.finish == crate::domain::agent::ModelFinish::Final && !output.is_empty() {
                            sink(ModelStreamEvent::text_delta(output));
                        }
                        Ok(response)
                    }) }
                };
                for method in &mut item.items {
                    let syn::TraitItem::Fn(method) = method else {
                        continue;
                    };
                    let Some(body) = &mut method.default else {
                        continue;
                    };
                    let expected = match method.sig.ident.to_string().as_str() {
                        "generate_native_once" => &native,
                        "generate_stream" => &stream,
                        _ => continue,
                    };
                    if compact(body) == compact(expected) {
                        PortCalls.visit_block_mut(body);
                    }
                }
            }
        }
        visit_mut::visit_item_trait_mut(self, item);
    }
}
struct PortCalls;
impl VisitMut for PortCalls {
    fn visit_expr_method_call_mut(&mut self, call: &mut syn::ExprMethodCall) {
        if call.method == "generate_once" || call.method == "generate_stream" {
            call.method = syn::Ident::new("__controlled_model_port_forward", call.method.span());
        }
        visit_mut::visit_expr_method_call_mut(self, call);
    }
}

fn read(path: &Path) -> Result<Source, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    // include! accepts a Rust expression as well as items. Parse the complete
    // fragment in either grammar; an expression is wrapped only to reuse the
    // same visitors, never dropped or exempted from production guards.
    let mut file = match syn::parse_file(&raw) {
        Ok(file) => file,
        Err(items_error) if path.extension().is_some_and(|extension| extension == "inc") => {
            let expression = syn::parse_str::<syn::Expr>(&raw).map_err(|expression_error| {
                format!("cannot classify Rust source {}: items: {items_error}; expression: {expression_error}", path.display())
            })?;
            syn::parse_quote! {const __ARCHITECTURE_FRAGMENT: () = #expression;}
        }
        Err(error) => {
            return Err(format!(
                "cannot classify Rust source {}: {error}",
                path.display()
            ));
        }
    };
    let is_test = test_only(&file.attrs);
    let mut edges = Edges {
        origin: path,
        module_dir: module_dir(path),
        inline_depth: 0,
        test_only: is_test,
        edges: vec![],
    };
    edges.visit_file(&file);
    if is_test {
        file.items.clear();
    } else {
        Production.visit_file_mut(&mut file);
    }
    let mut dispatch = file.clone();
    ControlledForwarding.visit_file_mut(&mut dispatch);
    Ok(Source {
        production: compact(&file),
        dispatch: compact(&dispatch),
        edges: edges.edges,
        test_only: is_test,
    })
}
fn paths(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            paths(&path, output)?;
        } else if path
            .extension()
            .is_some_and(|ext| ext == "rs" || ext == "inc")
        {
            output.push(normalized(path));
        }
    }
    Ok(())
}
pub fn production_sources(root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    classified_sources(root, false)
}
pub fn production_dispatch_sources(root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    classified_sources(root, true)
}
fn classified_sources(root: &Path, dispatch_only: bool) -> Result<Vec<(PathBuf, String)>, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let mut all = vec![];
    paths(&root, &mut all)?;
    let sources: BTreeMap<_, _> = all
        .into_iter()
        .map(|path| read(&path).map(|source| (path, source)))
        .collect::<Result<_, _>>()?;
    let mut production = BTreeSet::new();
    let mut queue = VecDeque::from([root.join("lib.rs"), root.join("main.rs")]);
    while let Some(path) = queue.pop_front() {
        let Some(source) = sources.get(&path) else {
            continue;
        };
        if source.test_only || !production.insert(path) {
            continue;
        }
        queue.extend(
            source
                .edges
                .iter()
                .filter(|edge| !edge.test_only)
                .map(|edge| edge.path.clone()),
        );
    }
    let mut tests = BTreeSet::new();
    let mut queue: VecDeque<_> = sources
        .iter()
        .flat_map(|(path, source)| {
            source.test_only.then_some(path.clone()).into_iter().chain(
                source
                    .edges
                    .iter()
                    .filter(|edge| edge.test_only)
                    .map(|edge| edge.path.clone()),
            )
        })
        .collect();
    while let Some(path) = queue.pop_front() {
        if !tests.insert(path.clone()) {
            continue;
        }
        if let Some(source) = sources.get(&path) {
            queue.extend(source.edges.iter().map(|edge| edge.path.clone()));
        }
    }
    // An orphan is still scanned. A file shared with production is still scanned.
    // Exclusion requires actual test-module reachability and no production path.
    Ok(sources
        .into_iter()
        .filter(|(path, _)| !tests.contains(path) || production.contains(path))
        .map(|(path, source)| {
            (
                path.strip_prefix(&root).unwrap().to_owned(),
                if dispatch_only {
                    source.dispatch
                } else {
                    source.production
                },
            )
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        for (name, source) in files {
            let path = directory.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
        directory
    }
    #[test]
    fn inline_test_module_does_not_hide_later_production_network_or_calls() {
        let root = fixture(&[(
            "lib.rs",
            r#"#[cfg(test)] mod arbitrary {fn helper(){client.post("test");}} fn production(){client.post("live");client.generate_once();}"#,
        )]);
        let projected = production_sources(root.path()).unwrap();
        let source = &projected[0].1;
        assert!(!source.contains("helper"));
        assert!(source.contains(".post(\"live\")"));
        assert!(source.contains(".generate_once("));
    }
    #[test]
    fn actual_test_module_path_and_include_subtree_are_excluded() {
        let root = fixture(&[
            (
                "lib.rs",
                r#"#[cfg(test)] mod support; #[cfg(all(test,unix))] #[path="named.rs"] mod other; fn production(){}"#,
            ),
            ("support.rs", r#"include!("support.inc");"#),
            ("support.inc", r#"fn helper(){client.post("test");}"#),
            ("named.rs", r#"fn another(){HttpProtocolClient::new();}"#),
        ]);
        let projected = production_sources(root.path()).unwrap();
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].0, Path::new("lib.rs"));
        assert!(!projected[0].1.contains("HttpProtocolClient"));
    }
    #[test]
    fn shared_and_orphan_production_sources_are_never_excluded_by_names() {
        let root = fixture(&[
            (
                "lib.rs",
                r#"#[cfg(test)] #[path="shared.rs"] mod helper; #[path="shared.rs"] mod live;"#,
            ),
            ("shared.rs", r#"fn production(){client.post("live");}"#),
            (
                "looks_like_tests.rs",
                r#"fn orphan(){client.generate_once();}"#,
            ),
        ]);
        let projected = production_sources(root.path()).unwrap();
        assert!(
            projected
                .iter()
                .any(|(path, source)| path == Path::new("shared.rs") && source.contains(".post("))
        );
        assert!(
            projected
                .iter()
                .any(|(path, source)| path == Path::new("looks_like_tests.rs")
                    && source.contains(".generate_once("))
        );
    }
    #[test]
    fn included_expression_fragments_retain_all_production_calls_and_test_scope() {
        let root = fixture(&[
            (
                "lib.rs",
                r#"const LABELS: &[(&str,&str)]=include!("labels.inc"); const LIVE: usize=include!("dispatch.inc"); #[cfg(test)] mod tests {const HELPER:usize=include!("test_dispatch.inc");}"#,
            ),
            ("labels.inc", r#"&[("label.key","保留内容")]"#),
            (
                "dispatch.inc",
                r#"{client.post("live");client.generate_once();0}"#,
            ),
            ("test_dispatch.inc", r#"{HttpProtocolClient::new();0}"#),
        ]);
        let projected = production_sources(root.path()).unwrap();
        let expression = projected
            .iter()
            .find(|(path, _)| path == Path::new("dispatch.inc"))
            .unwrap();
        assert!(expression.1.contains(".post("));
        assert!(expression.1.contains(".generate_once("));
        assert!(
            projected.iter().any(
                |(path, source)| path == Path::new("labels.inc") && source.contains("保留内容")
            )
        );
        assert!(
            !projected
                .iter()
                .any(|(path, _)| path == Path::new("test_dispatch.inc"))
        );
        fs::write(root.path().join("invalid.inc"), "&[").unwrap();
        assert!(
            production_sources(root.path())
                .unwrap_err()
                .contains("cannot classify Rust source")
        );
    }

    #[test]
    fn fail_closed_port_forwarding_does_not_hide_business_dispatch_or_modified_bodies() {
        let port = r#"trait ModelClient {
            fn generate_once(&self,config:C,request:R) {let _=(config,request);Box::pin(async {Err("single-attempt model invocation must be explicitly implemented".into())})}
            fn generate_native_once(&self,config:C,product:R,sink:S) {match sink {Some(sink)=>self.generate_stream(config,product,sink),None=>self.generate_once(config,product)}}
            fn generate_stream(&self,config:C,request:R,sink:S) {Box::pin(async move {let response=self.generate_once(config,request).await?;let output=response.output_text();if response.finish == crate::domain::agent::ModelFinish::Final && !output.is_empty() {sink(ModelStreamEvent::text_delta(output));}Ok(response)})}
        }"#;
        let root = fixture(&[("lib.rs", port)]);
        let raw = production_sources(root.path()).unwrap();
        assert!(raw[0].1.contains(".generate_once("));
        assert!(raw[0].1.contains(".generate_stream("));
        let dispatch = production_dispatch_sources(root.path()).unwrap();
        assert!(!dispatch[0].1.contains(".generate_once("));
        assert!(!dispatch[0].1.contains(".generate_stream("));

        fs::write(root.path().join("lib.rs"), format!("{port} fn business(){{client.generate_once();client.generate_stream();client.post(\"live\");}}")).unwrap();
        let dispatch = production_dispatch_sources(root.path()).unwrap();
        assert!(dispatch[0].1.contains(".generate_once("));
        assert!(dispatch[0].1.contains(".generate_stream("));
        assert!(dispatch[0].1.contains(".post("));

        let modified = port.replace("match sink {", "client.post(\"live\"); match sink {");
        fs::write(root.path().join("lib.rs"), modified).unwrap();
        let dispatch = production_dispatch_sources(root.path()).unwrap();
        assert!(dispatch[0].1.contains(".post("));
        assert!(dispatch[0].1.contains(".generate_once("));
        assert!(dispatch[0].1.contains(".generate_stream("));

        let unsafe_default =
            port.replace("let _=(config,request);", "self.generate(config,request);");
        fs::write(root.path().join("lib.rs"), unsafe_default).unwrap();
        let dispatch = production_dispatch_sources(root.path()).unwrap();
        assert!(dispatch[0].1.contains(".generate_once("));
        assert!(dispatch[0].1.contains(".generate_stream("));
    }

    #[test]
    fn unknown_cfg_and_parse_failures_remain_fail_closed() {
        let root = fixture(&[(
            "lib.rs",
            r#"#[cfg(any(test,unix))] fn production(){client.post("live");} #[cfg(not(test))] fn another(){client.generate_once();}"#,
        )]);
        let projected = production_sources(root.path()).unwrap();
        assert!(projected[0].1.contains(".post("));
        assert!(projected[0].1.contains(".generate_once("));
        fs::write(root.path().join("broken.rs"), "fn broken(").unwrap();
        assert!(
            production_sources(root.path())
                .unwrap_err()
                .contains("cannot classify Rust source")
        );
    }
}
