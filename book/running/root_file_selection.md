### Specifying the flow's root file to load
#### Supported File Extensions and Formats
`flowc` supports TOML, JSON and YAML file formats. It assumes these file extensions: ".toml", "yaml"|"yml" or "json".

#### Flow root file argument
The flow "path" argument (if present) can be a local (relative or absolute) file name, a "file:///" Url or an
"http://" or "https://" Url.

When the argument is not present it assumes a local file is being loaded, from the Current Working Directory,
using the Local File algorithm described below.

When the "file:///" Url scheme is used it assumes a local file as described below.

When "http://" or "https://" schemes are used, it will use the Url loading algorithm described below.

#### Local File
`flowc` tries to load a flow from it's root file using one of these three methods:
* If an existing directory path is specified, it looks for the default root flow file name ("root.{}") in that 
  directory, for each of the supported extensions. The first matching filename.extension is loaded.
  * E.g. `flowc` will load `./root.toml` if it exists
  * E.g. `flowc dirname` will load `./dirname/root.toml` if the file exists
  * E.g. `flowc /dirname` will load `/dirname/root.toml` if the file exists
* If a path to an existing file is passed, it uses that as the filename of the flow root file.
  * E.g. `flowc path/to/root.toml` will load `root.toml` from the `./path/to/` directory
  * E.g. `flowc path/to/root.yaml` will load `root.yaml` from the `./path/to/` directory, even if `root.json` 
  and `root.toml` also exist 
* If a path to an non-existent file or directory is passed, it will look for matching files with supported extensions
  * E.g. `flowc root` will load `./root.toml` if it exists in the Current Working Directory
  * E.g. `flowc root` will load `./root.json` if `root.toml` doesn't exist but `root.json` does
  * E.g. `flowc path/to/root` will load `path/to/root.toml` if it exists
  * E.g. `flowc path/to/root` will load `root.yaml` from the `./path/to/` directory, if it exists and `root.toml`
  does not
* If a path to an existing directory is specified, it looks for a file named ("dirname.{}") in that
  directory (where dirname is the name of the directory), for each of the supported extensions.

#### Urls and loading from the web
The flow root file (http resource) will attempt to be loaded from the Url thus:
* The Url supplied, as-is
* The Url supplied, appending each of the supported extensions (see above)
* The Url supplied, appending "/root.{extension}" for each of the supported extensions
* The Url supplied, appending "/" and the last path segment, for each of the supported extensions

#### Why the dirname option?
The dirname option above in the file and url algorithms is used to be able to name a flow (or library or other
file) after the directory it is in, and have it found specifying a shorter filename or url. Thus `path/dirname`
will find a file called `path/dirname/dirname.toml`.