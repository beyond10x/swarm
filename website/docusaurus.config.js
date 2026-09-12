// @ts-check
import {themes as prismThemes} from 'prism-react-renderer';

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Swarm',
  tagline: 'A swarm manager, specified',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  // Served by GitHub Pages from this repository's own Actions workflow, so the site lives under the
  // project path. `swarm.beyond10x.dev` has no DNS record as of 2026-09-12 — `dig` returns nothing —
  // and pointing `url` at a name that does not resolve while `baseUrl` is `/` breaks every asset
  // path on the address the site is actually reachable at.
  //
  // When that record exists: set the custom domain in the repository's Pages settings, then change
  // these two back to `https://swarm.beyond10x.dev` and `/`.
  url: 'https://beyond10x.github.io',
  baseUrl: '/swarm/',

  organizationName: 'beyond10x',
  projectName: 'swarm',

  onBrokenLinks: 'throw',

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
      navbar: {
        title: 'Swarm',
        logo: {
          alt: 'Swarm logo',
          src: 'img/logo.svg',
        },
        items: [
          {to: '/', label: 'About', position: 'left'},
        ],
      },
      footer: {
        style: 'dark',
        links: [],
        copyright: `Swarm — an Executable System Specification, ${new Date().getFullYear()}.`,
      },
      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
      },
      colorMode: {
        defaultMode: 'dark',
        respectPrefersColorScheme: true,
      },
    }),
};

export default config;
