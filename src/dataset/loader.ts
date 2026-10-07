import { parseDatasetJson, serializeDataset, type WoHoDataset } from "./schema";

export function loadDataset(input: string): WoHoDataset {
  return parseDatasetJson(input);
}

export function writeDataset(dataset: WoHoDataset): string {
  return serializeDataset(dataset);
}

export function datasetToJsonl(dataset: WoHoDataset): string {
  return dataset.examples.map((example) => JSON.stringify(example)).join("\n") + "\n";
}
