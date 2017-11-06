# Flow UI

This is a tool for visual programming written in Electron. At the moment it's just a skeleton Electron app while I play with it and learn Electron.

## Pre-requisites

NOTE: Due to this issue (https://github.com/electron-userland/electron-forge/issues/249, https://github.com/electron-userland/electron-forge/issues/277) (at the time of writing) packaging doesn't work correctly with npm@5 and you should downgrade to nmp@3 with:
```
npm install -g npm@3
```

You need [Git](https://git-scm.com) and [Node.js](https://nodejs.org/en/download/) (which comes with [npm](http://npmjs.com)) installed.

The project uses the electron-forge packaging tool which you can install with:
```
npm install -g electron-forge
```

See https://github.com/electron-userland/electron-forge for more details on how to use electron-forge.

## To Run

With pre-requisites installed, from your command line:

```bash
# Clone this repository
git clone https://github.com/andrewdavidmackenzie/flowui.git
# Go into the repository
cd flowui
# Run the app
electron-forge start
```

Note: If you're using Linux Bash for Windows, [see this guide](https://www.howtogeek.com/261575/how-to-run-graphical-linux-desktop-applications-from-windows-10s-bash-shell/) or use `node` from the command prompt.

## Packaging

You can package easily for the platform you are currently running on with:

```
electron-forge make
```

Which will leave generated artifacts in ./out/make

## Travis Locally

If you have travis-CI problems, and (like me) get tired of pushing changes to try and figure it out, you can run a travis-node-js Docker Image locally, log in to it and try and figure it out, thus:

- Download and install the Docker Engine.
- Select an image from Quay.io. If you're not using a language-specific image pick travis-ruby. Open a terminal and start an interactive Docker session using the image URL:
- docker run -it quay.io/travisci/travis-ruby /bin/bash
- Switch to the travis user:
- su - travis
- Clone your git repository into the current folder (/home/travis) of the image.
- Go into the 'flowui' directory
- Manually install any dependencies.
- Manually run your Travis CI build command.

## License

MIT
