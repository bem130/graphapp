use rquickjs::{Runtime, Context, Result};
use rquickjs::function::Func;

fn main() -> Result<()> {
    // QuickJSランタイムとコンテキストを作成
    let rt = Runtime::new()?;
    let ctx = Context::full(&rt)?;
    ctx.with(|ctx| {
        // print関数をグローバルに登録
        ctx.globals().set("print", Func::from(|msg: String| println!("{:?}", msg)))?;
        // console.logをprintに割り当て
        ctx.eval::<(), _>("globalThis.console = { log: print };")?;
        // 1. シンプルなJavaScript文字列を実行
        println!("--- シンプルなJavaScriptの実行 ---");
        let result = ctx.eval::<(), _>("console.log('JavaScriptからこんにちは！');");
        if let Err(e) = result {
            println!("JS実行エラー: {}", e);
        }

        // 2. Rust関数をJavaScriptに公開（例: add関数）
        ctx.globals().set(
            "rust_add",
            Func::from(|a: i32, b: i32| a + b),
        )?;
        ctx.eval::<(), _>("console.log('Rust関数からの合計: ' + rust_add(10, 20));")?;

        // 3. JavaScriptから値を取得
        println!("\n--- JavaScriptの変数をRustで取得 ---");
        ctx.eval::<(), _>("var message = 'これはJSからのメッセージです！';")?;
        let msg: String = ctx.eval("message")?;
        println!("JSからのメッセージ: {}", msg);
        Ok(())
    })
}
