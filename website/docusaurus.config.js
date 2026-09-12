// @ts-check
import {themes as prismThemes} from 'prism-react-renderer';
import docsSystemPlugin, {
  ecosystemFooterGroup,
  ecosystemNavbarItems,
} from '@beyond10x/docs-system/docusaurus';
import {PRISM_ADDITIONAL_LANGUAGES} from '@beyond10x/docs-system/code';

const organizationName = 'beyond10x';
const projectName = 'swarm';

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Swarm',
  tagline: 'A swarm manager, specified',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  // Served by GitHub Pages from this repository's own Actions workflow, so the site lives under the
  // project path. Verified against the live deployment: the site is reachable at
  // https://beyond10x.github.io/swarm/ and nowhere else.
  url: `https://${organizationName}.github.io`,
  baseUrl: `/${projectName}/`,

  organizationName,
  projectName,
  deploymentBranch: 'gh-pages',
  trailingSlash: false,

  onBrokenLinks: 'throw',

  // The shared beyond10x documentation system. The plugin injects
  // `@beyond10x/docs-system/styles/tokens.css` as a client module, which is where the palette,
  // the spacing scale, the focus ring and every `b10x-*` component style come from. Nothing in
  // `src/css/custom.css` may restate a token this file supplies.
  plugins: [docsSystemPlugin],

  // `@theme/Mermaid` is imported by `@beyond10x/docs-system/components`, so the theme has to be
  // present even though this site draws no Mermaid of its own yet.
  themes: ['@docusaurus/theme-mermaid'],
  markdown: {
    mermaid: true,
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: false,
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      image: 'img/social-card.png',
      colorMode: {
        defaultMode: 'light',
        respectPrefersColorScheme: true,
      },
      navbar: {
        title: 'Swarm',
        logo: {
          alt: 'Swarm',
          src: 'img/logo.svg',
        },
        items: [
          ...ecosystemNavbarItems(),
          {to: '/#specification', label: 'The specification', position: 'left'},
          {to: '/#loop', label: 'The loop', position: 'left'},
          {to: '/#status', label: 'What is true today', position: 'left'},
          {
            href: `https://github.com/${organizationName}/${projectName}`,
            label: 'GitHub',
            position: 'right',
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          ecosystemFooterGroup(),
          {
            title: 'Swarm',
            items: [
              {label: 'The specification', to: '/#specification'},
              {label: 'The runtime', to: '/#runtime'},
              {label: 'The loop', to: '/#loop'},
              {label: 'What is true today', to: '/#status'},
            ],
          },
          {
            title: 'Source',
            items: [
              {
                label: 'github.com/beyond10x/swarm',
                href: `https://github.com/${organizationName}/${projectName}`,
              },
              {
                label: 'The kernel: src/core/',
                href: `https://github.com/${organizationName}/${projectName}/tree/main/src/core`,
              },
              {
                label: 'Issues',
                href: `https://github.com/${organizationName}/${projectName}/issues`,
              },
            ],
          },
          {
            title: 'Built on',
            items: [
              {label: 'ESS', href: 'https://beyond10x.github.io/ess/'},
              {label: 'metaharness', href: 'https://beyond10x.github.io/metaharness/'},
            ],
          },
        ],
        copyright: `Swarm — a swarm manager whose kernel is an Executable System Specification. ${new Date().getFullYear()}.`,
      },
      mermaid: {
        theme: {light: 'neutral', dark: 'dark'},
      },
      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
        additionalLanguages: [...PRISM_ADDITIONAL_LANGUAGES],
      },
    }),
};

export default config;
