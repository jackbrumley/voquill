import { tokens } from './design-tokens.ts';

const speakerColors = ['#43b581', '#faa61a', '#7289da', '#f04747', '#b9bbbe'];

const personLabelPattern = /^Person\s+(\d+)$/i;

function hashLabel(label: string): number {
  let hash = 0;
  for (let i = 0; i < label.length; i += 1) {
    hash = (hash * 31 + label.charCodeAt(i)) | 0;
  }
  return Math.abs(hash);
}

export function getSpeakerColor(speaker: string | null): string {
  if (!speaker) return tokens.colors.textPrimary;

  const match = personLabelPattern.exec(speaker);
  if (match) {
    const index = Number(match[1]) - 1;
    return speakerColors[index % speakerColors.length];
  }

  return speakerColors[hashLabel(speaker) % speakerColors.length];
}