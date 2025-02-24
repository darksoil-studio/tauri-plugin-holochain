import { LitElement, html } from "lit";
import { state, customElement, property } from "lit/decorators.js";
import {
  InstalledCell,
  ActionHash,
  Record,
  AgentPubKey,
  EntryHash,
  AppClient,
  DnaHash,
} from "@holochain/client";
import { consume } from "@lit-labs/context";
import "@material/mwc-button";
import "@material/mwc-snackbar";
import { Snackbar } from "@material/mwc-snackbar";
import "@material/mwc-textfield";
import "@material/mwc-textarea";

import { clientContext } from "../../contexts.js";
import { Post } from "./types.js";

@customElement("create-post")
export class CreatePost extends LitElement {
  @consume({ context: clientContext, subscribe: true })
  client!: AppClient;

  @state()
  _title: string = "";

  @state()
  _content: string = "";

  firstUpdated() {}

  isPostValid() {
    return true && this._title !== "" && this._content !== "";
  }

  async createPost() {
    const post: Post = {
      title: this._title,
      content: this._content,
    };

    try {
      console.error("WAMR_LOG: UI before create post", Date.now());
      const record: Record = await this.client.callZome({
        cap_secret: null,
        role_name: "forum",
        zome_name: "posts",
        fn_name: "create_post",
        payload: post,
      });
      console.error("WAMR_LOG: UI after create post", Date.now());

      this.dispatchEvent(
        new CustomEvent("post-created", {
          composed: true,
          bubbles: true,
          detail: {
            postHash: record.signed_action.hashed.hash,
          },
        }),
      );
    } catch (e: any) {
      const errorSnackbar = this.shadowRoot?.getElementById(
        "create-error",
      ) as Snackbar;
      errorSnackbar.labelText = `Error creating the post: ${e}`;
      errorSnackbar.show();
    }
  }

  render() {
    return html` <mwc-snackbar id="create-error" leading> </mwc-snackbar>

      <div style="display: flex; flex-direction: column">
        <span style="font-size: 18px">Create Post</span>

        <div style="margin-bottom: 16px">
          <mwc-textfield
            outlined
            label="Title"
            .value=${this._title}
            @input=${(e: CustomEvent) => {
              this._title = (e.target as any).value;
            }}
            required
          ></mwc-textfield>
        </div>

        <div style="margin-bottom: 16px">
          <mwc-textarea
            outlined
            label="Content"
            .value=${this._content}
            @input=${(e: CustomEvent) => {
              this._content = (e.target as any).value;
            }}
            required
          ></mwc-textarea>
        </div>

        <mwc-button
          raised
          label="Create Post"
          .disabled=${!this.isPostValid()}
          @click=${() => this.createPost()}
        ></mwc-button>
      </div>`;
  }
}
