import { LitElement, html } from "lit";
import { state, customElement, property } from "lit/decorators.js";
import {
  AppClient,
  AgentPubKey,
  Link,
  EntryHash,
  ActionHash,
  Record,
  NewEntryAction,
} from "@holochain/client";
import { consume } from "@lit-labs/context";
import { Task } from "@lit-labs/task";
import "@material/mwc-circular-progress";
import "./post-detail.js";

import { clientContext } from "../../contexts";
import { PostsSignal } from "./types";

@customElement("all-posts")
export class AllPosts extends LitElement {
  @consume({ context: clientContext, subscribe: true })
  client!: AppClient;

  @state()
  loading = true;

  @state()
  hashes: Array<ActionHash> = [];

  async firstUpdated() {
    this.client.on("signal", (signal) => {
      if (signal.type !== "app" || signal.value.zome_name !== "posts") return;
      const payload = signal.value.payload as PostsSignal;
      if (payload.type !== "EntryCreated") return;
      this.hashes = [payload.action.hashed.hash, ...this.hashes];
    });
    const links: Link[] = await this.client.callZome({
      role_name: "forum",
      zome_name: "posts",
      fn_name: "get_all_posts",
      payload: null,
    });
    this.hashes = links.map((l) => l.target);
    this.loading = false;
    setInterval(async () => {
      const links: Link[] = await this.client.callZome({
        role_name: "forum",
        zome_name: "posts",
        fn_name: "get_all_posts",
        payload: null,
      });
      this.hashes = links.map((l) => l.target);
    }, 1000);
  }

  renderList(hashes: Array<ActionHash>) {
    if (hashes.length === 0) return html`<span>No posts found.</span>`;

    return html`
      <div style="display: flex; flex-direction: column; gap: 16px">
        ${hashes.map(
          (hash) => html` <post-detail .postHash=${hash}></post-detail> `,
        )}
      </div>
    `;
  }

  render() {
    if (this.loading)
      return html`
        <div
          style="display: flex; flex: 1; align-items: center; justify-content: center"
        >
          <mwc-circular-progress indeterminate></mwc-circular-progress>
        </div>
      `;
    return this.renderList(this.hashes);
  }
}
