import {
  CallZomeRequest,
  CallZomeRequestSigned,
  getNonceExpiration,
  randomNonce,
} from "@holochain/client";
import { encode } from "@msgpack/msgpack";
import { core } from "@tauri-apps/api";
import {
  attachConsole,
  trace,
  debug,
  info,
  warn,
  error,
} from "@tauri-apps/plugin-log";

attachConsole().then(() => {
  window.onerror = (e) => console.error(e);
  console.trace = trace;
  console.log = debug;
  console.info = info;
  console.warn = warn;
  console.error = error;
});

// Here we are trying to cover all platforms in different ways
// Windows doesn't support requests of type happ://APPID
// MacOs doesn't support requests of type http://APPID.localhost:4040
export enum IframeProtocol {
  Assets,
  LocalhostSubdomain,
  LocaltestMe,
}

async function fetchPing(origin: string) {
  const iframe = document.createElement("iframe");
  iframe.src = origin;
  iframe.style.display = "none";
  document.body.appendChild(iframe);

  return new Promise((resolve, reject) => {
    let resolved = false;

    const listener = (message: any) => {
      if (message.source === iframe.contentWindow) {
        resolved = true;
        document.body.removeChild(iframe);
        window.removeEventListener("message", listener);
        resolve(null);
      }
    };
    setTimeout(() => {
      if (resolved) return;
      document.body.removeChild(iframe);
      window.removeEventListener("message", listener);
      reject(new Error("Protocol failed to start."));
    }, 1000);

    window.addEventListener("message", listener);
  });
}

export function isWindows(): boolean {
  return navigator.appVersion.includes("Win");
}

async function getIframeProtocol(httpServerPort: number) {
  if (isWindows()) {
    try {
      await fetchPing(`http://ping.localhost:${httpServerPort}`);
      return IframeProtocol.LocalhostSubdomain;
    } catch (e) {
      return IframeProtocol.LocaltestMe;
    }
  } else {
    try {
      await fetchPing("happ://ping");
      return IframeProtocol.Assets;
    } catch (e) {
      try {
        await fetchPing(`http://ping.localhost:${httpServerPort}`);
        return IframeProtocol.LocalhostSubdomain;
      } catch (e) {
        return IframeProtocol.LocaltestMe;
      }
    }
  }
}

export function appOrigin(
  iframeProtocol: IframeProtocol,
  appId: string,
  httpServerPort: number,
): string {
  if (iframeProtocol === IframeProtocol.Assets) {
    return `happ://${appId}`;
  } else if (iframeProtocol === IframeProtocol.LocalhostSubdomain) {
    return `http://${appId}.localhost:${httpServerPort}`;
  } else {
    return `http://${appId}.localtest.me:${httpServerPort}`;
  }
}

function getAppIdFromOrigin(
  iframeProtocol: IframeProtocol,
  origin: string,
): string {
  if (iframeProtocol === IframeProtocol.Assets) {
    return origin.split("://")[1].split("?")[0].split("/")[0];
  } else {
    return origin.split("://")[1].split("?")[0].split(".")[0];
  }
}

export interface RuntimeInfo {
  http_server_port: number;
  app_port: number;
  admin_port: number;
}

const appId = (window as any).__APP_ID__;

core
  .invoke<RuntimeInfo>("plugin:holochain|get_runtime_info", {})
  .then((runtimeInfo: RuntimeInfo) => {
    getIframeProtocol(runtimeInfo.http_server_port).then((protocol) => {
      window.addEventListener("message", async (message) => {
        const appId = getAppIdFromOrigin(protocol, message.origin);

        const response = await handleRequest(runtimeInfo, appId, message.data);
        message.ports[0].postMessage({ type: "success", result: response });
      });
      buildFrame(runtimeInfo, protocol, appId);
    });
  });

export type Request =
  | {
      type: "sign-zome-call";
      zomeCall: CallZomeRequest;
    }
  | {
      type: "get-app-runtime-info";
    }
  | {
      type: "get-locales";
    };

async function handleRequest(
  runtimeInfo: RuntimeInfo,
  appId: string,
  request: Request,
) {
  switch (request.type) {
    case "get-app-runtime-info":
      return {
        appId,
        runtimeInfo,
      };
    case "sign-zome-call":
      return signZomeCallTauri(request.zomeCall);
    case "get-locales":
      return core.invoke("plugin:holochain|get_locales", {});
  }
}

function buildFrame(
  runtimeInfo: RuntimeInfo,
  iframeProtocol: IframeProtocol,
  appId: string,
) {
  const iframe = document.createElement("iframe");
  const origin = appOrigin(iframeProtocol, appId, runtimeInfo.http_server_port);

  iframe.src = `${origin}${window.location.search}`;
  iframe.frameBorder = "0";
  document.body.appendChild(iframe);
}

export const signZomeCallTauri = async (request: CallZomeRequest) => {
  const zomeCallUnsigned: CallZomeRequest = {
    provenance: request.provenance,
    cell_id: request.cell_id,
    zome_name: request.zome_name,
    fn_name: request.fn_name,
    payload: encode(request.payload),
    nonce: await randomNonce(),
    expires_at: getNonceExpiration(),
  };

  const signedZomeCall: CallZomeRequestSigned = await core.invoke(
    "plugin:holochain|sign_zome_call",
    {
      zomeCallUnsigned,
    },
  );

  return signedZomeCall;
};
