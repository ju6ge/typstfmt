#![doc = include_str!("../../README.md")]
#![deny(clippy::all)]

use itertools::Itertools;
use tracing::debug;
use tracing::instrument;
use tracing::warn;
use typst::syntax::ast::BinOp;
use typst::syntax::SyntaxKind;
use typst::syntax::SyntaxKind::*;
use typst::syntax::{parse, LinkedNode};
use Option::None;

mod config;
pub use config::Config;
mod utils;

mod content_blocks;

//formatting stuff starts here
mod args;
mod code_blocks;

mod context;
use context::Ctx;

pub fn format(s: &str, config: Config) -> String {
    let init = parse(s);
    let mut context = Ctx::from_config(config);
    let root = LinkedNode::new(&init);
    visit(&root, &mut context)
}

/// This is recursively called on the AST, the formatting is bottom up,
/// nodes will decide based on the size of their children and the max line length
/// how they will be formatted.
///
/// One assumed rule is that no kind should be formatting with surrounded space
#[instrument(skip_all,name = "V", fields(kind = format!("{:?}",node.kind())))]
fn visit(node: &LinkedNode, ctx: &mut Ctx) -> String {
    let mut res: Vec<String> = vec![];
    for child in node.children() {
        let child_fmt = visit(&child, ctx);
        res.push(child_fmt);
    }
    let res = match node.kind() {
        Binary => format_bin_left_assoc(node, &res, ctx),
        Named | Keyed => format_named_args(node, &res, ctx),
        CodeBlock => code_blocks::format_code_blocks(node, &res, ctx),
        ContentBlock => content_blocks::format_content_blocks(node, &res, ctx),
        Args | Params | Dict | Array | Destructuring | Parenthesized => {
            args::format_args(node, &res, ctx)
        }
        LetBinding => format_let_binding(node, &res, ctx),
        _ => format_default(node, &res, ctx),
    };
    if node.children().count() == 0 {
        debug!("visiting token {:?}", node.kind());
    } else {
        debug!("visiting parent: {:?}", node.kind());
    }
    res
}

/// formats a node for which no specific function was found. Last resort.
/// For the text of the node:
/// Trim spaces for Space nodes if they contain a linebreak.
/// avoids:
/// - putting two consecutive spaces.
/// - putting more than two consecutive newlines.
///
/// For the already formatted children, change nothing.
#[instrument(skip_all, ret)]
fn format_default(node: &LinkedNode, children: &[String], ctx: &mut Ctx) -> String {
    debug!("::format_default: {:?}", node.kind());
    debug!(
        "with children: {:?}",
        node.children().map(|c| c.kind()).collect_vec()
    );

    let mut res = String::new();
    ctx.push_in(node.text(), &mut res);
    for s in children {
        ctx.push_raw_in(s, &mut res);
    }
    res
}

#[instrument(skip_all, ret)]
pub(crate) fn format_bin_left_assoc(
    parent: &LinkedNode,
    children: &[String],
    ctx: &mut Ctx,
) -> String {
    let res = format_bin_left_assoc_tight(parent, children, ctx);

    if crate::utils::max_line_length(&res) >= ctx.config.max_line_length {
        warn!(
            "Breaking binary operation is not supported in typst (yet?) but would be great here."
        );
        // return format_bin_left_assoc_breaking(parent, children, ctx);
    }
    res
}

#[instrument(skip_all)]
pub(crate) fn format_bin_left_assoc_breaking(
    parent: &LinkedNode,
    children: &[String],
    ctx: &mut Ctx,
) -> String {
    let mut res = String::new();
    for (s, node) in children.iter().zip(parent.children()) {
        match node.kind() {
            x if BinOp::from_kind(x).is_some() => {
                ctx.push_in("\n", &mut res);
                ctx.push_raw_indent(s, &mut res);
                ctx.push_raw_in(" ", &mut res);
            }
            Space => {}
            _ => {
                ctx.push_raw_in(s, &mut res);
            }
        }
    }
    res
}

#[instrument(skip_all)]
pub(crate) fn format_bin_left_assoc_tight(
    parent: &LinkedNode,
    children: &[String],
    ctx: &mut Ctx,
) -> String {
    let mut res = String::new();
    for (s, node) in children.iter().zip(parent.children()) {
        match node.kind() {
            x if BinOp::from_kind(x).is_some() => {
                ctx.push_in(" ", &mut res);
                ctx.push_raw_in(s, &mut res);
                ctx.push_in(" ", &mut res);
            }
            Space => {}
            _ => {
                ctx.push_raw_in(s, &mut res);
            }
        }
    }
    res
}

#[instrument(skip_all, ret)]
pub(crate) fn format_named_args(parent: &LinkedNode, children: &[String], ctx: &mut Ctx) -> String {
    let mut res = String::new();
    for (s, node) in children.iter().zip(parent.children()) {
        match node.kind() {
            Colon => res.push_str(": "),
            Space => {}
            _ => {
                ctx.push_raw_in(s, &mut res);
            }
        }
    }
    res
}

#[instrument(skip_all, ret)]
pub(crate) fn format_let_binding(
    parent: &LinkedNode,
    children: &[String],
    ctx: &mut Ctx,
) -> String {
    let mut res = String::new();
    for (s, node) in children.iter().zip(parent.children()) {
        match node.kind() {
            Eq => {
                ctx.push_in(" ", &mut res);
                ctx.push_in(s, &mut res);
                ctx.push_in(" ", &mut res);
            }
            Space => ctx.push_in(s, &mut res),
            _ => {
                ctx.push_raw_in(s, &mut res);
            }
        }
    }
    res
}

#[cfg(test)]
mod tests;