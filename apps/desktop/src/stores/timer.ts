// =============================================================================
// Manual timer store — the running manual session, if any.
// =============================================================================
// Shared between App.svelte (sidebar pill, restored at startup) and
// Timeline.svelte (start/stop controls), so the timer stays visible no
// matter which page the user is on.
// =============================================================================

import { writable } from "svelte/store";
import type { Session } from "$lib/types";

export const manualTimer = writable<Session | null>(null);
