import type {
  CallZomeRequest,
  CallZomeRequestSigned,
  CallZomeRequestUnsigned,
} from "@holochain/client/lib/api/app/types.js";
import type { HostZomeCallSigner } from "@holochain/client/lib/environments/launcher.js";
import { encode } from "@msgpack/msgpack";
import { invoke } from "@tauri-apps/api/core";

const randomNonce = async () => randomByteArray(32);
const randomByteArray = async (length: number) => {
  return globalThis.crypto.getRandomValues(new Uint8Array(length));
};
const getNonceExpiration = () => (Date.now() + 5 * 60 * 1000) * 1000; // 5 mins from now in microseconds

window["__HC_ZOME_CALL_SIGNER__"] = {
  signZomeCall(request) {
    return signZomeCallTauri(request);
  },
} as HostZomeCallSigner;

const signZomeCallTauri = async (request: CallZomeRequest) => {
  const zomeCallUnsigned: CallZomeRequestUnsigned = {
    ...request,
    payload: encode(request.payload),
    cap_secret: null,
    expires_at: getNonceExpiration(),
    nonce: await randomNonce(),
  };

  const signedZomeCall: CallZomeRequestSigned = await invoke(
    "plugin:holochain|sign_zome_call",
    {
      zomeCallUnsigned,
    },
  );

  return signedZomeCall;
};
