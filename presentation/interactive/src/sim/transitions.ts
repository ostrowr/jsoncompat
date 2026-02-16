import type { RuntimeStory, StoryState, StoryTransition } from "../model/types";

export const getStateById = (story: RuntimeStory, stateId: string): StoryState => {
  const state = story.states.get(stateId);
  if (state === undefined) {
    throw new Error(`unknown state '${stateId}'`);
  }
  return state;
};

export const getStateIndex = (story: RuntimeStory, stateId: string): number => {
  return story.orderedStates.indexOf(stateId);
};

export const stateIdByIndex = (story: RuntimeStory, index: number): string => {
  const bounded = Math.max(0, Math.min(story.orderedStates.length - 1, index));
  return story.orderedStates[bounded] ?? story.orderedStates[0] ?? story.initialStateId;
};

export const getNextStateId = (story: RuntimeStory, currentStateId: string): string => {
  const idx = getStateIndex(story, currentStateId);
  if (idx < 0) {
    throw new Error(`unknown current state '${currentStateId}'`);
  }
  return stateIdByIndex(story, idx + 1);
};

export const getPreviousStateId = (story: RuntimeStory, currentStateId: string): string => {
  const idx = getStateIndex(story, currentStateId);
  if (idx < 0) {
    throw new Error(`unknown current state '${currentStateId}'`);
  }
  return stateIdByIndex(story, idx - 1);
};

export const getTransitionFrom = (story: RuntimeStory, fromStateId: string): StoryTransition | null => {
  return story.transitionsByFromState.get(fromStateId) ?? null;
};

export const getTransitionBetween = (
  story: RuntimeStory,
  fromStateId: string,
  toStateId: string,
): StoryTransition | null => {
  const direct = story.transitionsByFromState.get(fromStateId);
  if (direct !== undefined && direct.toStateId === toStateId) {
    return direct;
  }

  const reverse = story.transitionsByFromState.get(toStateId);
  if (reverse !== undefined && reverse.toStateId === fromStateId) {
    return reverse;
  }

  return null;
};
