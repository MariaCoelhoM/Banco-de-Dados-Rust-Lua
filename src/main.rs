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

        // Expecting a table { ok = bool, value = string|nil, error = string|nil }
        if let Value::Table(t) = ret {
            let ok = t.get::<_, bool>("ok").unwrap_or(false);
            let value = t.get::<_, Option<String>>("value").unwrap_or(None);
            let error = t.get::<_, Option<String>>("error").unwrap_or(None);
            Ok(TransformResult { ok, value, error })
        } else {
            // If extension returned something unexpected, treat as pass-through success
            Ok(TransformResult { ok: true, value: value.map(|s| s.to_string()), error: None })
        }
    } else {
        // No extension present: pass through
        Ok(TransformResult { ok: true, value: value.map(|s| s.to_string()), error: None })
    }
}

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    let ext_path = "extensions.lua";
    let ext_code = fs::read_to_string(ext_path).unwrap_or_else(|_| {
        eprintln!("[aviso] Arquivo {ext_path} não encontrado. Usando extensões padrão embutidas.");
        DEFAULT_LUA_EXTENSIONS.to_string()
    });

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
        if stdin.read_line(&mut line).is_err() { break; }
        let line = line.trim();
        if line.is_empty() { continue; }
        if line.eq_ignore_ascii_case("exit") || line.eq_ignore_ascii_case("quit") { break; }

        // Parse
        let mut parts = line.splitn(3, ' ');
        let cmd = parts.next().unwrap_or("").to_uppercase();
        match cmd.as_str() {
            "ADD" => {
                let key = match parts.next() {
                    Some(k) => k.to_string(),
                    None => { eprintln!("Erro: uso: ADD <chave> <valor>"); continue; }
                };
                let value = match parts.next() {
                    Some(v) => v.to_string(),
                    None => { eprintln!("Erro: uso: ADD <chave> <valor>"); continue; }
                };

                // Pass through Lua 'on_add'
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
                    None => { eprintln!("Erro: uso: GET <chave>"); continue; }
                };
                match db.get(&key) {
                    Some(raw) => {
                        // Pass through Lua 'on_get'
                        let tr = call_lua_transform(&lua, "on_get", &key, Some(raw))?;
                        if tr.ok {
                            let out = tr.value.unwrap_or_else(|| raw.clone());
                            println!("{out}");
                        } else {
                            eprintln!("Erro ao formatar: {}", tr.error.unwrap_or_else(|| "motivo desconhecido".to_string()));
                        }
                    }
                    None => {
                        // Allow fallback without underscore for CPF keys (ex.: cpf_zezinho vs cpfzezinho)
                        if key.starts_with("cpf") && !key.starts_with("cpf_") {
                            let alt = format!("cpf_{}", &key[3..]);
                            if let Some(raw) = db.get(&alt) {
                                let tr = call_lua_transform(&lua, "on_get", &alt, Some(raw))?;
                                if tr.ok {
                                    let out = tr.value.unwrap_or_else(|| raw.clone());
                                    println!("{out}");
                                } else {
                                    eprintln!("Erro ao formatar: {}", tr.error.unwrap_or_else(|| "motivo desconhecido".to_string()));
                                }
                                continue;
                            }
                        }
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

// Extensões padrão embutidas para fallback
const DEFAULT_LUA_EXTENSIONS: &str = r#"
-- ######## Extensões Lua ########
-- Retorno deve ser uma tabela: { ok = boolean, value = string|nil, error = string|nil }

local function table_ret(ok, value, error)
  return { ok = ok, value = value, error = error }
end

-- ===== CPF =====
local function cpf_all_same(cpf)
  return cpf:match("^(%d)%1+$") ~= nil
end

local function cpf_calc_digit(cpf, len)
  local sum = 0
  local weight = len + 1
  for i = 1, len do
    local d = tonumber(cpf:sub(i, i))
    sum = sum + d * (weight - i + 1)
  end
  local mod = sum % 11
  local dv = 11 - mod
  if dv >= 10 then dv = 0 end
  return dv
end

local function cpf_is_valid(cpf)
  if not cpf or not cpf:match("^%d%d%d%d%d%d%d%d%d%d%d$") then return false end
  if cpf_all_same(cpf) then return false end
  local d1 = cpf_calc_digit(cpf, 9)
  local d2 = cpf_calc_digit(cpf, 10) -- include d1 in position 10
  return d1 == tonumber(cpf:sub(10,10)) and d2 == tonumber(cpf:sub(11,11))
end

local function cpf_format(cpf)
  return string.format("%s.%s.%s-%s",
    cpf:sub(1,3), cpf:sub(4,6), cpf:sub(7,9), cpf:sub(10,11))
end

-- ===== DATA ISO =====
local function parse_iso_date(s)
  local y, m, d = s:match("^(%d%d%d%d)%-(%d%d)%-(%d%d)$")
  if not y then return nil, "formato inválido (YYYY-MM-DD)" end
  y, m, d = tonumber(y), tonumber(m), tonumber(d)
  if m < 1 or m > 12 then return nil, "mês inválido" end
  if d < 1 or d > 31 then return nil, "dia inválido" end
  -- valida com os.time
  local ok = os.time({year=y, month=m, day=d, hour=12})
  if not ok then return nil, "data inválida" end
  return {year=y, month=m, day=d}
end

local function br_format_date(y, m, d)
  return string.format("%02d/%02d/%04d", d, m, y)
end

-- Hooks principais
function on_add(key, value)
  if key:match("^cpf_") then
    if not value:match("^%d+$") then
      return table_ret(false, nil, "CPF deve conter apenas 11 dígitos")
    end
    if #value ~= 11 then
      return table_ret(false, nil, "CPF deve ter 11 dígitos")
    end
    if not cpf_is_valid(value) then
      return table_ret(false, nil, "CPF inválido (dígito verificador não confere)")
    end
    -- manter apenas dígitos armazenados
    return table_ret(true, value, nil)
  elseif key:match("^data_") then
    local dt, err = parse_iso_date(value)
    if not dt then
      return table_ret(false, nil, "Data ISO8601 inválida: " .. err)
    end
    -- armazena no mesmo formato de entrada ISO
    return table_ret(true, string.format("%04d-%02d-%02d", dt.year, dt.month, dt.day), nil)
  end
  -- padrão: aceitar e armazenar como veio
  return table_ret(true, value, nil)
end

function on_get(key, value)
  if key:match("^cpf_") then
    if not value:match("^%d%d%d%d%d%d%d%d%d%d%d$") then
      return table_ret(false, nil, "Valor armazenado de CPF inválido")
    end
    return table_ret(true, cpf_format(value), nil)
  elseif key:match("^data_") then
    local dt, err = parse_iso_date(value)
    if not dt then
      return table_ret(false, nil, "Valor de data inválido: " .. err)
    end
    return table_ret(true, br_format_date(dt.year, dt.month, dt.day), nil)
  end
  return table_ret(true, value, nil)
end
"#;

