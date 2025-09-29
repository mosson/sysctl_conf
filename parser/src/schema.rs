// use std::{collections::HashMap, io::BufRead};

// use crate::config::error::Error;

// #[derive(Clone, Debug, PartialEq, Eq)]
// pub enum ValueType {
//     StringType,
//     BoolType,
//     IntegerType,
// }

// impl ValueType {
//     pub fn check(&self, value: &str) -> Result<(), Error> {
//         match self {
//             Self::StringType => {
//                 value.parse::<String>().map_err(|e| {
//                     Error::InvalidSchema(value.to_string(), "String".to_string(), e.to_string())
//                 })?;
//             }
//             Self::BoolType => {
//                 value.parse::<bool>().map_err(|e| {
//                     Error::InvalidSchema(value.to_string(), "bool".to_string(), e.to_string())
//                 })?;
//             }
//             Self::IntegerType => {
//                 value.parse::<i64>().map_err(|e| {
//                     Error::InvalidSchema(value.to_string(), "i64".to_string(), e.to_string())
//                 })?;
//             }
//         }

//         Ok(())
//     }
// }

// impl TryFrom<&str> for ValueType {
//     type Error = Error;

//     fn try_from(value: &str) -> Result<Self, Self::Error> {
//         match value {
//             "string" => Ok(Self::StringType),
//             "bool" => Ok(Self::BoolType),
//             "integer" => Ok(Self::IntegerType),
//             _ => Err(Error::UndefinedType(value.to_string())),
//         }
//     }
// }

// #[derive(Debug)]
// #[allow(dead_code)]
// pub struct Schema(HashMap<String, ValueType>);

// impl Schema {
//     pub fn parse<T: BufRead>(handle: T) -> Result<Self, Error> {
//         let mut result: HashMap<String, ValueType> = HashMap::new();

//         for line in handle.lines() {
//             let line = line.map_err(|e| Error::Unknown(e.to_string()))?;

//             if line.trim().is_empty() {
//                 continue;
//             }

//             let pair = line.split("->").map(|s| s.trim()).collect::<Vec<_>>();

//             if pair.len() != 2 {
//                 return Err(Error::InvalidKeyValuePair(
//                     line.to_string(),
//                     " -> ".to_string(),
//                 ));
//             }

//             let (key, value) = (pair[0], pair[1]);
//             let value: ValueType = value.try_into()?;

//             result
//                 .entry(key.to_string())
//                 .and_modify(|v| *v = value.clone())
//                 .or_insert(value.clone());
//         }

//         Ok(Self(result))
//     }

//     pub fn get(&self, key: &str) -> Option<&ValueType> {
//         self.0.get(key)
//     }
// }

// #[cfg(test)]
// mod tests {
//     use std::io::{BufReader, Cursor};

//     use super::*;

//     use pretty_assertions::assert_eq;

//     #[test]
//     fn test_parse_valid() {
//         let source = r#"
//             endpoint -> string
//             debug -> bool
//             log.file -> string
//             retry -> integer
//         "#
//         .to_string();
//         let cursor = Cursor::new(source);
//         let handle = BufReader::new(cursor);
//         let result = Schema::parse(handle);

//         assert!(result.is_ok());
//         let result = result.unwrap();
//         let mut keys = result.0.keys().collect::<Vec<_>>();
//         keys.sort();

//         assert_eq!(keys, &["debug", "endpoint", "log.file", "retry",]);
//         assert_eq!(result.get("debug").unwrap(), &ValueType::BoolType);
//         assert_eq!(result.get("endpoint").unwrap(), &ValueType::StringType);
//         assert_eq!(result.get("log.file").unwrap(), &ValueType::StringType);
//         assert_eq!(result.get("retry").unwrap(), &ValueType::IntegerType);
//         assert!(result.get("nothing").is_none());
//     }

//     #[test]
//     fn test_parse_invalid() {
//         let source = r#"
//             endpoint -> string2
//         "#
//         .to_string();
//         let cursor = Cursor::new(source);
//         let handle = BufReader::new(cursor);
//         let result = Schema::parse(handle);

//         assert!(result.is_err());
//         assert_eq!(
//             result.unwrap_err().to_string(),
//             "未定義のデータ型です: string2"
//         );
//     }

//     #[test]
//     fn test_value_type_check() {
//         let checker = ValueType::StringType;
//         assert!(checker.check("foo").is_ok());
//         let checker = ValueType::BoolType;
//         assert!(checker.check("true").is_ok());
//         assert!(checker.check("false").is_ok());
//         assert!(checker.check("foo").is_err());
//         assert_eq!(
//             checker.check("foo").unwrap_err().to_string(),
//             "スキーマ違反です: foo は bool として解釈できません（provided string was not `true` or `false`）"
//         );
//         let checker = ValueType::IntegerType;
//         assert!(checker.check("12").is_ok());
//         assert!(checker.check("-44").is_ok());
//         assert!(checker.check("foo").is_err());
//         assert_eq!(
//             checker.check("foo").unwrap_err().to_string(),
//             "スキーマ違反です: foo は i64 として解釈できません（invalid digit found in string）"
//         );
//     }
// }
