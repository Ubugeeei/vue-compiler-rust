use super::{BaseConverter as BC, Element, JsExpr as Js, Prop, VStr};
use crate::core::{
    flags::PatchFlag,
    parser::{Directive, ElemProp},
    tokenizer::Attribute,
    util::is_component_tag,
};
use std::iter::IntoIterator;

pub struct BuildProps<'a> {
    pub props: Option<Js<'a>>,
    pub directives: Vec<Directive<'a>>,
    pub patch_flag: PatchFlag,
    pub dynamic_prop_names: Vec<VStr<'a>>,
}

#[derive(Default)]
struct PropFlags {
    has_ref: bool,
    has_class_binding: bool,
    has_style_binding: bool,
    has_hydration_event_binding: bool,
    has_dynamic_keys: bool,
    has_vnode_hook: bool,
}

#[derive(Default)]
struct CollectProps<'a> {
    props: Props<'a>,
    merge_args: Args<'a>,
    runtime_dirs: Dirs<'a>,
    dynamic_prop_names: Vec<VStr<'a>>,
    prop_flags: PropFlags,
}

type Props<'a> = Vec<Prop<'a>>;
type Args<'a> = Vec<Js<'a>>;
type Dirs<'a> = Vec<Directive<'a>>;

pub fn build_props<'a, T>(bc: &mut BC, e: &Element<'a>, elm_props: T) -> BuildProps<'a>
where
    T: IntoIterator<Item = ElemProp<'a>>,
{
    let mut cp = CollectProps::default();
    elm_props.into_iter().for_each(|prop| match prop {
        ElemProp::Dir(dir) => collect_dir(bc, dir, &mut cp),
        ElemProp::Attr(attr) => collect_attr(bc, e, attr, &mut cp),
    });
    let prop_expr = compute_prop_expr(cp.props, cp.merge_args);
    let patch_flag = build_patch_flag(cp.prop_flags);
    let prop_expr = pre_normalize_prop(prop_expr);
    BuildProps {
        props: prop_expr,
        directives: cp.runtime_dirs,
        patch_flag,
        dynamic_prop_names: cp.dynamic_prop_names,
    }
}

fn collect_attr<'a>(bc: &mut BC, e: &Element<'a>, attr: Attribute<'a>, cp: &mut CollectProps<'a>) {
    let Attribute {
        name,
        value,
        location,
        ..
    } = attr;
    let val = match value {
        Some(v) => v.content,
        None => VStr::raw(""),
    };
    // skip dynamic component is
    if name == "is" && (is_component_tag(e.tag_name) || val.starts_with("vue:")) {
        return;
    }
    let mut value_expr = Js::StrLit(val);
    if name == "ref" {
        cp.prop_flags.has_ref = true;
        if bc.inline && !val.is_empty() {
            value_expr = process_inline_ref(val);
        }
    }
    cp.props.push((Js::StrLit(val), value_expr));
}
fn collect_dir<'a>(bc: &mut BC, dir: Directive<'a>, cp: &mut CollectProps<'a>) {
    todo!()
}

fn process_inline_ref(val: VStr) -> Js {
    todo!("setup binding is pending")
}

fn compute_prop_expr<'a>(props: Props, args: Args) -> Option<Js<'a>> {
    todo!()
}

fn analyze_patch_flag() -> PatchFlag {
    todo!()
}

fn build_patch_flag(info: PropFlags) -> PatchFlag {
    todo!()
}

fn pre_normalize_prop(prop_expr: Option<Js>) -> Option<Js> {
    todo!("pre-normalize props, SSR should be skipped")
}
