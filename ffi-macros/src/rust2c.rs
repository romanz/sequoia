extern crate syn;
use syn::export::ToTokens;
extern crate proc_macro2;

const PREFIX: &'static str = "pgp_";

/// Derives the C type from the Rust type.
///
/// Returns the corresponding C type, and whether or not that type is
/// a pointer type.
fn ident2c(ident: &syn::Ident) -> (String, bool) {
    let ident_string = ident.to_string();

    // Special cases :(
    match ident_string.as_str() {
        "KeyID" => return ("pgp_keyid_t".into(), true),
        "TPKBuilder" => return ("pgp_tpk_builder_t".into(), true),
        "UserID" => return ("pgp_userid_t".into(), true),
        "UserIDBinding" => return ("pgp_user_id_binding_t".into(), true),
        "UserIDBindingIter" =>
            return ("pgp_user_id_binding_iter_t".into(), true),

        // Types from the libc crate.
        "c_char" => return ("char".into(), false),
        "c_int" => return ("int".into(), false),
        "c_uint" => return ("uint".into(), false),
        "bool" => return ("bool".into(), false),
        "size_t" | "ssize_t" | "time_t" |
        "int8_t" | "int16_t" | "int32_t" | "int64_t" |
        "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t"
            => return (ident_string.clone(), false),
        _ => (),
    }

    let mut s = String::new();
    s += PREFIX;
    let mut last_was_uppercase = false;
    for (i, c) in ident_string.chars().enumerate() {
        if c.is_uppercase() {
            if ! last_was_uppercase && i > 0 {
                s += "_";
            }
            s += &c.to_lowercase().to_string();
        } else {
            s += &c.to_string();
        }

        last_was_uppercase = c.is_uppercase();
    }

    s += "_t";
    (s, true)
}

fn type2c<T: ToTokens>(typ: T) -> String {
    let mut tokens = proc_macro2::TokenStream::new();
    typ.to_tokens(&mut tokens);
    let mut c_typ = String::new();
    let mut is_mutable = false;
    let mut is_pointer = 0;
    let mut ident = None;
    for tok in tokens {
        use proc_macro2::TokenTree::*;
        if false {
            eprintln!("mut: {}, ptr: {}, ident: {:?}, tok: {:?}",
                      is_mutable, is_pointer, ident, tok);
        }

        match tok {
            Ident(ref i) => {
                let i_ = format!("{}", i);
                match i_.as_str() {
                    "mut" => is_mutable = true,
                    "const" => is_mutable = false,
                    "Option" => (),
                    "Maybe" => {
                        is_pointer += 1;
                        is_mutable = true;
                    }
                    _ => {
                        if is_pointer > 0 && ! is_mutable {
                            c_typ += "const ";
                        }

                        ident = Some(i.clone());
                    },
                }
            },
            Punct(ref p) => {
                if ident.is_some() && p.as_char() != ':' {
                    // We already found the ident, now skip the <...>.
                    break;
                }
                match p.as_char() {
                    '*' | '&' => {
                        is_pointer += 1;
                    },
                    _ => (),
                }
                //eprintln!("{:?}", p);
            },
            Literal(ref l) => panic!("Unexpected {:?}", l),
            Group(ref g) => panic!("Unexpected {:?}", g),
        }
    }
    if let Some(c_ident) = ident {
        let (c_ident, is_pointer_type) = ident2c(&c_ident);
        if is_pointer_type {
            is_pointer -= 1;
        }
        c_typ += &c_ident;
    } else {
        panic!();
    }

    c_typ.push(' ');
    while is_pointer > 0 {
        c_typ.push('*');
        is_pointer -= 1;
    }

    //eprintln!("==> {:?} // {}", c_typ, is_pointer);
    c_typ
}

pub fn rust2c(fun: &syn::ItemFn) -> String {
    let decl = &fun.decl;
    let return_type = match &decl.output {
        syn::ReturnType::Default => "void".into(),
        syn::ReturnType::Type(_, ref typ) => type2c(typ).trim_end().to_string(),
    };
    let fun_ident = format!("{}", fun.ident);

    let mut s = String::new();
    s += &format!("{}\n{} (", return_type, fun_ident);
    let indent = fun_ident.len() + 2;

    for (i, arg) in decl.inputs.iter().enumerate() {
        // All but the first line need to be indented.
        if i > 0 {
            for _ in 0..indent {
                s.push(' ');
            }
        }

        match arg {
            &syn::FnArg::Captured(ref cap) => {
                let pat_ident = match &cap.pat {
                    &syn::Pat::Ident(ref i) => i,
                    _ => unimplemented!(),
                };
                s += &format!("{}{}", type2c(&cap.ty), pat_ident.ident);
            },
            _ => (),
        }

        // All but the last one need a comma.
        if i < decl.inputs.len() - 1 {
            s += ",\n";
        }
    }

    s += ");";
    s
}
