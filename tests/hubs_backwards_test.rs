use base64::engine::general_purpose::STANDARD as b64;
use base64::Engine;
use hex::FromHex;
use pretty_assertions::assert_eq;
use reqwest::{Client, Method, Response};
use serde_json::{json, Value};
use std::env;
use tokio;
/// We probably don't actually want to be normalizing the diffs
fn normalize_bytes(v: &mut Value) {
    match v {
        Value::String(s) if s.starts_with("0x") => {
            let bytes = Vec::from_hex(&s[2..]).expect("invalid hex");
            *v = json!(bytes);
        }
        Value::String(s)
            if s.len() > 16
                && s.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+/=\n\r".contains(c)) =>
        {
            let bytes = b64.decode(s).expect("invalid base64");
            *v = json!(bytes);
        }
        Value::Array(arr) => arr.iter_mut().for_each(normalize_bytes),
        Value::Object(map) => map.values_mut().for_each(normalize_bytes),
        _ => {}
    }
}

async fn fetch_json(client: &Client, method: Method, url: &str, body: Option<Vec<u8>>) -> Value {
    let mut req = client.request(method, url);
    if let Some(bytes) = body {
        req = req
            .header("Content-Type", "application/octet-stream")
            .body(bytes);
    }
    let resp: Response = req.send().await.expect("request failed");
    assert!(
        resp.status().is_success(),
        "non-200 on {}: {}",
        url,
        resp.status()
    );
    resp.json().await.expect("invalid JSON")
}
// test generator macro
macro_rules! neynar_test {
    // $name: identifier for the fn
    // $path: string literal for the endpoint path
    ($name:ident, $path:expr) => {
        #[tokio::test]
        async fn $name() {
            let client = Client::new();
            let local_base = "http://localhost:3381";
            let remote_base = "https://nemes.farcaster.xyz:2281";
            let local_url = format!("{}{}", local_base, $path);
            let remote_url = format!("{}{}", remote_base, $path);
            let mut local_json = fetch_json(&client, Method::GET, &local_url, None).await;
            let mut remote_json = fetch_json(&client, Method::GET, &remote_url, None).await;
            normalize_bytes(&mut local_json);
            normalize_bytes(&mut remote_json);
            assert_eq!(
                local_json,
                remote_json,
                "\nResponse mismatch for `{}`\n--- LOCAL ---\n{}\n--- REMOTE ---\n{}\n",
                $path,
                serde_json::to_string_pretty(&local_json).unwrap(),
                serde_json::to_string_pretty(&remote_json).unwrap(),
            );
        }
    };
}
// Generate one test for each endpoint:
neynar_test!(
    storage_limits_by_fid,
    "https://lamia.farcaster.xyz:2281/v1/storageLimitsByFid?fid=12345"
);
neynar_test!(username_proofs_by_fid, "/v1/userNameProofsByFid?fid=12345");
neynar_test!(
    username_proof_by_name,
    "/v1/userNameProofByName?name=adityapk"
);

// Version number is off otherwise looks good
neynar_test!(onchain_signers_by_fid, "/v1/onChainSignersByFid?fid=12345");

// Endpoint is not implemented/not working -> 404 on neynar docs
neynar_test!(
    onchain_id_registry_by_address,
    "/v1/onChainIdRegistryEventByAddress?address=0x74232bf61e994655592747e20bdf6fa9b9476f79"
);
neynar_test!(fetch_all_events, "/v1/events");
neynar_test!(fetch_event_by_id, "/v1/eventById?id=98765");

// The type enum is inconsistent causing 400 bad requests - response may also be wrong
neynar_test!(
    onchain_events_by_fid,
    "/v1/onChainEventsByFid?fid=3&event_type=1"
); // Had to add &event_type=1

neynar_test!(
    reactions_by_target,
    "/v1/reactionsByTarget?reaction_type=REACTION_TYPE_LIKE&url=https%3A%2F%2Fwarpcast.com%2Fquazia%2F0xcae9dc22"
);
neynar_test!(
    links_by_id,
    "/v1/linkById?link_type=follow&fid=2&target_fid=1"
);
neynar_test!(links_by_fid, "/v1/linksByFid?fid=12345");

neynar_test!(
    reaction_by_id,
    "/v1/reactionById?reaction_type=REACTION_TYPE_LIKE&fid=2&target_fid=2&target_hash=0x03aff391a6eb1772b20b4ead9a89f732be75fe27"
);
