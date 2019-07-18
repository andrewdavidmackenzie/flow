const {app, shell, Menu, webFrame, BrowserWindow} = require('electron');
const helpUrl = 'https://raw.githubusercontent.com/andrewdavidmackenzie/flow/master/ide/help.md';
const path = require('path');
//const url = require('url');

// Keep a global reference of the main window object, if you don't, the window will
// be closed automatically when the JavaScript object is garbage collected.
let mainWindow;

function createUI() {
    mainWindow = new BrowserWindow({
        width: 1400, height: 900,
        nodeIntegration: true,
        icon: path.join(__dirname, 'assets/icons/png/128x128.png')
    });

    process.once("loaded", () => {
        // Allow window.fetch() to access app files
        webFrame.registerURLSchemeAsPrivileged("app", {
            secure: false,
            bypassCSP: true,
            allowServiceWorkers: true,
            supportFetchAPI: true,
            corsEnabled: true
        });
    });

    const session = mainWindow.webContents.session;

    // Mute warnings in development about CSP
    // process.env['ELECTRON_DISABLE_SECURITY_WARNINGS'] = true;

    // Force a CSP
    session.webRequest.onHeadersReceived((details, callback) => {
        callback({ responseHeaders: Object.assign({
                "Content-Security-Policy": [ "default-src 'self'" ]
            }, details.responseHeaders)});
    });

    mainWindow.webContents.loadFile("index.html");

    // TODO this only when in debug mode or a debug build
    mainWindow.webContents.openDevTools({mode: 'bottom'});

    mainWindow.on('closed', () => {
        // Dereference the window object, usually you would store windows
        // in an array if your app supports multi windows, this is the time
        // when you should delete the corresponding element.
        mainWindow = null
    });
}

/*********************** MENU TEMPLATES ********************/
let menuTemplate = [
    {
    label: 'View',
    submenu: [{
        label: 'Toggle Full Screen',
        accelerator: (function () {
            if (process.platform === 'darwin') {
                return 'Ctrl+Command+F'
            } else {
                return 'F11'
            }
        })(),
        click: function (item, focusedWindow) {
            if (focusedWindow) {
                focusedWindow.setFullScreen(!focusedWindow.isFullScreen())
            }
        }
    }, {
        label: 'Toggle Developer Tools',
        accelerator: (function () {
            if (process.platform === 'darwin') {
                return 'Alt+Command+I'
            } else {
                return 'Ctrl+Shift+I'
            }
        })(),
        click: function (item, focusedWindow) {
            if (focusedWindow) {
                focusedWindow.toggleDevTools()
            }
        }
    }]
    },
    {
    label: 'Window',
    role: 'window',
    submenu: [{
        label: 'Minimize',
        accelerator: 'CmdOrCtrl+M',
        role: 'minimize'
    }, {
        label: 'Close',
        accelerator: 'CmdOrCtrl+W',
        role: 'close'
    }, {
        type: 'separator'
    }, {
        label: 'Reopen Window',
        accelerator: 'CmdOrCtrl+Shift+T',
        enabled: false,
        key: 'reopenMenuItem',
        click: function () {
            app.emit('activate')
        }
    }]
    },
    {
    label: 'Help',
    role: 'help',
    submenu: [{
        label: 'Learn More',
        click: function () {
            shell.openExternal(helpUrl)
        }
    }]
}];

function addUpdateMenuItems(items, position) {
    if (process.mas) return;

    const version = app.getVersion();
    let updateItems = [{
        label: `Version ${version}`,
        enabled: false
    }, {
        label: 'Checking for Update',
        enabled: false,
        key: 'checkingForUpdate'
    }, {
        label: 'Check for Update',
        visible: false,
        key: 'checkForUpdate',
        click: function () {
            require('electron').autoUpdater.checkForUpdates()
        }
    }, {
        label: 'Restart and Install Update',
        enabled: true,
        visible: false,
        key: 'restartToUpdate',
        click: function () {
            require('electron').autoUpdater.quitAndInstall()
        }
    }];

    items.splice.apply(items, [position, 0].concat(updateItems))
}

function findReopenMenuItem() {
    const menu = Menu.getApplicationMenu();
    if (!menu) return;

    let reopenMenuItem = null;
    menu.items.forEach(function (item) {
        if (item.submenu) {
            item.submenu.items.forEach(function (item) {
                if (item.key === 'reopenMenuItem') {
                    reopenMenuItem = item
                }
            })
        }
    });
    return reopenMenuItem;
}

function macMenus() {
    const name = app.getName();

    // App name Menu for Mac
    menuTemplate.unshift({
        label: name,
        submenu: [{
            label: `About ${name}`,
            role: 'about'
        },{
            type: 'separator'
        }, {
            label: `Hide ${name}`,
            accelerator: 'Command+H',
            role: 'hide'
        }, {
            label: 'Hide Others',
            accelerator: 'Command+Alt+H',
            role: 'hideothers'
        }, {
            label: 'Show All',
            role: 'unhide'
        }, {
            type: 'separator'
        }, {
            label: 'Quit',
            accelerator: 'Command+Q',
            click: function () {
                app.quit()
            }
        }]
    });

    // Window menu
    menuTemplate[2].submenu.push({
        type: 'separator'
    }, {
        label: 'Bring All to Front',
        role: 'front'
    });

    addUpdateMenuItems(menuTemplate[0].submenu, 1);
}

/*************************** CREATE MENUS *************************/
if (process.platform === 'darwin')
    macMenus();

if (process.platform === 'win32') {
    const helpMenu = menuTemplate[menuTemplate.length - 1].submenu;
    addUpdateMenuItems(helpMenu, 0)
}

/************************ EVENT HANDLERS *************************/
// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.on('ready', createUI);

// Quit when all windows are closed.
app.on('window-all-closed', function () {
    let reopenMenuItem = findReopenMenuItem();
    if (reopenMenuItem) reopenMenuItem.enabled = true;

    // On OS X it is common for applications and their menu bar
    // to stay active until the user quits explicitly with Cmd + Q
    if (process.platform !== 'darwin') {
        app.quit()
    }
});

app.on('ready', function () {
    const menu = Menu.buildFromTemplate(menuTemplate);
    Menu.setApplicationMenu(menu)
});

app.on('browser-window-created', function () {
    let reopenMenuItem = findReopenMenuItem();
    if (reopenMenuItem) reopenMenuItem.enabled = false
});

app.on('activate', () => {
    // On macOS it's common to re-create a window in the app when the
    // dock icon is clicked and there are no other windows open.
    if (mainWindow === null) {
        createUI()
    }
});

// In this file you can include the rest of your app's specific main process
// code. You can also put them in separate files and require them here.