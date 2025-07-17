import { LitElement, css, html } from "lit";
import { customElement, property, state } from "lit/decorators.js";
import { AppClient, AppWebsocket } from "@holochain/client";
import { provide } from "@lit-labs/context";
import "@material/mwc-circular-progress";

import { clientContext } from "./contexts.js";

import "./forum/posts/all-posts.js";
import "./forum/posts/create-post.js";

function getLauncherEnv() {
  return (window as any).__HC_LAUNCHER_ENV__ as any;
}

async function waitForAppSetup() {
  return new Promise((resolve, reject) => {
    const interval = setInterval(() => {
      const env = getLauncherEnv();
      if (
        env &&
        env.APP_INTERFACE_PORT != undefined &&
        env.APP_INTERFACE_TOKEN != undefined
      ) {
        resolve(undefined);
        clearInterval(interval);
      }
    }, 100);
    setTimeout(() => {
      reject(new Error("Timeout waiting for the app to be set up."));
      clearInterval(interval);
    }, 60_000);
  });
}

@customElement("holochain-app")
export class HolochainApp extends LitElement {
  @state() loading = true;

  @state() result: string | undefined;

  @provide({ context: clientContext })
  @property({ type: Object })
  client!: AppClient;

  error: any = undefined;

  async firstUpdated() {
    try {
      await waitForAppSetup();
      this.client = await AppWebsocket.connect();
    } catch (e) {
      this.error = e;
    }

    this.loading = false;
  }

  render() {
    if (this.loading)
      return html`
        <mwc-circular-progress indeterminate></mwc-circular-progress>
      `;

    if (this.error) return html`<span>ERROR: ${this.error}</span>`;

    return html`
      <main>
        <h1>Forum</h1>

        <div id="content">
          <h2>All Posts</h2>
          <all-posts id="all-posts" style="margin-bottom: 16px"></all-posts>
          <create-post></create-post>
        </div>
      </main>
    `;
  }

  static styles = css`
    :host {
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: flex-start;
      font-size: calc(10px + 2vmin);
      color: #1a2b42;
      max-width: 960px;
      margin: 0 auto;
      text-align: center;
      background-color: var(--lit-element-background-color);
    }

    main {
      flex-grow: 1;
    }

    .app-footer {
      font-size: calc(12px + 0.5vmin);
      align-items: center;
    }

    .app-footer a {
      margin-left: 5px;
    }
  `;
}
