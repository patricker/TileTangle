import type {ReactNode} from 'react';
import clsx from 'clsx';
import Heading from '@theme/Heading';
import styles from './styles.module.css';
import LivePlaygroundPreview from '../homepage/LivePlaygroundPreview';
import LanguagesCodePreview from '../homepage/LanguagesCodePreview';
import UnicodeRulesPreview from '../homepage/UnicodeRulesPreview';

type FeatureItem = {
  title: string;
  visual: ReactNode;
  description: ReactNode;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'Live Playground',
    visual: <LivePlaygroundPreview />,
    description: (
      <>Place tiles in-browser. Toggle anchors and cross-check sets to visualize move generation. See the <a href="/docs/classic-demo">Classic Crossword Demo</a>.</>
    ),
  },
  {
    title: 'Languages & Bindings',
    visual: <LanguagesCodePreview />,
    description: (
      <>Rust core surfaced via WebAssembly and Python. Minimal, deterministic APIs for quick integration.</>
    ),
  },
  {
    title: 'Unicode & Rules',
    visual: <UnicodeRulesPreview />,
    description: (
      <>Unicode-aware tiles and configurable rules: dictionaries, reading direction, multi-grapheme tiles, and stacking.</>
    ),
  },
];

function Feature({title, visual, description}: FeatureItem) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center">{visual}</div>
      <div className="text--center padding-horiz--md">
        <Heading as="h3">{title}</Heading>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): ReactNode {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
