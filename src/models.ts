import type { EngineInfo, ModelInfo } from './types.ts';

/// Lookups over the model catalog and engine list the backend reports. A
/// model is identified by engine and size together: the same size can exist
/// under several engines.

export function findModel(models: ModelInfo[], engine: string, size: string): ModelInfo | undefined {
  return models.find((model) => model.engine === engine && model.size === size);
}

export function isModelDownloaded(models: ModelInfo[], engine: string, size: string): boolean {
  return findModel(models, engine, size)?.downloaded ?? false;
}

export function engineUsesGpu(engines: EngineInfo[], engine: string): boolean {
  return engines.find((info) => info.name === engine)?.uses_gpu ?? false;
}

export function enginesInCategory(engines: EngineInfo[], category: EngineInfo['category']): EngineInfo[] {
  return engines.filter((engine) => engine.category === category);
}

export function engineUsesCustomWords(engines: EngineInfo[], engine: string): boolean {
  return engines.find((info) => info.name === engine)?.uses_custom_words ?? false;
}
