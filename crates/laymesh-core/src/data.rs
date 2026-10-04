use crate::{Diagnostic, Loc, Result, model::V};
use serde_json::Value as Json;
use indexmap::IndexMap;

// Keep user JSON object order without changing Scene JSON representation.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum OrderedJson {
    Object(IndexMap<String, OrderedJson>),
    Array(Vec<OrderedJson>),
    String(String),
    Number(serde_json::Number),
    Bool(bool),
    Null,
}

pub fn load(kind: &str, path: &str, bytes: &[u8], file: &str, loc: Loc) -> Result<V> {
    let error = |message: String| Diagnostic::new("E_DATA", message, file, loc);
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let valid_number =
        |n: f64| n.is_finite() && (n.fract() != 0. || n.abs() <= 9_007_199_254_740_991.);
    if kind == "dict" {
        let value: OrderedJson = serde_json::from_slice(bytes).map_err(|e| error(format!("无效 JSON：{e}")))?;
        if !matches!(value,OrderedJson::Object(_)) { return Err(error("dict JSON 需要对象 / dict JSON requires an object".into())); }
        fn convert(v: OrderedJson) -> std::result::Result<V, String> {
            Ok(match v {
                OrderedJson::Null => V::Null,
                OrderedJson::Bool(v) => V::Bool(v),
                OrderedJson::String(v) => V::text(v),
                OrderedJson::Number(v) => {
                    let n = v.as_f64().filter(|n| n.is_finite() && (n.fract()!=0. || n.abs()<=9_007_199_254_740_991.)).ok_or("字典数值必须有限且能精确表示")?;
                    V::num(n)
                }
                OrderedJson::Array(v) => V::List(v.into_iter().map(convert).collect::<std::result::Result<_,_>>()?),
                OrderedJson::Object(v) => V::Dict(v.into_iter().map(|(k,v)|Ok((k,convert(v)?))).collect::<std::result::Result<_,String>>()?),
            })
        }
        return convert(value).map_err(error);
    }
    let cell = |v: &Json, strings: bool| -> Result<V> {
        match v {
            Json::Null => Ok(V::Null),
            Json::String(s) if strings => Ok(V::text(s)),
            Json::Number(n) if n.as_f64().is_some_and(valid_number) => {
                Ok(V::num(n.as_f64().unwrap()))
            }
            _ => Err(error(
                "数据单元格需要可精确表示的有限数字、null，或表格字符串".into(),
            )),
        }
    };
    if kind == "table" && path.to_lowercase().ends_with(".csv") {
        let mut reader = csv::Reader::from_reader(bytes);
        let headers = reader.headers().map_err(|e| error(e.to_string()))?.clone();
        let mut columns: IndexMap<String, Vec<V>> = IndexMap::new();
        for name in &headers {
            if name.trim().is_empty() || columns.insert(name.into(), vec![]).is_some() {
                return Err(error("CSV 表头需要非空、无重复的列名".into()));
            }
        }
        let mut rows = 0;
        for record in reader.records() {
            let record = record.map_err(|e| error(e.to_string()))?;
            rows += 1;
            for (name, value) in headers.iter().zip(record.iter()) {
                let value = value.trim();
                let v = if value.is_empty() {
                    V::Null
                } else if let Ok(n) = value.parse::<f64>() {
                    if !valid_number(n) {
                        return Err(error("CSV 数值必须有限且能精确表示".into()));
                    }
                    V::num(n)
                } else {
                    V::text(value)
                };
                columns.get_mut(name).unwrap().push(v);
            }
        }
        if rows == 0 || columns.is_empty() {
            return Err(error("CSV 需要非空数据行".into()));
        }
        return Ok(V::Dict(
            columns.into_iter().map(|(k, v)| (k, V::List(v))).collect(),
        ));
    }
    if kind == "array" {
        let value: Json = serde_json::from_slice(bytes).map_err(|e| error(format!("无效 JSON：{e}")))?;
        let values = value
            .as_array()
            .filter(|a| !a.is_empty())
            .ok_or_else(|| error("array 需要非空的一维或二维数组".into()))?;
        if let Some(first) = values[0].as_array() {
            let width = first.len();
            if width == 0 {
                return Err(error("二维数组须为非空矩形矩阵".into()));
            }
            return Ok(V::List(
                values
                    .iter()
                    .map(|row| {
                        let row = row
                            .as_array()
                            .filter(|r| r.len() == width)
                            .ok_or_else(|| error("二维数组须为非空矩形矩阵".into()))?;
                        Ok(V::List(
                            row.iter().map(|v| cell(v, false)).collect::<Result<_>>()?,
                        ))
                    })
                    .collect::<Result<_>>()?,
            ));
        }
        return Ok(V::List(
            values
                .iter()
                .map(|v| cell(v, false))
                .collect::<Result<_>>()?,
        ));
    }
    let object: IndexMap<String, Json> = serde_json::from_slice(bytes).map_err(|e|error(format!("table JSON 需要列对象：{e}")))?;
    if object.is_empty() { return Err(error("table JSON 需要非空的列对象".into())); }
    let mut length = None;
    let mut columns = IndexMap::new();
    for (name, values) in &object {
        let values = values
            .as_array()
            .filter(|a| !a.is_empty() && !name.is_empty() && length.is_none_or(|n| n == a.len()))
            .ok_or_else(|| error("table 各列必须为等长非空数组，列名不能为空".into()))?;
        length = Some(values.len());
        columns.insert(
            name.clone(),
            V::List(
                values
                    .iter()
                    .map(|v| cell(v, true))
                    .collect::<Result<_>>()?,
            ),
        );
    }
    Ok(V::Dict(columns))
}
