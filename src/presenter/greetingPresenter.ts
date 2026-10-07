import { greet } from "../model/greeting";

/** What the view shows after a submit. */
export type GreetingState = {
  message: string;
  error: string | null;
};

export const initialGreeting: GreetingState = { message: "", error: null };

/**
 * Validates the name, asks the model, and turns the answer or the refusal into
 * view state. `ask` is the model call, passed in so tests need no Tauri runtime.
 */
export async function submitName(
  name: string,
  ask: (name: string) => Promise<string> = greet,
): Promise<GreetingState> {
  if (name.trim() === "") {
    return { message: "", error: "Enter a name first." };
  }
  try {
    return { message: await ask(name), error: null };
  } catch (err) {
    // A Rust AppError arrives as its message string.
    return { message: "", error: String(err) };
  }
}
