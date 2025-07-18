function createDialog() {
    if (!!localStorage.getItem("__dialogShown"))
        return;
    localStorage.setItem("__dialogShown", Date.now().toString());
    if (document.getElementById('shipyard-dialog'))
        return;
    const styles = document.createElement("link");
    styles.rel = "stylesheet";
    styles.href =
        "https://early.webawesome.com/webawesome@3.0.0-beta.2/dist/styles/themes/default.css";
    const dialog = document.createElement("script");
    dialog.type = "module";
    dialog.src =
        "https://early.webawesome.com/webawesome@3.0.0-beta.2/dist/components/dialog/dialog.js";
    const button = document.createElement("script");
    button.type = "module";
    button.src =
        "https://early.webawesome.com/webawesome@3.0.0-beta.2/dist/components/button/button.js";
    const div = document.createElement("div");
    div.className = "wa-theme-default wa-palette-default wa-brand-blue";
    div.innerHTML = `<wa-dialog id="shipyard-dialog" label="Created with the p2p Shipyard" open>
  <div style="display: flex; flex-direction: column; gap: 16px">
    <img src="https://substackcdn.com/image/fetch/f_auto,q_auto:good,fl_progressive:steep/https%3A%2F%2Fsubstack-post-media.s3.amazonaws.com%2Fpublic%2Fimages%2Ff39196ef-fdf7-470a-8092-b0dc07d210d6_1600x914.jpeg" style="width: 300px; align-self: center; border-radius: 8px">
    <span>
    	This app was created with the <a href="https://darksoil.studio/p2p-shipyard">p2p Shipyard</a>.
    </span>
    <span>
    	To remove this dialog, purchase a license for the p2p Shipyard at:
    </span>

  	<a href="https://darksoil.studio/p2p-shipyard/pricing">https://darksoil.studio/p2p-shipyard/pricing</a>
	</div>
  <wa-button slot="footer" variant="brand" data-dialog="close">Close</wa-button>
</wa-dialog>`;
    document.body.appendChild(styles);
    document.body.appendChild(dialog);
    document.body.appendChild(button);
    setTimeout(() => {
        document.body.appendChild(div);
    }, 100);
}
createDialog();
