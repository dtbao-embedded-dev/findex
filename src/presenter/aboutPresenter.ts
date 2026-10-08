import { getAbout, type About } from "../model/about";

/** What the title bar shows beside the icon, and the status bar's copyright line. */
export type TitleState = {
  name: string;
  /** For example `v0.1.0`; empty until known. */
  version: string;
  /** For example `© 2026 dtbao`, from `bundle.copyright`; empty until known. */
  copyright: string;
};

export const initialTitle: TitleState = { name: "Findex", version: "", copyright: "" };

/** Asks the model for the name, version and copyright; on failure the title keeps the name. */
export async function loadTitle(ask: () => Promise<About> = getAbout): Promise<TitleState> {
  try {
    const about = await ask();
    return {
      name: about.name,
      version: about.version === "" ? "" : `v${about.version}`,
      copyright: about.copyright,
    };
  } catch {
    return initialTitle;
  }
}
