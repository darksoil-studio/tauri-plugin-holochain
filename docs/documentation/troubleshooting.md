# Troubleshooting

## Tauri Issues

### `tauri::generate_context!() panics`

If you get this error:

```
   Compiling holochain_cascade v0.3.1
error: proc macro panicked
  --> src-tauri/src/lib.rs:47:14
   |
47 |         .run(tauri::generate_context!())
   |              ^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: message: failed to read icon /tmp/forum-scaffold-tauri-happ/src-tauri/icons/32x32.png: No such file or directory (os error 2)

error: could not compile `forum-scaffold-tauri-happ` (lib) due to previous error
    Error failed to build app: failed to build app
```

It means that you haven't generated the Tauri icons for your app yet. Follow these steps:

1. Create or download the icon for your app. **It must be a square PNG image.

2. In the root of your project, run this command:

::: code-group
```bash [npm]
npm run tauri icon <PATH_TO_YOUR_ICON_IN_PNG_FORMAT>
```

```bash [yarn]
yarn tauri icon <PATH_TO_YOUR_ICON_IN_PNG_FORMAT>
```

```bash [pnpm]
pnpm tauri icon <PATH_TO_YOUR_ICON_IN_PNG_FORMAT>
```
:::

---

### `Error Have you installed the Android SDK?` 

If you get this error:

```
Error Have you installed the Android SDK? The `ANDROID_HOME` environment variable isn't set, and is required: environment variable not found: environment variable not found
npm run tauri android dev exited with code 1
```

It means you are trying to run your app in an Android device, but you are not inside your `androidDev` devShell, which is the one that includes the Android development environment and tooling.

Enter the `androidDev` devShell with:

```bash
nix develop .#androidDev
```

And try again.

---

### `Error Android Studio project directory src-tauri/gen/android doesn't exist.`

If you get this error: 

```
 Error Android Studio project directory src-tauri/gen/android doesn't exist. Please run `tauri android init` and try again.
```

It means you haven't initialized your project for Android developm̀ent yet. You can do so by running:

```bash
npm run tauri android init -- --skip-targets-install
```

And then try running your command again.

## NixOS Issues

### Connect to devices

In NixOS, the command `adb devices` needs root permissions, so you need to run it like this:

```bash
sudo adb devices
```

### Firewall

In NixOS, the firewall is enabled by default, which means that you can't directly run `npm run network:android` and have your Android device connect to the vite server running on your computer.

Disable the firewall to enable your Android device to connect to it:

1. Identify firewall rule number: 

```bash
sudo iptables -L INPUT --line-numbers
```

Choose the firewall rule number that's blocking the connection (there is usually just one rule, if so pick that one).

2. Remove firewall rule:

```bash
sudo iptables -D INPUT <RULE_NUM>
```
