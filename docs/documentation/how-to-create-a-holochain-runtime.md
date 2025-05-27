# How to create a holochain runtime

A **holochain runtime** is an end-user application that is able to install and open holochain apps and web-apps. Examples of existing runtimes include the [launcher](https://github.com/holochain/launcher) and [moss](https://github.com/lightningrodlabs/we).

## Scaffolding

> [!WARNING]
> p2p Shipyard relies heavily on [`nix`](https://nixos.org/) to achieve reproducible environments. Unfortunately, `nix` does not support Windows. So if you have a Windows OS, you will need to [install Windows Subsystem Linux](https://learn.microsoft.com/en-us/windows/wsl/install) and run all the steps in this guide inside of its environment.

0. If you haven't already, [install the nix package manager](https://nixos.org/download/#nix-install-linux) with: 

::: code-group
```bash [Linux]
sh <(curl -L https://nixos.org/nix/install) --daemon
```
```bash [MacOs]
sh <(curl -L https://nixos.org/nix/install)
```
:::

And follow along its instructions and prompts.

1. Add the appropriate nix caches to your environment:

```bash
nix run nixpkgs#cachix use holochain-ci
nix run nixpkgs#cachix use darksoil-studio
```

2. In the folder where you want to create your new holochain runtime, run this command:

```bash
nix run github:darksoil-studio/tauri-plugin-holochain#scaffold-holochain-runtime
```

And follow along its instructions and prompts.

3. Take a look into the repository structure that the scaffold command created, specially:

- `flake.nix`: with the `tauri-plugin-holochain` input and its `devShells`.
- `package.json`: added set up scripts and some `devDependencies`.
- `src-tauri`: here is where the code for the backend of the tauri app lives. For now it's a simple Tauri app that includes the `tauri-plugin-holochain`.
- `index.html`: main `index.html` file that will be displayed when the app is opened.
- `src`: this is where the code for the UI lives.
  - The scaffolded template contains a very bare bones vanilla JS app. Look in `src/main.ts` to see how the frontend for your runtime can connect to the `AdminWebsocket`.

That's it! We now have a working skeleton for a holochain runtime. 

> [!WARNING]
> The scaffolded tauri app is missing icons, which are needed for the app to compile. Run through the rest of this guide and the following one ("Getting to know Tauri") to be able to generate the icons for your Tauri app.

## Development Environment

The `scaffold-holochain-apps` has added the necessary nix `devShells` to your `flake.nix` file so that you don't need to follow install anything to get the tauri or Android development environment.

> [!NOTE]
> Nix `devShells` are packages that describe development environments, with all their dependencies and environment variables, so that the developer does not need to configure manually their setup.

As usual, run this command to enter the development environment:

```bash
nix develop
```

This can take a while while it builds all the required dependencies.

Next, run these commands:

::: code-group
```bash [npm]
npm install
npm run tauri dev
```

```bash [yarn]
yarn install
yarn tauri dev
```

```bash [pnpm]
pnpm install
pnpm tauri dev
```
:::

This will start an instance of the app.

Under the hood, these commands are running tauri CLI commands. As such, we should get to know Tauri a bit better to be comfortable while developing the app. Go to [Getting to know Tauri](./getting-to-know-tauri.md) to familiarize yourself with it.

