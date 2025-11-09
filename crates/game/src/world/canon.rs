use serde::Serialize;

pub fn canonical_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    // Use a custom serializer that ensures sorted keys
    let json_value = serde_json::to_value(value)?;
    let sorted_json = sort_json_value(json_value);
    let json_string = serde_json::to_string(&sorted_json)?;

    // Ensure LF line endings
    let mut s = json_string;
    s = s.replace("\r\n", "\n");
    Ok(s)
}

fn sort_json_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            // Sort the keys in the map
            let mut sorted_items = map.into_iter().collect::<Vec<_>>();
            sorted_items.sort_by(|(a, _), (b, _)| a.cmp(b));

            let mut sorted_map = serde_json::Map::new();
            for (k, v) in sorted_items {
                sorted_map.insert(k, sort_json_value(v));
            }
            serde_json::Value::Object(sorted_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(sort_json_value).collect())
        }
        _ => value, // For primitives, return as is
    }
}

#[allow(dead_code)]
struct CanonicalFormatter<W> {
    writer: W,
}

#[allow(dead_code)]
impl<W> CanonicalFormatter<W> {
    fn new(writer: W) -> Self {
        Self { writer }
    }
}

impl<W> serde_json::ser::Formatter for CanonicalFormatter<W>
where
    W: std::io::Write,
{
    fn begin_array<WOut>(&mut self, writer: &mut WOut) -> std::io::Result<()>
    where
        WOut: ?Sized + std::io::Write,
    {
        writer.write_all(b"[")
    }

    fn end_array<WOut>(&mut self, writer: &mut WOut) -> std::io::Result<()>
    where
        WOut: ?Sized + std::io::Write,
    {
        writer.write_all(b"]")
    }

    fn begin_array_value<WOut>(&mut self, writer: &mut WOut, first: bool) -> std::io::Result<()>
    where
        WOut: ?Sized + std::io::Write,
    {
        if !first {
            writer.write_all(b",")?;
        }
        Ok(())
    }

    fn end_array_value<WOut>(&mut self, _writer: &mut WOut) -> std::io::Result<()>
    where
        WOut: ?Sized + std::io::Write,
    {
        Ok(())
    }

    fn begin_object<WOut>(&mut self, writer: &mut WOut) -> std::io::Result<()>
    where
        WOut: ?Sized + std::io::Write,
    {
        writer.write_all(b"{")
    }

    fn end_object<WOut>(&mut self, writer: &mut WOut) -> std::io::Result<()>
    where
        WOut: ?Sized + std::io::Write,
    {
        writer.write_all(b"}")
    }

    fn begin_object_key<WOut>(&mut self, writer: &mut WOut, first: bool) -> std::io::Result<()>
    where
        WOut: ?Sized + std::io::Write,
    {
        if !first {
            writer.write_all(b",")?;
        }
        Ok(())
    }

    fn end_object_key<WOut>(&mut self, _writer: &mut WOut) -> std::io::Result<()>
    where
        WOut: ?Sized + std::io::Write,
    {
        Ok(())
    }

    fn begin_object_value<WOut>(&mut self, writer: &mut WOut) -> std::io::Result<()>
    where
        WOut: ?Sized + std::io::Write,
    {
        writer.write_all(b":")
    }

    fn end_object_value<WOut>(&mut self, _writer: &mut WOut) -> std::io::Result<()>
    where
        WOut: ?Sized + std::io::Write,
    {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_json() {
        // Test that keys are sorted and output is consistent
        #[derive(Serialize)]
        struct TestObj {
            z_field: u32,
            a_field: u32,
            m_field: u32,
        }

        let obj = TestObj {
            z_field: 3,
            a_field: 1,
            m_field: 2,
        };

        let json = canonical_json(&obj).unwrap();
        // Should be: {"a_field":1,"m_field":2,"z_field":3}
        assert!(json.find("a_field").unwrap() < json.find("m_field").unwrap());
        assert!(json.find("m_field").unwrap() < json.find("z_field").unwrap());

        // Verify the exact format
        assert_eq!(json, r#"{"a_field":1,"m_field":2,"z_field":3}"#);
    }

    #[test]
    fn test_nested_objects() {
        #[derive(Serialize)]
        struct Inner {
            y: u32,
            x: u32,
        }

        #[derive(Serialize)]
        struct Outer {
            inner: Inner,
            b: u32,
            a: u32,
        }

        let obj = Outer {
            inner: Inner { y: 2, x: 1 },
            b: 4,
            a: 3,
        };

        let json = canonical_json(&obj).unwrap();
        // Should have sorted keys at all levels
        assert!(json.contains(r#"{"x":1,"y":2}"#)); // inner object keys sorted
    }

    #[test]
    fn test_arrays() {
        #[derive(Serialize)]
        struct TestObj {
            arr: Vec<u32>,
        }

        let obj = TestObj { arr: vec![3, 1, 2] };

        let json = canonical_json(&obj).unwrap();
        assert!(json.contains(r#"[3,1,2]"#)); // arrays preserve order
    }
}
