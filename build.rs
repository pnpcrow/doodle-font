fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .set("ProductName", "Doodle Font")
            .set("FileDescription", "두들 폰트 · Doodle Font")
            .compile()
            .expect("Windows icon resource");
    }
}
