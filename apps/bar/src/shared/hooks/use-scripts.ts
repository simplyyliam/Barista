import { commands, events, type ScriptInfo, type TrustRequest } from "@/shared/bindings"
import { useSnapshot } from "./use-snapshot"

/** Every script in the scripts folder with its state. `null` until Rust answers; `scriptsChanged` keeps it fresh. */
export const useScripts = () =>
  useSnapshot<ScriptInfo[]>(commands.getScripts, (h) => events.scriptsChanged.listen((e) => h(e.payload)))

/** The scripts waiting for the user's yes, oldest file name first. An empty list means nothing is waiting. */
export const useScriptTrustRequests = () =>
  useSnapshot<TrustRequest[]>(commands.getScriptTrustRequests, (h) =>
    events.scriptTrustRequested.listen((e) => h(e.payload)),
  )
