-- Arquivo de extensões Lua carregado pelo Rust (você pode editar/expandir)
-- Retornar { ok = boolean, value = string|nil, error = string|nil } em on_add/on_get

local function table_ret(ok, value, error)
  return { ok = ok, value = value, error = error }
end

-- ===== CPF =====
local function cpf_all_same(cpf)
  return cpf:match("^(%d)%1+$") ~= nil
end

local function cpf_calc_digit_partial(cpf, len, start_weight)
  local sum = 0
  local weight = start_weight
  for i = 1, len do
    local d = tonumber(cpf:sub(i, i))
    sum = sum + d * weight
    weight = weight - 1
  end
  local mod = sum % 11
  local dv = 11 - mod
  if dv >= 10 then dv = 0 end
  return dv
end

local function cpf_is_valid(cpf)
  if not cpf or not cpf:match("^%d%d%d%d%d%d%d%d%d%d%d$") then return false end
  if cpf_all_same(cpf) then return false end
  local d1 = cpf_calc_digit_partial(cpf, 9, 10)
  local d2 = cpf_calc_digit_partial(cpf, 10, 11)
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
  local ok = os.time({year=y, month=m, day=d, hour=12})
  if not ok then return nil, "data inválida" end
  return {year=y, month=m, day=d}
end

local function br_format_date(y, m, d)
  return string.format("%02d/%02d/%04d", d, m, y)
end

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
    return table_ret(true, value, nil)
  elseif key:match("^data_") then
    local dt, err = parse_iso_date(value)
    if not dt then
      return table_ret(false, nil, "Data ISO8601 inválida: " .. err)
    end
    return table_ret(true, string.format("%04d-%02d-%02d", dt.year, dt.month, dt.day), nil)
  end
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
