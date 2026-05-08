import type { IndexProgressEvent } from './indexer';

export interface LoadingProgress {
  percent: number;
  percentLabel: string;
  detail: string;
  currentFile: string | null;
}

const STATUS_PERCENT_PATTERN = /^(\d+(?:\.\d+)?)%\s*·\s*(.+)$/;

function clampPercent(value: number) {
  return Math.min(Math.max(value, 0), 100);
}

function fileName(path: string | null | undefined) {
  return path?.split('/').pop() || null;
}

function fromStatus(projectStatus: string | null): LoadingProgress | null {
  if (!projectStatus) {
    return null;
  }

  const match = projectStatus.match(STATUS_PERCENT_PATTERN);
  if (!match) {
    return {
      percent: 5,
      percentLabel: '5%',
      detail: projectStatus,
      currentFile: null,
    };
  }

  const percent = clampPercent(Number(match[1]));
  return {
    percent,
    percentLabel: `${Math.round(percent)}%`,
    detail: match[2],
    currentFile: null,
  };
}

function fromIndexProgress(indexProgress: IndexProgressEvent): LoadingProgress {
  if (indexProgress.filesTotal === 0) {
    return {
      percent: 20,
      percentLabel: '20%',
      detail: 'Discovering indexable files...',
      currentFile: null,
    };
  }

  const ratio = Math.min(indexProgress.filesSeen / indexProgress.filesTotal, 1);
  const percent = clampPercent(20 + ratio * 34);
  return {
    percent,
    percentLabel: `${Math.round(percent)}%`,
    detail: `Indexed ${indexProgress.filesSeen}/${indexProgress.filesTotal} files`,
    currentFile: fileName(indexProgress.currentPath),
  };
}

export function getLoadingProgress(
  indexProgress: IndexProgressEvent | null,
  projectStatus: string | null,
): LoadingProgress {
  const statusProgress = fromStatus(projectStatus);

  if (indexProgress && (!statusProgress || statusProgress.percent < 55)) {
    return fromIndexProgress(indexProgress);
  }

  return (
    statusProgress ?? {
      percent: 5,
      percentLabel: '5%',
      detail: 'Project is loading...',
      currentFile: null,
    }
  );
}
