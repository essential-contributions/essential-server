use super::*;

#[test]
fn test_map() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Map {
        #[serde(
            serialize_with = "super::serialize_map",
            deserialize_with = "super::deserialize_map"
        )]
        map: BTreeMap<ContentAddress, BTreeMap<Key, Value>>,
    }

    let map: BTreeMap<_, BTreeMap<_, _>> = (0..10)
        .map(|i| {
            (
                ContentAddress([i; 32]),
                (0..10).map(|i| (vec![i], vec![i])).collect(),
            )
        })
        .collect();

    let map = Map { map };
    let s = serde_json::to_string(&map).unwrap();
    let map2: Map = serde_json::from_str(&s).unwrap();
    assert_eq!(map.map, map2.map);
}
