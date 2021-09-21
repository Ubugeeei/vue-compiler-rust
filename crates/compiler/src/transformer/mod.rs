/*!
Transform IRNode.
This module contains the canonical transformations from vue-next and
the original ones for the parity of features not implemented in Convert.

## Canonical
* hoistStatic
* transformExpression
* ~~vOnce (moved to convert)~~
* ~~vMemo (moved to convert)~~
* trackScopes

## Original
* collect_entities:
track all helpers/components/directives used in AST.
Vue track it by helper/helperString.
* optimize_text:
1. merge consecutive text call
2. wrap text in createTextVNode
* patch_flag:
seems patch flag can be extracted out
 */

mod collect_entities;
mod optimize_text;

use super::converter::{
    self as C, BaseConvertInfo, BaseRoot, ConvertInfo, IRNode, IRRoot, JsExpr as Js, RuntimeDir,
};

pub type BaseIf<'a> = C::IfNodeIR<BaseConvertInfo<'a>>;
pub type BaseFor<'a> = C::ForNodeIR<BaseConvertInfo<'a>>;
pub type BaseVNode<'a> = C::VNodeIR<BaseConvertInfo<'a>>;
pub type BaseRenderSlot<'a> = C::RenderSlotIR<BaseConvertInfo<'a>>;
pub type BaseVSlot<'a> = C::VSlotIR<BaseConvertInfo<'a>>;
pub type BaseSlotFn<'a> = C::Slot<BaseConvertInfo<'a>>;

pub trait Transformer {
    type IR;
    /// transform will change ir node inplace
    /// usually transform will have multiple passes
    fn transform(&mut self, root: &mut Self::IR);
}

use std::marker::PhantomData;
struct NoopTransformer<T>(PhantomData<T>);

impl<T> Transformer for NoopTransformer<T> {
    type IR = T;
    fn transform(&mut self, _root: &mut Self::IR) {
        // noop
    }
}

type Passes<T> = [Box<dyn CoreTransformPass<T>>];

trait CoreTransformer<T: ConvertInfo>: Transformer {
    fn transform_js_expr(&mut self, e: &mut T::JsExpression, ps: &mut Passes<T>);

    #[inline(always)]
    fn enter<F>(&mut self, mut f: F, ps: &mut Passes<T>)
    where
        F: FnMut(&mut Box<dyn CoreTransformPass<T>>),
    {
        for pass in ps {
            f(pass);
        }
    }
    #[inline(always)]
    fn exit<F>(&mut self, mut f: F, ps: &mut Passes<T>)
    where
        F: FnMut(&mut Box<dyn CoreTransformPass<T>>),
    {
        for pass in ps.iter_mut().rev() {
            f(pass);
        }
    }
    fn transform_root(&mut self, root: &mut IRRoot<T>, ps: &mut Passes<T>);
    fn transform_ir(&mut self, ir: &mut IRNode<T>, ps: &mut Passes<T>) {
        use IRNode as I;
        match ir {
            I::TextCall(t) => self.transform_text(t, ps),
            I::If(i) => self.transform_if(i, ps),
            I::For(f) => self.transform_for(f, ps),
            I::VNodeCall(v) => self.transform_vnode(v, ps),
            I::RenderSlotCall(r) => self.transform_slot_outlet(r, ps),
            I::CommentCall(c) => self.transform_comment(c, ps),
            I::VSlotUse(s) => self.transform_v_slot(s, ps),
            I::AlterableSlot(a) => self.transform_slot_fn(a, ps),
        }
    }
    fn transform_children(&mut self, children: &mut Vec<IRNode<T>>, ps: &mut Passes<T>) {
        for child in children.iter_mut() {
            self.transform_ir(child, ps);
        }
    }
    fn transform_text(&mut self, t: &mut T::TextType, ps: &mut Passes<T>) {
        self.enter(|p| p.enter_text(t), ps);
        self.exit(|p| p.exit_text(t), ps);
    }
    fn transform_if(&mut self, i: &mut C::IfNodeIR<T>, ps: &mut Passes<T>) {
        self.enter(|p| p.enter_if(i), ps);
        for branch in i.branches.iter_mut() {
            if let Some(c) = branch.condition.as_mut() {
                self.transform_js_expr(c, ps);
            }
            self.transform_ir(&mut branch.child, ps);
        }
        self.exit(|p| p.exit_if(i), ps);
    }
    fn transform_for(&mut self, f: &mut C::ForNodeIR<T>, ps: &mut Passes<T>) {
        self.enter(|p| p.enter_for(f), ps);
        self.transform_js_expr(&mut f.source, ps);
        // TODO val, key, index should not counted as expr?
        self.transform_ir(&mut f.child, ps);
        self.exit(|p| p.exit_for(f), ps);
    }
    fn transform_vnode(&mut self, v: &mut C::VNodeIR<T>, ps: &mut Passes<T>) {
        self.enter(|p| p.enter_vnode(v), ps);
        self.transform_js_expr(&mut v.tag, ps);
        if let Some(props) = v.props.as_mut() {
            self.transform_js_expr(props, ps);
        }
        self.transform_children(&mut v.children, ps);
        for dir in v.directives.iter_mut() {
            self.transform_runtime_dir(dir, ps);
        }
        self.exit(|p| p.exit_vnode(v), ps);
    }
    fn transform_runtime_dir(&mut self, dir: &mut RuntimeDir<T>, ps: &mut Passes<T>) {
        self.transform_js_expr(&mut dir.name, ps);
        if let Some(expr) = dir.expr.as_mut() {
            self.transform_js_expr(expr, ps);
        }
        if let Some(arg) = dir.arg.as_mut() {
            self.transform_js_expr(arg, ps);
        }
        if let Some(mods) = dir.mods.as_mut() {
            self.transform_js_expr(mods, ps);
        }
    }
    fn transform_slot_outlet(&mut self, r: &mut C::RenderSlotIR<T>, ps: &mut Passes<T>) {
        self.enter(|p| p.enter_slot_outlet(r), ps);
        self.transform_js_expr(&mut r.slot_name, ps);
        if let Some(props) = r.slot_props.as_mut() {
            self.transform_js_expr(props, ps);
        }
        self.transform_children(&mut r.fallbacks, ps);
        self.exit(|p| p.exit_slot_outlet(r), ps);
    }
    fn transform_v_slot(&mut self, s: &mut C::VSlotIR<T>, ps: &mut Passes<T>) {
        self.enter(|p| p.enter_v_slot(s), ps);
        for slot in s.stable_slots.iter_mut() {
            self.transform_slot_fn(slot, ps);
        }
        for slot in s.alterable_slots.iter_mut() {
            self.transform_ir(slot, ps);
        }
        self.exit(|p| p.exit_v_slot(s), ps);
    }
    fn transform_slot_fn(&mut self, slot: &mut C::Slot<T>, ps: &mut Passes<T>) {
        self.enter(|p| p.enter_slot_fn(slot), ps);
        // TODO slot param should not counted as expr?
        self.transform_js_expr(&mut slot.name, ps);
        self.transform_children(&mut slot.body, ps);
        self.exit(|p| p.exit_slot_fn(slot), ps);
    }
    fn transform_comment(&mut self, c: &mut T::CommentType, ps: &mut Passes<T>) {
        self.enter(|p| p.enter_comment(c), ps);
        self.exit(|p| p.exit_comment(c), ps);
    }
}

pub trait CoreTransformPass<T: ConvertInfo> {
    fn enter_root(&mut self, r: &mut IRRoot<T>) {}
    fn exit_root(&mut self, r: &mut IRRoot<T>) {}
    fn enter_text(&mut self, t: &mut T::TextType) {}
    fn exit_text(&mut self, t: &mut T::TextType) {}
    fn enter_if(&mut self, i: &mut C::IfNodeIR<T>) {}
    fn exit_if(&mut self, i: &mut C::IfNodeIR<T>) {}
    fn enter_for(&mut self, f: &mut C::ForNodeIR<T>) {}
    fn exit_for(&mut self, f: &mut C::ForNodeIR<T>) {}
    fn enter_vnode(&mut self, v: &mut C::VNodeIR<T>) {}
    fn exit_vnode(&mut self, v: &mut C::VNodeIR<T>) {}
    fn enter_slot_outlet(&mut self, r: &mut C::RenderSlotIR<T>) {}
    fn exit_slot_outlet(&mut self, r: &mut C::RenderSlotIR<T>) {}
    fn enter_v_slot(&mut self, s: &mut C::VSlotIR<T>) {}
    fn exit_v_slot(&mut self, s: &mut C::VSlotIR<T>) {}
    fn enter_slot_fn(&mut self, s: &mut C::Slot<T>) {}
    fn exit_slot_fn(&mut self, s: &mut C::Slot<T>) {}
    fn enter_js_expr(&mut self, e: &mut T::JsExpression) {}
    fn exit_js_expr(&mut self, e: &mut T::JsExpression) {}
    fn enter_comment(&mut self, c: &mut T::CommentType) {}
    fn exit_comment(&mut self, c: &mut T::CommentType) {}
}

type BaseTransformPass<'a> = dyn CoreTransformPass<BaseConvertInfo<'a>>;
pub struct BaseTransformer<'a, const N: usize> {
    passes: [Box<BaseTransformPass<'a>>; N],
}

impl<'a, const N: usize> Transformer for BaseTransformer<'a, N> {
    type IR = BaseRoot<'a>;
    fn transform(&mut self, node: &mut Self::IR) {
        let mut passes = vec![];
        self.transform_root(node, &mut passes);
    }
}

impl<'a, const N: usize> CoreTransformer<BaseConvertInfo<'a>> for BaseTransformer<'a, N> {
    fn transform_root(
        &mut self,
        r: &mut IRRoot<BaseConvertInfo<'a>>,
        ps: &mut Passes<BaseConvertInfo<'a>>,
    ) {
        self.enter(|p| p.enter_root(r), ps);
        self.transform_children(&mut r.body, ps);
        self.exit(|p| p.exit_root(r), ps);
    }

    fn transform_js_expr(&mut self, e: &mut Js<'a>, ps: &mut Passes<BaseConvertInfo<'a>>) {
        self.enter(|p| p.enter_js_expr(e), ps);
        match e {
            Js::Call(_, args) => {
                for arg in args.iter_mut() {
                    self.transform_js_expr(arg, ps);
                }
            }
            Js::Compound(exprs) => {
                for expr in exprs.iter_mut() {
                    self.transform_js_expr(expr, ps);
                }
            }
            Js::Array(arr) => {
                for item in arr.iter_mut() {
                    self.transform_js_expr(item, ps);
                }
            }
            Js::Props(props) => {
                for (key, val) in props.iter_mut() {
                    self.transform_js_expr(key, ps);
                    self.transform_js_expr(val, ps);
                }
            }
            Js::Src(_) | Js::Simple(..) | Js::StrLit(_) | Js::Symbol(_) => {
                // no further recursion.
            }
        }
        self.exit(|p| p.exit_js_expr(e), ps);
    }
}

// default transforms
pub fn post_process_v_for_child() {
    // 1. inject key to slot
    // 2. Reuse the child's codegenNode but mark it as a block.
}

#[cfg(test)]
mod test {
    use super::*;
    pub use crate::converter::test::base_convert;
    pub fn get_transformer<'a, P>(pass: P) -> BaseTransformer<'a, 1>
    where
        P: CoreTransformPass<BaseConvertInfo<'a>> + 'static,
    {
        BaseTransformer {
            passes: [Box::new(pass)],
        }
    }
}
