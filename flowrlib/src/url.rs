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