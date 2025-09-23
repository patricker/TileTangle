import React from 'react';
import styles from '../PlaygroundLayout.module.css';
import {Panel, ToggleField} from './ui';
import type {SetupState} from './config';

const parseNumber = (value: string): number => Number(value);

export type BoardSetupPanelProps = {
  draft: SetupState;
  onDraftChange: (updates: Partial<SetupState>) => void;
  onApply: () => void;
};

export function BoardSetupPanel({draft, onDraftChange, onApply}: BoardSetupPanelProps): JSX.Element {
  return (
    <Panel
      title="Board Setup"
      subtitle="Adjust dimensions and masks, then relaunch with Apply."
      actions={
        <button type="button" className={styles.applyButton} onClick={onApply}>
          Apply configuration
        </button>
      }
      density="compact"
    >
      <div className={styles.fieldGrid}>
        <label className={styles.field}>
          <span>Width</span>
          <input
            type="number"
            min={2}
            max={30}
            value={draft.width}
            onChange={event => onDraftChange({width: parseNumber(event.target.value)})}
          />
        </label>
        <label className={styles.field}>
          <span>Height</span>
          <input
            type="number"
            min={2}
            max={30}
            value={draft.height}
            onChange={event => onDraftChange({height: parseNumber(event.target.value)})}
          />
        </label>
        <label className={styles.field} title="Number of layers when 3D mode is active">
          <span>Layers</span>
          <input
            type="number"
            min={1}
            max={12}
            value={draft.depth}
            onChange={event => onDraftChange({depth: parseNumber(event.target.value)})}
          />
        </label>
        <label className={styles.field}>
          <span>Rack size</span>
          <input
            type="number"
            min={1}
            max={50}
            value={draft.rackSize}
            onChange={event => onDraftChange({rackSize: parseNumber(event.target.value)})}
          />
        </label>
      </div>
      <label className={styles.field}>
        <span>Shape</span>
        <select
          value={draft.shape}
          data-testid="board-shape-select"
          onChange={event => onDraftChange({shape: event.target.value as SetupState['shape']})}
        >
          <option value="rect">Full grid</option>
          <option value="diamond">Diamond</option>
          <option value="cross">Cross</option>
          <option value="hexagon">Hexagon</option>
          <option value="triangle">Triangle</option>
          <option value="ring">Hollow ring</option>
        </select>
      </label>
      <label className={styles.field}>
        <span>Bonuses</span>
        <select
          value={draft.bonusPreset}
          onChange={event => onDraftChange({bonusPreset: event.target.value as SetupState['bonusPreset']})}
        >
          <option value="auto">Auto (match shape)</option>
          <option value="classic">Classic crossword</option>
          <option value="hex">Hex rings</option>
          <option value="triangle">Triangle bands</option>
          <option value="ring">Hollow frame</option>
          <option value="none">None</option>
        </select>
      </label>
      <div className={styles.helperText}>Layers only apply when 3D mode is enabled.</div>
      <div className={styles.helperText}>Auto picks a bonus layout tuned to the current shape.</div>
    </Panel>
  );
}

type LanguagePanelProps = {
  useDict: boolean;
  onUseDictChange: (value: boolean) => void;
  dictEngine: 'fst' | 'set' | 'dawg' | 'gaddag';
  onDictEngineChange: (value: 'fst' | 'set' | 'dawg' | 'gaddag') => void;
  useAnagram: boolean;
  onUseAnagramChange: (value: boolean) => void;
  rtl: boolean;
  onRtlChange: (value: boolean) => void;
  dictLoading: boolean;
};

export function LanguagePanel({
  useDict,
  onUseDictChange,
  dictEngine,
  onDictEngineChange,
  useAnagram,
  onUseAnagramChange,
  rtl,
  onRtlChange,
  dictLoading,
}: LanguagePanelProps): JSX.Element {
  return (
    <Panel title="Language & Dictionary" subtitle="Guard rails for move validation." density="compact">
      <ToggleField label="Use dictionary validation" checked={useDict} onChange={onUseDictChange} />
      <label className={styles.field}>
        <span>Engine</span>
        <select value={dictEngine} onChange={event => onDictEngineChange(event.target.value as LanguagePanelProps['dictEngine'])} disabled={!useDict || dictLoading}>
          <option value="fst">FST</option>
          <option value="set">Set</option>
          <option value="dawg">DAWG</option>
          <option value="gaddag">GADDAG</option>
        </select>
      </label>
      <ToggleField
        label="Anagram commit"
        checked={useAnagram}
        onChange={onUseAnagramChange}
        title="Reorder placed tiles into any valid anagram when committing the move"
      />
      <ToggleField label="RTL reading direction" checked={rtl} onChange={onRtlChange} />
      {dictLoading && (
        <span data-testid="dictionary-loading" className={styles.helperText}>
          Loading dictionary…
        </span>
      )}
    </Panel>
  );
}

export type StackingControlsProps = {
  stackOn: boolean;
  onStackOnChange: (value: boolean) => void;
  stackScoring: 'top' | 'sum';
  onStackScoringChange: (value: 'top' | 'sum') => void;
  forbidSame: boolean;
  onForbidSameChange: (value: boolean) => void;
};

export function StackingControls({
  stackOn,
  onStackOnChange,
  stackScoring,
  onStackScoringChange,
  forbidSame,
  onForbidSameChange,
}: StackingControlsProps): JSX.Element {
  return (
    <>
      <ToggleField label="Enable stacking" checked={stackOn} onChange={onStackOnChange} />
      {stackOn && (
        <div className={styles.fieldStack}>
          <label className={styles.field}>
            <span>Scoring</span>
            <select value={stackScoring} onChange={event => onStackScoringChange(event.target.value as 'top' | 'sum')}>
              <option value="top">Top only</option>
              <option value="sum">Sum stack</option>
            </select>
          </label>
          <ToggleField label="Forbid identical overlays" checked={forbidSame} onChange={onForbidSameChange} />
        </div>
      )}
    </>
  );
}

export function StackingPanel(props: StackingControlsProps): JSX.Element {
  return (
    <Panel title="Stacking Rules" subtitle="Experiment with layered tiles." density="compact">
      <StackingControls {...props} />
    </Panel>
  );
}
