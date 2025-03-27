#![allow(dead_code)]
#![allow(unused_variables)]

use block::NorgBlock;
use janetrs::{Janet, JanetKeyword, JanetTuple, TaggedJanet};

pub mod block;
pub mod inline;
pub mod parser;

fn export(block: NorgBlock, lang: String) -> Result<String, ()> {
    match block {
        NorgBlock::Section {
            params,
            level,
            heading,
            contents,
        } => todo!(),
        NorgBlock::Paragraph { params, inlines } => todo!(),
        NorgBlock::InfirmTag { params, name } => todo!(),
        NorgBlock::CarryoverTag { params, name } => todo!(),
        NorgBlock::RangedTag {
            params,
            name,
            content,
        } => {
            // 1. find macro definition defined in janet
            // 2. run with parameters
            // 3. parse result as Vec<Block>
            // 4. run expand_block for all of them
            //
            // wait, no. just implement whole (export-block) on janet side
            // benefit: exporter itself can be portable
            todo!("");
        },
        NorgBlock::Embed { params, mut export } => {
            let args = [
                Janet::keyword(JanetKeyword::new(lang)),
                Janet::tuple(JanetTuple::builder(0).finalize()),
            ];
            let res = export.call(&args).unwrap().unwrap();
            match res {
                TaggedJanet::Array(janet_array) => todo!(),
                TaggedJanet::String(janet_string) => {
                    return Ok(janet_string.to_string());
                }
                _ => return Err(()),
            }
        }
    }
}
