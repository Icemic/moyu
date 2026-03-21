use sixu::format::Literal;

pub(crate) fn convert_to_literal(value: serde_json::Value) -> Literal {
    match value {
        serde_json::Value::Null => Literal::Null,
        serde_json::Value::Bool(v) => Literal::Boolean(v),
        serde_json::Value::Number(number) => {
            if let Some(v) = number.as_i64() {
                Literal::Integer(v)
            } else if let Some(v) = number.as_f64() {
                Literal::Float(v)
            } else {
                Literal::String(number.to_string())
            }
        }
        serde_json::Value::String(v) => Literal::String(v),
        serde_json::Value::Array(values) => {
            let converted_values = values.into_iter().map(convert_to_literal).collect();
            Literal::Array(converted_values)
        }
        serde_json::Value::Object(map) => {
            let converted_map = map
                .into_iter()
                .map(|(k, v)| (k, convert_to_literal(v)))
                .collect();
            Literal::Object(converted_map)
        }
    }
}
