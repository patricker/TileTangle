import type {ReactNode} from 'react';
import clsx from 'clsx';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  Svg: React.ComponentType<React.ComponentProps<'svg'>>;
  description: ReactNode;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'Live Playground',
    Svg: require('@site/static/img/undraw_docusaurus_mountain.svg').default,
    description: (
      <>Try the engine in-browser via WASM. Place tiles, inspect board JSON, and iterate quickly. See the <a href="/docs/classic-demo">Classic Crossword Demo</a>.</>
    ),
  },
  {
    title: 'Rust Core + WASM',
    Svg: require('@site/static/img/undraw_docusaurus_tree.svg').default,
    description: (
      <>Deterministic Rust engine, packaged for web with wasm-bindgen. Clean API for JS and other bindings.</>
    ),
  },
  {
    title: 'Unicode & Rules',
    Svg: require('@site/static/img/undraw_docusaurus_react.svg').default,
    description: (
      <>Unicode-first dictionary, configurable boards/bonuses, and classic crossword validation & scoring.</>
    ),
  },
];

function Feature({title, Svg, description}: FeatureItem) {
  return (
    <div className={clsx('col col--4')}>
      <div className="text--center">
        <Svg className={styles.featureSvg} role="img" />
      </div>
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
