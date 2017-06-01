/// Tangle request message.
///
/// ```javascript
/// ["chat.send_message", {"request_id": "123"}, ["text"], {}]
/// ```
use std::str;
use std::ascii::AsciiExt;

use serde_json::{self, Value as Json, Map, Error as JsonError};
use serde::ser::{Serialize, Serializer, SerializeTuple};
use serde::de::Error;

pub type Meta = Map<String, Json>;
pub type Args = Vec<Json>;
pub type Kwargs = Map<String, Json>;


/// Decode Websocket json message into Meta & Message structs.
pub fn decode_message(s: &str)
    -> Result<(String, Meta, Args, Kwargs), JsonError>
{

    let res = serde_json::from_str::<Request>(s)?;
    res.validate()?;
    let Request(method, meta, args, kwargs) = res;
    Ok((method, meta, args, kwargs))
}


/// Returns true if Meta object contains 'active' key and
/// it either set to true or uint timeout (in seconds).
pub fn get_active(meta: &Meta) -> Option<u64>
{
    meta.get(&"active".to_string()).and_then(|v| v.as_u64())
}


#[derive(Serialize)]
pub struct AuthData {
    pub http_cookie: Option<String>,
    pub http_authorization: Option<String>,
    pub url_querystring: String,
}

// Private tools

pub struct Auth<'a>(pub &'a String, pub &'a AuthData);

impl<'a> Serialize for Auth<'a> {
    fn serialize<S: Serializer>(&self, serializer: S)
        -> Result<S::Ok, S::Error>
    {
        let mut tup = serializer.serialize_tuple(3)?;
        tup.serialize_element(&json!({"connection_id": self.0}))?;
        tup.serialize_element(&json!([]))?;
        tup.serialize_element(&self.1)?;
        tup.end()
    }
}

pub struct Call<'a>(pub &'a Meta, pub &'a String, pub &'a Args, pub &'a Kwargs);

impl<'a> Serialize for Call<'a> {
    fn serialize<S: Serializer>(&self, serializer: S)
        -> Result<S::Ok, S::Error>
    {
        let mut tup = serializer.serialize_tuple(3)?;
        tup.serialize_element(&MetaWithExtra {
            meta: self.0,
            extra: json!({"connection_id": self.1}),
        })?;
        tup.serialize_element(&self.2)?;
        tup.serialize_element(&self.3)?;
        tup.end()
    }
}

pub struct MetaWithExtra<'a> {
    pub meta: &'a Meta,
    pub extra: Json,
}
impl<'a> Serialize for MetaWithExtra<'a> {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        if let Json::Object(ref extra) = self.extra {
            s.collect_map(extra.iter()
                .chain(self.meta.iter()
                    .filter(|&(&ref k,_)| extra.get(k).is_none())))
        } else {
            s.collect_map(self.meta.iter())
        }
    }
}

#[derive(Deserialize)]
struct Request(String, Meta, Args, Kwargs);

impl Request {
    fn validate(&self) -> Result<(), JsonError> {
        let method = self.0.as_str();
        if method.len() == 0 {
            return Err(JsonError::custom("invalid method"))
        }
        if method.starts_with("tangle.") {
            return Err(JsonError::custom("invalid method"))
        }
        if !method.chars().all(|c| c.is_ascii() &&
            (c.is_alphanumeric() || c == '-' || c == '_' || c == '.'))
        {
            return Err(JsonError::custom("invalid method"))
        }
        match self.1.get("request_id") {
            Some(&Json::Number(_)) => {},
            Some(&Json::String(ref s)) if s.len() > 0 => {}
            _ => return Err(JsonError::custom("invalid request_id"))
        }
        Ok(())
    }
}


#[cfg(test)]
mod test {
    use serde_json::Value as Json;
    use serde_json::to_string as json_encode;

    use chat::message::{self, Call, Meta, Args, Kwargs, Auth, AuthData};

    #[test]
    fn decode_message_errors() {
        macro_rules! error_starts {
            ($a:expr, $b:expr) => {
                {
                    let rv = message::decode_message($a);
                    assert!(rv.is_err(),
                        format!("unexpectedly valid: {}", $a));
                    let err = format!("{}", rv.err().unwrap());
                    assert!(err.starts_with($b),
                        format!("{}: {} != {}", $a, err, $b));
                }
            };
            ($( $a:expr, $b:expr ),+) => {
                $( error_starts!($a, $b) );*
            };
        }

        error_starts!(
            "",
                "EOF while parsing a value"
        );
        error_starts!(
            "[invalid json",
                "expected value at line 1"
        );
        error_starts!(
            "{}",
                "invalid type: map, expected tuple struct"
        );
        error_starts!(
            "[]",
                "invalid length 0"
        );
        error_starts!(
            "[1, 2, 3, 4, 5]",
                "invalid type: integer `1`, expected a string"
        );
        error_starts!(
            "[1, 2, 3, 4]",
                "invalid type: integer `1`, expected a string"
        );
        error_starts!(
            "[null, null, null, 4]",
                "invalid type: unit value, expected a string"
        );
        error_starts!(
            "[\"1\", 2, 3, 4]",
                "invalid type: integer `2`, expected a map"
        );
        error_starts!(
            "[\"1\", {}, 3, 4]",
                "invalid type: integer `3`, expected a sequence"
        );
        error_starts!(
            "[\"1\", {}, [], 4]",
                "invalid type: integer `4`, expected a map"
        );
        error_starts!(
            "[\"1\", {}, [], {}]",
                "invalid request_id"
        );
        error_starts!(
            "[\"1\", {\"request_id\": null}, [], {}]",
                "invalid request_id"
        );
        error_starts!(
            "[\"foo\", {\"request_id\": []}, [], {}]",
                "invalid request_id"
        );
        error_starts!(
            "[\"foo\", {\"request_id\": {}}, [], {}]",
                "invalid request_id"
        );
        error_starts!(
            "[\"foo\", {\"request_id\": \"\"}, [], {}]",
                "invalid request_id"
        );
        error_starts!(
            "[\"\", {\"request_id\": 123}, [], {}]",
                "invalid method"
        );
        error_starts!(
            "[\"bad/method\", {\"request_id\": 123}, [], {}]",
                "invalid method"
        );
        error_starts!(
            "[\"very bad method\", {\"request_id\": 123}, [], {}]",
                "invalid method"
        );
        error_starts!(
            "[\"tangle.auth\", {\"request_id\": 123}, [], {}]",
                "invalid method"
        );
        error_starts!(
            "[\"   tangle.auth\", {\"request_id\": 123}, [], {}]",
                "invalid method"
        );
        error_starts!(
            "[\"   bad.method   \", {\"request_id\": 123}, [], {}]",
                "invalid method"
        );
    }

    #[test]
    fn decode_message() {
        let res = message::decode_message(r#"
            ["some.method", {"request_id": "123"}, ["Hello"], {"world!": "!"}]
            "#).unwrap();
        let (method, meta, args, kwargs) = res;
        assert_eq!(method, "some.method".to_string());
        match meta.get("request_id").unwrap() {
            &Json::String(ref s) => assert_eq!(s, &"123".to_string()),
            _ => unreachable!(),
        }
        assert_eq!(args.len(), 1);
        match kwargs.get("world!".into()).unwrap() {
            &Json::String(ref s) => assert_eq!(s, &"!".to_string()),
            _ => unreachable!(),
        }
    }

    #[test]
    fn encode_auth() {
        let res = json_encode(&Auth(&"conn:1".to_string(), &AuthData {
            http_cookie: None, http_authorization: None,
            url_querystring: "".to_string(),
        })).unwrap();
        assert_eq!(res, concat!(
            r#"[{"connection_id":"conn:1"},[],{"#,
            r#""http_cookie":null,"http_authorization":null,"#,
            r#""url_querystring":""}]"#));

        let kw = AuthData {
            http_cookie: Some("auth=ok".to_string()),
            http_authorization: None,
            url_querystring: "".to_string(),
        };

        let res = json_encode(&Auth(&"conn:2".to_string(), &kw)).unwrap();
        assert_eq!(res, concat!(
            r#"[{"connection_id":"conn:2"},"#,
            r#"[],{"http_cookie":"auth=ok","#,
            r#""http_authorization":null,"url_querystring":""}]"#));
    }

    #[test]
    fn encode_call() {
        let mut meta = Meta::new();
        let mut args = Args::new();
        let mut kw = Kwargs::new();
        let cid = "123".to_string();

        let res = json_encode(&Call(&meta, &cid, &args, &kw)).unwrap();
        assert_eq!(res, "[{\"connection_id\":\"123\"},[],{}]");

        meta.insert("request_id".into(), json!("123"));
        args.push(json!("Hello"));
        args.push(json!("World!"));
        kw.insert("room".into(), json!(123));

        let res = json_encode(&Call(&meta, &cid, &args, &kw)).unwrap();
        assert_eq!(res, concat!(
            r#"[{"connection_id":"123","request_id":"123"},"#,
            r#"["Hello","World!"],"#,
            r#"{"room":123}]"#));

        meta.insert("connection_id".into(), json!("321"));
        let res = json_encode(&Call(&meta, &cid, &args, &kw)).unwrap();
        assert_eq!(res, concat!(
            r#"[{"connection_id":"123","request_id":"123"},"#,
            r#"["Hello","World!"],"#,
            r#"{"room":123}]"#));
    }

    #[test]
    fn get_active() {
        let mut meta = Meta::new();

        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), json!(""));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), json!(true));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), json!(123i64));
        assert_eq!(message::get_active(&meta).unwrap(), 123u64);

        meta.insert("active".into(), json!(-123));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), json!(123f64));
        assert!(message::get_active(&meta).is_none());

        meta.insert("active".into(), json!(123));
        assert_eq!(message::get_active(&meta).unwrap(), 123u64);
    }
}
