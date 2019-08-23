#[macro_use]
extern crate neon;
extern crate crypto;

use neon::prelude::*;
use self::crypto::digest::Digest;
use self::crypto::sha1::Sha1;

fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string("hello node"))
}

fn gen_sha1_key(mut cx: FunctionContext) -> JsResult<JsString> {
    let input = cx.argument::<JsString>(0)?;
    let mut hasher = Sha1::new();
    hasher.input_str(&input.value());
    Ok(cx.string(hasher.result_str()))
}

pub struct User {
    id: i32,
    first_name: String,
    last_name: String,
    email: String,
}

declare_types! {
  pub class JsUser for User {
    init(mut cx) {
      let id = cx.argument::<JsNumber>(0)?;
      let first_name: Handle<JsString> = cx.argument::<JsString>(1)?;
      let last_name: Handle<JsString> = cx.argument::<JsString>(2)?;
      let email: Handle<JsString> = cx.argument::<JsString>(3)?;

      Ok(User {
        id: id.value() as i32,
        first_name: first_name.value(),
        last_name: last_name.value(),
        email: email.value(),
      })
    }
  }
}

register_module!(mut m, {
    // Export a function
    m.export_function("hello", hello)?;
    m.export_function("gen_sha1_key", gen_sha1_key)?;
    // Export a class
    m.export_class::<JsUser>("User")?;
    // Export strings, numbers, booleans, etc
    let baz = m.string("baz");
    m.export_value("baz", baz)?;
    Ok(())
});
