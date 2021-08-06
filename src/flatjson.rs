use serde_json::value::{Number, Value as SerdeValue};
use std::fmt::Debug;

type Index = usize;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum OptionIndex {
    Nil,
    Index(Index),
}

const NIL: usize = usize::MAX;

impl From<usize> for OptionIndex {
    fn from(i: usize) -> Self {
        if i == NIL {
            OptionIndex::Nil
        } else {
            OptionIndex::Index(i)
        }
    }
}

#[derive(Debug)]
pub struct FlatJson(Vec<Row>);

#[derive(Debug)]
pub struct Row {
    parent: OptionIndex,
    // Should these also be set on the CloseContainers?
    prev_sibling: OptionIndex,
    next_sibling: OptionIndex,

    depth: usize,
    index: Index,
    // start_index: usize
    // end_index: usize
    key: Option<String>,
    value: Value,
}

#[derive(Debug)]
enum ContainerType {
    Object,
    Array,
}

#[derive(Debug)]
enum Value {
    Null,
    Boolean(bool),
    Number(Number),
    String(String),
    EmptyObject,
    EmptyArray,
    OpenContainer {
        container_type: ContainerType,
        first_child: Index,
        close_index: Index,
    },
    CloseContainer {
        container_type: ContainerType,
        last_child: Index,
        open_index: Index,
    },
}

pub fn parse_top_level_json(json: String) -> serde_json::Result<FlatJson> {
    let serde_value = serde_json::from_str(&json)?;
    let mut flat_json = vec![];
    let mut parents = vec![OptionIndex::Nil];

    flatten_json(serde_value, &mut flat_json, &mut parents);

    Ok(FlatJson(flat_json))
}

fn flatten_json(serde_value: SerdeValue, flat_json: &mut Vec<Row>, parents: &mut Vec<OptionIndex>) {
    let depth = parents.len() - 1;
    let parent = *parents.last().unwrap();

    let row = Row {
        parent,
        prev_sibling: OptionIndex::Nil,
        next_sibling: OptionIndex::Nil,
        depth,
        index: 0,
        key: None,
        value: Value::Null,
    };

    match serde_value {
        SerdeValue::Null => flat_json.push(row),
        SerdeValue::Bool(b) => flat_json.push(Row {
            value: Value::Boolean(b),
            ..row
        }),
        SerdeValue::Number(n) => flat_json.push(Row {
            value: Value::Number(n),
            ..row
        }),
        SerdeValue::String(s) => flat_json.push(Row {
            value: Value::String(s),
            ..row
        }),
        SerdeValue::Array(vs) => {
            if vs.len() == 0 {
                flat_json.push(Row {
                    value: Value::EmptyArray,
                    ..row
                })
            } else {
                let open_index = flat_json.len();
                parents.push(OptionIndex::Index(open_index));

                flat_json.push(Row {
                    value: Value::OpenContainer {
                        container_type: ContainerType::Array,
                        first_child: open_index + 1,
                        // Set once done processing the array.
                        close_index: 0,
                    },
                    ..row
                });

                let mut prev_sibling: OptionIndex = OptionIndex::Nil;
                let mut child_index = 0;

                for (i, v) in vs.into_iter().enumerate() {
                    child_index = flat_json.len();

                    flatten_json(v, flat_json, parents);
                    let mut child = &mut flat_json[child_index];

                    child.index = i;
                    child.prev_sibling = prev_sibling;

                    if let OptionIndex::Index(prev_sibling_index) = prev_sibling {
                        flat_json[prev_sibling_index].next_sibling =
                            OptionIndex::Index(child_index);
                    }

                    prev_sibling = OptionIndex::Index(child_index);
                }

                let close_index = flat_json.len();
                flat_json.push(Row {
                    parent,
                    // Currently not set on the CloseContainer value.
                    prev_sibling: OptionIndex::Nil,
                    next_sibling: OptionIndex::Nil,
                    depth,
                    index: 0,
                    key: None,
                    value: Value::CloseContainer {
                        container_type: ContainerType::Array,
                        last_child: child_index,
                        // Set once done processing the array.
                        open_index,
                    },
                });

                if let Value::OpenContainer {
                    close_index: ref mut close_index_of_open_value,
                    ..
                } = &mut flat_json[open_index].value
                {
                    *close_index_of_open_value = close_index;
                }

                parents.pop();
            }
        }
        SerdeValue::Object(obj) => {
            if obj.len() == 0 {
                flat_json.push(Row {
                    value: Value::EmptyObject,
                    ..row
                })
            } else {
                let open_index = flat_json.len();
                parents.push(OptionIndex::Index(open_index));

                flat_json.push(Row {
                    value: Value::OpenContainer {
                        container_type: ContainerType::Object,
                        first_child: open_index + 1,
                        // Set once done processing the array.
                        close_index: 0,
                    },
                    ..row
                });

                let mut prev_sibling: OptionIndex = OptionIndex::Nil;
                let mut child_index = 0;

                for (i, (k, v)) in obj.into_iter().enumerate() {
                    child_index = flat_json.len();

                    flatten_json(v, flat_json, parents);
                    let mut child = &mut flat_json[child_index];

                    child.index = i;
                    child.prev_sibling = prev_sibling;
                    child.key = Some(k);

                    if let OptionIndex::Index(prev_sibling_index) = prev_sibling {
                        flat_json[prev_sibling_index].next_sibling =
                            OptionIndex::Index(child_index);
                    }

                    prev_sibling = OptionIndex::Index(child_index);
                }

                let close_index = flat_json.len();
                flat_json.push(Row {
                    parent,
                    // Currently not set on the CloseContainer value.
                    prev_sibling: OptionIndex::Nil,
                    next_sibling: OptionIndex::Nil,
                    depth,
                    index: 0,
                    key: None,
                    value: Value::CloseContainer {
                        container_type: ContainerType::Object,
                        last_child: child_index,
                        // Set once done processing the array.
                        open_index,
                    },
                });

                if let Value::OpenContainer {
                    close_index: ref mut close_index_of_open_value,
                    ..
                } = &mut flat_json[open_index].value
                {
                    *close_index_of_open_value = close_index;
                }

                parents.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBJECT_1: &'static str = r#"{
        "a": 2,
        "b": [
            4,
            "5"
        ],
        "c": {
            "d": null,
            "e": true,
            "f": 9
        },
        "g": 11
    }"#;

    #[test]
    fn test_flatten_json() {
        let fj = parse_top_level_json(OBJECT_1.to_owned()).unwrap();

        assert_prev_siblings(
            &fj,
            vec![NIL, NIL, 1, NIL, 3, NIL, 2, NIL, 7, 8, NIL, 6, NIL],
        )
    }

    fn assert_prev_siblings<T: Into<OptionIndex> + Debug + Copy>(
        fj: &FlatJson,
        prev_siblings: Vec<T>,
    ) {
        assert_eq!(
            fj.0.len(),
            prev_siblings.len(),
            "length of prev_siblings is incorrect"
        );

        for (i, (elem, expected_prev_sibling)) in fj.0.iter().zip(prev_siblings.iter()).enumerate()
        {
            assert_eq!(
                Into::<OptionIndex>::into(*expected_prev_sibling),
                elem.prev_sibling,
                "incorrect prev_sibling at index {}",
                i
            );
        }
    }
}