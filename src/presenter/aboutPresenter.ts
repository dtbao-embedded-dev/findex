import { getAbout, type About } from "../model/about";

/** What the title bar shows beside the icon. */
export type TitleState = {
  name: string;
  /** For example `v0.1.0`; empty until known. */
  version: string;
};

export const initialTitle: TitleState = { name: "Findex", version: "" };

/** Asks the model for the name and version; on failure the title keeps the name. */
export async function loadTitle(ask: () => Promise<About> = getAbout): Promise<TitleState> {
  try {
    const about = await ask();
    return { name: about.name, version: about.version === "" ? "" : `v${about.version}` };
  } catch {
    return initialTitle;
  }
}
