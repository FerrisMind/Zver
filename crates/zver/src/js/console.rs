use boa_engine::{Context, JsValue, NativeFunction, js_string, object::ObjectInitializer, property::Attribute};

/// Инициализирует глобальный объект console с методами log, error, warn
pub fn init_console(context: &mut Context) {
    // console.log
    let console_log = NativeFunction::from_fn_ptr(|_this, args, _context| {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{}", arg.display());
        }
        println!();
        Ok(JsValue::undefined())
    });

    // console.error
    let console_error = NativeFunction::from_fn_ptr(|_this, args, _context| {
        eprint!("ERROR: ");
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                eprint!(" ");
            }
            eprint!("{}", arg.display());
        }
        eprintln!();
        Ok(JsValue::undefined())
    });

    // console.warn
    let console_warn = NativeFunction::from_fn_ptr(|_this, args, _context| {
        eprint!("WARN: ");
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                eprint!(" ");
            }
            eprint!("{}", arg.display());
        }
        eprintln!();
        Ok(JsValue::undefined())
    });

    let console = ObjectInitializer::new(context)
        .function(console_log, js_string!("log"), 0)
        .function(console_error, js_string!("error"), 0)
        .function(console_warn, js_string!("warn"), 0)
        .build();

    let _ = context.register_global_property(
        js_string!("console"),
        console,
        Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
    );
}
