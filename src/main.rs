#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use janetrs::{
    Janet, JanetArgs, JanetKeyword, JanetTuple, TaggedJanet,
    client::JanetClient, env::CFunOptions,
};
use norg_rs::{block::NorgBlock, inline::NorgInline};

#[janetrs::janet_fn(arity(fix(1)))]
fn export_block(args: &mut [Janet]) -> Janet {
    use janetrs::JanetType::*;
    let block: NorgBlock = args.get_matches(0, &[Struct]).try_into().unwrap();
    let lang: JanetKeyword = args.get_matches(1, &[Keyword]).try_into().unwrap();
    match block {
        NorgBlock::Section {
            params,
            level,
            heading,
            contents,
        } => todo!(),
        NorgBlock::Paragraph { params, inlines } => {
        },
        NorgBlock::InfirmTag { params, name } => todo!(),
        NorgBlock::CarryoverTag { params, name } => todo!(),
        NorgBlock::RangedTag {
            params,
            name,
            content,
        } => todo!(),
        NorgBlock::Embed { params, mut export } => {
            let args = [
                Janet::keyword(lang),
                Janet::tuple(JanetTuple::builder(0).finalize()),
            ];
            return export.call(&args).unwrap();
        }
    }
    Janet::nil()
}

#[janetrs::janet_fn(arity(fix(1)))]
fn export_inlines(args: &mut [Janet]) -> Janet {
    use janetrs::JanetType::*;
    let inlines: JanetTuple = args.get_matches(0, &[Tuple]).try_into().unwrap();
    dbg!(inlines);
    Janet::nil()
}

fn main() {
    let text = std::fs::read("test.norg").unwrap();
    let ast = norg_rs::parser::parse(&text);

    let client = {
        let mut client = JanetClient::init_with_default_env().unwrap();
        client.add_c_fn(CFunOptions::new(c"neorg/export/block", export_block_c));
        client.add_c_fn(CFunOptions::new(c"neorg/export/inlines", export_inlines_c));
        client
    };

    // expand the tags
    // TODO: do I really need this step? tags can be expanded on export step.
    let ast = expand_ast(&client, ast);

    // export document to other format
    let target = "html";
    let export = |ao: NorgBlock| {
        export_block(&mut [ao.into(), Janet::keyword(target.into())])
    };
    let res = export(ast[2].clone());
    dbg!(res);
}

fn expand_ast(client: &JanetClient, tree: Vec<NorgBlock>) -> Vec<NorgBlock> {
    tree.into_iter()
        .map(|node| {
            use norg_rs::block::NorgBlock::*;
            match node {
                Section {
                    params,
                    level,
                    heading,
                    contents,
                } => Section {
                    params,
                    level,
                    heading,
                    contents: expand_ast(client, contents),
                },
                InfirmTag { name, params } => {
                    let tag = client
                        .run(format!(
                            r#"
                                (import ./stdlib :prefix "")
                                norg/tag/{name}
                            "#
                        ))
                        .unwrap();
                    let TaggedJanet::Function(mut fun) = tag.unwrap() else {
                        unimplemented!();
                    };
                    fun.call([
                        Janet::nil(),
                        match params.clone() {
                            Some(params) => JanetTuple::builder(1)
                                .put(Janet::string(params.into()))
                                .finalize()
                                .into(),
                            None => Janet::nil(),
                        },
                    ])
                    .unwrap()
                    .try_into()
                    .unwrap()
                }
                RangedTag {
                    params,
                    name,
                    content,
                } => {
                    let tag = client
                        .run(format!(
                            r#"
                        (import ./stdlib :prefix "")
                        norg/tag/{name}
                    "#
                        ))
                        .unwrap();
                    let TaggedJanet::Function(mut fun) = tag.unwrap() else {
                        unimplemented!();
                    };
                    fun.call([
                        Janet::nil(),
                        match params.clone() {
                            Some(params) => JanetTuple::builder(1)
                                .put(Janet::string(params.into()))
                                .finalize()
                                .into(),
                            None => JanetTuple::builder(0).finalize().into(),
                        },
                        Janet::tuple(content.iter().map(|x| x.as_str()).collect()),
                    ])
                    .unwrap()
                    .try_into()
                    .unwrap()
                }
                n => n,
            }
        })
        .collect()
}
