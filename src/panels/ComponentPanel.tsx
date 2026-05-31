import type { ComponentKind } from '../types/components';
import { COMP_DEF, CATEGORIES } from '../types/components';

const ICON: Record<string, string> = {
  and: 'AND', or: 'OR', not: 'NOT', nand: 'NAND', nor: 'NOR', xor: 'XOR', xnor: 'XNOR',
  switch: 'SW', led: 'LED', button: 'BTN', clock: 'CLK', random: 'RND', constant: 'C1',
  sevenSegment: '7SG', delayLine: 'DLY', splitter: 'SPL', merger: 'MRG',
};

export default function ComponentPanel({ onPick }: { onPick: (k: ComponentKind) => void }) {
  return (
    <div className="comp-panel">
      {Object.entries(CATEGORIES).map(([key, cat]) => (
        <div key={key}>
          <div className="comp-cat-label">{cat.label}</div>
          <div className="comp-grid">
            {cat.kinds.map((k) => (
              <div key={k} className="comp-item" onClick={() => onPick(k)}>
                <div className="comp-icon">{ICON[k] ?? COMP_DEF[k].label}</div>
                <div className="comp-name">{COMP_DEF[k].label}</div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
