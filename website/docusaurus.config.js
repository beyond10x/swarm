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

  url: 'https://swarm.beyond10x.dev',
  baseUrl: '/',

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
