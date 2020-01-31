/// `join` is a helper function for operating on urls in a flow
/// If the second url has a scheme, assume it is absolute and just use it
/// If the second url does not have a scheme, assume it is relative to the first url:
/// - remove the filename from the first url
/// - join the second relative url onto the end and return it
pub fn join(base_url: &str, other_url: &str) -> String {
    let parts: Vec<_> = other_url.split(":").collect();
    match parts[0] {
        "file" | "http" | "https" | "lib" => {
            other_url.to_string()
        }

        _ => {
            let mut full_path_parts: Vec<&str> = base_url.split("/").collect();
            let last_part_index = full_path_parts.len() - 1;
            full_path_parts[last_part_index] = other_url;
            let full = full_path_parts.join("/");
            full
        }
    }
}

#[cfg(test)]
mod test {
    mod absolute_other_url {
        #[test]
        fn http_and_file_url() {
            let file_url = "file:///my/folder/file";
            assert_eq!(file_url, super::super::join("http://ibm.com", file_url));
        }

        #[test]
        fn file_and_file_url() {
            let file_url = "file:///my/folder/file";
            assert_eq!(file_url, super::super::join("file:///another/file", file_url));
        }

        #[test]
        fn http_and_http_url() {
            let http_url = "http://redhat.com";
            assert_eq!(http_url, super::super::join("http://ibm.com", http_url));
        }

        #[test]
        fn http_and_https_url() {
            let https_url = "https://redhat.com";
            assert_eq!(https_url, super::super::join("http://ibm.com", https_url));
        }

        #[test]
        fn http_and_lib_url() {
            let lib_url = "lib://mylib/myfunction";
            assert_eq!(lib_url, super::super::join("http://ibm.com", lib_url));
        }
    }

    mod relative_other_url {
        #[test]
        fn http_and_absolute_path_url() {
            let path = "module/file";
            assert_eq!(super::super::join("http://ibm.com/folder/", path), "http://ibm.com/folder/module/file");
        }
    }
}