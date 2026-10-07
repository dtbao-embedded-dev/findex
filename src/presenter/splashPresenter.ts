import { getAbout, type About } from "../model/about";
import { finishSplash } from "../model/splash";

/** What the splash shows. */
export type SplashState = {
  name: string;
  version: string;
  copyright: string;
};

export const initialSplash: SplashState = { name: "Findex", version: "", copyright: "" };

/**
 * Long enough to read the splash, short enough not to be a wait: a splash that
 * flashes for a frame reads as a glitch.
 */
export const MIN_SPLASH_MS = 900;

/** Asks the model for the about text; on failure the splash still shows the name. */
export async function loadSplash(
  ask: () => Promise<About> = getAbout,
): Promise<SplashState> {
  try {
    const about = await ask();
    return {
      name: about.name,
      version: about.version === "" ? "" : `Version ${about.version}`,
      copyright: about.copyright,
    };
  } catch {
    return initialSplash;
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Resolves once `work` is done and the splash has been up for `minMs`, whichever
 * is later. A failed `work` still resolves: main must be shown either way.
 */
export async function whenReady(
  work: Promise<unknown>,
  minMs: number = MIN_SPLASH_MS,
  wait: (ms: number) => Promise<void> = sleep,
): Promise<void> {
  await Promise.all([work.catch(() => undefined), wait(minMs)]);
}

/**
 * Shows main and closes the splash once `work` is done and the minimum time has
 * passed. A failure to reach Rust is ignored: its own timer shows main anyway.
 */
export async function revealMain(
  work: Promise<unknown>,
  finish: () => Promise<void> = finishSplash,
  minMs: number = MIN_SPLASH_MS,
  wait?: (ms: number) => Promise<void>,
): Promise<void> {
  await whenReady(work, minMs, wait);
  await finish().catch(() => undefined);
}
