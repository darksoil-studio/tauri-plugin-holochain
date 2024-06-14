import type {
  CallZomeRequest,
  CallZomeRequestSigned,
  CallZomeRequestUnsigned,
} from "@holochain/client/lib/api/app/types.js";
import type { HostZomeCallSigner } from "@holochain/client/lib/environments/launcher.js";
import { encode } from "@msgpack/msgpack";
import { invoke } from "@tauri-apps/api/core";

import {
  attachConsole,
  trace,
  debug,
  info,
  warn,
  error,
} from "@tauri-apps/plugin-log";

const randomNonce = async () => randomByteArray(32);
const randomByteArray = async (length: number) => {
  return globalThis.crypto.getRandomValues(new Uint8Array(length));
};
const getNonceExpiration = () => (Date.now() + 5 * 60 * 1000) * 1000; // 5 mins from now in microseconds

setTimeout(() => {
  attachConsole().then(() => {
    window.onerror = (e) => console.error(e);
    console.trace = trace;
    console.log = debug;
    console.info = info;
    console.warn = warn;
    console.error = error;
  });
});

window["__HC_ZOME_CALL_SIGNER__"] = {
  signZomeCall(request) {
    return signZomeCallTauri(request);
  },
} as HostZomeCallSigner;

type TauriByteArray = number[]; // Tauri requires a number array instead of a Uint8Array

interface CallZomeRequestSignedTauri
  extends Omit<
    CallZomeRequestSigned,
    "cap_secret" | "cell_id" | "provenance" | "nonce"
  > {
  cell_id: [TauriByteArray, TauriByteArray];
  provenance: TauriByteArray;
  nonce: TauriByteArray;
  expires_at: number;
}

interface CallZomeRequestUnsignedTauri
  extends Omit<
    CallZomeRequestUnsigned,
    "cap_secret" | "cell_id" | "provenance" | "nonce"
  > {
  cell_id: [TauriByteArray, TauriByteArray];
  provenance: TauriByteArray;
  nonce: TauriByteArray;
  expires_at: number;
}

const signZomeCallTauri = async (request: CallZomeRequest) => {
  const zomeCallUnsigned: CallZomeRequestUnsignedTauri = {
    provenance: Array.from(request.provenance),
    cell_id: [Array.from(request.cell_id[0]), Array.from(request.cell_id[1])],
    zome_name: request.zome_name,
    fn_name: request.fn_name,
    payload: Array.from(encode(request.payload)),
    nonce: Array.from(await randomNonce()),
    expires_at: getNonceExpiration(),
  };

  const signedZomeCallTauri: CallZomeRequestSignedTauri = await invoke(
    "plugin:holochain|sign_zome_call",
    {
      zomeCallUnsigned,
    },
  );

  const signedZomeCall: CallZomeRequestSigned = {
    provenance: Uint8Array.from(signedZomeCallTauri.provenance),
    cap_secret: null,
    cell_id: [
      Uint8Array.from(signedZomeCallTauri.cell_id[0]),
      Uint8Array.from(signedZomeCallTauri.cell_id[1]),
    ],
    zome_name: signedZomeCallTauri.zome_name,
    fn_name: signedZomeCallTauri.fn_name,
    payload: Uint8Array.from(signedZomeCallTauri.payload),
    signature: Uint8Array.from(signedZomeCallTauri.signature),
    expires_at: signedZomeCallTauri.expires_at,
    nonce: Uint8Array.from(signedZomeCallTauri.nonce),
  };

  return signedZomeCall;
};
