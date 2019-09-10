extern crate wasm_bindgen;
extern crate sha1;


use wasm_bindgen::prelude::*;

//TODO check if this is copy or reference passing between rust and js as we should only do copy in the response part
#[wasm_bindgen]
pub fn hash(data: &str) -> String{
    sha1::Sha1::from(data).digest().to_string()
}
