use mlua::{Lua, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

#[derive(Debug)]
struct TransformResult {
    ok: bool,
    value: Option<String>,
    error: Option<String>,
}

fn call_lua_transform(lua: &Lua, func_name: &str, key: &str, value: Option<&str>) -> mlua::Result<TransformResult> {
    let globals = lua.globals();
    let func = globals.get::<_, Option<mlua::Function>>(func_name)?;
    if let Some(f) = func {
        let args = match value {
            Some(v) => (key.to_string(), Some(v.to_string())),
            None => (key.to_string(), Option::<String>::None),
        };
        let ret: Value = f.call(args)?;

        if let Value::Table(t) = ret {
            let ok = t.get::<_, bool>("ok").unwrap_or(false);
            let value = t.get::<_, Option<String>>("value").unwrap_or(None);
            let error = t.get::<_, Option<String>>("error").unwrap_or(None);
            Ok(TransformResult { ok, value, error })
        } else {
            Ok(TransformResult { ok: true, value: value.map(|s| s.to_string()), error: None })
        }
    } else {
        Ok(TransformResult { ok: true, value: value.map(|s| s.to_string()), error: None })
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    let ext_path = "extensions.lua";
    let ext_code = fs::read_to_string(ext_path)
        .expect("Arquivo extensions.lua não encontrado. Crie-o antes de rodar.");

    lua.load(&ext_code)
        .set_name("extensions.lua")
        .exec()?;

    let mut db: HashMap<String, String> = HashMap::new();
    println!("KV (Rust + Lua). Comandos: ADD <chave> <valor> | GET <chave> | EXIT");

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.eq_ignore_ascii_case("exit") || line.eq_ignore_ascii_case("quit") {
            break;
        }

        let mut parts = line.splitn(3, ' ');
        let cmd = parts.next().unwrap_or("").to_uppercase();
        match cmd.as_str() {
            "ADD" => {
                let key = match parts.next() {
                    Some(k) => k.to_string(),
                    None => {
                        eprintln!("Erro: uso: ADD <chave> <valor>");
                        continue;
                    }
                };
                let value = match parts.next() {
                    Some(v) => v.to_string(),
                    None => {
                        eprintln!("Erro: uso: ADD <chave> <valor>");
                        continue;
                    }
                };

                let tr = call_lua_transform(&lua, "on_add", &key, Some(&value))?;
                if tr.ok {
                    let store_val = tr.value.unwrap_or(value);
                    db.insert(key.clone(), store_val);
                    println!("OK");
                } else {
                    eprintln!("Erro ao adicionar: {}", tr.error.unwrap_or_else(|| "motivo desconhecido".to_string()));
                }
            }
            "GET" => {
                let key = match parts.next() {
                    Some(k) => k.to_string(),
                    None => {
                        eprintln!("Erro: uso: GET <chave>");
                        continue;
                    }
                };
                match db.get(&key) {
                    Some(raw) => {
                        let tr = call_lua_transform(&lua, "on_get", &key, Some(raw))?;
                        if tr.ok {
                            let out = tr.value.unwrap_or_else(|| raw.clone());
                            println!("{out}");
                        } else {
                            eprintln!("Erro ao formatar: {}", tr.error.unwrap_or_else(|| "motivo desconhecido".to_string()));
                        }
                    }
                    None => {
                        eprintln!("Chave não encontrada");
                    }
                }
            }
            _ => {
                eprintln!("Comando inválido. Use: ADD | GET | EXIT");
            }
        }
    }

    Ok(())
}
