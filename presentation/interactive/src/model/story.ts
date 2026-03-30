import defaultStoryJson from "../../story/default-story.json";
import type { RuntimeStory } from "./types";
import { materializeStory, parseStoryDefinition } from "./story-loader";

export const createDefaultRuntimeStory = (): RuntimeStory => {
  const parsed = parseStoryDefinition(defaultStoryJson);
  return materializeStory(parsed);
};
