use flowrlib::provider::Provider;

pub struct HttpProvider;

impl Provider for HttpProvider {
    fn resolve(&self, url_str: &str, _default_filename: &str) -> Result<(String, Option<String>), String> {
/*        let url = Url::parse(url_str)
            .map_err(|_| format!("COuld not convert '{}' to valid Url", url_str))?;
        if url.path().ends_with('/') {
            info!("'{}' is a directory, so attempting to find context file in it", url);
            Ok((HttpProvider::find_default_file(&url_str).unwrap(), None))
        } else {
            Ok((url.to_string(), None))
        }
        */
        Ok((url_str.into(), None))
    }

    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
        /*
            let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(
        "https://api.github.com/repos/rustwasm/wasm-bindgen/branches/master",
        &opts,
    )
    .unwrap();

    request
        .headers()
        .set("Accept", "application/vnd.github.v3+json")
        .unwrap();

    let window = web_sys::window().unwrap();
    let request_promise = window.fetch_with_request(&request);

    let future = JsFuture::from(request_promise)
        .and_then(|resp_value| {
            // `resp_value` is a `Response` object.
            assert!(resp_value.is_instance_of::<Response>());
            let resp: Response = resp_value.dyn_into().unwrap();
            resp.json()
        })
        .and_then(|json_value: Promise| {
            // Convert this other `Promise` into a rust `Future`.
            JsFuture::from(json_value)
        })
        .and_then(|json| {
            // Use serde to parse the JSON into a struct.
            let branch_info: Branch = json.into_serde().unwrap();

            // Send the `Branch` struct back to JS as an `Object`.
            future::ok(JsValue::from_serde(&branch_info).unwrap())
        });
        */
        Ok(Vec::from("hello"))
    }
}

impl HttpProvider {
    /*
        Passed a path to a directory, it searches for the first file it can find fitting the pattern
        "context.*", for known file extensions
    */
//    fn find_default_file(_url: &str) -> Result<String, String> {
//        Err("Not implemented yet".to_string())
//    }
}
