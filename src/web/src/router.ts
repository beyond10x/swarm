import { createRouter, createWebHistory } from 'vue-router'

// Route table owned by the integrator. View files are owned as noted; owners overwrite their file
// and never this table.
export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'splash', component: () => import('./views/SplashView.vue') },                       
    { path: '/swarm/:id', name: 'swarm', component: () => import('./views/SwarmView.vue'), props: true }, // one swarm: its canvas and its instances
    { path: '/ui', name: 'ui', component: () => import('./views/ComponentsView.vue') },                   // the component library
  ],
})
